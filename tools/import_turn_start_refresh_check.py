#!/usr/bin/env python3
"""Witness the native movement refresh at civ 2's turn-50 pass boundary.

Reuse the measured turn-49 trade-turn context so the reconstructed state is the
checked engine's staged snapshot context, restore the pre-order substitutions,
then run the two native candidates for the movement refresh with write hooks on
the unit-record region: the annual round-close entry (0x4702fc) and the
per-civ turn start (0x4725c8) for civ 2. The witness records every byte each
writes into the records with the writing PC, whether the map, viewer caches
and CRT move, and therefore the runtime movement state that civ 2's unit-3
orders execute from. The recorded southward order is then measured again from
the refreshed state, because the published measurement used the staged
snapshot's stale end-of-49 exhaustion record. Bound: the record scan covers
the snapshot's 38 slots - the witnessed state's array - so units created after
the snapshot (the engine's slots 38/39 by turn 50) are outside the witness;
their flag bytes are already clear in the engine, making the staged clear a
no-op there. Every diff is computed from the run.
"""
import argparse
import json
import os
from pathlib import Path
import struct

from unicorn import UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from import_event_tail_check import digest
from import_opponent_combat_view_check import CRT, TILES, run_attack
from import_opponent_trade_contact_check import entry
from import_phase_check import read
from import_round_close_check import ENTRY, HOST_IMPORTS, prepare_round
from import_unit_setup_check import UNIT_RECORDS
from totctl import parse_rules

TURN_START = 0x4725C8
SLOTS = 38
RECORDS_END = UNIT_RECORDS + 40 * SLOTS
DRAW = 0x5B11D0
FRAME = 0x108000
INSTRUCTION_CAP = 5_000_000

# The staged snapshot's pre-order state, verbatim from the published
# reconstruction (import-opponent-trade-contact.json context).
TURN50_TILES = {
    (42, 36): [129, 0, 160, 2, 36, 86],
    (42, 38): [5, 0, 160, 2, 36, 80],
    (43, 37): [130, 3, 160, 2, 36, 87],
    (41, 35): [2, 1, 160, 2, 36, 37],
}
NEIGHBOR34 = bytes([43, 0, 37, 0, 0, 0, 0, 0, 3, 5, 0, 0, 0, 0, 0, 0, 0,
                    255, 6, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 51,
                    0, 0, 0, 0, 0, 0, 0, 0, 0])
RESIDENT10_TURN50 = bytes([44, 0, 36, 0, 0, 0, 64, 4, 2, 5, 9, 4, 0, 5,
                           49, 0, 1, 255, 0, 0, 23, 0, 47, 0, 255, 255,
                           255, 255, 255, 255, 15, 0, 0, 0, 2, 0, 6, 0,
                           0, 0])
MOVER_SNAPSHOT = bytes([42, 0, 36, 0, 0, 0, 64, 0, 16, 2, 6, 32, 0, 1, 49,
                        0, 0, 255, 255, 255, 48, 0, 28, 0, 255, 255, 255,
                        255, 255, 255, 11, 0, 0, 0, 2, 0, 1, 0, 0, 0])
SNAPSHOT_RECORDS = ((3, MOVER_SNAPSHOT), (34, NEIGHBOR34), (10, RESIDENT10_TURN50))
SNAPSHOT_CRT = 0xB404D22D


def differences(before, after):
    return [[i, old, new] for i, (old, new) in enumerate(zip(before, after, strict=True))
            if old != new]


def write_snapshot_state(original, cpu, tile):
    """Restore the staged snapshot's pre-order records, tiles, turn and CRT."""
    original.write(0x607648, 50, '<h')
    original.write(0x607655, 2, '<B')
    original.write(CRT, SNAPSHOT_CRT, '<I')
    for slot, record in SNAPSHOT_RECORDS:
        address = UNIT_RECORDS + 40 * slot
        prev, nxt = (read(original, address + off, '<h') for off in (26, 28))
        if prev >= 0:
            original.write(UNIT_RECORDS + 40 * prev + 28, nxt, '<h')
        if nxt >= 0:
            original.write(UNIT_RECORDS + 40 * nxt + 26, prev, '<h>')
        cpu.mem_write(address, bytes(record))
    for xy, fields in TURN50_TILES.items():
        cpu.mem_write(tile(*xy), bytes(fields))


