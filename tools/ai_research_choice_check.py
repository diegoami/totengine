#!/usr/bin/env python3
"""Check the original research selector, with explicit eligibility/value stubs.

The selector and CRT execute original bytes. This isolates selection from
the still-undecoded campaign-dependent value routine; it is NOT a comparison
of full original AI research decisions. Requires external Unicorn and
CIVTOT_HOME. Emits derived numeric vectors only.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct

from research_check import OriginalCost


def choose(candidates, human, difficulty, acquired, seed):
    best, score, admitted, draws = -1, -1, False, 0
    for i, (available, group, disabled, value) in enumerate(candidates):
        if not available or not group or disabled:
            continue
        if human and admitted and difficulty and (i - acquired) % 3 == 0:
            continue
        admitted = True
        if human or 0 < ((value - 1) & 0xffffffff) < 0x80000000:
            seed = (seed * 214013 + 2531011) & 0xffffffff
            roll = (seed >> 16) & 32767
            draws += 1
            current = value + roll % 3 - 1 if human else roll % abs(value)
            current = (current + 0x80000000) % 0x100000000 - 0x80000000
        else:
            current = 0
        if current > score:
            best, score = i, current
    return best, seed, draws


class OriginalChoice(OriginalCost):
    def __init__(self, binary):
        super().__init__(binary)
        from unicorn import UC_HOOK_CODE
        self.candidates = []
        self.draws = 0
        self.cpu.hook_add(UC_HOOK_CODE, self.boundary)

    def boundary(self, cpu, address, _size, _data):
        if address == 0x5b11d0:
            self.draws += 1
        if address not in (0x49ecb6, 0x4a2098, 0x49c95d):
            return
        sp = cpu.reg_read(self.sp)
        ret, civ, tech = struct.unpack('<III', cpu.mem_read(sp, 12))
        assert civ == 1 and tech < 100
        available, group, _disabled, value = self.candidates[tech]
        result = {0x49ecb6: available, 0x4a2098: group, 0x49c95d: value}[address]
        cpu.reg_write(self.ax, result & 0xffffffff)
        cpu.reg_write(self.sp, sp + 4)
        cpu.reg_write(self.ip, ret)

    def run_choice(self, candidates, human, difficulty, acquired, seed):
        self.candidates, self.draws = candidates, 0
        for i, (_, _, disabled, _) in enumerate(candidates):
            self.cpu.mem_write(0x5d0a2a + 20 * i,
                               struct.pack('<bb', -2, -2 if disabled else -1))
        for address, value, fmt in [
            (0x60765b, 2 * human, '<B'), (0x607658, difficulty, '<B'),
            (0x5f68b8 + 3348, acquired, '<B'), (0x5e2590, seed, '<I')]:
            self.write(address, value, fmt)
        self.cpu.mem_write(0x108000, struct.pack('<II', 0x100000, 1))
        self.cpu.reg_write(self.sp, 0x108000)
        self.cpu.emu_start(0x49f98b, 0x100000, count=30000)
        assert self.cpu.reg_read(self.ip) == 0x100000
        best = self.cpu.reg_read(self.ax)
        if best == 0xffffffff:
            best = -1
        state = struct.unpack('<I', self.cpu.mem_read(0x5e2590, 4))[0]
        return best, state, self.draws


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--vectors', type=Path)
    args = parser.parse_args()
    binary = (Path(os.environ['CIVTOT_HOME']) / 'civ2.bak').read_bytes()
    original = OriginalChoice(binary)
    rng = random.Random(106)
    rows = ['# human difficulty acquired seed winner final_crt draws; '
            'then 100 available:group:disabled:value entries']
    for i in range(512):
        human, difficulty, acquired = i % 2, (i // 2) % 6, rng.randrange(100)
        seed = rng.getrandbits(32)
        candidates = [(rng.randrange(2), rng.randrange(2), rng.randrange(2),
                       rng.choice([-2147483648, -10, -1, 0, 1, 2, 3, 9, 40,
                                   32768, 2147483647])) for _ in range(100)]
        # Deterministic boundary families: none, all zero, singleton, ties,
        # all positive; remaining cases mix independently gated slots.
        if i < 24:
            candidates = [(int(i >= 12), 1, 0, 0) for _ in range(100)]
        elif i < 48:
            candidates = [(int(j == acquired), 1, 0, 1) for j in range(100)]
        elif i < 72:
            candidates = [(1, 1, 0, 2) for _ in range(100)]
        expected = choose(candidates, human, difficulty, acquired, seed)
        actual = original.run_choice(candidates, human, difficulty, acquired, seed)
        assert actual == expected, (i, actual, expected)
        if i < 128:
            rows.append(' '.join(map(str, (human, difficulty, acquired, seed, *actual)))
                        + ';' + ' '.join(':'.join(map(str, c)) for c in candidates))
    if args.vectors:
        args.vectors.write_text('\n'.join(rows) + '\n')
    print(json.dumps({
        'source': 'civ2.bak 0x49f98b research selector and 0x5b11d0 CRT',
        'source_sha256': hashlib.sha256(binary).hexdigest(),
        'original_x86_contexts': 512, 'committed_vectors': len(rows) - 1,
        'compared': ['selected advance', 'final CRT state', 'CRT draw count'],
        'replacement_callees': {
            '0x49ecb6': 'supplied availability', '0x4a2098': 'supplied group permission',
            '0x49c95d': 'supplied context-dependent advance value'},
        'scope': 'selection kernel only; full native values and campaign decisions pending',
    }, indent=2))


if __name__ == '__main__':
    main()
