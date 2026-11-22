#!/usr/bin/env python3
"""Measure event state words at native save, load and startup boundaries.

The writer runs its event-only path over two synthetic records with no string
slots. File I/O is redirected to a bounded in-memory buffer. The existing
event reader trace reads that buffer using its documented substitutes. A
separate startup fragment checks the records before any event actions.
No original game data or saves are emitted.
"""
import argparse
import itertools
import json
import os
import struct
from pathlib import Path

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from import_event_tail_check import FRAME, HEAD, RECORD_SIZE, ROUND_FLAG, digest, frame, read, run, trace
from totctl import walk_save_bytes
from yields_check import OriginalCity

WRITE_ENTRY = 0x4605DF
WRITE_STOP = 0x460ADA
RECORDS = (0x700000, 0x700200)
SPENT = 0x20000000
ACTIVE = (0x20000000, 0x80000000)
STARTUP_ENTRY = 0x463FA7
STARTUP_SELECTOR = 0x4D3285
AFTER_SELECTOR = 0x463FBE
SKIP_SELECTOR = 0x463FC8
DISPATCH = 0x4D3AE8
ROUND_ENTRY = 0x4744ED
AFTER_RESET = 0x474520
SKIP_ROUND_START = 0x474574
RESET_MARKS = 0x575D9A


def write_events(binary, words, *, enabled):
    original = OriginalCity(binary)
    frame(original)
    original.write(FRAME - 0x118, 1, '<I')
    original.write(FRAME + 0x0C, 0, '<I')
    original.write(ROUND_FLAG, int(enabled), '<I')
    original.write(HEAD, RECORDS[0], '<I')
    for record, (action, modifier), trigger in zip(RECORDS, words, (0x100, 0x40), strict=True):
        original.cpu.mem_write(record, bytes(RECORD_SIZE + 8))
        original.write(record, trigger, '<I')
        original.write(record + 4, action, '<I')
        original.write(record + 8, modifier, '<I')
    original.write(RECORDS[0] + RECORD_SIZE, RECORDS[1], '<I')
    original.write(RECORDS[1] + RECORD_SIZE + 4, RECORDS[0], '<I')
    output = bytearray()
    sizes = []

    def capture(cpu, address, _size, _data):
        if WRITE_ENTRY <= address < WRITE_STOP:
            return
        assert address in (0x5B4850, 0x5B4250), f'unexpected writer call {address:#x}'
        esp = cpu.reg_read(UC_X86_REG_ESP)
        ret = read(original, esp, '<I')
        if address == 0x5B4850:
            src, handle = struct.unpack('<2I', cpu.mem_read(esp + 4, 8))
            assert handle == 1
            payload = bytes(cpu.mem_read(src, 5))
            assert payload == b'EVNT\0'
            payload = payload[:-1]
            value = 0  # fputs success
        else:
            src, width, count, handle = struct.unpack('<4I', cpu.mem_read(esp + 4, 16))
            assert count == handle == 1 and width in (4, RECORD_SIZE)
            assert len(sizes) < 4
            payload = bytes(cpu.mem_read(src, width))
            value = 1  # fwrite one item
        sizes.append(len(payload))
        output.extend(payload)
        cpu.reg_write(UC_X86_REG_EAX, value)
        cpu.reg_write(UC_X86_REG_ESP, esp + 4)
        cpu.reg_write(UC_X86_REG_EIP, ret)

    hook = original.cpu.hook_add(UC_HOOK_CODE, capture)
    try:
        original.cpu.emu_start(WRITE_ENTRY, WRITE_STOP, count=10000)
    finally:
        original.cpu.hook_del(hook)
    assert original.cpu.reg_read(original.ip) == WRITE_STOP
    assert sizes == ([4, 4, RECORD_SIZE, RECORD_SIZE] if enabled else [])
    return bytes(output)


