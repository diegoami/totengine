#!/usr/bin/env python3
"""Execute complete original advance valuation and research selection.

All callees execute unmodified original bytes, including dependency traversal,
knowledge, wonder ownership/expiry and CRT. Synthetic contexts are numeric,
authored inputs, not saved game assets. Requires external Unicorn/CIVTOT_HOME.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct

from research_check import OriginalCost


def context(rng, case):
    civ = 1 + case % 7
    human = (1 << civ) if case % 2 else 0
    human |= 1 << (1 + (civ % 7))
    flags, scenario = rng.choice((0, 4, 128, 132)), rng.randrange(2)
    leader = (case // 2) % 23 - 1
    trait = rng.choice((-128, -1, 0, 1, 2, 127))
    sea, space = rng.randrange(-12, 100), rng.randrange(256)
    values = [civ, human, flags, scenario, leader, trait, sea, space]
    # Acyclic tree, with all direct/indirect dependency depths represented.
    for i in range(100):
        prerequisites = [rng.randrange(-2, i) if i else -1 for _ in range(2)]
        values += [rng.randrange(-10, 25), rng.randrange(-5, 6), *prerequisites]
    values += [rng.randrange(-2, 100) for _ in range(80)]
    values += [rng.randrange(-2, 100) for _ in range(68)]
    values += [rng.randrange(-2, 100) for _ in range(28)]
    values += [rng.randrange(4, 16), rng.randrange(16, 25)]
    values += [rng.randrange(100) for _ in range(8)]
    values += [rng.randrange(10) for _ in range(8)]
    values += [rng.choice((0, 32, 8192, 8196)) for _ in range(8)]
    # Four physical landmass indices, including the ignored sentinel 255.
    values += [rng.randrange(2) for _ in range(32)]
    # Sparse global knowledge allows both active and expired wonders.
    masks = [rng.choice((0, 0, 0, 0, 2, 4, 8, 16, 32, 64, 128)) for _ in range(100)]
    values[7] = masks[76]
    values += masks
    values += [8]
    for owner in range(8):
        values += [owner, rng.randrange(-2, 30), rng.randrange(2)]
    values += [rng.randrange(-2, 8) for _ in range(28)]
    return values


class OriginalValue(OriginalCost):
    def load(self, values):
        data = iter(values)
        take = lambda n: [next(data) for _ in range(n)]
        civ, human, flags, scenario, leader, trait, sea, space = take(8)
        self.civ = civ
        self.write(0x60765b, human, '<B')
        self.write(0x607640, flags, '<H')
        self.write(0x5f5e50, scenario, '<H')
        self.write(0x5f68ae + civ * 3348, leader, '<h')
        self.write(0x606f4a + leader * 60, trait, '<b')
        self.write(0x5f6912 + civ * 3348, sea, '<h')
        for i in range(100):
            base, modifier, p1, p2 = take(4)
            self.cpu.mem_write(0x5d0a26 + i * 20, struct.pack('<bb', base, modifier))
            self.cpu.mem_write(0x5d0a2a + i * 20, struct.pack('<bb', p1, p2))
            self.write(0x5d0a25 + i * 20, 1, '<B')
            self.write(0x5d0a1c + i * 20, 0, '<i')
        for i, value in enumerate(take(80)):
            self.write(0x5f4713 + i * 40, value, '<b')
        for i, value in enumerate(take(68)):
            self.write(0x5f6686 + i * 8, value, '<b')
        self.cpu.mem_write(0x5f5c18, struct.pack('<28b', *take(28)))
        self.cpu.mem_write(0x5f5ec1, bytes(take(2)))
        for c, value in enumerate(take(8)):
            self.write(0x5f68b8 + c * 3348, value, '<B')
        for c, value in enumerate(take(8)):
            self.write(0x5f68c6 + c * 3348, value, '<B')
        for c, value in enumerate(take(8)):
            self.write(0x5f68c8 + civ * 3348 + c * 4, value, '<I')
        for c in range(8):
            self.cpu.mem_write(0x5f6e70 + c * 3348, bytes(256))
            for land, value in zip((1, 2, 254, 255), take(4)):
                self.write(0x5f6e70 + c * 3348 + land, value, '<B')
        masks = take(100)
        self.cpu.mem_write(0x6076d2, bytes(masks))
        self.write(0x60771e, space, '<B')
        for c in range(8):
            packed = bytearray(13)
            for i, mask in enumerate(masks):
                if mask & (1 << c):
                    packed[i // 8] |= 1 << (i % 8)
            self.cpu.mem_write(0x5f6900 + c * 3348, bytes(packed))
        cities = next(data)
        self.write(0x607668, cities, '<h')
        for i in range(cities):
            owner, size, live = take(3)
            self.write(0x5fd15a + i * 92, owner, '<B')
            self.write(0x5fd15b + i * 92, size, '<b')
            self.write(0x5fd1a6 + i * 92, live, '<I')
        for i, value in enumerate(take(28)):
            self.write(0x607736 + i * 2, value, '<h')
        assert next(data, None) is None
        # Group 0 admits all these contexts; clear leader technology-group flags.
        self.cpu.mem_write(0x606f78 + leader * 60, bytes(8))

    def call(self, entry, *args):
        self.cpu.mem_write(0x108000, struct.pack('<' + 'I' * (len(args) + 1),
                                               0x100000, *args))
        self.cpu.reg_write(self.sp, 0x108000)
        self.cpu.emu_start(entry, 0x100000, count=2000000)
        assert self.cpu.reg_read(self.ip) == 0x100000
        result = self.cpu.reg_read(self.ax)
        return result if result < 0x80000000 else result - 0x100000000


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--vectors', type=Path)
    args = parser.parse_args()
    binary = (Path(os.environ['CIVTOT_HOME']) / 'civ2.bak').read_bytes()
    native = OriginalValue(binary)
    rng = random.Random(10610)
    rows = ['# numeric context (schema in probe/test) | 100 native values | '
            '100 native eligibility bits | seed difficulty winner final_crt']
    for case in range(128):
        values = context(rng, case)
        native.load(values)
        scores = [native.call(0x49c95d, native.civ, i) for i in range(100)]
        available = [native.call(0x49ecb6, native.civ, i)
                     and native.call(0x4a2098, native.civ, i)
                     and values[8 + 4*i + 2:8 + 4*i + 4] != [-2, -2]
                     for i in range(100)]
        seed, difficulty = rng.getrandbits(32), case % 6
        native.write(0x5e2590, seed, '<I')
        native.write(0x607658, difficulty, '<B')
        winner = native.call(0x49f98b, native.civ)
        final = struct.unpack('<I', native.cpu.mem_read(0x5e2590, 4))[0]
        rows.append(' | '.join(' '.join(map(str, row)) for row in
                    (values, scores, list(map(int, available)),
                     [seed, difficulty, winner, final])))
    if args.vectors:
        args.vectors.write_text('\n'.join(rows) + '\n')
    print(json.dumps({
        'source_sha256': hashlib.sha256(binary).hexdigest(),
        'routines': ['0x49c95d advance valuation', '0x49f98b complete selection'],
        'contexts': 128, 'advance_values': 12800, 'complete_selections': 128,
        'replacement_callees': [],
        'scope': 'authored acyclic Rules/cache inputs, all leader cases, human/AI, '
                 'six selection difficulties; no live round or player adapter parity',
    }, indent=2))


if __name__ == '__main__':
    main()
