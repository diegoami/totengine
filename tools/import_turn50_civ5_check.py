#!/usr/bin/env python3
"""Measure civ 5's turn-50 order from the refreshed staged-snapshot state.

The handover boundary after the human's turn 50: civ 5's second recorded order
is `move 19 -> (44,42)`, the fortified slot-19 type-2 unit at (44,40) stepping
south into civ 5's own size-one city 9. On turn 49 native refused this same
order (done flag 0->64, counter bump, no movement spent). This witness asks
what native does from the state the engine's staged route presents at civ 5's
turn-50 pass:

- the staged snapshot's records/tiles (the published reconstruction),
- slot 19 and city 9 at their recorded squares,
- the measured annual round-close reset (spent and counter zeroed; the flag
  word stored back unchanged),
- the measured per-civ turn start for civ 5 (0x4725c8), stopped at its first
  map write as the civ-2 witness does, and
- the recorded southward order (native movement entry, direction 3).

Scope: the staged snapshot is the engine's construction, not a native
pre-order trajectory, and the turn-50 state after civs 2-4 is not witnessed
here. The result describes the native kernel from this refreshed state, which
is the same bounded convention as the published civ-2 turn-50 measurement.
Every diff is computed from the run; byte-identical across reruns.
"""
import argparse
import json
import os
from pathlib import Path
import struct

from unicorn.x86_const import UC_X86_REG_EIP

from import_event_tail_check import digest
from import_opponent_combat_view_check import CRT, TILES, run_attack
from import_opponent_trade_contact_check import CITY9, MOVER5, entry
from import_phase_check import read
from import_round_close_check import ENTRY, prepare_round
from import_turn_start_refresh_check import (
    capture,
    differences,
    run_hooked,
    write_snapshot_state,
)
from import_unit_setup_check import UNIT_RECORDS
from totctl import parse_rules

TURN_START = 0x4725C8
SLOT = 19
FLAG = 6
SPENT = 10
COUNTER = 16


