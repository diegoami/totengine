#!/usr/bin/env python3
"""Execute original hut/research tech-group gates (external Unicorn needed)."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct

from research_check import OriginalCost


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--vectors', type=Path)
    args = parser.parse_args()
    binary = (Path(os.environ['CIVTOT_HOME']) / 'civ2.bak').read_bytes()
    original = OriginalCost(binary)
    rng = random.Random(82)
    rows = ['# group eight_flags_decimal hut_allowed research_allowed']
    for i in range(1000):
        group = i % 9 - 1
        flags = [rng.randrange(4) for _ in range(8)]
        original.write(0x5d0a1c, group, '<i')
        original.write(0x5f68ae + 3348, 2, '<h')
        original.cpu.mem_write(0x606f78 + 120, bytes(flags))
        allowed = []
        for address in (0x4a2098, 0x4a2111):
            original.cpu.mem_write(0x108000, struct.pack('<III', 0x100000, 1, 0))
            original.cpu.reg_write(original.sp, 0x108000)
            original.cpu.emu_start(address, 0x100000, count=100)
            assert original.cpu.reg_read(original.ip) == 0x100000
            allowed.append(original.cpu.reg_read(original.ax))
        if i < 128:
            rows.append(f'{group} ' + ''.join(map(str, flags)) + ' ' + ' '.join(map(str, allowed)))
    if args.vectors:
        args.vectors.write_text('\n'.join(rows) + '\n')
    print(json.dumps({'source': 'civ2.bak 0x4a2098 and 0x4a2111',
                      'source_sha256': hashlib.sha256(binary).hexdigest(),
                      'original_x86_contexts': 1000, 'committed_vectors': len(rows) - 1,
                      'scope': 'original gates, supplied parsed tech group and leader flags; no replacement callees'}, indent=2))


if __name__ == '__main__':
    main()
