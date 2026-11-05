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
        self.cpu.emu_start(start, stop, count=1000000)
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

    def select(self, seed, human, cities, landmass_cities, flags, near):
        self.write(0x5e2590, seed, '<I')
        self.write(0x60765b, 2 if human else 0, '<B')
        self.write(0x5f6910 + 3348, cities, '<h')
        self.write(0x5f6e70 + 3348 + 1, landmass_cities, '<B')
        self.write(0x5f7170 + 3348 + 1, flags, '<B')
        self.write(0x5e6f70, near, '<i')
        self.write(0x6705a0, 2, '<h')
        self.write(0x6705a2, 1, '<h')
        self.write(0x6762f8, 0, '<I')
        self.write(0x5deaa8, 0x700000, '<I')
        self.cpu.mem_write(0x700000, bytes((1, 0, 0, 1, 0, 0)))
        for offset, value in ((-0x64, 1), (-0x30, 0), (-0x34, 0)):
            self.write(self.frame + offset, value, '<I')
        self.fragment(0x54acbf, 0x54adb7)
        return self.integer(self.frame - 0x58), self.integer(0x5e2590)

    def nomad_redirect(self, cities, unhomed, known):
        from unicorn import UC_HOOK_CODE
        from unicorn.x86_const import UC_X86_REG_EBP
        self.write(0x6076ee, 2 if known else 0, '<B')
        self.write(0x607666, unhomed, '<h')
        self.write(0x5f6910 + 3348, cities, '<h')
        self.write(0x6762f8, 0, '<I')
        self.write(self.frame - 0x64, 1, '<I')
        self.write(self.frame - 0x58, 5, '<I')
        self.write(0x5f4712, 5, '<b')
        for i in range(unhomed):
            unit = bytearray(40)
            unit[9], unit[18] = 1, 255
            struct.pack_into('<I', unit, 30, i + 1)
            self.cpu.mem_write(0x607c58 + 40 * i, bytes(unit))
        stops = {0x54adb7, 0x54b1ac}
        def stop(cpu, address, size, data):
            if address in stops:
                cpu.emu_stop()
        hook = self.cpu.hook_add(UC_HOOK_CODE, stop)
        try:
            self.cpu.reg_write(UC_X86_REG_EBP, self.frame)
            self.cpu.reg_write(self.sp, self.frame - 0x100)
            self.cpu.emu_start(0x54b09b, 0x100000, count=1000000)
            assert self.cpu.reg_read(self.ip) in stops
        finally:
            self.cpu.hook_del(hook)
        return self.integer(self.frame - 0x58)

    def ring(self, cities, turn, single, mode, tech_mask, blocked, rough, water, impassable, flat, x, y):
        from unicorn import UC_HOOK_CODE
        width = height = 16
        for a, v, fmt in ((0x607782, mode, '<B'), (0x607648, turn, '<h'),
                          (0x607652, 0, '<B'), (0x607638, 0x8000 if flat else 0, '<H'),
                          (0x607668, 0, '<h'), (0x5f6910 + 3348, cities, '<h'),
                          (0x6705a0, width, '<h'), (0x6705a2, height, '<h'),
                          (0x6762f8, 0, '<I'), (0x5deaa8, 0x700000, '<I'),
                          (0x5f4a83, 50, '<b')):
            self.write(a, v, fmt)
        self.cpu.mem_write(0x6076d2, bytes(100))
        for i, tech in enumerate((51, 50, 34)):
            self.write(0x6076d2 + tech, 2 if tech_mask & (1 << i) else 0, '<B')
        tiles = bytearray([1, 0, 0, 1, 0, 240] * (width * height // 2))
        dx, dy = (1, 2, 1, 0, -1, -2, -1, 0), (-1, 0, 1, 2, 1, 0, -1, -2)
        for k in range(8):
            nx, ny = x + dx[k], y + dy[k]
            if not flat:
                nx %= width
            if not (0 <= nx < width and 0 <= ny < height):
                continue
            offset = 6 * (ny * (width // 2) + nx // 2)
            terrain = 3 if impassable & (1 << k) else 10 if water & (1 << k) else 2 if rough & (1 << k) else 1
            tiles[offset] = terrain
            if blocked & (1 << k):
                tiles[offset + 1], tiles[offset + 5] = 1, 16
        self.cpu.mem_write(0x700000, bytes(tiles))
        self.cpu.mem_write(0x66f8c0, bytes(11 * 24))
        self.write(0x66f8c6 + 3 * 24, -2, '<b')
        for t in range(11):
            self.write(0x5d11f0 + t * 24, 3 if t == 2 else 1, '<b')
        for offset, value in ((-0x64, 1), (-0x30, x), (-0x34, y), (-8, single), (-0x40, 0)):
            self.write(self.frame + offset, value, '<I')
        requests = []
        def boundary(cpu, address, size, data):
            if address not in (0x56a1c0, 0x467d19):
                return
            sp = cpu.reg_read(self.sp)
            ret = self.integer(sp)
            if address == 0x56a1c0:
                t, civ, nx, ny, map_id = struct.unpack('<5i', cpu.mem_read(sp + 4, 20))
                assert (civ, map_id) == (0, 0)
                requests.append((t, nx, ny))
                cpu.reg_write(self.ax, len(requests) - 1)
            cpu.reg_write(self.sp, sp + 4)
            cpu.reg_write(self.ip, ret)
        hook = self.cpu.hook_add(UC_HOOK_CODE, boundary)
        try:
            self.fragment(0x54b704, 0x54babd)
        finally:
            self.cpu.hook_del(hook)
        return requests

    def barbarian_gate(self, seed, near, cities, turn, human, difficulty, level, reference_rank, rank, landmass_cities, own_far):
        from unicorn import UC_HOOK_CODE
        from unicorn.x86_const import UC_X86_REG_EBP
        for a, v, fmt in ((0x5e2590, seed, '<I'), (0x607648, turn, '<h'),
                          (0x60765b, 2 if human else 0, '<B'), (0x607658, difficulty, '<B'),
                          (0x607659, level, '<B'), (0x607771, 2, '<B'),
                          (0x607774, reference_rank, '<B'), (0x607773, rank, '<B'),
                          (0x5e6f70, near, '<i'), (0x5f6910 + 3348, cities, '<h'),
                          (0x5f6e70 + 3348 + 1, landmass_cities, '<B'),
                          (0x6705a0, 64, '<h'), (0x6705a2, 64, '<h'),
                          (0x607638, 0x8000, '<H'), (0x607668, 1, '<h'),
                          (0x6762f8, 0, '<I'), (0x5deaa8, 0x700000, '<I'),
                          (0x65b640, 0, '<B')):
            self.write(a, v, fmt)
        self.cpu.mem_write(0x700000, bytes([1, 0, 0, 1, 0, 16]) * 2048)
        city = bytearray(92)
        struct.pack_into('<hh', city, 0, 40 if own_far else 0, 40 if own_far else 0)
        city[10] = 1
        struct.pack_into('<I', city, 86, 1)
        self.cpu.mem_write(0x5fd150, bytes(city))
        for offset, value in ((-0x64, 1), (-0x30, 8), (-0x34, 8), (-0x58, 3), (-8, 0)):
            self.write(self.frame + offset, value, '<I')
        stops = {0x54adb7, 0x54bdd7, 0x54b704}
        def stop(cpu, address, size, data):
            if address in stops:
                cpu.emu_stop()
        hook = self.cpu.hook_add(UC_HOOK_CODE, stop)
        try:
            self.cpu.reg_write(UC_X86_REG_EBP, self.frame)
            self.cpu.reg_write(self.sp, self.frame - 0x100)
            self.cpu.emu_start(0x54b520, 0x100000, count=10000)
            ip = self.cpu.reg_read(self.ip)
            assert ip in stops, hex(ip)
        finally:
            self.cpu.hook_del(hook)
        outcome = -1 if ip == 0x54bdd7 else self.integer(self.frame - 0x58)
        return outcome, self.integer(self.frame - 8), self.integer(0x5e2590)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--vectors', type=Path)
    parser.add_argument('--dispatch-vectors', type=Path)
    parser.add_argument('--nomad-vectors', type=Path)
    parser.add_argument('--ring-vectors', type=Path)
    parser.add_argument('--barbarian-gate-vectors', type=Path)
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
    dispatch = ['# seed human cities landmass_cities flags nearest_distance outcome next_crt_state']
    nomads = ['# cities unhomed_settlers global_tech28 outcome']
    for i in range(1000):
        inputs = (rng.getrandbits(32), i % 2, i % 7, (i // 7) % 3, i % 256,
                  (3, 4, 12, 13, 9999)[i % 5])
        result = original.select(*inputs)
        if i < 256:
            dispatch.append(' '.join(map(str, inputs + result)))
        inputs = (i % 41, i % 9, i % 2)
        result = original.nomad_redirect(*inputs)
        if i < 128:
            nomads.append(' '.join(map(str, inputs + (result,))))
    if args.dispatch_vectors:
        args.dispatch_vectors.write_text('\n'.join(dispatch) + '\n')
    if args.nomad_vectors:
        args.nomad_vectors.write_text('\n'.join(nomads) + '\n')
    ring = ['# cities turn single mode tech_mask blocked rough water impassable flat x y count [type x y]...']
    for i in range(1000):
        inputs = (i % 12, i % 200, i % 2, i % 3, i % 8, rng.randrange(256),
                  rng.randrange(256), rng.randrange(256), rng.randrange(256), i % 2,
                  (0, 8, 14)[i % 3], (0, 8, 14)[(i // 3) % 3])
        requests = original.ring(*inputs)
        if i < 128:
            ring.append(' '.join(map(str, inputs + (len(requests),) + tuple(v for request in requests for v in request))))
    if args.ring_vectors:
        args.ring_vectors.write_text('\n'.join(ring) + '\n')
    gates = ['# seed near cities turn human difficulty level reference_rank rank landmass_cities own_far outcome single next_crt_state']
    for i in range(1000):
        inputs = (rng.getrandbits(32), (3, 4, 20)[i % 3], i % 5, i % 100,
                  i % 2, i % 6, i % 4, (i // 7) % 8, (i // 11) % 8, (i // 3) % 3, i % 2)
        result = original.barbarian_gate(*inputs)
        if i < 256:
            gates.append(' '.join(map(str, inputs + result)))
    if args.barbarian_gate_vectors:
        args.barbarian_gate_vectors.write_text('\n'.join(gates) + '\n')
    print(json.dumps({'source': 'civ2.bak 0x54bae5..0x54bd12, 0x54b453..0x54b4d1, 0x54af81..0x54b03c; site helper 0x5703be',
                      'source_sha256': hashlib.sha256(binary).hexdigest(),
                      'scope': 'supplied dispatch; mercenary type, metals payment, post-founding tribe size/buildings; real CRT and building callee; not full hut parity',
                      'original_x86_cases': dict(zip(('mercenary', 'metals', 'tribe'), counts)),
                      'site_value_cases': 128,
                      'selection_cases': 1000, 'nomad_redirect_cases': 1000,
                      'barbarian_ring_cases': 1000, 'barbarian_gate_cases': 1000,
                      'barbarian_ring_scope': '0x54b704..0x54babd; original placement/type decisions; 0x56a1c0 records successful creation requests, 0x467d19 redraw skipped; not full unit mutation parity',
                      'game_family': {'save_byte': 982, 'ram': '0x607782', 'loader': '0x41924f Original=0; 0x4192bf SciFi=1; 0x41932f Fantasy=2'},
                      'committed_vectors': len(lines) - 1}, indent=2))


if __name__ == '__main__':
    main()
