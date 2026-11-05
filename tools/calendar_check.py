#!/usr/bin/env python3
"""Execute the original turn-to-year routine and retain derived vectors only.

CIVTOT_HOME required. Optional Unicorn dependency belongs outside this repo.
The full 0x46da89 routine and its real clamp execute without replacements.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct

from research_check import OriginalCost
from totctl import walk_save


class OriginalCalendar(OriginalCost):
    def year(self, turn, difficulty, flags, limit, increment, start):
        for a, v, f in ((0x607658, difficulty, '<B'), (0x607640, flags, '<H'),
                        (0x60764c, limit, '<h'), (0x5f5ea4, increment, '<h'),
                        (0x5f5ea6, start, '<h')):
            self.write(a, v, f)
        self.cpu.mem_write(0x108000, struct.pack('<Ii', 0x100000, turn))
        self.cpu.reg_write(self.sp, 0x108000)
        self.cpu.emu_start(0x46da89, 0x100000, count=2000)
        assert self.cpu.reg_read(self.ip) == 0x100000
        value = self.cpu.reg_read(self.ax)
        return value if value < (1 << 31) else value - (1 << 32)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--vectors', type=Path)
    args = parser.parse_args()
    binary = (Path(os.environ['CIVTOT_HOME']) / 'civ2.bak').read_bytes()
    original = OriginalCalendar(binary)
    table = [list(struct.iter_unpack('<iii', original.cpu.mem_read(0x5d4fa8 + 72 * rank, 72))) for rank in range(4)]
    rng = random.Random(138)
    cases = []
    for i in range(2000):
        case = (rng.randrange(0, 2000), i % 6, (0, 4, 8, 12, 128, 132, 136, 140)[i % 8],
                (-1, 0, 200, 500)[i % 4], (0, 1, -1, 5, -12)[i % 5], (-4000, 0, 1, 392, 2000)[i % 5])
        result = original.year(*case)
        if i < 256:
            cases.append(case + (result,))
    boundary = []
    for difficulty in range(6):
        for turn in (0, 1, 2, 50, 100, 200, 300, 400, 500, 600, 1000):
            case = (turn, difficulty, 0, -1, 0, 0)
            result = original.year(*case)
            cases.append(case + (result,))
            boundary.append([turn, difficulty, result])
    saved = []
    home = Path(os.environ['CIVTOT_HOME'])
    for name in ('Original/Ph_Auto2.SAV', 'FallRome/ROMFALL.scn'):
        path = home / name
        data = path.read_bytes()
        turn, stored_year, limit = struct.unpack_from('<hhh', data, 668)
        flags = struct.unpack_from('<H', data, 660)[0]
        increment = start = 0
        if flags & 128:
            walk = walk_save(str(path))
            leaders = 1 if data[678] == 0 else data[687].bit_count()
            position = walk['cities']['offset'] + 92 * walk['cities']['count'] + 1453 + 60 * leaders
            increment, start = struct.unpack_from('<hh', data, position + 84)
        case = (turn, data[684], flags, limit, increment, start)
        computed = original.year(*case)
        cases.append(case + (computed,))
        saved.append({'save': name, 'inputs': case, 'stored_year': stored_year, 'computed_year': computed})
    if args.vectors:
        args.vectors.write_text('# turn difficulty flags turn_limit increment start_year year\n' + '\n'.join(' '.join(map(str, c)) for c in cases) + '\n')
    print(json.dumps({'source': 'civ2.bak 0x46da89, invoked by 0x470346 after turn increment',
                      'source_sha256': hashlib.sha256(binary).hexdigest(),
                      'calendar_table': table, 'original_x86_cases': 2068, 'saved': saved,
                      'committed_vectors': len(cases), 'standard_boundaries': boundary,
                      'scope': 'year computation only, including scenario increments and post-limit elapsed years'}, indent=2))


if __name__ == '__main__':
    main()