def snapshot_records(original):
    return {slot: bytes(original.cpu.mem_read(UNIT_RECORDS + 40 * slot, 40))
            for slot, _ in SNAPSHOT_RECORDS}


def capture(original, caches, area):
    return {'crt': read(original, CRT, '<I'),
            'tiles': bytes(original.cpu.mem_read(TILES, area * 6)),
            'caches': [bytes(original.cpu.mem_read(c, area)) for c in caches]}


def run_hooked(original, cpu, start, argument, area, viewer, stop_on_tiles=True):
    """Run a native entry with record-write capture. With stop_on_tiles, the
    first map write stops the emulation so no AI order can execute. Returns
    (writes, draws, stopped_at, completed)."""
    writes = []
    stopped_at = []
    draws = []

    def on_write(cpu, _access, address, size, value, _data):
        if UNIT_RECORDS <= address < RECORDS_END:
            eip = cpu.reg_read(UC_X86_REG_EIP)
            old = bytes(cpu.mem_read(address, size))
            new = list((value & ((1 << (8 * size)) - 1)).to_bytes(size, 'little'))
            writes.append([f'{eip:08x}', (address - UNIT_RECORDS) // 40,
                           (address - UNIT_RECORDS) % 40, size,
                           list(old), new])
            return
        if TILES <= address < TILES + 6 * area and stop_on_tiles:
            stopped_at.append(f'{address:08x}')
            cpu.emu_stop()

    def on_code(cpu, at, size, _data):
        if at == DRAW:
            draws.append(read(original, CRT, '<I'))
        instruction = bytes(cpu.mem_read(at, size))
        if not instruction.startswith(b'\xff\x15'):
            return
        host = HOST_IMPORTS[int.from_bytes(instruction[2:], 'little')]
        sp = cpu.reg_read(UC_X86_REG_ESP)

        def argument_value(index):
            return read(original, sp + 4 * index, '<I')

        if host == 'SetRect':
            cpu.mem_write(argument_value(0), bytes(cpu.mem_read(sp + 4, 16)))
            argc, result = 5, 1
        elif host == 'GetCurrentDirectoryA':
            assert argument_value(0) > 9
            cpu.mem_write(argument_value(1), b'C:\\CIVTOT\0')
            argc, result = 2, 9
        elif host == 'SetCurrentDirectoryA':
            argc, result = 1, 1
        elif host == 'GetLastError':
            argc, result = 0, 2
        else:
            assert host == 'SetEnvironmentVariableA', host
            argc, result = 2, 1
        cpu.reg_write(UC_X86_REG_ESP, sp + 4 * argc)
        cpu.reg_write(UC_X86_REG_EAX, result)
        cpu.reg_write(UC_X86_REG_EIP, at + size)

    hook_write = cpu.hook_add(UC_HOOK_MEM_WRITE, on_write)
    hook_code = cpu.hook_add(UC_HOOK_CODE, on_code)
    try:
        cpu.mem_write(FRAME, struct.pack('<II', 0x100000,
                                         argument & 0xffffffff))
        cpu.reg_write(original.sp, FRAME)
        cpu.emu_start(start, 0, count=INSTRUCTION_CAP)
    finally:
        cpu.hook_del(hook_write)
        cpu.hook_del(hook_code)
    completed = cpu.reg_read(UC_X86_REG_EIP) == 0x100000
    return writes, draws, stopped_at, completed


