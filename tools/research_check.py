#!/usr/bin/env python3
"""Check decoded research costs against the user's original x86 routine.

CIVTOT_HOME required; --emulate needs Unicorn (install outside the repo).
--vectors writes 128 deterministic derived cases for Rust regression tests.
--series optionally checks the existing std1 turn01..turn26 saved series.
No executable bytes or saves are emitted.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct

from totctl import parse_rules, walk_save


def trunc(a, b):
    return a // b if a >= 0 else -((-a) // b)


def decoded_cost(difficulty, human, acquired, adjustment, reference, turn,
                 advance_count, flags, paradigm, skip):
    raw = acquired + adjustment
    count = max(1, raw)
    factor = 2 * min(difficulty, 4) + 6 if human else 14 - min(difficulty, 4)
    if not flags & 128 or not skip:
        if reference > count:
            factor -= int(difficulty != 0)
            factor -= int(difficulty == 5 and reference > count + 4 and turn > 150)
        else:
            factor += trunc(count - reference, 3)
        if count >= 20:
            factor += max(0, min(6, count - trunc(turn, 8)))
    factor = trunc(factor * paradigm, 10)
    bonus = (factor * 3) >> 2
    if raw < 20:
        bonus = trunc(bonus * raw, 20)
    factor += bonus
    if advance_count > 67:
        factor = trunc(factor * 67, advance_count)
    if flags & 4:
        factor = trunc(factor * 5, 4)
    if flags & 8:
        factor = trunc(factor * 4, 5)
    if human:
        factor = max(factor, 11 - count)
    result = factor * count
    return result if 1 <= result <= 32000 else 32000


class OriginalCost:
    def __init__(self, binary):
        from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
        from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EAX, UC_X86_REG_EIP
        self.sp, self.ax, self.ip = UC_X86_REG_ESP, UC_X86_REG_EAX, UC_X86_REG_EIP
        self.cpu = Uc(UC_ARCH_X86, UC_MODE_32)
        self.cpu.mem_map(0x400000, 0x400000)
        self.cpu.mem_map(0x100000, 0x10000)
        pe = struct.unpack_from('<I', binary, 0x3c)[0]
        count = struct.unpack_from('<H', binary, pe + 6)[0]
        optional = struct.unpack_from('<H', binary, pe + 20)[0]
        for i in range(count):
            offset = pe + 24 + optional + 40 * i
            _, address, size, raw = struct.unpack_from('<IIII', binary, offset + 8)
            if size and address < 0x400000:
                self.cpu.mem_write(0x400000 + address, binary[raw:raw + size])

    def write(self, address, value, fmt):
        self.cpu.mem_write(address, struct.pack(fmt, value))

    def run(self, case):
        d, human, acquired, adjustment, reference, turn, advance_count, flags, paradigm, skip = case
        for address, value, fmt in [
            (0x607658, d, '<B'), (0x60765b, 2 * human, '<B'),
            (0x607648, turn, '<h'), (0x60766a, advance_count, '<h'),
            (0x607640, flags, '<H'), (0x607770, 2, '<B'),
            (0x5f5ec3, max(0, paradigm), '<B'),
            (0x5f5ea2, paradigm, '<h'), (0x5f5ea4, skip, '<h'),
            (0x5f68a0 + 3348 + 24, acquired, '<B'),
            (0x5f68a0 + 3348 + 26, adjustment, '<B'),
            (0x5f68a0 + 6696 + 24, reference, '<B'),
            (0x5f68a0 + 6696 + 26, 0, '<B')]:
            self.write(address, value, fmt)
        self.cpu.mem_write(0x108000, struct.pack('<II', 0x100000, 1))
        self.cpu.reg_write(self.sp, 0x108000)
        # Both cost and its only callee (clamp at 0x566a30) execute from
        # original bytes. Return to a sentinel; no Windows API is called.
        self.cpu.emu_start(0x4a1ae0, 0x100000, count=2000)
        assert self.cpu.reg_read(self.ip) == 0x100000, "original cost routine did not return"
        return self.cpu.reg_read(self.ax)


def save_cases(path, rules_path):
    data = path.read_bytes()
    rules = parse_rules(str(rules_path))
    walk = walk_save(str(path))
    flags = struct.unpack_from('<H', data, 660)[0]
    paradigm = int(rules['COSMIC']['rows'][11][0])
    skip = 0
    if flags & 128:
        leaders = 1 if data[678] == 0 else data[687].bit_count()
        offset = walk['cities']['offset'] + 92 * walk['cities']['count'] + 1453 + 60 * leaders
        paradigm, skip = struct.unpack_from('<hh', data, offset + 82)
    reference = data[964]
    ref_count = sum(data[2944 + 3348 * reference + k] for k in (24, 26))
    for civ in range(1, 8):
        base = 2944 + 3348 * civ
        case = (data[684], int(bool(data[687] & (1 << civ))), data[base + 24], data[base + 26],
                ref_count, struct.unpack_from('<h', data, 668)[0],
                struct.unpack_from('<h', data, 702)[0], flags, paradigm, skip)
        yield civ, case


def saved_series(root, rules):
    def read(path):
        data = path.read_bytes()
        walk = walk_save(str(path))
        base = walk['cities']['offset']
        science = [0] * 8
        for i in range(walk['cities']['count']):
            city = data[base + 92 * i:base + 92 * (i + 1)]
            science[city[10]] += struct.unpack_from('<H', city, 76)[0]
        return data, science
    files = [root / f'turn{i:02d}.sav' for i in range(1, 27)]
    increments, discoveries = [], []
    for first, second in zip(files, files[1:]):
        a, before_science = read(first)
        b, after_science = read(second)
        turn = struct.unpack_from('<h', a, 668)[0]
        if struct.unpack_from('<h', b, 668)[0] != turn + 1:
            continue
        cases = dict(save_cases(first, rules))
        for civ in range(1, 8):
            base = 2944 + 3348 * civ
            progress, target = struct.unpack_from('<hh', a, base + 16)
            after, after_target = struct.unpack_from('<hh', b, base + 16)
            # Isolate science by retaining one-turn pairs with a selected
            # target, non-anarchy, and stable positive stored city science.
            if target < 0 or a[base + 29] == 0 or before_science[civ] != after_science[civ] or not before_science[civ]:
                continue
            price = decoded_cost(*cases[civ])
            added = before_science[civ]
            same = a[base + 96:base + 109] == b[base + 96:base + 109]
            if same and a[base + 24] == b[base + 24] and target == after_target:
                assert progress + added == after, (first.name, civ, progress, added, after)
                increments.append([turn, civ, progress, added, after])
            elif (b[base + 96 + target // 8] & (1 << (target % 8))
                  and b[base + 24] == a[base + 24] + 1 and progress + added >= price and after == 0):
                discoveries.append([turn, civ, target, price, progress, added, after])
    assert increments and discoveries
    return {'method': 'existing std1 saves; one-turn pairs with stable positive city science',
            'increments_checked': len(increments), 'discoveries_checked': len(discoveries),
            'increment_columns': ['turn', 'civ', 'before', 'science', 'after'],
            'increments': increments, 'discovery_columns': ['turn', 'civ', 'target', 'cost', 'before', 'science', 'after'],
            'discoveries': discoveries}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--emulate', action='store_true')
    parser.add_argument('--vectors', type=Path)
    parser.add_argument('--series', type=Path)
    args = parser.parse_args()
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    emulator = OriginalCost(binary) if args.emulate else None
    if args.vectors and not emulator:
        parser.error('--vectors requires --emulate')
    result = {'source': 'civ2.bak 0x4a1ae0 cost; 0x4a1ebf accumulation; 0x4a145e completion',
              'source_sha256': hashlib.sha256(binary).hexdigest(), 'save_costs': []}
    for save, rules in [('Original/Ph_Auto2.SAV', 'Original/Rules.txt'), ('FallRome/ROMFALL.scn', 'FallRome/Rules.txt')]:
        costs = []
        for civ, case in save_cases(home / save, home / rules):
            price = decoded_cost(*case)
            if emulator:
                assert price == emulator.run(case), (save, civ, case, price)
            costs.append(price)
        result['save_costs'].append({'save': save, 'civs': list(range(1, 8)), 'costs': costs})
    if emulator:
        rng = random.Random(82)
        cases = []
        for _ in range(1000):
            d, h = rng.randrange(6), rng.randrange(2)
            acquired, adjustment, reference = rng.randrange(110), rng.randrange(10), rng.randrange(110)
            turn = rng.randrange(500)
            advance_count = rng.choice([33, 50, 67, 89, 100, 200])
            flags = rng.choice([0, 4, 8, 12, 128, 132, 136])
            paradigm, skip = rng.randrange(-5, 30), rng.choice([0, 1, -5])
            if not flags & 128:
                paradigm = max(0, paradigm)
            case = (d, h, acquired, adjustment, reference, turn, advance_count, flags, paradigm, skip)
            expected = emulator.run(case)
            assert decoded_cost(*case) == expected, (case, expected)
            cases.append((*case, expected))
        result['emulated_cases_exact'] = 1014
        if args.vectors:
            header = '# difficulty human acquired adjustment reference turn advance_count flags paradigm skip expected\n'
            args.vectors.write_text(header + ''.join(' '.join(map(str, row)) + '\n' for row in cases[:128]))
    if args.series:
        result['saved_series'] = saved_series(args.series, home / 'Original/Rules.txt')
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == '__main__':
    main()