def probe_startup(original, records):
    """Run the separate game-start selector using the records just loaded."""
    stops = {AFTER_SELECTOR, SKIP_SELECTOR, DISPATCH}
    calls = (STARTUP_SELECTOR, DISPATCH)

    def snapshot():
        return [bytes(original.cpu.mem_read(record, RECORD_SIZE + 8))
                for record in records]

    initial = snapshot()
    assert read(original, ROUND_FLAG) == 1
    assert run(original, STARTUP_ENTRY, stops, calls=calls) == AFTER_SELECTOR
    assert snapshot() == initial
    original.write(ROUND_FLAG, 0, '<I')
    assert run(original, STARTUP_ENTRY, stops, calls=calls) == SKIP_SELECTOR
    assert snapshot() == initial

    # Positive control: a SCENARIOLOADED record reaches the dispatcher call
    # boundary. Stop before the dispatcher executes any actions.
    first = records[0]
    trigger = read(original, first, '<I')
    startup_stack = original.cpu.reg_read(UC_X86_REG_ESP)
    original.write(ROUND_FLAG, 1, '<I')
    original.write(first, 0x20, '<I')
    try:
        assert run(original, STARTUP_ENTRY, stops, calls=calls) == DISPATCH
        esp = original.cpu.reg_read(UC_X86_REG_ESP)
        assert read(original, esp + 4, '<I') == first
    finally:
        original.cpu.reg_write(UC_X86_REG_ESP, startup_stack)
        original.write(first, trigger, '<I')
    assert snapshot() == initial

    # Single-player round guard. Stop just after the native mark reset and
    # before delay processing or trigger evaluation.
    original.write(0x607652, 0, '<B')
    original.write(0x5D4FA0, 0, '<I')
    original.write(0x5D1570, 4, '<B')
    round_stops = {AFTER_RESET, SKIP_ROUND_START}
    assert run(original, ROUND_ENTRY, round_stops, calls=(RESET_MARKS,)) == SKIP_ROUND_START
    assert snapshot() == initial

    original.write(0x5D1570, 0, '<B')
    try:
        assert run(original, ROUND_ENTRY, round_stops, calls=(RESET_MARKS,)) == AFTER_RESET
        expected = [bytearray(raw) for raw in initial]
        for raw, active in zip(expected, ACTIVE, strict=True):
            modifier = struct.unpack_from('<I', raw, 8)[0]
            assert not modifier & 0x80  # non-Continuous inputs
            struct.pack_into('<I', raw, 8, modifier & ~active)
        assert snapshot() == expected
    finally:
        for record, raw in zip(records, initial, strict=True):
            original.cpu.mem_write(record, raw)
    return {'trigger_words': [read(original, record, '<I') for record in records],
            'round_flag_set_stop': f'{AFTER_SELECTOR:08x}',
            'round_flag_clear_stop': f'{SKIP_SELECTOR:08x}',
            'scenario_loaded_control_stop': f'{DISPATCH:08x}',
            'records_preserved': True,
            'resumed_round_preserved_records': True,
            'fresh_round_cleared_only_active_marks': True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    evidence = json.loads((root / 'docs/evidence/import-event-tail.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(binary) == evidence['binary_sha256']
    save = (home / evidence['source']['path']).read_bytes()
    assert digest(save) == evidence['source']['sha256']
    cities = walk_save_bytes(save)['cities']
    event_offset = cities['offset'] + cities['size'] + evidence['event_offset_after_cities']
    pinned = trace(binary, save[event_offset:], include_record_words=True, after_load=probe_startup)
    assert pinned['round_flag_at_stop'] == 1 and pinned['remaining'] == 0
    pinned_words = pinned['action_and_modifier_words']
    assert pinned_words == [[0x5001, 0x10000108], [0x5001, 0x40000008]]
    assert all(not (action & (SPENT | 0x40) or modifier & active)
               for (action, modifier), active in zip(pinned_words, ACTIVE, strict=True))

    cases = []
    for first_spent, second_spent, first_active, second_active in itertools.product(
        (False, True), repeat=4
    ):
        words = [
            [0x40 | (SPENT if first_spent else 0), 0x10000000 | (ACTIVE[0] if first_active else 0)],
            [0x40 | (SPENT if second_spent else 0), 0x40000000 | (ACTIVE[1] if second_active else 0)],
        ]
        wire = write_events(binary, words, enabled=True)
        assert len(wire) == 8 + 2 * RECORD_SIZE
        assert wire.startswith(b'EVNT') and struct.unpack_from('<I', wire, 4)[0] == 2
        written = [list(struct.unpack_from('<II', wire, 12 + i * RECORD_SIZE))
                   for i in range(2)]
        loaded = trace(binary, wire, include_record_words=True, after_load=probe_startup)
        assert loaded['round_flag_at_stop'] == 1 and loaded['remaining'] == 0
        assert written == words == loaded['action_and_modifier_words']
        assert loaded['after_load'] == pinned['after_load']
        cases.append({'input': words, 'written': written, 'read': loaded['action_and_modifier_words'],
                      'startup_preserved_records': loaded['after_load']['records_preserved'],
                      'resumed_round_preserved_records': loaded['after_load']['resumed_round_preserved_records'],
                      'fresh_round_cleared_only_active_marks': loaded['after_load']['fresh_round_cleared_only_active_marks']})
    disabled = write_events(binary, cases[-1]['input'], enabled=False)
    assert not disabled
    report = {
        'schema': 2,
        'binary_sha256': evidence['binary_sha256'],
        'source': evidence['source'],
        'writer': {'entry': '004605df', 'stop': '00460ada',
                   'write_sizes': [4, 4, RECORD_SIZE, RECORD_SIZE],
                   'substituted_helpers': ['005b4850', '005b4250'],
                   'bytes_with_round_flag_clear': len(disabled)},
        'pinned_action_and_modifier_words': pinned_words,
        'startup': {'entry': f'{STARTUP_ENTRY:08x}', 'selector': f'{STARTUP_SELECTOR:08x}',
                    'pinned_witness': pinned['after_load'], 'synthetic_cases': len(cases)},
        'round_boundary': {'entry': f'{ROUND_ENTRY:08x}', 'reset': f'{RESET_MARKS:08x}',
                           'resume_stop': f'{SKIP_ROUND_START:08x}',
                           'fresh_stop': f'{AFTER_RESET:08x}'},
        'cases': cases,
        'scope': 'Single-player event section writer and reader, a separate game-start selector, and non-Continuous round mark reset. Intervening loader and UI calls, action execution and engine restoration remain outside the trace.',
    }
    args.out.write_text(json.dumps(report, indent=2) + '\n')
    print(f'{len(cases)} native event-state write/read comparisons match')


if __name__ == '__main__':
    main()
