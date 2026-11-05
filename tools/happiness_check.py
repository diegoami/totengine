#!/usr/bin/env python3
"""Check happiness arithmetic against isolated original x86 city-totals stages.

Garrison count, unit unrest and luxury output are explicit inputs: this probe
does not claim to verify how those inputs are obtained or a complete turn.
CIVTOT_HOME required; Unicorn installed outside the repo.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct

from research_check import OriginalCost, trunc
from connections_check import OriginalConnection, saved_model, synthetic_cases
from totctl import parse_rules, walk_save


def happiness(v):
    index, size, government, difficulty, human, cities, content, empire, flags, specialists, luxuries, garrison, away, home, buildings, techs, own, local = v
    has = lambda i: bool(buildings & (1 << i))
    knows = lambda i: bool(techs & (1 << i))
    holds = lambda i: bool(own & (1 << i))
    here = lambda i: bool(local & (1 << i))
    happy = 0
    if human:
        factor = max(1, trunc((empire - 2 * difficulty + (2 if flags & 4 else 0)) * ((government >> 1) + 2), 2))
        initial = content - difficulty
        if initial <= 2 and has(1) and away == 0 and home == 0:
            initial = 2
        unhappy = size + 1 - initial
        if government != 3:
            unhappy += trunc(cities - factor + index % factor, factor)
    else:
        unhappy = size + 4 - content
    angry = max(0, unhappy - size)
    unhappy = min(size, unhappy)

    def normalize(happy, unhappy, angry):
        happy = max(0, min(size, happy))
        while angry != 0 and angry > unhappy:
            angry -= 1
            unhappy += 1
        unhappy = max(0, min(size, unhappy))
        while max(0, min(99, size - specialists)) < happy + unhappy:
            if angry:
                angry -= 1
            else:
                happy = max(0, happy - 1)
            unhappy = max(0, unhappy - 1)
        while angry and size - specialists > happy + unhappy:
            angry -= 1
            unhappy += 1
        return happy, unhappy, angry

    happy, unhappy, angry = normalize(happy, unhappy, angry)
    happy = luxuries >> 1
    happy, unhappy, angry = normalize(happy, unhappy, angry)
    if has(14):
        unhappy -= 3 + knows(24)
    if knows(55) and (has(11) or holds(10)):
        unhappy -= 3 - knows(15) + knows(82)
    if has(4):
        unhappy -= (knows(56) + knows(9)) * (2 if holds(5) else 1)
    if (has(7) or has(1)) and government == 6:
        happy += 1
    happy, unhappy, angry = normalize(happy, unhappy, angry)
    if government == 4:
        unhappy = angry = 0
    elif government < 5:
        unhappy -= max(0, min(unhappy, min(garrison * (2 if government == 3 else 1), 6 if government == 3 else 3)))
    else:
        penalty = int(not (holds(21) or has(33))) + int(government == 6)
        unhappy += max(0, away - int(government == 5 and away != 0)) * penalty
    happy, unhappy, angry = normalize(happy, unhappy, angry)
    if holds(1):
        happy += 1 + 2 * here(1)
    if holds(27):
        happy += 1
    if here(13):
        unhappy = 0
    if holds(15):
        unhappy -= 2
    return normalize(happy, unhappy, angry)


class OriginalHappiness(OriginalCost):
    def run(self, v):
        from unicorn.x86_const import UC_X86_REG_EBP
        index, size, government, difficulty, human, cities, content, empire, flags, specialists, luxuries, garrison, away, home, buildings, techs, own, local = v
        frame = 0x108100
        self.write(frame + 8, index, '<I')
        self.write(frame - 0x28, 1, '<I')
        self.write(0x5fd15a + 92 * index, 1, '<B')
        self.write(0x5fd15b + 92 * index, size, '<b')
        self.cpu.mem_write(0x5fd186 + 92 * index, buildings.to_bytes(5, 'little'))
        self.write(0x5fd15a + 92 * 200, 1, '<B')
        for address, value, fmt in (
                (0x60765b, 2 if human else 0, '<B'), (0x607658, difficulty, '<B'),
                (0x607640, flags, '<H'), (0x5f5ebf, content, '<B'), (0x5f5ec0, empire, '<B'),
                (0x5f68bd + 3348, government, '<B'), (0x5f6910 + 3348, cities, '<h'),
                (0x6537cc, specialists, '<i'), (0x65387c, luxuries, '<i'),
                (0x653864, away, '<i'), (0x6537dc, home, '<i')):
            self.write(address, value, fmt)
        self.cpu.mem_write(0x5f5c18, bytes([255] * 28))
        self.cpu.mem_write(0x5f6900 + 3348, techs.to_bytes(13, 'little'))
        for i in range(28):
            self.write(0x607736 + 2 * i, index if local & (1 << i) else 200 if own & (1 << i) else -1, '<h')
        self.cpu.reg_write(UC_X86_REG_EBP, frame)
        self.cpu.reg_write(self.sp, frame - 0x100)
        def execute(start, stop):
            self.cpu.emu_start(start, stop, count=100000)
            assert self.cpu.reg_read(self.ip) == stop, hex(self.cpu.reg_read(self.ip))
        if government < 4:
            execute(0x4c4925, 0x4c4d24)
            self.write(0x6537e4, min(garrison * (2 if government == 3 else 1), 6 if government == 3 else 3), '<i')
            execute(0x4c4e28, 0x4c4fa9)
        else:
            execute(0x4c4925, 0x4c4fa9)
        return tuple(struct.unpack('<i', self.cpu.mem_read(a, 4))[0] for a in (0x6537d0, 0x653828, 0x65381c))


def saved_cases(home, binary, name):
    """Run original support, finance and happiness on current saved inputs.

    Tile assignment/trade are supplied from the stored records. This is
    deliberately independent of tile-yield parity and complete turn order.
    """
    from unicorn.x86_const import UC_X86_REG_EBP
    path = home / name
    rules_path = home / Path(name).parent / 'Rules.txt'
    model, cities = saved_model(path, rules_path)
    data = path.read_bytes()
    walk = walk_save(str(path))
    rules = parse_rules(str(rules_path))
    original = OriginalConnection(binary)
    original.load(model)
    count = walk['units']['count']
    start = walk['cities']['offset'] - 40 * count
    original.cpu.mem_write(0x607c58, data[start:start + 40 * count])
    original.cpu.mem_write(0x5fd150, b''.join(cities))
    original.cpu.mem_write(0x5f68a0, data[2944:29728])
    original.cpu.mem_write(0x607638, data[652:1250])
    original.write(0x5de648, 0, '<i')
    original.write(0x5d75a8, 0, '<i')
    for i, row in enumerate(rules['UNITS']['rows'][:80]):
        if len(row) > 11:
            original.write(0x5f4709 + 40 * i, int(row[2]), '<b')
            original.write(0x5f470c + 40 * i, int(row[5].rstrip('aA')), '<b')
            original.write(0x5f4712 + 40 * i, int(row[11]), '<b')
    for i, row in enumerate(rules['COSMIC']['rows']):
        if row and row[0].lstrip('-').isdigit():
            original.write(0x5f5eb8 + i, int(row[0]) & 255, '<B')
    ids = {s: i for i, s in enumerate(rules['CIVILIZE']['comments'])}
    for i, row in enumerate(rules['ENDWONDER']['rows'][:28]):
        original.write(0x5f5c18 + i, ids.get(row[0], -1), '<b')
    vectors, mismatches = [], []
    def call(address, *args):
        original.cpu.mem_write(0x108000, struct.pack('<' + 'I' * (len(args) + 1), 0x100000, *args))
        original.cpu.reg_write(original.sp, 0x108000)
        original.cpu.emu_start(address, 0x100000, count=1_000_000)
        assert original.cpu.reg_read(original.ip) == 0x100000
    for i, city in enumerate(cities):
        # The examined saved cities use governments 0/1/2. Fundamentalist
        # free-type flags and scenario-specific air exceptions are outside
        # this saved-input check, rather than guessed table fields.
        assert data[2944 + 3348 * city[10] + 29] <= 2
        call(0x4c1bdf, i)
        original.write(0x65384c, city[83], '<i')
        call(0x4c3f0c, i, struct.unpack_from('<H', city, 80)[0], 0, 0)
        science, taxes, luxuries = (struct.unpack('<i', original.cpu.mem_read(a, 4))[0]
                                   for a in (0x6537f8, 0x6537d4, 0x65387c))
        assert (science, taxes) == struct.unpack_from('<HH', city, 76), (name, i, science, taxes)
        specialist_bits = struct.unpack_from('<I', city, 24)[0]
        specialists = sum(specialist_bits >> (2 * k) & 3 != 0 for k in range(16))
        original.write(0x6537cc, specialists, '<i')
        frame = 0x108100
        original.write(frame + 8, i, '<I')
        original.write(frame - 0x28, city[10], '<I')
        original.cpu.reg_write(UC_X86_REG_EBP, frame)
        original.cpu.reg_write(original.sp, frame - 0x100)
        original.cpu.emu_start(0x4c4925, 0x4c4fa9, count=1_000_000)
        assert original.cpu.reg_read(original.ip) == 0x4c4fa9
        value = tuple(struct.unpack('<i', original.cpu.mem_read(a, 4))[0]
                      for a in (0x6537d0, 0x653828, 0x65381c))
        vectors.append((name, i, luxuries, *value))
        if value[:2] != (city[84], city[85]):
            mismatches.append({'city_index': i, 'stored': [city[84], city[85]], 'original_recomputed': list(value[:2])})
    return vectors, {'save': name, 'original_x86_cities': len(cities),
                     'stored_happiness_exact': len(cities) - len(mismatches),
                     'stored_discrepancies': mismatches,
                     'saved_city_counts': [struct.unpack_from('<h', data, 2944 + 3348 * c + 112)[0] for c in range(8)]}


def military_cases(binary):
    specs = [('home', 8, 8, 0, 0, False, 0, 1),
             ('own-city', 16, 8, 0, 0, False, 0, 0),
             ('foreign-city', 28, 8, 0, 0, False, 0, 0),
             ('open', 20, 8, 0, 0, False, 1, 0),
             ('near-fortress', 14, 8, 0, 0, True, 0, 0),
             ('far-fortress', 40, 8, 0, 0, True, 1, 0),
             ('foreign-fortress', 26, 8, 0, 0, True, 1, 0),
             ('air-home', 8, 8, 1, 0, False, 1, 1),
             ('air-role3-home', 8, 8, 1, 3, False, 0, 1),
             ('air-role3-away', 20, 8, 1, 3, False, 0, 0)]
    results = []
    for name, x, y, domain, role, fortress, away, home in specs:
        model = next(synthetic_cases())[2]
        model.tiles = bytearray(model.tiles)
        cities = bytearray(3 * 92)
        for i, (cx, cy, owner) in enumerate(((8, 8, 1), (16, 8, 1), (28, 8, 2))):
            struct.pack_into('<hh', cities, 92 * i, cx, cy)
            cities[92 * i + 10] = owner
            cities[92 * i + 11] = 6
            struct.pack_into('<I', cities, 92 * i + 86, i + 1)
            tile = 6 * (cy * 32 + cx // 2)
            model.tiles[tile + 1] |= 2
            model.tiles[tile + 5] = owner << 4
        tile = 6 * (y * 32 + x // 2)
        model.tiles[tile + 1] |= 1 | (64 if fortress else 0)
        model.tiles[tile + 5] = 16
        o = OriginalConnection(binary)
        o.load(model)
        o.write(0x5de648, 0, '<i')
        o.cpu.mem_write(0x5fd150, bytes(cities))
        unit = bytearray(40)
        struct.pack_into('<hh', unit, 0, x, y)
        unit[9] = 1
        struct.pack_into('<hhI', unit, 26, -1, -1, 1)
        o.cpu.mem_write(0x607c58, bytes(unit))
        o.write(0x607666, 1, '<h')
        o.write(0x607668, 3, '<h')
        o.cpu.mem_write(0x5f4700, bytes(40))
        o.write(0x5f4709, domain, '<b')
        o.write(0x5f470c, 1, '<b')
        o.write(0x5f4712, role, '<b')
        o.write(0x5f68bd + 3348, 6, '<B')
        o.write(0x607782, 0, '<B')
        o.write(0x60765b, 2, '<B')
        o.write(0x607648, 10, '<h')
        o.cpu.mem_write(0x108000, struct.pack('<2I', 0x100000, 0))
        o.cpu.reg_write(o.sp, 0x108000)
        o.cpu.emu_start(0x4c1bdf, 0x100000, count=1_000_000)
        assert o.cpu.reg_read(o.ip) == 0x100000
        actual = tuple(struct.unpack('<i', o.cpu.mem_read(a, 4))[0] for a in (0x653864, 0x6537dc))
        assert actual == (away, home), (name, actual)
        results.append({'case': name, 'unit_unrest': away, 'home_armed': home})
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--vectors', type=Path)
    parser.add_argument('--saved-vectors', type=Path)
    args = parser.parse_args()
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    original = OriginalHappiness(binary)
    rng = random.Random(86)
    cases = []
    for _ in range(1000):
        size = rng.randrange(1, 40)
        local = rng.getrandbits(28)
        cases.append((rng.randrange(20), size, rng.randrange(7), rng.randrange(6),
                      rng.randrange(2), rng.randrange(1, 80), rng.choice((5, 7, 9)), 14,
                      rng.choice((0, 4)), rng.randrange(min(size, 16) + 1), rng.randrange(80),
                      rng.randrange(8), rng.randrange(8), rng.randrange(8),
                      sum(1 << i for i in (1, 4, 7, 11, 14, 33) if rng.randrange(2)),
                      rng.getrandbits(100), rng.getrandbits(28) | local, local))
    for v in cases:
        expected = original.run(v)
        assert happiness(v) == expected, (v, happiness(v), expected)
    if args.vectors:
        args.vectors.write_text('# index size government difficulty human cities content empire flags specialists luxuries garrison away home buildings techs own_wonders local_wonders happy unhappy angry\n'
                               + ''.join(' '.join(map(str, (*v, *happiness(v)))) + '\n' for v in cases[:128]))
    saved_vectors, saved = [], []
    for name in ('Original/Ph_Auto2.SAV', 'FallRome/ROMFALL.scn'):
        vectors, check = saved_cases(home, binary, name)
        saved_vectors.extend(vectors)
        saved.append(check)
    if args.saved_vectors:
        args.saved_vectors.write_text('# save city_index luxuries happy unhappy angry\n'
                                      + ''.join(' '.join(map(str, v)) + '\n' for v in saved_vectors))
    print(json.dumps({'source': 'civ2.bak 0x4c4925..0x4c4fa9; synthetic inputs exclude garrison lookup, saved cases execute it',
                      'source_sha256': hashlib.sha256(binary).hexdigest(),
                      'scope': 'happiness arithmetic; luxury/garrison/unit unrest inputs supplied; active-wonder input; no full-turn claim',
                      'original_x86_cases': len(cases), 'rust_vectors': 128,
                      'military_input_cases': military_cases(binary),
                      'saved_inputs': saved}, indent=2))


if __name__ == '__main__':
    main()