def apply_writes(original, writes):
    for _pc, slot, offset, size, _old, new in writes:
        address = UNIT_RECORDS + 40 * slot + offset
        original.cpu.mem_write(address, bytes(new))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent / 'docs/evidence'
    contact = json.loads((root / 'import-opponent-contact.json').read_text())
    annual = json.loads((root / 'import-round-close.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(binary) == contact['binary_sha256'] == annual['binary_sha256']
    for role in ('source', 'rules'):
        item = contact[role]
        assert item == annual[role] and digest((home / item['path']).read_bytes()) == item['sha256']
    rules = parse_rules(home / contact['rules']['path'])
    city_lines = (home / 'Original/City.txt').read_bytes().splitlines()

    # Run the published reconstruction through its stale-state refusal, then
    # restore the pre-order snapshot state on top of it.
    state = {}

    def after_annual(original, viewer, city_region):
        cpu = original.cpu
        width = read(original, 0x6705A0, '<h')
        area = read(original, 0x6705A4, '<h')

        def tile(x, y):
            return TILES + 6 * (y * (width // 2) + x // 2)

        measured = entry(original, viewer, city_region, rules, city_lines)
        assert measured['explored_tiles'] == 59
        state.update(original=original, viewer=viewer, tile=tile, area=area,
                     caches=[read(original, 0x5DEAD0 + 4 * civ, '<I')
                             for civ in range(1, 8)])
        write_snapshot_state(original, cpu, tile)
        assert read(original, UNIT_RECORDS + 40 * 3 + 30, '<I') == 11
        records = snapshot_records(original)
        assert records[3] == MOVER_SNAPSHOT and records[34] == NEIGHBOR34
        assert records[10] == RESIDENT10_TURN50

    prepared, _measured = prepare_round(
        home, binary, contact['source']['path'], rules, after_annual)
    assert prepared['annual_update'] == annual['annual_update']

    original = state['original']
    cpu = original.cpu
    tile = state['tile']
    area = state['area']
    caches = state['caches']
    viewer = state['viewer']

    def check_state():
        before = capture(original, caches, area)
        records = snapshot_records(original)
        assert records[3] == MOVER_SNAPSHOT and records[34] == NEIGHBOR34
        assert records[10] == RESIDENT10_TURN50
        return before

    # Case A: the annual round-close entry on the snapshot state, under the
    # same convention as the published annual measurement (the outer loop
    # restores the human owner before the call).
    write_snapshot_state(original, cpu, tile)
    original.write(0x607655, viewer, '<B')
    before_annual = check_state()
    original.write(0x607655, viewer, '<B')
    writes_a, draws_a, stopped_a, completed_a = run_hooked(
        original, cpu, ENTRY, 0xffffffff, area, viewer, stop_on_tiles=False)
    after_annual_state = capture(original, caches, area)
    tile_writes_a = differences(before_annual['tiles'], after_annual_state['tiles'])
    cache_changes_a = [(seat, *change)
                       for seat, (old, new) in enumerate(zip(before_annual['caches'],
                                                             after_annual_state['caches'],
                                                             strict=True), 1)
                       for change in differences(old, new)]
    print('case A (annual) completed:', completed_a, 'stopped:', stopped_a,
          'draws:', len(draws_a))
    for write in writes_a:
        print('  write', write)
    print('  tiles:', tile_writes_a, 'caches:', cache_changes_a)
    visible_a = bytes(b & (1 << viewer) for b in after_annual_state['tiles'][4::6])
    assert sum(bool(b) for b in visible_a) == 59
    assert cache_changes_a == [], cache_changes_a
    assert completed_a, hex(cpu.reg_read(UC_X86_REG_EIP))
    annual_writes = [w for w in writes_a]
    explored_before_a = sum(bool(b & (1 << viewer)) for b in before_annual['tiles'][4::6])
    explored_after_a = sum(bool(b & (1 << viewer)) for b in after_annual_state['tiles'][4::6])
    assert explored_before_a == explored_after_a == 59
    # The annual's map writes are byte-1 bookkeeping only: unit-presence and
    # improvement mirrors reconciled after the round's moves. No terrain
    # (+0), visibility (+4) or owner/fertility (+5) byte moves, and the
    # viewer caches are untouched, so no knowledge row changes.
    assert all(change[0] % 6 == 1 for change in tile_writes_a), tile_writes_a
    print('annual tile writes are byte-1 bookkeeping only:', len(tile_writes_a))
    # The movement refresh: every live slot's spent byte is zeroed and its
    # order counter reset; dead and empty records are skipped; the flag word
    # is stored back unchanged. Live means a positive transient id at +30
    # (deletion markers are -100*(civ+1), empty slots 0).
    def unit_id(slot):
        return read(original, UNIT_RECORDS + 40 * slot + 30, '<I')

    live_slots = sorted(slot for slot in range(SLOTS)
                        if 0 < unit_id(slot) < 0x80000000)
    refreshed = sorted({slot for _pc, slot, _off, _size, _old, _new in annual_writes})
    assert refreshed == live_slots, (refreshed, live_slots)
    for pc, slot, offset, size, old, new in annual_writes:
        if pc == '00470230':
            assert (offset, size, new) == (10, 1, [0]), (slot, old, new)
        elif pc == '0047023e':
            assert (offset, size, new) == (16, 1, [0]), (slot, old, new)
        else:
            assert pc in ('0047025f', '004702e5') and (offset, size) == (6, 2)
            assert new == old, (pc, slot, old, new)
    mover_refresh = [w for w in annual_writes if w[1] == 3 and w[2] == 10]
    assert mover_refresh == [['00470230', 3, 10, 1, [6], [0]]], mover_refresh

    # Case B: the per-civ turn start for civ 2 on the restored snapshot state.
    write_snapshot_state(original, cpu, tile)
    before_turn = check_state()
    writes_b, draws_b, stopped_b, completed_b = run_hooked(
        original, cpu, TURN_START, 2, area, viewer)
    after_turn = capture(original, caches, area)
    tile_writes_b = differences(before_turn['tiles'], after_turn['tiles'])
    cache_changes_b = [(seat, *change)
                       for seat, (old, new) in enumerate(zip(before_turn['caches'],
                                                             after_turn['caches'],
                                                             strict=True), 1)
                       for change in differences(old, new)]
    print('case B (turn start) completed:', completed_b, 'stopped:', stopped_b,
          'draws:', len(draws_b))
    for write in writes_b:
        print('  write', write)
    print('  tiles:', tile_writes_b, 'caches:', cache_changes_b)
    visible_b = bytes(b & (1 << viewer) for b in after_turn['tiles'][4::6])
    assert sum(bool(b) for b in visible_b) == 59
    turn_writes = [w for w in writes_b]

    # The turn-start flag-clear witness: every 0x471d85 write targets a live
    # civ-2 unit record and clears only the 0x40 bit of the word at offset 6;
    # the written set is exactly the live owner-2 records.
    width_now = read(original, 0x6705A0, '<h')
    live_owner2 = sorted(slot for slot in range(SLOTS)
                         if read(original, UNIT_RECORDS + 40 * slot + 9, '<B') == 2
                         and 0 < read(original, UNIT_RECORDS + 40 * slot + 30, '<I') < 0x80000000)
    cleared = sorted({slot for pc, slot, offset, size, old, new in turn_writes
                      if pc == '00471d85'})
    for pc, slot, offset, size, old, new in turn_writes:
        assert (offset, size) == (6, 2), (pc, slot, offset, size)
        assert new == [old[0] & ~0x40, old[1]], (pc, slot, old, new)
        assert read(original, UNIT_RECORDS + 40 * slot + 9, '<B') == 2
    assert cleared == live_owner2, (cleared, live_owner2)
    tile_byte = stopped_b[0] if stopped_b else None
    tile_info = None
    if tile_byte:
        index = (int(tile_byte, 16) - TILES) // 6
        tile_info = {'tile_record': index,
                     'record_xy': [index % (width_now // 2), index // (width_now // 2)],
                     'byte_offset': (int(tile_byte, 16) - TILES) % 6,
                     'seen_by_viewer': bool(after_turn['tiles'][6 * index + 4] & (1 << viewer))}
    print('tile write provenance:', tile_info)

    # Case C: the recorded southward order from the refreshed state. Restore
    # the pre-order snapshot state, apply exactly the witnessed refresh bytes
    # (annual then turn start, the route's boundary order), then measure the
    # order with the same harness as the published refusal.
    write_snapshot_state(original, cpu, tile)
    records = snapshot_records(original)
    assert records[3] == MOVER_SNAPSHOT and records[34] == NEIGHBOR34
    assert records[10] == RESIDENT10_TURN50
    apply_writes(original, annual_writes)
    apply_writes(original, turn_writes)
    refreshed_mover = snapshot_records(original)[3]
    print('refreshed mover:', list(refreshed_mover))

    before_raw3 = bytes(cpu.mem_read(UNIT_RECORDS + 40 * 3, 40))
    before_raw34 = bytes(cpu.mem_read(UNIT_RECORDS + 40 * 34, 40))
    before_raw10 = bytes(cpu.mem_read(UNIT_RECORDS + 40 * 10, 40))
    before_tiles = bytes(cpu.mem_read(TILES, area * 6))
    before_caches = [bytes(cpu.mem_read(c, area)) for c in caches]
    crt_before_c = read(original, CRT, '<I')
    result_c, _hosts, _display, observations_c = run_attack(
        original, (3, 3, 3), memory_rects=True)
    after_raw3 = bytes(cpu.mem_read(UNIT_RECORDS + 40 * 3, 40))
    after_raw34 = bytes(cpu.mem_read(UNIT_RECORDS + 40 * 34, 40))
    after_raw10 = bytes(cpu.mem_read(UNIT_RECORDS + 40 * 10, 40))
    after_tiles = bytes(cpu.mem_read(TILES, area * 6))
    mover_changes_c = differences(before_raw3, after_raw3)
    neighbor_changes_c = differences(before_raw34, after_raw34)
    resident_changes_c = differences(before_raw10, after_raw10)
    tile_changes_c = [[i, old, new] for i, (old, new)
                      in enumerate(zip(before_tiles, after_tiles, strict=True))
                      if old != new]
    cache_changes_c = [(seat, *change)
                       for seat, (old, new) in enumerate(zip(before_caches,
                                                             [bytes(cpu.mem_read(c, area))
                                                              for c in caches],
                                                             strict=True), 1)
                       for change in differences(old, new)]
    print('case C result:', result_c, 'observations:', observations_c)
    print('  mover:', mover_changes_c)
    print('  tiles:', tile_changes_c)
    visible_c = bytes(b & (1 << viewer) for b in after_tiles[4::6])
    assert sum(bool(b) for b in visible_c) == 59

    report = {
        'schema': 1,
        **{key: contact[key] for key in ('source', 'rules', 'binary_sha256')},
        'annual_refresh': {
            'entry': f'{ENTRY:08x}', 'arguments': [-1],
            'instruction_cap': INSTRUCTION_CAP,
            'completed': completed_a, 'stopped_at_tile': stopped_a,
            'record_writes': annual_writes,
            'tile_writes': tile_writes_a, 'cache_changes': cache_changes_a,
            'crt_before': before_annual['crt'],
            'crt_after': after_annual_state['crt'],
            'crt_draws': len(draws_a), 'draw_values': draws_a,
            'explored_tiles': 59,
        },
        'turn_start_refresh': {
            'entry': f'{TURN_START:08x}', 'arguments': [2],
            'instruction_cap': INSTRUCTION_CAP,
            'completed': completed_b, 'stopped_at_tile': stopped_b,
            'record_writes': turn_writes,
            'live_owner2_slots': live_owner2,
            'map_write_provenance': tile_info,
            'tile_writes': tile_writes_b, 'cache_changes': cache_changes_b,
            'crt_before': before_turn['crt'], 'crt_after': after_turn['crt'],
            'crt_draws': len(draws_b), 'draw_values': draws_b,
            'explored_tiles': 59,
        },
        'refreshed_turn50_order': {
            'arguments': [3, 3, 3], 'return': result_c,
            'refreshed_mover_record': list(refreshed_mover),
            'mover_raw_changes': mover_changes_c,
            'neighbor_raw_changes': neighbor_changes_c,
            'resident_raw_changes': resident_changes_c,
            'tile_changes': tile_changes_c, 'cache_changes': cache_changes_c,
            'crt_state': crt_before_c,
            'crt_unchanged': read(original, CRT, '<I') == crt_before_c,
            'observation_calls': observations_c, 'explored_tiles': 59,
        },
        'scope': __doc__.strip(),
    }
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')
    print('wrote', args.out)


if __name__ == '__main__':
    main()
