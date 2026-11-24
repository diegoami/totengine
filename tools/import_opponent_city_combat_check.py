#!/usr/bin/env python3
"""Measure the prepared post-contact attack on the occupied Original city.

Move the opponent attacker into the witnessed approach square, supply
its spent movement, the verified encounter treaty and its outgoing CRT
state, then execute the eastward attack. One trace stops at the first
city-panel font import. With the native redraw flag clear, a second
control follows the attack callback through return.
"""
import argparse
import json
import os
from pathlib import Path

from import_event_tail_check import digest
from import_loader_exit_check import rules_refresh
from import_opponent_combat_view_check import CRT, TILES, run_attack
from import_phase_check import CIV, FRAME, STRIDE, frame, read
from import_unit_setup_check import prepare, UNIT_RECORDS
from totctl import parse_rules


def measure(home, binary, reference, *, closed_panel):
    original, viewer, _, regions, _ = prepare(
        home, binary, reference['source']['path'], exhaust=False,
    )
    frame(original)
    original.write(FRAME - 0x1990, 1, '<I')
    original.write(FRAME - 0xe9c, 0, '<I')
    rules_refresh(original, parse_rules(home / reference['rules']['path']))
    width, height = (read(original, at, '<h') for at in (0x6705A0, 0x6705A2))
    area = width * height // 2
    original.write(0x6705A4, area, '<h')
    original.write(0x607648, 44, '<h')
    original.write(0x607655, 5, '<B')
    if closed_panel:
        original.write(0x5D156C, 0, '<B')
        original.cpu.mem_map(0, 4096)
        original.write(0, 0xffffffff, '<I')

    for (a, b), word in zip(((5, 6), (6, 5)),
                            reference['engine_seed_control']['treaty_words'], strict=True):
        original.write(CIV + a * STRIDE + 40 + 4 * b, word, '<I')
    actor = UNIT_RECORDS + 40 * 35
    assert (read(original, actor, '<h'), read(original, actor + 2, '<h')) == (46, 32)
    original.write(actor + 2, 28, '<h')
    original.write(actor + 6, read(original, actor + 6, '<B') | 0x40, '<B')
    original.write(actor + 10, 3, '<B')
    original.write(actor + 13, 7, '<B')
    original.write(actor + 17, 255, '<B')
    for y, occupied in ((32, False), (28, True)):
        tile = TILES + 6 * (y * (width // 2) + 23)
        flags = read(original, tile + 1, '<B')
        original.write(tile + 1, flags | 1 if occupied else flags & ~1, '<B')
    seed = reference['engine_seed_control']['crt_after']
    original.write(CRT, seed, '<I')
    cpu = original.cpu
    caches = [read(original, 0x5DEAD0 + 4 * civ, '<I') for civ in range(1, 8)]

    def snapshot():
        tiles = bytes(cpu.mem_read(TILES, area * 6))
        city = regions[2][0] + 92 * 9
        units = []
        for slot in (35, 36):
            addr = UNIT_RECORDS + 40 * slot
            units.append({
                'slot': slot, 'live': bool(read(original, addr + 30, '<I')),
                'position': [read(original, addr + off, '<h') for off in (0, 2)],
                'spent': read(original, addr + 10, '<B'), 'damage': read(original, addr + 12, '<B'),
            })
        return {
            'units': units,
            'city': {'slot': 9, 'size': read(original, city + 11, '<B'),
                     'live': bool(read(original, city + 86)),
                     'tile_flags': tiles[6 * (28 * (width // 2) + 24) + 1]},
            'explored_tiles': sum(bool(mask & (1 << viewer)) for mask in tiles[4::6]),
        }, tiles, [bytes(cpu.mem_read(cache, area)) for cache in caches]

    before, map_before, remembered = snapshot()
    arguments = (35, 1, 3)
    result, hosts, display, observes = run_attack(
        original, arguments, city_panel_boundary=not closed_panel,
    )
    after, map_after, remembered_after = snapshot()
    assert bytes(mask & (1 << viewer) for mask in map_before[4::6]) == bytes(
        mask & (1 << viewer) for mask in map_after[4::6]
    )
    map_changes = [(i, a, b) for i, (a, b) in enumerate(zip(map_before, map_after, strict=True)) if a != b]
    all_memory_changes = [
        (civ, i, a, b)
        for civ, (old, new) in enumerate(zip(remembered, remembered_after, strict=True), 1)
        for i, (a, b) in enumerate(zip(old, new, strict=True)) if a != b
    ]
    memory_changes = [(i, a, b) for civ, i, a, b in all_memory_changes if civ == viewer]
    assert memory_changes == ([(1144, 2, 0)] if closed_panel else [])
    assert result == (0 if closed_panel else None)
    assert before['city']['size'] == 1 and after['city']['size'] == 0
    assert after['units'] == [
        {'slot': 35, 'live': True, 'position': [46, 28], 'spent': 6, 'damage': 8},
        {'slot': 36, 'live': False, 'position': [-700, -700], 'spent': 0, 'damage': 10},
    ]
    assert before['explored_tiles'] == after['explored_tiles'] == 59
    assert not observes
    return {
        'arguments': arguments, 'crt_before': seed, 'crt_after': read(original, CRT, '<I'),
        'before': before, 'after': after, 'viewer_exploration_unchanged': True,
        'viewer_memory_changes': memory_changes, 'return': result,
        'map_changes': map_changes, 'all_memory_changes': all_memory_changes,
        'stopped_at': f'{cpu.reg_read(original.ip):08x}', 'host_substitutes': hosts,
        'display_substitutes': display,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    ref = json.loads((root / 'docs/evidence/import-opponent-contact.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(binary) == ref['binary_sha256']
    for role in ('source', 'rules'):
        item = ref[role]
        assert digest((home / item['path']).read_bytes()) == item['sha256']
    report = {'schema': 1, **{key: ref[key] for key in ('source', 'rules', 'binary_sha256')},
              'attack': measure(home, binary, ref, closed_panel=False),
              'closed_panel_attack': measure(home, binary, ref, closed_panel=True),
              'scope': __doc__.strip()}
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
