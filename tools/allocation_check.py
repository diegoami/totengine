#!/usr/bin/env python3
"""Original deleted-slot/capacity decisions and full-width identity writes.

CIVTOT_HOME + external Unicorn dependency required. No original bytes emitted.
The slot fragment stops before any notification or unit-field initialization.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct

from research_check import OriginalCost


class OriginalAllocation(OriginalCost):
    def slot(self, count, human, military, free):
        from unicorn import UC_HOOK_CODE
        from unicorn.x86_const import UC_X86_REG_EBP
        frame = 0x108100
        records = bytearray(40 * count)
        for i in range(count):
            struct.pack_into('<I', records, 40 * i + 30, 0 if i == free else 0x10000 + i)
        self.cpu.mem_write(0x607c58, bytes(records))
        for address, value, fmt in ((0x607666, count, '<h'), (0x60765b, 2 if human else 0, '<B'),
                                    (0x5f690e + 3348, military, '<h'), (frame + 12, 1, '<I')):
            self.write(address, value, fmt)
        stops = {0x56a3de, 0x56a386}
        def stop(cpu, address, size, data):
            if address in stops:
                cpu.emu_stop()
        hook = self.cpu.hook_add(UC_HOOK_CODE, stop)
        try:
            self.cpu.reg_write(UC_X86_REG_EBP, frame)
            self.cpu.reg_write(self.sp, frame - 0x100)
            self.cpu.emu_start(0x56a2d9, 0x100000, count=100000)
            ip = self.cpu.reg_read(self.ip)
            assert ip in stops
        finally:
            self.cpu.hook_del(hook)
        return -1 if ip == 0x56a386 else struct.unpack('<i', self.cpu.mem_read(frame - 12, 4))[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--vectors', type=Path)
    args = parser.parse_args()
    binary = (Path(os.environ['CIVTOT_HOME']) / 'civ2.bak').read_bytes()
    original = OriginalAllocation(binary)
    cases = []
    for count in (0, 1, 3, 1948, 1949, 2039, 2040, 2047, 2048):
        for human in (0, 1):
            for military in sorted(set((0, min(count, 1948), min(count, 1949)))):
                for free in sorted(set((-1, 0 if count else -1, count // 2 if count else -1))):
                    cases.append((count, human, military, free, original.slot(count, human, military, free)))
    from unicorn.x86_const import UC_X86_REG_EBP
    identities = []
    frame = 0x108100
    for seed in (1, 65535, 65536, 0xfffffffe):
        original.write(frame - 12, 0, '<I')
        original.write(0x5d1500, seed, '<I')
        original.cpu.reg_write(UC_X86_REG_EBP, frame)
        original.cpu.reg_write(original.sp, frame - 0x100)
        original.cpu.emu_start(0x56a451, 0x56a469, count=20)
        unit = struct.unpack('<I', original.cpu.mem_read(0x607c76, 4))[0]
        original.write(frame - 4, 0, '<I')
        original.write(0x5d1504, seed, '<I')
        original.cpu.emu_start(0x45f0c5, 0x45f0e4, count=20)
        city = struct.unpack('<I', original.cpu.mem_read(0x5fd1a6, 4))[0]
        assert unit == city == seed
        identities.append({'counter': seed, 'unit_u32_at_30': unit, 'city_u32_at_86': city})
    if args.vectors:
        args.vectors.write_text('# slot_count human military_count free_index allocated_index\n' + '\n'.join(' '.join(map(str, c)) for c in cases) + '\n')
    print(json.dumps({'source': 'civ2.bak 0x56a2d9..0x56a3de (failure boundary 0x56a386); identity writes 0x56a451 and 0x45f0c5',
                      'source_sha256': hashlib.sha256(binary).hexdigest(), 'slot_cases': len(cases),
                      'scope': 'original slot and capacity decision; no allocation UI; alive IDs exceed 16 bits',
                      'identity_writes': identities}, indent=2))


if __name__ == '__main__':
    main()
