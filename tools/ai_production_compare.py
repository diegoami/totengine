#!/usr/bin/env python3
"""Compare engine build intentions with complete original city AI selection.

Requires CIVTOT_HOME, external Unicorn, and a built ai_build_choices example.
Reports derived numeric choices only. This is prepared saved-input comparison,
not original full-turn/campaign parity. Source files are never modified.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess

from yields_check import OriginalCity
from ai_research_adapter_check import prepare
from ai_research_value_check import OriginalValue


def load(home, binary, name):
    o = OriginalCity(binary)
    data, walk, rules, cities = o.load_saved(home, name)
    o.call = OriginalValue.call.__get__(o)
    provenance, _ = prepare(home, o, name)
    o.cpu.mem_map(0, 4096)
    o.write(0x6031b8, 1, '<h')
    o.write(0x6762f8, 0, '<i')
    o.write(0x607652, 0, '<B')
    advanced = rules.get('UNITS_ADVANCED', {}).get('rows', [])
    def reference(token):
        o.cpu.mem_write(0x107000, token.encode('ascii')+b'\0')
        return o.call(0x490440, 0x107000)
    for i, row in enumerate(rules['UNITS']['rows'][:80]):
        if len(row) < 13:
            continue
        for offset, field in ((9, 2), (10, 3), (11, 4), (12, 5), (13, 6),
                              (14, 7), (15, 8), (16, 9), (17, 10), (18, 11)):
            value = int(row[field].rstrip('.aAdDhHfF'))
            if offset == 10:
                value *= int(rules['COSMIC']['rows'][0][0])
            if offset == 14:
                value *= 10
            if offset == 16:
                value = max(1, min(200, value))
            o.write(0x5f4700+40*i+offset, value & 255, '<B')
        o.write(0x5f4704+40*i, int(row[13], 2), '<I')
        o.write(0x5f4708+40*i, reference(row[1]), '<b')
        o.write(0x5f4714+40*i, int(advanced[i][0], 2) if i < len(advanced) else 255, '<B')
    for i, row in enumerate(rules['IMPROVE']['rows'][:68]):
        o.write(0x5f6684+8*i, int(row[1]) & 255, '<B')
        o.write(0x5f6685+8*i, int(row[2]) & 255, '<B')
    return o, provenance, cities


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine-example', type=Path, required=True)
    parser.add_argument('--holdout-root', type=Path, required=True)
    parser.add_argument('--vectors', type=Path, required=True)
    args = parser.parse_args()
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home/'civ2.bak').read_bytes()
    sources = [('standard', home, 'Original/Ph_Auto2.SAV', 'calibration'),
               ('fallrome', home, 'FallRome/ROMFALL.scn', 'holdout')]
    sources += [(f'std1-{name}', args.holdout_root, name, 'holdout')
                for name in ('turn01.sav', 'turn10.sav', 'final_Ph_Auto.SAV')]
    reports, rows = [], ['# case difficulty city native ai_v1 builtin final_NR final_CRT']
    for label, root, name, partition in sources:
        native, source, cities = load(root, binary, name)
        baseline = bytes(native.cpu.mem_read(0x400000, 0x400000))
        matches = {'ai-v1': 0, 'builtin': 0}
        total = 0
        for difficulty in range(6):
            output = subprocess.check_output([str(args.engine_example.resolve()), str(root/name), str(difficulty)], text=True)
            choices = {}
            for line in output.splitlines():
                policy, city, _civ, _size, item = line.split()
                choices[policy, int(city)] = int(item)
            for i, city in enumerate(cities):
                if not struct.unpack_from('<I', city, 86)[0] or not 1 <= city[10] <= 7:
                    continue
                native.cpu.mem_write(0x400000, baseline)
                native.write(0x607658, difficulty, '<B')
                native.write(0x5dd53c, 106, '<I')
                native.write(0x5e2590, 106, '<I')
                item = native.call(0x47e26b, i, 0, 0)
                states = [struct.unpack('<I', native.cpu.mem_read(a, 4))[0] for a in (0x5dd53c, 0x5e2590)]
                a, b = choices['ai-v1', i], choices['builtin', i]
                matches['ai-v1'] += item == a
                matches['builtin'] += item == b
                rows.append(f'{label} {difficulty} {i} {item} {a} {b} '+ ' '.join(map(str, states)))
                total += 1
        source['save'] = label
        source.update({'partition': partition, 'choices': total, 'exact_matches': matches})
        reports.append(source)
    args.vectors.write_text('\n'.join(rows)+'\n')
    print(json.dumps({'source_sha256': hashlib.sha256(binary).hexdigest(),
                      'routine': '0x47e26b', 'replacement_callees': [], 'sources': reports,
                      'scope': 'Original complete build selector on prepared saved inputs; per-case NR/CRT seed 106, '
                               'city accounting collection off and presentation context quiet, source mechanics mask retained. '
                               'Engine choices use own observations and private generators. Exact agreement is descriptive, '
                               'not full original AI parity or a statistical acceptance gate.'}, indent=2))


if __name__ == '__main__':
    main()
