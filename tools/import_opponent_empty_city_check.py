#!/usr/bin/env python3
"""Measure the prepared opponent entry into an own empty city.

Reuse the measured second founding to create city slot 10 at (27, 17)
natively, then supply the checked engine's mover and resident records,
city record, tile bytes and CRT for the turn-48 southward approach from
(27, 15). Native movement runs through return with the established
closed-panel presentation substitutions and leaves the human viewer's
explored tiles and memory untouched.
"""
import argparse
import json
import os
from pathlib import Path

from unicorn import UC_HOOK_CODE

from import_event_tail_check import digest
from import_opponent_combat_view_check import CRT, TILES, run_attack
from import_opponent_founding_check import measure as founding
from import_phase_check import CIV, STRIDE, read
from import_round_close_check import prepare_round
from import_unit_setup_check import UNIT_RECORDS
from totctl import parse_rules

CRT_STATE = 0x29E9892A
MOVER = bytes([27, 0, 15, 0, 0, 0, 64, 0, 15, 6, 0, 0, 0, 5, 57, 0, 0, 28,
               255, 255, 26, 0, 24, 0, 255, 255, 255, 255, 14, 0, 31, 0,
               0, 0, 2, 0, 6, 0, 0, 0])
RESIDENT = bytes([27, 0, 15, 0, 0, 0, 192, 10, 0, 6, 0, 0, 0, 1, 49, 0, 0, 255,
                  1, 0, 25, 0, 9, 0, 255, 255, 1, 0, 255, 255, 20, 0,
                  0, 0, 2, 0, 2, 0, 0, 0])
CITY = bytes([27, 0, 17, 0, 0, 0, 128, 0, 2, 0, 6, 1, 6, 0, 0, 0, 0, 0, 0, 0,
              0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 32, 51, 0, 0, 0, 0,
              0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 18, 0, 0, 0, 0, 0, 0, 3,
              0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
              0, 0, 0, 0, 0, 0, 12, 0, 0, 0, 0, 0])
TILE_ORIGIN = [9, 1, 192, 2, 64, 96]
TILE_DEST = [2, 2, 192, 2, 64, 99]


def differences(before, after):
    return [(i, old, new) for i, (old, new) in enumerate(zip(before, after, strict=True)) if old != new]


