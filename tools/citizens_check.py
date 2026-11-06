#!/usr/bin/env python3
"""Execute native city citizen edit transitions; retain numeric data only.

The UI's current-city accessor returns synthetic city 0; its small-city message
is intercepted. All transition branches and packed-field helpers run unchanged.
Totals/redraw/turn processing are outside these explicitly bounded comparisons.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct

from research_check import OriginalCost


class OriginalCitizens(OriginalCost):
    def run_edit(self, worked, packed, size, slot, blocked, role):
        from unicorn import UC_HOOK_CODE
        from unicorn.x86_const import UC_X86_REG_EBP
        base = 0x107000
        self.cpu.mem_write(base - 256, bytes(512))
        self.cpu.reg_write(UC_X86_REG_EBP, base)
        self.cpu.reg_write(self.sp, base - 128)
        self.write(base + 8, slot, '<i')
        self.write(base - 24, slot, '<i')
        self.write(0x5fd182, worked, '<I')
        self.write(0x5fd168, packed, '<I')
        self.write(0x5fd15b, size, '<B')
        self.write(0x653884, worked >> 26, '<i')
        self.cpu.mem_write(0x6537b0, bytes([blocked] * 20))
        start, end = (0x4dd19f, 0x4dd24d) if role else (0x4dddc6, 0x4ddec9)
        messages = []

        def ui(cpu, address, length, data):
            if address in (end, 0x4dd286, 0x4ddeff):
                cpu.reg_write(self.ip, 0x100000)
            elif address in (0x411c50, 0x411c20):
                if address == 0x411c20:
                    messages.append('population-five')
                sp = cpu.reg_read(self.sp)
                ret = struct.unpack('<I', cpu.mem_read(sp, 4))[0]
                cpu.reg_write(self.ax, 0)
                cpu.reg_write(self.sp, sp + 4)
                cpu.reg_write(self.ip, ret)

        hook = self.cpu.hook_add(UC_HOOK_CODE, ui)
        try:
            self.cpu.emu_start(start, 0x100000, count=10000)
            assert self.cpu.reg_read(self.ip) == 0x100000
        finally:
            self.cpu.hook_del(hook)
        after_worked = struct.unpack('<I', self.cpu.mem_read(0x5fd182, 4))[0]
        after_packed = struct.unpack('<I', self.cpu.mem_read(0x5fd168, 4))[0]
        return after_worked, after_packed, len(messages)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--gold', required=True)
    p.add_argument('--vectors', required=True)
    args = p.parse_args()
    binary = (Path(os.environ['CIVTOT_HOME']) / 'civ2.bak').read_bytes()
    native = OriginalCitizens(binary)
    rng = random.Random(130135)
    lines = []
    for size in [1, 4, 5, 6, 16, 30]:
        for slot in range(16):
            for kind in [1, 2, 3]:
                packed = (rng.getrandbits(32) & ~(3 << (2 * slot))) | (kind << (2 * slot))
                worked = (1 << 20) | (16 << 26)
                actual = native.run_edit(worked, packed, size, slot, 0, True)
                lines.append(' '.join(map(str, [1, size, worked, packed, slot, 0, *actual])))
    for count in [0, 1, 2, 15, 16]:
        for slot in range(21):
            for blocked in [0, 1, 4, 8, 16, 32]:
                for selected in [False, True]:
                    bits = (1 << 20) | ((1 << slot) if selected else 0)
                    worked = bits | (count << 26)
                    packed = rng.getrandbits(32)
                    actual = native.run_edit(worked, packed, 20, slot, blocked, False)
                    lines.append(' '.join(map(str, [0, 20, worked, packed, slot, blocked, *actual])))
    Path(args.vectors).write_text('# role size worked specialists slot blocked | next_worked next_specialists small_city_message\n' + '\n'.join(lines) + '\n')
    report = {
        'binary_sha256': hashlib.sha256(binary).hexdigest(),
        'cases': len(lines), 'specialist_cases': 6 * 16 * 3,
        'worker_cases': 5 * 21 * 6 * 2,
        'worker_entry_stop': ['0x4dddc6', '0x4ddec9'],
        'specialist_entry_stop': ['0x4dd19f', '0x4dd24d'],
        'early_return_stops': ['0x4dd286', '0x4ddeff'],
        'ui_substitutions': {'0x411c50': 'current city index 0', '0x411c20': 'record small-city message'},
        'scope': 'Original raw citizen edit branches and original packed setters/getters. Synthetic packed-field input boundary; totals refresh, allocation, rendering, network notifications and full city-turn parity are not claimed.',
    }
    Path(args.gold).write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report))


if __name__ == '__main__':
    main()
