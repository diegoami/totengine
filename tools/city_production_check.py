#!/usr/bin/env python3
"""Execute original city purchase, sale and shield-retention arithmetic.

Only numeric derived data is retained. Current-city and description UI helpers
are substituted in isolated fragments; arithmetic, clamp and packed setters
execute from the user's unpatched civ2.bak. This does not drive native dialogs.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct

from research_check import OriginalCost


class NativeProduction(OriginalCost):
    frame = 0x108000

    def prepare(self):
        from unicorn.x86_const import UC_X86_REG_EBP
        self.cpu.mem_write(self.frame - 0x1800, bytes(0x1800))
        self.cpu.reg_write(UC_X86_REG_EBP, self.frame)
        self.cpu.reg_write(self.sp, self.frame - 0x1000)

    def fragment(self, start, end, alternate=()):
        from unicorn import UC_HOOK_CODE
        hit = None

        def ui(cpu, address, size, data):
            nonlocal hit
            if address in alternate:
                hit = address
                cpu.reg_write(self.ip, end)
            elif address in (0x411c50, 0x41f4b8):
                sp = cpu.reg_read(self.sp)
                ret = struct.unpack('<I', cpu.mem_read(sp, 4))[0]
                cpu.reg_write(self.ax, 0)
                cpu.reg_write(self.sp, sp + 4)
                cpu.reg_write(self.ip, ret)

        hook = self.cpu.hook_add(UC_HOOK_CODE, ui)
        try:
            self.cpu.emu_start(start, end, count=10000)
            assert self.cpu.reg_read(self.ip) == end
        finally:
            self.cpu.hook_del(hook)
        return hit if hit is not None else end

    def call(self, start, *args):
        self.cpu.mem_write(self.frame, struct.pack('<' + 'I' * (len(args) + 1), 0x100000, *(a & 0xffffffff for a in args)))
        self.cpu.reg_write(self.sp, self.frame)
        self.cpu.emu_start(start, 0x100000, count=10000)
        assert self.cpu.reg_read(self.ip) == 0x100000
        value = self.cpu.reg_read(self.ax)
        return value if value < 0x80000000 else value - 0x100000000

    def hurry(self, item, cost, rows, stored):
        self.prepare()
        self.write(self.frame - 0x6e4, item, '<i')
        self.write(self.frame - 0x6f0, -item, '<i')
        self.write(0x5fd18b, item, '<b')
        self.write(0x5fd16e, stored, '<h')
        self.write(0x5f5ebc, rows, '<B')
        self.write((0x5f4710 + item * 40) if item >= 0 else (0x5f6684 - item * 8), cost, '<B')
        # Capitalization rejection is directly observed at the native branch.
        if item == -39:
            self.fragment(0x4eb474, 0x4eb481)
            return -1
        self.fragment(0x4eb474, 0x4eb5fd)
        return struct.unpack('<i', self.cpu.mem_read(self.frame - 0x6e0, 4))[0]

    def retention(self, old, new, stored, difficulty, penalty):
        self.write(0x5fd18b, old, '<b')
        self.write(0x5fd16e, stored, '<h')
        self.write(0x607658, difficulty, '<B')
        self.write(0x5f5eca, penalty, '<B')
        self.call(0x4c0ee6, 0)  # Native capture of initial category + shields.
        return self.call(0x4c0f37, 0, new)

    def sale(self, cost, rows):
        self.prepare()
        self.write(self.frame - 0x20, 2, '<i')
        self.write(0x5f6684 + 2 * 8, cost, '<B')
        self.write(0x5f5ebc, rows, '<B')
        self.fragment(0x4e4737, 0x4e4750)
        return struct.unpack('<i', self.cpu.mem_read(self.frame - 0xc, 4))[0]

    def buy_commit(self, item, cost, rows, stored, treasury):
        price = self.hurry(item, cost, rows, stored)
        assert 0 < price <= treasury
        self.write(self.frame - 0x6f4, 1, '<i')
        self.write(0x5f68aa + 3348, treasury, '<i')
        self.fragment(0x4eb7cf, 0x4eb83c)
        return (price, struct.unpack('<i', self.cpu.mem_read(0x5f68aa + 3348, 4))[0],
                struct.unpack('<H', self.cpu.mem_read(0x5fd16e, 2))[0],
                struct.unpack('<i', self.cpu.mem_read(0x653824, 4))[0],
                struct.unpack('<i', self.cpu.mem_read(0x6537a8, 4))[0])

    def sell_commit(self, improvement, cost, rows, treasury):
        income = self.sale(cost, rows)
        self.cpu.mem_write(0x5fd150, bytes(92))
        self.write(0x5fd15a, 1, '<B')
        self.cpu.mem_write(0x5fd186, bytes([255] * 5))
        self.write(self.frame - 0x20, improvement, '<i')
        self.write(0x5f68aa + 3348, treasury, '<i')
        self.fragment(0x4e4895, 0x4e4909)
        gold = struct.unpack('<i', self.cpu.mem_read(0x5f68aa + 3348, 4))[0]
        buildings = int.from_bytes(self.cpu.mem_read(0x5fd186, 5), 'little')
        flags = struct.unpack('<I', self.cpu.mem_read(0x5fd156, 4))[0]
        return income, gold, buildings, flags

    def hurry_allowed(self, item, flags, price, treasury):
        self.prepare()
        self.write(0x5fd18b, item, '<b')
        self.write(0x5fd156, flags, '<I')
        self.write(0x5f68aa + 3348, treasury, '<i')
        self.write(self.frame - 0x6f4, 1, '<i')
        self.write(self.frame - 0x6e0, price, '<i')
        self.fragment(0x4eb644, 0x4eb6c8)
        return struct.unpack('<i', self.cpu.mem_read(self.frame - 0x6dc, 4))[0]

    def sale_allowed(self, flags):
        self.prepare()
        self.write(0x5fd156, flags, '<I')
        self.write(0x607657, 0, '<B')  # Ordinary play, no editor override.
        self.write(0x607640, 0, '<H')
        end = self.fragment(0x4e45e1, 0x4e464d, (0x4e4620,))
        return int(end == 0x4e464d)

    def switch_sequence(self, initial, stored, difficulty, penalty, steps):
        self.write(0x5fd18b, initial, '<b')
        self.write(0x5fd16e, stored, '<h')
        self.write(0x607658, difficulty, '<B')
        self.write(0x5f5eca, penalty, '<B')
        self.call(0x4c0ee6, 0)
        result = []
        for item in steps:
            retained = self.call(0x4c0f37, 0, item)
            self.prepare()
            self.write(self.frame - 0x10, item, '<i')
            self.write(self.frame - 0x64, retained, '<i')
            self.write(self.frame - 0x13f4, 1, '<i')
            self.fragment(0x4ecd16, 0x4ecdcf)
            actual_item = struct.unpack('<b', self.cpu.mem_read(0x5fd18b, 1))[0]
            actual_shields = struct.unpack('<H', self.cpu.mem_read(0x5fd16e, 2))[0]
            result.append((actual_item, actual_shields))
        return result


def expected_hurry(item, cost, rows, stored):
    if item == -39:
        return -1
    signed_cost = cost if item < 0 or cost < 128 else cost - 256
    remaining = max(0, min(999, signed_cost * rows - stored))
    price = 2 * remaining
    if item >= 0:
        price += remaining * remaining // 20
    elif item <= -36:
        price *= 2
    return price * (2 if stored == 0 else 1)


def category(item):
    return 0 if item >= 0 else 1 if item > -36 else 2


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--gold', type=Path, required=True)
    p.add_argument('--vectors', type=Path, required=True)
    args = p.parse_args()
    binary = (Path(os.environ['CIVTOT_HOME']) / 'civ2.bak').read_bytes()
    native = NativeProduction(binary)
    rng = random.Random(140148)
    lines = []
    items = [0, 1, 17, -1, -35, -36, -38, -39, -40, -67]
    for item in items:
        for cost in (0, 1, 4, 17, 127, 255):
            for rows in (1, 7, 10, 13, 127):
                for stored in (-1, 0, 1, 40, 999, 32000):
                    got = native.hurry(item, cost, rows, stored)
                    assert got == expected_hurry(item, cost, rows, stored), (item, cost, rows, stored, got)
                    lines.append(f'H {item} {cost} {rows} {stored} {got}')
    for _ in range(1024):
        old, new = rng.choice(items), rng.choice(items)
        stored = rng.choice([-32768, -1, 0, 1, 3, 17, 50, 999, 32767])
        difficulty, penalty = rng.randrange(6), rng.choice([0, 25, 50, 75, 100, 127, 255])
        got = native.retention(old, new, stored, difficulty, penalty)
        expected = stored if difficulty == 0 or category(old) == category(new) else stored - (stored & 65535) * penalty // 100
        assert got == expected, (old, new, stored, difficulty, penalty, got)
        lines.append(f'P {old} {new} {stored} {difficulty} {penalty} {got}')
    for cost in (0, 1, 4, 17, 127, 255):
        for rows in (0, 1, 7, 10, 13, 127, 255):
            got = native.sale(cost, rows)
            assert got == cost * rows
            lines.append(f'S {cost} {rows} {got}')
    buys = sales = 0
    for item in (0, 1, -1, -35, -36, -38, -40, -67):
        for cost in (1, 4, 17, 127):
            for rows in (7, 10, 13):
                for stored in (0, 1, 3, 40):
                    if stored >= cost * rows:
                        continue
                    price = expected_hurry(item, cost, rows, stored)
                    treasury = price + rng.randrange(5000)
                    got = native.buy_commit(item, cost, rows, stored, treasury)
                    assert got == (price, treasury - price, cost * rows, category(item), cost * rows)
                    lines.append(f'B {item} {cost} {rows} {stored} {treasury} ' + ' '.join(map(str, got)))
                    buys += 1
    for improvement in range(2, 36):
        for cost, rows in ((1, 7), (4, 10), (17, 13)):
            treasury = rng.randrange(5000)
            got = native.sell_commit(improvement, cost, rows, treasury)
            expected = (cost * rows, treasury + cost * rows, ((1 << 40) - 1) & ~(1 << improvement), 4)
            assert got == expected, (improvement, cost, rows, treasury, got)
            lines.append(f'C {improvement} {cost} {rows} {treasury} ' + ' '.join(map(str, got)))
            sales += 1
    for item in (0, 1, -1, -36, -67):
        price = expected_hurry(item, 4, 10, 11)
        for flags in (0, 1, 4, 5, 0x20000):
            for treasury in (-1, 0, price - 1, price, price + 1):
                got = native.hurry_allowed(item, flags, price, treasury)
                assert got == int(treasury >= price and not (item >= 0 and flags & 1))
                lines.append(f'Q {item} 4 10 11 {flags} {treasury} {price} {got}')
    for flags in (0, 1, 4, 5, 0x20000, 0xffffffff):
        got = native.sale_allowed(flags)
        assert got == int(not flags & 4)
        lines.append(f'L {flags} {got}')
    sequences = 0
    steps = [-5, 0, -40, -4]
    for initial in (0, -4, -40):
        for stored in (0, 1, 3, 17, 51, 999):
            for difficulty in (0, 2, 5):
                for penalty in (0, 50, 100):
                    results = native.switch_sequence(initial, stored, difficulty, penalty, steps)
                    for step, (item, actual) in enumerate(zip(steps, results)):
                        expected = stored if difficulty == 0 or category(initial) == category(item) else stored - stored * penalty // 100
                        assert actual == (item, expected)
                        lines.append(f'K {sequences} {step} {initial} {stored} {difficulty} {penalty} {item} {actual[0]} {actual[1]}')
                    sequences += 1
    args.vectors.write_text(
        '# H item cost rows stored price_or_minus_one\n'
        '# P old new stored difficulty penalty retained | S cost rows income\n'
        '# B item cost rows stored treasury price next_treasury next_box captured_class captured_budget\n'
        '# C improvement cost rows treasury income next_treasury next_buildings next_flags\n'
        '# Q item cost rows stored flags treasury price admitted | L flags sale_admitted\n'
        '# K sequence step initial_item initial_shields difficulty penalty selected_item actual_item actual_shields\n'
        + '\n'.join(lines) + '\n')
    report = {
        'schema': 1, 'binary_sha256': hashlib.sha256(binary).hexdigest(),
        'cases': len(lines), 'hurry_cases': 1800, 'retention_cases': 1024, 'sale_cases': 42,
        'confirmed_buy_cases': buys, 'confirmed_sale_cases': sales,
        'buy_funds_disorder_cases': 125, 'standard_sale_gate_cases': 6,
        'prepared_switch_sequences': sequences, 'prepared_switch_transitions': sequences * len(steps),
        'hurry_entry_stop': ['0x4eb474', '0x4eb5fd'], 'capitalization_stop': '0x4eb481',
        'retention_capture_and_kernel': ['0x4c0ee6', '0x4c0f37'],
        'accepted_switch_entry_stop': ['0x4ecd16', '0x4ecdcf'],
        'sale_entry_stop': ['0x4e4737', '0x4e4750'],
        'confirmed_buy_entry_stop': ['0x4eb7cf', '0x4eb83c'],
        'confirmed_sale_entry_stop': ['0x4e4895', '0x4e4909'],
        'buy_funds_disorder_entry_stop': ['0x4eb644', '0x4eb6c8'],
        'standard_sale_gate_entry_stops': ['0x4e45e1', '0x4e464d', '0x4e4620'],
        'ui_substitutions': {'0x411c50': 'current city index 0', '0x41f4b8': 'omit item-name presentation'},
        'scope': 'Unpatched original arithmetic and confirmed ledger/building/shield writes, including clamp/category/packed-building helpers, on synthetic fields. Separate fragments check treasury/disorder admission and the already-sold gate in ordinary non-editor play. UI access/name formatting are substituted. Commit fragments assume caller confirmation and stop before refresh/render/notifications. Full dialog legality, total refresh and complete city-window baseline lifecycle are not established. Retention takes the captured initial category/shield context; callers must not invent a fresh or client-supplied baseline.',
    }
    args.gold.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report))


if __name__ == '__main__':
    main()
