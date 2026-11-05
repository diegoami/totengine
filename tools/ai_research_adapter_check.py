#!/usr/bin/env python3
"""Native saved-input research comparisons; no game files are modified.

Valuation/selection execute all original callees. A separate original leader
loader check replaces only token readers (not field storage or arithmetic).
Knowledge masks/naval counts are refreshed explicitly; other caches are saved
inputs. Custom saved human leader overrides are not loaded.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct

from ai_research_value_check import OriginalValue
from totctl import parse_rules, walk_save


def leader_loader(native, rows):
    from unicorn import UC_HOOK_CODE
    section, row, column = 0, -1, 0
    reads = []

    def tokens(cpu, address, _size, _data):
        nonlocal section, row, column
        if address not in (0x486659, 0x4866dc, 0x486a4a, 0x4868bf, 0x486935):
            return
        sp = cpu.reg_read(native.sp)
        ret = struct.unpack('<I', cpu.mem_read(sp, 4))[0]
        result = 0
        if address == 0x486659:
            section += 1
            row, column = -1, 0
        elif address == 0x4866dc:
            row, column = row + 1, 0
        else:
            if section == 2 and column < len(rows[row]):
                field = rows[row][column].strip()
                if address == 0x4868bf:
                    result = int(field)
                    reads.append((row, column, ret))
            column += 1
        cpu.reg_write(native.ax, result & 0xffffffff)
        cpu.reg_write(native.sp, sp + 4)
        cpu.reg_write(native.ip, ret)

    hook = native.cpu.hook_add(UC_HOOK_CODE, tokens)
    try:
        native.call(0x41447d, 1)
    finally:
        native.cpu.hook_del(hook)
    actual = [struct.unpack('<b', native.cpu.mem_read(0x606f4a + 60*i, 1))[0]
              for i in range(21)]
    assert actual == [int(row[9]) for row in rows[:21]]
    assert len([r for r in reads if r[1] == 9 and r[2] == 0x41487d]) == 21
    return actual


def prepare(home, native, name):
    source = home / name
    rules_path = source.parent / 'Rules.txt'
    data = source.read_bytes()
    rules = parse_rules(str(rules_path))
    walk = walk_save(str(source))
    native.cpu.mem_write(0x607638, data[652:1250])
    native.cpu.mem_write(0x5f68a0, data[2944:29728])
    start, count = walk['cities']['offset'], walk['cities']['count']
    native.cpu.mem_write(0x5fd150, data[start:start+92*count])
    # Existing save framing locates scenario parameters after the city table.
    scenario = 0
    if data[660] & 128:
        humans = 1 if data[678] == 0 else data[687].bit_count()
        scenario = struct.unpack_from('<H', data, start+92*count+1453+60*humans)[0]
    native.write(0x5f5e50, scenario, '<H')
    ids = {a.strip().lstrip('*').lower(): i for i, a in enumerate(rules['CIVILIZE']['comments']) if a.strip()}
    refs_verified = set()
    def ref(token):
        native.cpu.mem_write(0x107000, token.encode('ascii')+b'\0')
        actual = native.call(0x490440, 0x107000)
        expected = -1 if token.lower() == 'nil' else -2 if token.lower() == 'no' else ids[token.lower()]
        assert actual == expected, (token, actual, expected)
        refs_verified.add(token)
        return actual
    for i, row in enumerate(rules['CIVILIZE']['rows'][:100]):
        native.cpu.mem_write(0x5d0a26+20*i, struct.pack('<bb', int(row[1]), int(row[2])))
        native.cpu.mem_write(0x5d0a2a+20*i, struct.pack('<bb', ref(row[3]), ref(row[4])))
        native.write(0x5d0a25+20*i, int(row[3] != 'no'), '<B')
        groups = rules.get('CIVILIZE2', {}).get('rows', [])
        native.write(0x5d0a1c+20*i, int(groups[i][0]) if i < len(groups) else -1, '<i')
    for i in range(80):
        row = rules['UNITS']['rows'][i] if i < len(rules['UNITS']['rows']) else []
        native.write(0x5f4713+40*i, ref(row[12]) if len(row) >= 13 else -2, '<b')
    for i, row in enumerate(rules['IMPROVE']['rows'][:68]):
        native.write(0x5f6686+8*i, ref(row[3]), '<b')
    for i, row in enumerate(rules['ENDWONDER']['rows'][:28]):
        native.write(0x5f5c18+i, ref(row[0]), '<b')
    for i in range(2):
        native.write(0x5f5ec1+i, int(rules['COSMIC']['rows'][9+i][0]), '<B')
    modifiers = leader_loader(native, rules['LEADERS']['rows'])
    for i in range(21):
        groups = rules.get('LEADERS2', {}).get('rows', [])
        flags = [int(v) for v in groups[i][0].strip()[:8]] if i < len(groups) else [0]*8
        native.cpu.mem_write(0x606f78+60*i, bytes(flags))
    masks = [sum((1 << c) for c in range(8)
                 if data[2944+3348*c+96+i//8] & (1 << (i % 8))) for i in range(100)]
    native.cpu.mem_write(0x6076d2, bytes(masks))
    sea = [0]*8
    units_start = walk['units']['region'][0]
    for i in range(walk['units']['count']):
        offset = units_start+40*i
        owner, typ = data[offset+9] & 15, data[offset+8]
        live = struct.unpack_from('<I', data, offset+30)[0]
        if live and owner < 8 and int(rules['UNITS']['rows'][typ][2]) == 2:
            sea[owner] += 1
    saved_sea = [struct.unpack_from('<h', data, 2944+3348*c+114)[0] for c in range(8)]
    for c in range(8):
        native.write(0x5f6912+3348*c, sea[c], '<h')
    provenance = {'save': name, 'save_sha256': hashlib.sha256(data).hexdigest(),
                  'rules_sha256': hashlib.sha256(rules_path.read_bytes()).hexdigest(),
                  'saved_sea_counts': saved_sea, 'refreshed_sea_counts': sea,
                  'leader_modifiers_verified': len(modifiers),
                  'reference_tokens_verified': len(refs_verified)}
    return provenance, sea


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--vectors', type=Path)
    args = parser.parse_args()
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home/'civ2.bak').read_bytes()
    native = OriginalValue(binary)
    rows = ['# save civ sea_count | 100 native values | 100 eligible bits | '
            'six [seed difficulty winner final_crt] groups']
    sources = []
    for name in ('Original/Ph_Auto2.SAV', 'FallRome/ROMFALL.scn'):
        provenance, sea = prepare(home, native, name)
        sources.append(provenance)
        for civ in range(1, 8):
            scores = [native.call(0x49c95d, civ, i) for i in range(100)]
            eligible = []
            for i in range(100):
                prereqs = bytes(native.cpu.mem_read(0x5d0a2a+20*i, 2))
                eligible.append(int(bool(native.call(0x49ecb6, civ, i)
                                         and native.call(0x4a2098, civ, i)
                                         and prereqs != b'\xfe\xfe')))
            choices = []
            for difficulty in range(6):
                seed = 106000 + civ*6 + difficulty
                native.write(0x5e2590, seed, '<I')
                native.write(0x607658, difficulty, '<B')
                winner = native.call(0x49f98b, civ)
                final = struct.unpack('<I', native.cpu.mem_read(0x5e2590, 4))[0]
                choices += [seed, difficulty, winner, final]
            rows.append(f'{name} {civ} {sea[civ]} | ' + ' | '.join(
                ' '.join(map(str, v)) for v in (scores, eligible, choices)))
    if args.vectors:
        args.vectors.write_text('\n'.join(rows)+'\n')
    print(json.dumps({'source_sha256': hashlib.sha256(binary).hexdigest(),
                      'sources': sources, 'valuations': 1400, 'selections': 84,
                      'scoring_replacement_callees': [],
                      'leader_loader': {'routine': '0x41447d', 'fields_verified': 42,
                                        'replacement_callees': ['0x486659 section', '0x4866dc row',
                                            '0x486a4a string token', '0x4868bf numeric token',
                                            '0x486935 title token']},
                      'scope': 'saved inputs, Rules leader traits, refreshed ownership/naval caches; '
                               'no custom saved human leader overrides or live AI parity'}, indent=2))


if __name__ == '__main__':
    main()
