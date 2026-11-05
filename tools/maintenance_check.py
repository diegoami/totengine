#!/usr/bin/env python3
"""Execute original upkeep and bankruptcy routines, using invented inputs.

CIVTOT_HOME required, optional Unicorn outside the repository. Bankruptcy
sets original global 0x6031b8=1 to suppress notifications; the monetary and
building mutations execute from original bytes without call stubs.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct

from research_check import OriginalCost
from totctl import parse_rules, walk_save


def upkeep(v):
    base, improvement, difficulty, k35, k53, smith, government = v
    if improvement == 2:
        base -= int(difficulty < 2 and base != 0)
        base += k35 + k53
    if base == 1 and smith:
        base = 0
    if government == 4 and improvement in (4, 11, 14):
        base = 0
    return base


class OriginalMaintenance(OriginalCost):
    def call(self, address, *args):
        self.cpu.mem_write(0x108000, struct.pack('<' + 'I' * (len(args) + 1), 0x100000, *args))
        self.cpu.reg_write(self.sp, 0x108000)
        self.cpu.emu_start(address, 0x100000, count=100000)
        assert self.cpu.reg_read(self.ip) == 0x100000
        return self.cpu.reg_read(self.ax)

    def run(self, v):
        base, improvement, difficulty, k35, k53, smith, government = v
        self.write(0x5f6685 + 8 * improvement, base, '<B')
        self.write(0x607658, difficulty, '<B')
        self.write(0x5f68bd + 3348, government, '<B')
        self.cpu.mem_write(0x5f6900 + 3348, bytes(13))
        for tech, known in ((35, k35), (53, k53)):
            self.write(0x5f6900 + 3348 + tech // 8, int(known) << (tech % 8), '<B')
        self.write(0x5d0a25 + 20 * 53, 1, '<B')
        self.write(0x5fd15a, 1, '<B')
        self.cpu.mem_write(0x5f5c18, bytes([255] * 28))
        self.cpu.mem_write(0x607736, struct.pack('<28h', *([-1] * 28)))
        self.write(0x607736 + 2 * 17, 0 if smith else -1, '<h')
        return self.call(0x4ca5ae, 1, improvement)

    def bankruptcy(self):
        self.run((1, 2, 3, 0, 0, 0, 2))
        self.cpu.mem_write(0x5fd186, bytes([0b11100, 0, 0, 0, 0]))
        for i, price in ((2, 4), (3, 6), (4, 4)):
            self.write(0x5f6684 + 8 * i, price, '<B')
            self.write(0x5f6685 + 8 * i, 1, '<B')
        self.write(0x5f68aa + 3348, 0, '<i')
        self.write(0x6537fc, 10, '<i')
        self.write(0x6031b8, 1, '<h')
        self.call(0x4ca6e0, 0)
        treasury = struct.unpack('<i', self.cpu.mem_read(0x5f68aa + 3348, 4))[0]
        buildings = list(self.cpu.mem_read(0x5fd186, 5))
        assert (treasury, buildings) == (38, [0b11000, 0, 0, 0, 0])
        return {'treasury_before': 0, 'treasury_after': treasury, 'sold': [2],
                'remaining_buildings': [3, 4], 'notification_global': 1}


def saved_cases(home, binary, name):
    data = (home / name).read_bytes()
    rules = parse_rules(str(home / Path(name).parent / 'Rules.txt'))
    walk = walk_save(str(home / name))
    o = OriginalMaintenance(binary)
    o.cpu.mem_write(0x5f68a0, data[2944:29728])
    o.cpu.mem_write(0x607638, data[652:1250])
    start = walk['cities']['offset']
    o.cpu.mem_write(0x5fd150, data[start:start + 92 * walk['cities']['count']])
    ids = {s: i for i, s in enumerate(rules['CIVILIZE']['comments'])}
    for i in range(100):
        row = rules['CIVILIZE']['rows'][i] if i < len(rules['CIVILIZE']['rows']) else []
        first = row[3] if len(row) > 3 else 'no'
        prereq = -1 if first == 'nil' else -2 if first == 'no' else ids[first]
        o.write(0x5d0a25 + 20 * i, int(first != 'no'), '<B')
        o.write(0x5d0a2a + 20 * i, prereq, '<b')
    for i, row in enumerate(rules['ENDWONDER']['rows'][:28]):
        o.write(0x5f5c18 + i, ids.get(row[0], -1), '<b')
    for i, row in enumerate(rules['IMPROVE']['rows'][:40]):
        if len(row) > 2:
            o.write(0x5f6685 + 8 * i, int(row[2]) & 255, '<B')
    return [(name, civ, improvement, o.call(0x4ca5ae, civ, improvement))
            for civ in range(1, 8) for improvement in range(1, 36)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--vectors', type=Path)
    parser.add_argument('--saved-vectors', type=Path)
    args = parser.parse_args()
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    original = OriginalMaintenance(binary)
    rng = random.Random(86)
    cases = [(rng.randrange(6), rng.choice((2, 4, 11, 14, 5, 6, 22)), rng.randrange(6),
              rng.randrange(2), rng.randrange(2), rng.randrange(2), rng.randrange(7)) for _ in range(1000)]
    for v in cases:
        assert original.run(v) == upkeep(v), (v, original.run(v), upkeep(v))
    if args.vectors:
        args.vectors.write_text('# base improvement difficulty knows35 knows53 smith government upkeep\n'
                               + ''.join(' '.join(map(str, (*v, upkeep(v)))) + '\n' for v in cases[:128]))
    saved = []
    for name in ('Original/Ph_Auto2.SAV', 'FallRome/ROMFALL.scn'):
        saved.extend(saved_cases(home, binary, name))
    if args.saved_vectors:
        args.saved_vectors.write_text('# save civ improvement upkeep\n' + ''.join(' '.join(map(str, v)) + '\n' for v in saved))
    print(json.dumps({'source': 'civ2.bak 0x4ca5ae, 0x4ca6e0',
                      'source_sha256': hashlib.sha256(binary).hexdigest(),
                      'original_x86_cases': len(cases), 'rust_vectors': 128,
                      'bankruptcy': original.bankruptcy(),
                      'saved_input_costs': len(saved),
                      'scope': 'pure upkeep, saved Rules/civ inputs and one bankruptcy case; no full-turn parity claim'}, indent=2))


if __name__ == '__main__':
    main()
