#!/usr/bin/env python3
"""Measure the prepared armed arrival into a friendly field stack.

Use the pinned round-close context, then supply the measured mover and
resident records, map bytes and CRT from the checked engine approach.
Native movement runs through return with the established closed-panel
presentation substitutions.
"""
import argparse
import json
import os
from pathlib import Path

from import_event_tail_check import digest
from import_opponent_combat_view_check import CRT, TILES, run_attack
from import_phase_check import read
from import_round_close_check import prepare_round
from import_unit_setup_check import UNIT_RECORDS
from totctl import parse_rules


def arrival(original, viewer, _city, seed, *, later=False):
    width = read(original, 0x6705A0, '<h')
    area = read(original, 0x6705A4, '<h')
    original.write(CRT, seed, '<I')
    original.write(0x607655, 7, '<B')
    if later:
        original.write(0x607648, 45, '<h')
    original.write(0x5D156C, 0, '<B')
    cache = read(original, 0x5DEAD0 + 4 * viewer, '<I')

    def unit(slot):
        address = UNIT_RECORDS + 40 * slot
        return {
            'slot': slot, 'position': [read(original, address + i, '<h') for i in (0, 2)],
            'previous': read(original, address + 26, '<h'), 'next': read(original, address + 28, '<h'),
            'spent': read(original, address + 10, '<B'), 'direction': read(original, address + 13, '<B'),
        }

    def knowledge():
        cpu = original.cpu
        tiles = cpu.mem_read(TILES, area * 6)
        return bytes(b & (1 << viewer) for b in tiles[4::6]), bytes(cpu.mem_read(cache, area))

    assert unit(24)['next'] == 4 and unit(4)['previous'] == 24
    origin, target, charge = (47, 49, 6) if later else (45, 47, 3)
    for slot, y, spent, orders in ((4, target, charge, 6), (24, origin, 0, 2)):
        address = UNIT_RECORDS + 40 * slot
        original.write(address + 2, y, '<h')
        original.write(address + 26, -1, '<h')
        original.write(address + 28, -1, '<h')
        original.write(address + 10, spent, '<B')
        original.write(address + 17, orders, '<B')
    tiles = ((47, [1, 1, 224, 2, 128, 115]), (49, [7, 1, 224, 2, 128, 112])) if later else (
        (45, [3, 17, 224, 2, 128, 112]), (47, [1, 1, 224, 2, 128, 115]),
    )
    for y, raw in tiles:
        original.cpu.mem_write(TILES + 6 * (y * (width // 2) + 4), bytes(raw))
    before = [unit(24), unit(4)]
    visible, memory = knowledge()
    result, hosts, display, observations = run_attack(original, (24, 3, 3))
    after = [unit(24), unit(4)]
    assert result == 1 and observations == [[24, 1]]
    assert after == [
        {'slot': 24, 'position': [9, target], 'previous': -1, 'next': 4, 'spent': charge, 'direction': 3},
        {'slot': 4, 'position': [9, target], 'previous': 24, 'next': -1, 'spent': charge, 'direction': 5},
    ]
    assert (visible, memory) == knowledge() and read(original, CRT, '<I') == seed
    return {
        'arguments': [24, 3, 3], 'before': before, 'after': after, 'crt_state': seed,
        'explored_tiles': sum(bool(mask) for mask in visible), 'viewer_memory_unchanged': True,
        'observation_calls': observations, 'host_substitutes': hosts, 'display_substitutes': display,
    }


def turn_47_arrival(original, viewer, _city):
    width = read(original, 0x6705A0, '<h')
    area = read(original, 0x6705A4, '<h')
    seed = 0x3b0ca297
    original.write(CRT, seed, '<I')
    original.write(0x607655, 7, '<B')
    original.write(0x607648, 47, '<h')
    original.write(0x5D156C, 0, '<B')
    cache = read(original, 0x5DEAD0 + 4 * viewer, '<I')
    cpu = original.cpu

    def tile(x, y):
        return TILES + 6 * (y * (width // 2) + x // 2)

    def knowledge():
        tiles = cpu.mem_read(TILES, area * 6)
        return bytes(b & (1 << viewer) for b in tiles[4::6]), bytes(cpu.mem_read(cache, area))

    def address(slot):
        return UNIT_RECORDS + slot * 40

    def snapshot(slot):
        at = address(slot)
        return {
            'slot': slot, 'position': [read(original, at + off, '<h') for off in (0, 2)],
            'previous': read(original, at + 26, '<h'), 'next': read(original, at + 28, '<h'),
            'spent': read(original, at + 10, '<B'), 'direction': read(original, at + 13, '<B'),
        }

    for slot, x, spent, orders in ((28, 12, 0, 1), (7, 10, 6, 28)):
        at = address(slot)
        previous, next_unit = (read(original, at + off, '<h') for off in (26, 28))
        if previous >= 0:
            original.write(address(previous) + 28, next_unit, '<h')
        if next_unit >= 0:
            original.write(address(next_unit) + 26, previous, '<h')
        if previous == next_unit == -1:
            old = tile(*snapshot(slot)['position']) + 1
            original.write(old, read(original, old, '<B') & ~1, '<B')
        for off, value in ((0, x), (2, 44), (26, -1), (28, -1)):
            original.write(at + off, value, '<h')
        for off, value in ((6, 0), (10, spent), (12, 0), (13, 255), (17, orders)):
            original.write(at + off, value, '<B')
    cpu.mem_write(tile(12, 44), bytes([4, 1, 224, 2, 128, 112]))
    cpu.mem_write(tile(10, 44), bytes([3, 1, 224, 2, 128, 112]))

    visible, memory = knowledge()
    args = (28, 5, 3)
    # The old observation snapshot carried nine spent thirds into this turn.
    # Run that input first and require the move to be refused without writes.
    original.write(address(28) + 10, 9, '<B')
    capture = [(address(slot), 40) for slot in (28, 7)] + [(TILES, area * 6), (cache, area)]
    saved = [bytes(cpu.mem_read(start, size)) for start, size in capture]
    refused, _, _, refused_observations = run_attack(original, args)
    assert refused == 0 and not refused_observations
    after_refusal = [bytes(cpu.mem_read(start, size)) for start, size in capture]
    assert saved[2:] == after_refusal[2:]
    refused_changes = [(slot, offset, before, after)
                       for slot, old, new in zip((28, 7), saved[:2], after_refusal[:2], strict=True)
                       for offset, (before, after) in enumerate(zip(old, new, strict=True))
                       if before != after]
    for (start, _), data in zip(capture[:2], saved[:2], strict=True):
        cpu.mem_write(start, data)
    assert read(original, CRT, '<I') == seed

    original.write(address(28) + 10, 0, '<B')
    before = [snapshot(28), snapshot(7)]
    result, hosts, display, observations = run_attack(original, args)
    after = [snapshot(28), snapshot(7)]
    assert result == 1 and observations == [[28, 1]], (result, observations, after)
    assert after == [
        {'slot': 28, 'position': [10, 44], 'previous': -1, 'next': 7, 'spent': 6, 'direction': 5},
        {'slot': 7, 'position': [10, 44], 'previous': 28, 'next': -1, 'spent': 6, 'direction': 255},
    ]
    assert read(original, address(28) + 6, '<B') == 0x40, read(original, address(28) + 6, '<B')
    assert (visible, memory) == knowledge() and read(original, CRT, '<I') == seed
    return {
        'arguments': args, 'before': before, 'after': after, 'crt_state': seed,
        'stale_movement_refused': {'spent': 9, 'return': refused, 'unit_record_changes': refused_changes},
        'explored_tiles': sum(bool(mask) for mask in visible), 'viewer_memory_unchanged': True,
        'observation_calls': observations, 'host_substitutes': hosts, 'display_substitutes': display,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent / 'docs/evidence'
    hut = json.loads((root / 'import-opponent-hut.json').read_text())
    annual = json.loads((root / 'import-round-close.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(binary) == hut['binary_sha256'] == annual['binary_sha256']
    for role in ('source', 'rules'):
        item = hut[role]
        assert item == annual[role] and digest((home / item['path']).read_bytes()) == item['sha256']
    rules = parse_rules(home / hut['rules']['path'])
    prepared, result = prepare_round(
        home, binary, hut['source']['path'], rules,
        lambda original, viewer, city: arrival(original, viewer, city, hut['hut']['crt_after']),
    )
    assert prepared['annual_update'] == annual['annual_update']
    assert result['explored_tiles'] == 59
    prepared, later = prepare_round(
        home, binary, hut['source']['path'], rules,
        lambda original, viewer, city: arrival(original, viewer, city, 0x34bba6a0, later=True),
    )
    assert prepared['annual_update'] == annual['annual_update']
    assert later['explored_tiles'] == 59
    prepared, turn_47 = prepare_round(
        home, binary, hut['source']['path'], rules, turn_47_arrival,
    )
    assert prepared['annual_update'] == annual['annual_update']
    assert turn_47['explored_tiles'] == 59
    with args.out.open('x') as output:
        out = {'schema': 1, **{key: hut[key] for key in ('source', 'rules', 'binary_sha256')},
               'arrival': result, 'turn_45_arrival': later, 'turn_47_arrival': turn_47, 'scope': __doc__.strip()}
        output.write(json.dumps(out, indent=2) + '\n')


if __name__ == '__main__':
    main()
