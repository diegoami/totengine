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
    with args.out.open('x') as output:
        out = {'schema': 1, **{key: hut[key] for key in ('source', 'rules', 'binary_sha256')},
               'arrival': result, 'turn_45_arrival': later, 'scope': __doc__.strip()}
        output.write(json.dumps(out, indent=2) + '\n')


if __name__ == '__main__':
    main()