def record(original, slot):
    return bytes(original.cpu.mem_read(UNIT_RECORDS + 40 * slot, 40))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent / 'docs/evidence'
    contact = json.loads((root / 'import-opponent-contact.json').read_text())
    annual = json.loads((root / 'import-round-close.json').read_text())
    refresh = json.loads((root / 'import-turn-start-refresh.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(binary) == contact['binary_sha256'] == annual['binary_sha256']
    for role in ('source', 'rules'):
        item = contact[role]
        assert item == annual[role] and digest((home / item['path']).read_bytes()) == item['sha256']
    rules = parse_rules(home / contact['rules']['path'])
    city_lines = (home / 'Original/City.txt').read_bytes().splitlines()

    state = {}

    def after_annual(original, viewer, city_region):
        cpu = original.cpu
        width = read(original, 0x6705A0, '<h')
        area = read(original, 0x6705A4, '<h')

        def tile(x, y):
            return TILES + 6 * (y * (width // 2) + x // 2)

        measured = entry(original, viewer, city_region, rules, city_lines)
        assert measured['explored_tiles'] == 59
        state.update(original=original, viewer=viewer, tile=tile, area=area, city_region=city_region,
                     caches=[read(original, 0x5DEAD0 + 4 * civ, '<I')
                             for civ in range(1, 8)])
        write_snapshot_state(original, cpu, tile)
        # The staged civ-5 record and its city, then the turn-49 refusal
        # outcome the recorded order met (flag set, counter bumped).
        cpu.mem_write(UNIT_RECORDS + 40 * SLOT, MOVER5)
        cpu.mem_write(city_region[0] + 92 * 9, CITY9)
        original.write(UNIT_RECORDS + 40 * SLOT + FLAG, 64, '<B')
        original.write(UNIT_RECORDS + 40 * SLOT + COUNTER, 1, '<B')

    prepared, _measured = prepare_round(
        home, binary, contact['source']['path'], rules, after_annual)
    assert prepared['annual_update'] == annual['annual_update']

    original = state['original']
    cpu = original.cpu
    tile = state['tile']
    area = state['area']
    caches = state['caches']
    viewer = state['viewer']

    def restore():
        write_snapshot_state(original, cpu, tile)
        cpu.mem_write(UNIT_RECORDS + 40 * SLOT, MOVER5)
        cpu.mem_write(state['city_region'][0] + 92 * 9, CITY9)
        original.write(UNIT_RECORDS + 40 * SLOT + FLAG, 64, '<B')
        original.write(UNIT_RECORDS + 40 * SLOT + COUNTER, 1, '<B')

    crt_before = read(original, CRT, '<I')
    mover_before_annual = record(original, SLOT)
    _writes_a, draws_a, _stopped_a, completed_a = run_hooked(
        original, cpu, ENTRY, 0xffffffff, area, viewer, stop_on_tiles=False)
    assert completed_a, hex(cpu.reg_read(UC_X86_REG_EIP))
    annual_mover = record(original, SLOT)
    # The annual's own writes are captured on this first run; the case runs
    # below repeat the same kernel from the restored state.
    annual_changes = differences(mover_before_annual, annual_mover)
    crt_after_annual = read(original, CRT, '<I')
    assert annual_changes == [[SPENT, 3, 0], [COUNTER, 1, 0]], annual_changes
    assert read(original, UNIT_RECORDS + 40 * SLOT + FLAG, '<B') == 64

    def case(name, turn_start):
        """Restore the staged state, run the annual, optionally run civ 5's
        turn start, then run the recorded order and describe every change."""
        restore()
        writes, draws, stopped, completed = run_hooked(
            original, cpu, ENTRY, 0xffffffff, area, viewer, stop_on_tiles=False)
        assert completed
        start_writes = []
        start_stopped = []
        if turn_start:
            start_writes, start_draws, start_stopped, start_completed = run_hooked(
                original, cpu, TURN_START, 5, area, viewer)
            assert not start_completed
            assert start_stopped
        else:
            start_draws = []
        before = record(original, SLOT)
        before_tiles = bytes(cpu.mem_read(TILES, area * 6))
        before_caches = [bytes(cpu.mem_read(c, area)) for c in caches]
        crt_before_order = read(original, CRT, '<I')
        result, _hosts, _display, observations = run_attack(
            original, (SLOT, 3, 3), memory_rects=True)
        changes = differences(before, record(original, SLOT))
        tile_changes = differences(before_tiles, bytes(cpu.mem_read(TILES, area * 6)))
        cache_changes = [(seat, *change)
                         for seat, (old, new) in enumerate(zip(
                             before_caches,
                             [bytes(cpu.mem_read(c, area)) for c in caches],
                             strict=True), 1)
                         for change in differences(old, new)]
        visible = sum(bool(b & (1 << viewer))
                      for b in bytes(cpu.mem_read(TILES, area * 6))[4::6])
        return {
            'case': name, 'turn_start': turn_start,
            'annual_draws': len(draws), 'turn_start_draws': len(start_draws),
            'turn_start_stopped_at': start_stopped,
            'turn_start_writes_on_slot': [w for w in start_writes if w[1] == SLOT],
            'before': list(before), 'return': result,
            'mover_raw_changes': changes,
            'tile_changes': tile_changes,
            'cache_changes': cache_changes,
            'crt_before': f'{crt_before_order:08x}',
            'crt_after': f'{read(original, CRT, "<I"):08x}',
            'observation_calls': observations,
            'explored_tiles': visible,
        }

    # Control: the published turn-49 state and refusal (no annual, no turn
    # start, the pre-order record with flag 0 / counter 0 / spent 3).
    restore()
    cpu.mem_write(UNIT_RECORDS + 40 * SLOT, MOVER5)
    before = record(original, SLOT)
    before_tiles = bytes(cpu.mem_read(TILES, area * 6))
    result, _hosts, _display, observations = run_attack(
        original, (SLOT, 3, 3), memory_rects=True)
    stale = {
        'case': 'turn-49 pre-order snapshot (no refresh)', 'turn_start': False,
        'before': list(before), 'return': result,
        'mover_raw_changes': differences(before, record(original, SLOT)),
        'tile_changes': differences(before_tiles, bytes(cpu.mem_read(TILES, area * 6))),
        'observation_calls': observations,
        'explored_tiles': sum(bool(b & (1 << viewer))
                              for b in bytes(cpu.mem_read(TILES, area * 6))[4::6]),
    }
    cases = [stale,
             case('refreshed (annual + civ 5 turn start)', True),
             case('annual only (done flag stays set)', False),
             case('annual + cleared flag (no turn start)', False)]
    # The third case needs the flag cleared manually; rebuild it here.
    restore()
    run_hooked(original, cpu, ENTRY, 0xffffffff, area, viewer, stop_on_tiles=False)
    original.write(UNIT_RECORDS + 40 * SLOT + FLAG, 0, '<B')
    before = record(original, SLOT)
    before_tiles = bytes(cpu.mem_read(TILES, area * 6))
    result, _hosts, _display, observations = run_attack(
        original, (SLOT, 3, 3), memory_rects=True)
    cases[3].update({
        'before': list(before), 'return': result,
        'mover_raw_changes': differences(before, record(original, SLOT)),
        'tile_changes': differences(before_tiles, bytes(cpu.mem_read(TILES, area * 6))),
    })

    report = {
        'schema': 1,
        'source': contact['source'], 'rules': contact['rules'],
        'binary_sha256': contact['binary_sha256'],
        'refresh_evidence': refresh['binary_sha256'],
        'annual': {
            'mover_raw_changes': annual_changes,
            'draws': len(draws_a), 'crt_before': f'{crt_before:08x}',
            'crt_after': f'{crt_after_annual:08x}',
        },
        'cases': cases,
        'scope': __doc__.strip(),
    }
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'annual': report['annual'],
                      'cases': [{key: c.get(key) for key in
                                 ('case', 'return', 'mover_raw_changes',
                                  'tile_changes', 'observation_calls')}
                                for c in report['cases']]}, indent=2))


if __name__ == '__main__':
    main()
