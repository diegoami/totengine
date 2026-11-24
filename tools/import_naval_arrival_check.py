#!/usr/bin/env python3
"""Measure the stock barbarian sea arrival at the close of turn 47.

Run the native arrival routine in the prepared stock round context with
the checked turn and CRT. Restore the saved landmass table. Presentation
and network calls are suppressed. Allocation and orders execute natively.
"""
import argparse
import json
import os
from pathlib import Path

from unicorn import UC_HOOK_CODE
from import_round_close_check import prepare_round
from import_event_tail_check import digest
from import_opponent_combat_view_check import CRT, TILES, run_attack
from import_unit_setup_check import UNIT_RECORDS
from import_phase_check import read
from totctl import parse_rules, walk_save_bytes


def arrivals(original, viewer, _city, source):
    cpu = original.cpu
    section = walk_save_bytes(source)['sections_derived']['blob_10240']
    assert section['size'] == 10240
    offset = section['offset']
    cpu.mem_write(0x61BCA8, source[offset:offset + 10240])
    original.write(0x607648, 47, '<h')
    original.write(CRT, 0x3B0CA297, '<I')
    original.write(0x6031b8, 1, '<I')
    original.write(0x607655, viewer, '<B')
    area = read(original, 0x6705A4, '<h')
    seen = read(original, 0x5DEAD0 + 4 * viewer, '<I')
    hosts = []
    watched = (0x569C88, 0x40CEF0, 0x42095E, 0x467D19)

    def capture():
        count = read(original, 0x607666, '<h')
        ids = {}
        for slot in range(count):
            at = UNIT_RECORDS + 40 * slot
            if read(original, at + 30):
                ids[slot] = (
                    read(original, at, '<h'), read(original, at + 2, '<h'),
                    read(original, at + 8, '<B'), read(original, at + 9, '<B'),
                    read(original, at + 17, '<B'), read(original, at + 6, '<H'),
                    read(original, at + 26, '<h'), read(original, at + 28, '<h'),
                )
        return ids, bytes(cpu.mem_read(TILES, area * 6)), bytes(cpu.mem_read(seen, area))

    def boundary(cpu, address, size, data):
        if address not in watched:
            return
        sp = cpu.reg_read(original.sp)
        hosts.append(hex(address))
        cpu.reg_write(original.ax, 0)
        cpu.reg_write(original.sp, sp + 4)
        cpu.reg_write(original.ip, read(original, sp))

    before, old_tiles, old_memory = capture()
    hook = cpu.hook_add(UC_HOOK_CODE, boundary)
    try:
        original.call(0x46E6CD)
    finally:
        cpu.hook_del(hook)
    after, tiles, memory = capture()
    assert cpu.reg_read(original.ip) == 0x100000
    mapped = {slot: unit for slot, unit in after.items() if slot not in before}
    assert [(u[0], u[1], u[2], u[3], u[4], u[5]) for u in mapped.values()] == [
        (0, 40, 46, 0, 3, 0), (0, 40, 4, 0, 3, 0), (0, 40, 32, 0, 255, 8192),
    ]
    assert old_memory == memory
    assert [b & (1 << viewer) for b in old_tiles[4::6]] == [b & (1 << viewer) for b in tiles[4::6]]
    assert read(original, CRT, '<I') == 0x819a8b33
    return {
        'viewer': viewer, 'turn': 47,
        'units': [
            {'slot': slot, 'position': [x, y], 'type': kind, 'owner': owner,
             'orders': orders, 'veteran': bool(attr & 0x2000), 'previous': prev, 'next': nxt}
            for slot, (x, y, kind, owner, orders, attr, prev, nxt) in mapped.items()
        ],
        'tile_changes': [(i // 6, i % 6, old, new)
                         for i, (old, new) in enumerate(zip(old_tiles, tiles, strict=True)) if old != new],
        'explored_tiles': sum(bool(mask & (1 << viewer)) for mask in tiles[4::6]),
        'viewer_memory_unchanged': True, 'crt_before': 0x3b0ca297,
        'crt_after_arrival': read(original, CRT, '<I'),
        'substitutes': hosts,
    }


def field_move(original, viewer, arrival):
    """Run the eastward turn-48 order with the distant native stack still present."""
    cpu = original.cpu
    original.write(CRT, 0x29e9892a, '<I')
    original.write(0x607648, 48, '<h')
    original.write(0x607655, 2, '<B')
    unit = UNIT_RECORDS
    origin = tuple(read(original, unit + offset, '<h') for offset in (0, 2))
    assert tuple(read(original, unit + offset, '<h') for offset in (26, 28)) == (-1, -1)
    width = read(original, 0x6705a0, '<h')
    area = read(original, 0x6705a4, '<h')

    def tile(x, y):
        return TILES + 6 * (y * (width // 2) + x // 2)

    old = tile(*origin) + 1
    original.write(old, read(original, old, '<B') & ~1, '<B')
    for offset, value in ((0, 62), (2, 36), (26, -1), (28, -1)):
        original.write(unit + offset, value, '<h')
    for offset, value in ((6, 0), (7, 2), (8, 0), (9, 2), (10, 0), (11, 0),
                          (12, 0), (13, 2), (16, 0), (17, 7), (18, 255)):
        original.write(unit + offset, value, '<B')
    cpu.mem_write(tile(62, 36), bytes([8, 1, 0, 2, 4, 32]))
    cpu.mem_write(tile(64, 36), bytes([9, 0, 0, 2, 4, 32]))
    remembered = read(original, 0x5DEAD0 + 4 * viewer, '<I')

    def capture():
        return (
            [bytes(cpu.mem_read(UNIT_RECORDS + 40 * record['slot'], 40)) for record in arrival['units']],
            [byte & (1 << viewer) for byte in cpu.mem_read(TILES, area * 6)[4::6]],
            bytes(cpu.mem_read(remembered, area)),
        )

    before = capture()
    result, hosts, display, observation = run_attack(original, (0, 1, 3))
    after = capture()
    assert before == after
    assert result == 1 and observation == [[0, 1]]
    position = tuple(read(original, unit + offset, '<h') for offset in (0, 2))
    assert position == (64, 36)
    spent, direction, orders = (read(original, unit + offset, '<B') for offset in (10, 13, 17))
    assert (spent, direction, orders) == (6, 1, 7)
    assert read(original, CRT, '<I') == 0x29e9892a
    return {
        'turn': 48, 'unit': 0, 'from': [62, 36], 'to': position,
        'spent': spent, 'direction': direction, 'orders': orders,
        'unit_flags': read(original, unit + 6, '<H'), 'crt': read(original, CRT, '<I'),
        'remote_stack_unchanged': True, 'viewer_memory_unchanged': True,
        'explored_tiles': sum(bool(mask) for mask in after[1]),
        'host_substitutes': hosts, 'display_substitutes': display,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    home = Path(os.environ['CIVTOT_HOME'])
    evidence = Path(__file__).resolve().parent.parent / 'docs/evidence/import-round-close.json'
    pinned = json.loads(evidence.read_text())
    source = (home / pinned['source']['path']).read_bytes()
    rules_path = home / pinned['rules']['path']
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(source) == pinned['source']['sha256']
    assert digest(rules_path.read_bytes()) == pinned['rules']['sha256']
    assert digest(binary) == pinned['binary_sha256']
    def measure(original, viewer, city):
        arrival = arrivals(original, viewer, city, source)
        return arrival, field_move(original, viewer, arrival)

    prepared, (native, movement) = prepare_round(
        home, binary, pinned['source']['path'], parse_rules(rules_path),
        measure,
    )
    assert prepared['annual_update'] == pinned['annual_update']
    assert native['explored_tiles'] == 59
    with args.out.open('x') as out:
        keys = ('source', 'rules', 'binary_sha256')
        out.write(json.dumps({'schema': 1, **{key: pinned[key] for key in keys},
                              'arrival': native, 'turn_48_movement': movement, 'scope': __doc__.strip()}, indent=2) + '\n')


if __name__ == '__main__':
    main()
