#!/usr/bin/env python3
"""Execute original hut outcome fragments, including their real CRT draws.

CIVTOT_HOME and optional external Unicorn dependency required. These checks
supply the dispatch case: they do not claim full entry/redirect/spawn parity.
No game assets or executable bytes are emitted.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct

from research_check import OriginalCost

TECHS = (11, 64, 55, 42, 17, 34, 35, 39)


class OriginalHuts(OriginalCost):
    frame = 0x108100

    def fragment(self, start, stop):
        from unicorn.x86_const import UC_X86_REG_EBP
        self.cpu.reg_write(UC_X86_REG_EBP, self.frame)
        self.cpu.reg_write(self.sp, self.frame - 0x100)
        self.cpu.emu_start(start, stop, count=10000)
        assert self.cpu.reg_read(self.ip) == stop, hex(self.cpu.reg_read(self.ip))

    def integer(self, address, fmt='<I'):
        return struct.unpack(fmt, self.cpu.mem_read(address, struct.calcsize(fmt)))[0]

    def run(self, kind, seed, mode, map_id, mask, year, difficulty):
        self.write(0x5e2590, seed, '<I')
        self.write(0x607782, mode, '<B')
        self.write(0x6762f8, map_id, '<I')
        self.write(0x60764a, year, '<h')
        self.write(0x607658, difficulty, '<B')
        if kind == 0:
            self.cpu.mem_write(0x6076d2, bytes(100))
            for i, tech in enumerate(TECHS):
                self.write(0x6076d2 + tech, 2 if mask & (1 << i) else 0, '<B')
            self.fragment(0x54bae5, 0x54bd12)
            a, b = self.integer(self.frame - 0x68), 0
        elif kind == 1:
            self.write(self.frame - 0x64, 1, '<I')
            self.write(0x5f68aa + 3348, 0, '<i')
            self.fragment(0x54b453, 0x54b4d1)
            a, b = self.integer(self.frame - 0x20), 0
            assert self.integer(0x5f68aa + 3348) == a
        elif kind == 2:
            self.write(self.frame - 0x5c, 0, '<I')
            self.write(0x5fd15b, 1, '<B')
            self.cpu.mem_write(0x5fd186, bytes(5))
            # Original 0x4300af writes building bits; no replacement callee.
            self.fragment(0x54af81, 0x54b03c)
            a, b = self.integer(0x5fd15b, '<B'), self.integer(0x5fd186, '<B')
        else:
            self.write(0x6705a0, 2, '<h')
            self.write(0x6705a2, 1, '<h')
            self.write(0x6762f8, 0, '<I')
            self.write(0x5deaa8, 0x700000, '<I')
            self.cpu.mem_write(0x700000, bytes((1, 0, map_id << 5, 1, 0, mask)))
            self.cpu.mem_write(0x108000, struct.pack('<III', 0x100000, 0, 0))
            self.cpu.reg_write(self.sp, 0x108000)
            self.cpu.emu_start(0x5703be, 0x100000, count=1000)
            assert self.cpu.reg_read(self.ip) == 0x100000
            a, b = self.cpu.reg_read(self.ax), 0
        return a, b, self.integer(0x5e2590)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--vectors', type=Path)
    args = parser.parse_args()
    binary = (Path(os.environ['CIVTOT_HOME']) / 'civ2.bak').read_bytes()
    original = OriginalHuts(binary)
    rng = random.Random(79)
    lines = ['# kind seed mode map tech_mask year difficulty result building_bits next_crt_state']
    counts = []
    for kind in range(3):
        for k in range(1000):
            inputs = (kind, rng.getrandbits(32), k % 3, k % 4, k % 256,
                      (-4000, 999, 1000, 1001, 2000)[k % 5], k % 6)
            result = original.run(*inputs)
            if k < 256:
                lines.append(' '.join(map(str, inputs + result)))
        counts.append(1000)
    for bits in range(8):
        for fertility in range(16):
            inputs = (3, 0, 0, bits, fertility, 0, 0)
            result = original.run(*inputs)
            lines.append(' '.join(map(str, inputs + result)))
    if args.vectors:
        args.vectors.write_text('\n'.join(lines) + '\n')
    print(json.dumps({'source': 'civ2.bak 0x54bae5..0x54bd12, 0x54b453..0x54b4d1, 0x54af81..0x54b03c; site helper 0x5703be',
                      'source_sha256': hashlib.sha256(binary).hexdigest(),
                      'scope': 'supplied dispatch; mercenary type, metals payment, post-founding tribe size/buildings; real CRT and building callee; not full hut parity',
                      'original_x86_cases': dict(zip(('mercenary', 'metals', 'tribe'), counts)),
                      'site_value_cases': 128,
                      'game_family': {'save_byte': 982, 'ram': '0x607782', 'loader': '0x41924f Original=0; 0x4192bf SciFi=1; 0x41932f Fantasy=2'},
                      'committed_vectors': len(lines) - 1}, indent=2))


if __name__ == '__main__':
    main()
