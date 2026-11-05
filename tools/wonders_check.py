#!/usr/bin/env python3
"""Execute original wonder registration and box reset, without UI calls.

CIVTOT_HOME required; optional Unicorn development dependency must be outside
the repo. The fragments are 0x4c7910..0x4c7936 and 0x4c7f97..0x4c7fae.
No claim about full-turn or completion-effects parity is made by this probe.
"""
import hashlib
import json
import os
from pathlib import Path
import struct

from research_check import OriginalCost


def main():
    from unicorn.x86_const import UC_X86_REG_EBP
    binary = (Path(os.environ['CIVTOT_HOME']) / 'civ2.bak').read_bytes()
    original = OriginalCost(binary)
    frame = 0x108100
    cases = []
    for wonder in range(28):
        index = wonder + 1
        flags = 0x82
        original.write(frame + 8, index, '<I')
        original.write(frame - 0x34, wonder, '<I')
        original.write(0x607736 + 2 * wonder, -1, '<h')
        original.write(0x5fd156 + 92 * index, flags, '<I')
        original.write(0x5fd16e + 92 * index, 300 + wonder, '<h')
        for start, stop in ((0x4c7910, 0x4c7936), (0x4c7f97, 0x4c7fae)):
            original.cpu.reg_write(UC_X86_REG_EBP, frame)
            original.cpu.reg_write(original.sp, frame - 0x100)
            original.cpu.emu_start(start, stop, count=100)
            assert original.cpu.reg_read(original.ip) == stop
        registered = struct.unpack('<h', original.cpu.mem_read(0x607736 + 2 * wonder, 2))[0]
        new_flags = struct.unpack('<I', original.cpu.mem_read(0x5fd156 + 92 * index, 4))[0]
        box = struct.unpack('<h', original.cpu.mem_read(0x5fd16e + 92 * index, 2))[0]
        assert (registered, new_flags, box) == (index, flags | 0x100, 0)
        cases.append({'wonder': wonder, 'city_index': registered, 'flags_after': new_flags, 'box_after': box})
    print(json.dumps({'source': 'civ2.bak 0x4c7910..0x4c7936 and 0x4c7f97..0x4c7fae',
                      'source_sha256': hashlib.sha256(binary).hexdigest(),
                      'scope': 'single-player registration/box reset only; no UI or effect dispatch',
                      'original_x86_cases': cases}, indent=2))


if __name__ == '__main__':
    main()
