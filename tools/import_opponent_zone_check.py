#!/usr/bin/env python3
"""Measure the prepared opponent move near a known rival.

Start from the pinned round-close control. Place the three measured units
on their checked approach squares, supply the observed map bytes and CRT,
and execute native movement. Repeat with the rival away from both squares.
"""
import argparse
import json
import os
from pathlib import Path

from import_event_tail_check import digest
from import_opponent_combat_view_check import CRT, TILES, run_attack
from import_phase_check import CIV, STRIDE, read
from import_round_close_check import prepare_round
from import_unit_setup_check import UNIT_RECORDS
from totctl import parse_rules


def measure(original, viewer, city_region, seed, *, neighbor, reverse=False):
    width = read(original, 0x6705A0, '<h')
    area = read(original, 0x6705A4, '<h')
    original.write(0x607648, 47 if reverse else 46, '<h')
    original.write(0x607655, 2 if reverse else 5, '<B')
    original.write(0x5D156C, 0, '<B')
    original.write(CRT, seed, '<I')

    def tile(x, y):
        return TILES + 6 * (y * (width // 2) + x // 2)

    def unit(slot):
        at = UNIT_RECORDS + 40 * slot
        return {
            'position': [read(original, at + off, '<h') for off in (0, 2)],
            'spent': read(original, at + 10, '<B'), 'direction': read(original, at + 13, '<B'),
            'orders': read(original, at + 17, '<B'),
        }

    for slot in (3, 10, 19):
        old = tile(*unit(slot)['position']) + 1
        original.write(old, read(original, old, '<B') & ~1, '<B')
    rival = (39, 35) if reverse else ((39, 33) if neighbor else (43, 29))
    placements = ((3, (39, 33), 0, 255), (10, rival, 9, 255), (19, (41, 39), 9, 2)) if reverse else (
        (3, rival, 3, 255), (10, (39, 35), 0, 255), (19, (41, 37), 0, 2),
    )
    if reverse:
        original.write(UNIT_RECORDS + 40 * 10 + 16, 1, '<B')
    for slot, (x, y), spent, orders in placements:
        at = UNIT_RECORDS + 40 * slot
        for off, value in ((0, x), (2, y), (26, -1), (28, -1)):
            original.write(at + off, value, '<h')
        original.write(at + 10, spent, '<B')
        original.write(at + 17, orders, '<B')
        flag = tile(x, y) + 1
        original.write(flag, read(original, flag, '<B') | 1, '<B')
    for x, y, raw in ((39, 35, [5, 1, 0, 2, 36, 80]),
                      (38, 34, [4, 0, 0, 2, 36, 32]),
                      (39, 33, [9, int(neighbor), 0, 2, 36, 32])):
        original.cpu.mem_write(tile(x, y), bytes(raw))
    cache = read(original, 0x5DEAD0 + 4 * viewer, '<I')
    count = read(original, 0x607666, '<h')
    regions = [(CIV, STRIDE * 8), (UNIT_RECORDS, count * 40), city_region, (TILES, area * 6), (cache, area)]

    def snapshot():
        return [bytes(original.cpu.mem_read(at, length)) for at, length in regions]

    before = snapshot()
    arguments = (3, 4, 3) if reverse else (10, 6, 3)
    actor_before = unit(arguments[0])
    result, hosts, display, observations = run_attack(original, arguments)
    after = snapshot()
    actor_after = unit(arguments[0])
    assert [bit & (1 << viewer) for bit in before[3][4::6]] == [
        bit & (1 << viewer) for bit in after[3][4::6]
    ] and before[-1] == after[-1]
    assert read(original, CRT, '<I') == seed
    unit_changes = [(i // 40, i % 40, old, new)
                    for i, (old, new) in enumerate(zip(before[1], after[1], strict=True))
                    if old != new]
    if neighbor:
        assert result == 0 and not observations
        assert before[0] == after[0] and before[2:] == after[2:]
        assert unit_changes == [(arguments[0], 16, 0, 1)]
    else:
        assert result == 1 and observations == [[10, 1]]
        assert actor_after == {'position': [38, 34], 'spent': 6, 'direction': 6, 'orders': 255}
    repeated = None
    if reverse:
        prior = after
        for increment in range(1, 20):
            retry, hosts_retry, display_retry, observations_retry = run_attack(original, arguments)
            post = snapshot()
            changes = [(i // 40, i % 40, old, new)
                       for i, (old, new) in enumerate(zip(prior[1], post[1], strict=True))
                       if old != new]
            assert retry == 0 and not observations_retry
            assert prior[0] == post[0] and prior[2:] == post[2:]
            if increment == 19:
                assert changes == [(3, 10, 0, 6), (3, 16, 19, 0)]
            else:
                assert changes == [(3, 16, increment, increment + 1)]
            assert read(original, CRT, '<I') == seed
            prior = post
        repeated = {'additional_refusals': 19, 'last_unit_byte': 0, 'spent_after': 6,
                    'other_captured_regions_unchanged': True,
                    'host_substitutes': hosts_retry, 'display_substitutes': display_retry}
    return {
        'arguments': arguments, 'rival_position': rival, 'before': actor_before,
        'after': actor_after, 'return': result, 'observation_calls': observations,
        'crt_state': seed, 'explored_tiles': sum(bool(b & (1 << viewer)) for b in after[3][4::6]),
        'viewer_memory_unchanged': True, 'unit_record_changes': unit_changes,
        'other_captured_regions_unchanged': before[0] == after[0] and before[2:] == after[2:],
        'host_substitutes': hosts, 'display_substitutes': display, 'repeat': repeated,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent / 'docs/evidence'
    annual = json.loads((root / 'import-round-close.json').read_text())
    contact = json.loads((root / 'import-opponent-contact.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(binary) == annual['binary_sha256'] == contact['binary_sha256']
    for role in ('source', 'rules'):
        item = annual[role]
        assert item == contact[role] and digest((home / item['path']).read_bytes()) == item['sha256']
    rules = parse_rules(home / annual['rules']['path'])
    seed = contact['known_turn_46']['crt_state']
    cases = {}
    for name, neighbor, reverse in (
        ('adjacent_rival', True, False), ('distant_rival', False, False), ('reverse_approach', True, True),
    ):
        prepared, case = prepare_round(
            home, binary, annual['source']['path'], rules,
            lambda original, viewer, city: measure(
                original, viewer, city, 0x3b0ca297 if reverse else seed,
                neighbor=neighbor, reverse=reverse,
            ),
        )
        assert prepared['annual_update'] == annual['annual_update'] and case['explored_tiles'] == 59
        cases[name] = case
    with args.out.open('x') as output:
        report = {'schema': 1, **{key: annual[key] for key in ('source', 'rules', 'binary_sha256')},
                  'cases': cases, 'scope': __doc__.strip()}
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