def entry(original, viewer, city_region, rules, city_lines):
    width = read(original, 0x6705A0, '<h')
    area = read(original, 0x6705A4, '<h')
    cpu = original.cpu

    def tile(x, y):
        return TILES + 6 * (y * (width // 2) + x // 2)

    founded = founding(original, viewer, city_region, rules, city_lines, 6)
    assert founded['city_slot'] == 10 and founded['explored_tiles'] == 59

    original.write(0x607648, 48, '<h')
    original.write(0x607655, 6, '<B')
    original.write(0x5D156C, 0, '<B')
    original.write(CRT, CRT_STATE, '<I')
    caches = [read(original, 0x5DEAD0 + 4 * civ, '<I') for civ in range(1, 8)]
    city = city_region[0] + 92 * 10
    cpu.mem_write(UNIT_RECORDS + 40 * 1, MOVER)
    cpu.mem_write(UNIT_RECORDS + 40 * 14, RESIDENT)
    cpu.mem_write(city, CITY)
    cpu.mem_write(tile(27, 15), bytes(TILE_ORIGIN))
    cpu.mem_write(tile(27, 17), bytes(TILE_DEST))
    assert (read(original, city + 10, '<B'), read(original, city + 11, '<B')) == (6, 1)
    assert read(original, tile(27, 17) + 1, '<B') == 2

    def knowledge():
        tiles = cpu.mem_read(TILES, area * 6)
        return (bytes(b & (1 << viewer) for b in tiles[4::6]),
                [bytes(cpu.mem_read(c, area)) for c in caches])

    def unit(slot):
        address = UNIT_RECORDS + 40 * slot
        return {
            'slot': slot,
            'position': [read(original, address + off, '<h') for off in (0, 2)],
            'previous': read(original, address + 26, '<h'),
            'next': read(original, address + 28, '<h'),
            'spent': read(original, address + 10, '<B'),
            'direction': read(original, address + 13, '<B'),
        }

    def snapshot():
        return (bytes(cpu.mem_read(TILES, area * 6)),
                [bytes(cpu.mem_read(address, area)) for address in caches],
                bytes(cpu.mem_read(CIV, STRIDE * 8)),
                bytes(cpu.mem_read(city, 92)))

    before_units = [unit(1), unit(14)]
    visible, memory = knowledge()
    before = snapshot()
    result, hosts, display, observations = run_attack(original, (1, 3, 3))
    after_units = [unit(1), unit(14)]
    after = snapshot()
    assert result == 1 and observations == [[1, 1]], (result, observations, after_units)
    assert after_units == [
        {'slot': 1, 'position': [27, 17], 'previous': -1, 'next': -1,
         'spent': 3, 'direction': 3},
        {'slot': 14, 'position': [27, 15], 'previous': -1, 'next': -1,
         'spent': 0, 'direction': 1},
    ], after_units
    mover_changes = differences(MOVER, bytes(cpu.mem_read(UNIT_RECORDS + 40, 40)))
    resident_changes = differences(RESIDENT, bytes(cpu.mem_read(UNIT_RECORDS + 40 * 14, 40)))
    assert mover_changes == [(2, 15, 17), (10, 0, 3), (13, 5, 3), (28, 14, 255), (29, 0, 255)]
    assert resident_changes == [(26, 1, 255), (27, 0, 255)]
    after_tiles, after_caches, after_civs, after_city = after
    assert before[2:] == after[2:], 'civ records or the city record changed'
    assert read(original, CRT, '<I') == CRT_STATE
    assert read(original, tile(27, 17) + 1, '<B') == 3
    assert read(original, tile(27, 15) + 1, '<B') == 1
    explored_writes = []
    for x, y in ((26, 16), (28, 16), (29, 17), (28, 18)):
        index = tile(x, y)
        explored_writes.append([index - TILES + 4, 0, 64])
        assert read(original, index + 4, '<B') == 64
    tile_changes = [[index, old, new] for index, old, new in differences(before[0], after_tiles)]
    assert sorted(tile_changes) == sorted(
        [[tile(27, 17) - TILES + 1, 2, 3]] + explored_writes), tile_changes
    after_visible, after_memory = knowledge()
    assert visible == after_visible and sum(bool(b) for b in visible) == 59
    cache_changes = [(seat, *change)
                     for seat, (old, new) in enumerate(zip(memory, after_memory, strict=True), 1)
                     for change in differences(old, new)]
    assert cache_changes == [(6, (tile(27, 15) - TILES) // 6, 0, 1),
                             (6, (tile(27, 17) - TILES) // 6, 2, 3)], cache_changes
    return {
        'arguments': [1, 3, 3], 'return': result, 'crt_state': CRT_STATE,
        'founding_setup': {'city_slot': founded['city_slot'],
                           'location': founded['location'],
                           'crt_state': founded['crt_state']},
        'before': before_units, 'after': after_units,
        'mover_raw_changes': mover_changes, 'resident_raw_changes': resident_changes,
        'tile_changes': tile_changes, 'explored_tiles': 59,
        'viewer_memory_unchanged': True,
        'scout_cache_changes': cache_changes,
        'observation_calls': observations, 'host_substitutes': hosts,
        'display_substitutes': display,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent / 'docs/evidence'
    founding_evidence = json.loads((root / 'import-opponent-founding.json').read_text())
    annual = json.loads((root / 'import-round-close.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(binary) == founding_evidence['binary_sha256'] == annual['binary_sha256']
    for role in ('source', 'rules'):
        item = founding_evidence[role]
        assert item == annual[role] and digest((home / item['path']).read_bytes()) == item['sha256']
    rules = parse_rules(home / founding_evidence['rules']['path'])
    city_lines = (home / 'Original/City.txt').read_bytes().splitlines()
    prepared, measured = prepare_round(
        home, binary, founding_evidence['source']['path'], rules,
        lambda original, viewer, city: entry(original, viewer, city, rules, city_lines),
    )
    assert prepared['annual_update'] == annual['annual_update']
    assert measured['explored_tiles'] == 59
    with args.out.open('x') as output:
        out = {'schema': 1,
               **{key: founding_evidence[key] for key in ('source', 'rules', 'binary_sha256')},
               'empty_city_entry': measured, 'scope': __doc__.strip()}
        output.write(json.dumps(out, indent=2) + '\n')


if __name__ == '__main__':
    main()
