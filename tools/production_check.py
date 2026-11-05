#!/usr/bin/env python3
"""Execute original AI shield-row arithmetic (civ2.bak 0x4c1c2c..0x4c1dd3).

CIVTOT_HOME required; Unicorn is an optional dependency outside the repository.
The isolated fragment follows unit-support calculation and stops before the
unit loop. Only the original clamp callee executes; no calls are stubbed.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct

from research_check import OriginalCost


def rows(case):
    cosmic, difficulty, human_mask, civ, turn, leader, w23, w25, m73, m58, m76 = case
    if human_mask & (1 << civ):
        return cosmic
    value = 13 - difficulty + int(difficulty < 3) + int(difficulty == 0)
    if turn >= 200 and difficulty > 1 and human_mask & (1 << leader):
        adjustment = 2
        # movsx on the global tech mask is intentional, including owner 7.
        def signed(v):
            return v if v < 128 else v - 256
        if w23 != -1 and signed(m73) & ~human_mask:
            adjustment -= 1
            if not (m73 & human_mask and m58 & human_mask):
                adjustment -= 1
        if w25 != -1 and signed(m76) & ~human_mask:
            adjustment -= 1
            if not m76 & human_mask:
                adjustment -= 1
        value -= max(0, min(2, adjustment))
    if cosmic != 10:
        value = cosmic * value // 10
        value += value & 1
    return value


class OriginalRows(OriginalCost):
    def run(self, case):
        from unicorn.x86_const import UC_X86_REG_EBP
        cosmic, difficulty, human_mask, civ, turn, leader, w23, w25, m73, m58, m76 = case
        for address, value, fmt in [
            (0x5f5ebc, cosmic, '<B'), (0x607658, difficulty, '<B'),
            (0x60765b, human_mask, '<B'), (0x607648, turn, '<h'),
            (0x607781, leader, '<B'), (0x607764, w23, '<h'),
            (0x607768, w25, '<h'), (0x60771b, m73, '<B'),
            (0x60770c, m58, '<B'), (0x60771e, m76, '<B')]:
            self.write(address, value, fmt)
        frame = 0x108100
        self.write(frame - 0x18, civ, '<I')
        self.cpu.reg_write(UC_X86_REG_EBP, frame)
        self.cpu.reg_write(self.sp, frame - 0x100)
        self.cpu.emu_start(0x4c1c2c, 0x4c1dd3, count=1000)
        assert self.cpu.reg_read(self.ip) == 0x4c1dd3
        return struct.unpack('<i', self.cpu.mem_read(0x6537fc, 4))[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--vectors', type=Path)
    args = parser.parse_args()
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    original = OriginalRows(binary)
    rng = random.Random(89)
    cases = []
    for cosmic in (7, 10, 13):
        for difficulty in range(6):
            for turn in (199, 200, 201):
                for human in (False, True):
                    cases.append((cosmic, difficulty, 2 if human else 4, 1, turn, 2,
                                  -1, -1, 0, 0, 0))
    for _ in range(1000):
        cases.append((rng.choice((7, 10, 13)), rng.randrange(6), rng.randrange(256),
                      rng.randrange(1, 8), rng.choice((199, 200, 201, 350)), rng.randrange(8),
                      rng.choice((-2, -1, 0, 10)), rng.choice((-2, -1, 0, 10)),
                      rng.randrange(256), rng.randrange(256), rng.randrange(256)))
    for case in cases:
        assert original.run(case) == rows(case), case
    saves = []
    for name in ('Original/Ph_Auto2.SAV', 'FallRome/ROMFALL.scn'):
        data = (home / name).read_bytes()
        for tech in range(100):
            mask = sum(1 << civ for civ in range(1, 8)
                       if data[2944 + 3348 * civ + 96 + tech // 8] >> (tech % 8) & 1)
            assert data[806 + tech] == mask, (name, tech)
        saves.append({'save': name, 'power_order': list(data[974:982]), 'knowledge_masks_exact': 100})
    if args.vectors:
        args.vectors.write_text('# cosmic difficulty humans civ turn leader w23 w25 tech73 tech58 tech76 rows\n'
                               + ''.join(' '.join(map(str, (*c, rows(c)))) + '\n' for c in cases))
    print(json.dumps({'source': 'civ2.bak 0x4c1c2c..0x4c1dd3',
                      'source_sha256': hashlib.sha256(binary).hexdigest(),
                      'original_x86_cases': len(cases),
                      'saves': saves,
                      'default_rows_by_difficulty': [rows((10, d, 0, 1, 0, 0, -1, -1, 0, 0, 0)) for d in range(6)],
                      'scope': 'shield rows, original fragment and clamp; excludes unit support'}, indent=2))


if __name__ == '__main__':
    main()
