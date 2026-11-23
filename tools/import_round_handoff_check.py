#!/usr/bin/env python3
"""Trace the pinned resumed turn into the next living civilization's city phase.

Supply already translated characters at the native general poll and forward
them to the original character callback. The exhausted viewing control
compares one Enter under two process preferences. The selected-unit trace
starts with saved movement and spends it with four Space orders. The stop
precedes opponent city processing. Windows dispatch and the rest of the
round remain outside the trace.
"""
import argparse
from collections import deque
import hashlib
import json
import os
from pathlib import Path
import struct

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from import_first_order_check import ALLOWED_CALLS as ORDER_CALLS
from import_input_handoff_check import (
    IDLE_FILTER_CALL, INPUT_PREPARATION, OBSERVATION_REDRAW, PEEK_MESSAGE_CALL, POLL_HELPERS, witness,
)
from import_phase_check import CIV, FRAME, STRIDE, read, run
from import_unit_setup_check import ACTIVE_SLOT

CHARACTER = 0x40F592
TRANSLATE = 0x576EF6
DISPATCH = 0x576F00
DISPATCH_RETURN = DISPATCH + 6
TURN_START = 0x4725C8
CITY_PHASE = 0x4CB01D
PRESENTATION = (
    0x40D510, 0x4413D0, 0x442570, 0x46D7ED, 0x41AF00,
    0x5346BE, 0x40D7E7, 0x4BF9B6, OBSERVATION_REDRAW,
)
CALLS = POLL_HELPERS + PRESENTATION + ORDER_CALLS + (
    INPUT_PREPARATION, 0x411C70, 0x40DD3E, 0x40E10C, 0x5B1770, 0x40F04A, 0x411C50,
    0x4ED4B6, 0x4EB017, 0x532848, TURN_START, 0x4AC70E, 0x420E98,
    0x471CDF, 0x4709D1, 0x4F004F, CITY_PHASE, 0x473127, 0x472864,
)
WINDOW_CONTEXT = (
    (0x5CED04, 1), (0x5CEB1C, -1), (0x5CEB20, -1),
    (0x657848 + 0x1A00, -1), (0x5E5F18 + 0xC14, -1),
)


def trace(original, player, regions, *, clear_preference_bit, skip_selected, at_city_entry=None):
    cpu = original.cpu
    for address, value in WINDOW_CONTEXT:
        original.write(address, value, '<i')
    saved_options = read(original, 0x60763A, '<I')
    assert saved_options & 0x40
    options = saved_options & ~0x40 if clear_preference_bit else saved_options
    original.write(0x60763A, options, '<I')

    area = read(original, 0x6705A4, '<h')
    regions = regions + [(0x700000, 6 * area)] + [
        (read(original, 0x5DEAD0 + 4 * civ, '<I'), area) for civ in range(1, 8)
    ]

    def capture():
        return [bytes(cpu.mem_read(a, n)) for a, n in regions]

    def phase():
        record = CIV + STRIDE * player
        return {
            'turn': read(original, 0x607648, '<h'),
            'active_civ': read(original, 0x607655, '<B'),
            'loop_civ': read(original, FRAME - 0x6AC),
            'resume_civ': read(original, 0x5D1570, '<B'),
            'human_mask': read(original, 0x60765B, '<B'),
            'active_slot': read(original, ACTIVE_SLOT, '<h'),
            'display_mode': read(original, 0x67A6E8),
            'viewing_loop_running': read(original, 0x5F5A6C, '<B'),
            'research_target': read(original, record + 18, '<h'),
            'research_progress': read(original, record + 16, '<h'),
        }

    before = phase()
    baseline = capture()
    units = baseline[1]
    owned = [slot for slot in range(len(units) // 40)
             if units[40 * slot + 9] == player and int.from_bytes(units[40 * slot + 30:40 * slot + 34], 'little')]
    expected_spent = 0 if skip_selected else 255
    assert all(units[40 * slot + 10] == expected_spent for slot in owned)
    pending = deque([32] * len(owned) if skip_selected else [13])
    stops = {CITY_PHASE}
    messages = []
    filtered_polls = 0
    host = []
    msg = None
    turn_starts = []
    substituted = []
    def host_boundary(emu, at, _size, _data):
        nonlocal msg, filtered_polls
        sp = emu.reg_read(UC_X86_REG_ESP)
        if at == PEEK_MESSAGE_CALL:
            if not pending:
                stops.add(at)
                return
            msg = read(original, sp, '<I')
            assert msg != 0 and [read(original, sp + off) for off in (4, 8, 12, 16)] == [0, 0, 0, 1]
            key = pending.popleft()
            messages.append({'character': key, 'active_slot': read(original, ACTIVE_SLOT, '<h'),
                             'display_mode': read(original, 0x67A6E8)})
            emu.mem_write(msg, struct.pack('<7I', 1, 0x102, key, 1, 0, 0, 0))
            host.append('PeekMessageA')
            emu.reg_write(UC_X86_REG_ESP, sp + 20)
            emu.reg_write(UC_X86_REG_EAX, 1)
            emu.reg_write(UC_X86_REG_EIP, at + 6)
        elif at == IDLE_FILTER_CALL:
            assert [read(original, sp + off) for off in (4, 8, 12, 16)] == [0, 0x3BD, 0x3BD, 1]
            filtered_polls += 1
            emu.reg_write(UC_X86_REG_ESP, sp + 20)
            emu.reg_write(UC_X86_REG_EAX, 0)
            emu.reg_write(UC_X86_REG_EIP, at + 6)
        elif at == TURN_START:
            turn_starts.append({'argument': read(original, sp + 4), 'phase': phase()})
        elif at == TRANSLATE:
            assert read(original, sp, '<I') == msg
            host.append('TranslateMessageA')
            emu.reg_write(UC_X86_REG_EAX, 0)
            emu.reg_write(UC_X86_REG_ESP, sp + 4)
            emu.reg_write(UC_X86_REG_EIP, at + 6)
        elif at == DISPATCH:
            assert read(original, sp, '<I') == msg
            assert read(original, msg + 4, '<I') == 0x102
            host.append('DispatchMessageA')
            # Forward WM_CHAR to the documented game callback. Its cdecl
            # return leaves the character word for us to pop below.
            emu.mem_write(sp - 4, struct.pack('<II', DISPATCH_RETURN, read(original, msg + 8)))
            emu.reg_write(UC_X86_REG_ESP, sp - 4)
            emu.reg_write(UC_X86_REG_EIP, CHARACTER)
        elif at == DISPATCH_RETURN:
            emu.reg_write(UC_X86_REG_ESP, sp + 4)
        elif at in PRESENTATION:
            substituted.append(f'{at:08x}')
            emu.reg_write(UC_X86_REG_EAX, 0)
            emu.reg_write(UC_X86_REG_ESP, sp + 4)
            emu.reg_write(UC_X86_REG_EIP, read(original, sp, '<I'))

    hook = cpu.hook_add(UC_HOOK_CODE, host_boundary)
    try:
        stop = run(original, PEEK_MESSAGE_CALL, stops, calls=CALLS, instruction_limit=400000)
    finally:
        cpu.hook_del(hook)
    assert not pending
    assert host == ['PeekMessageA', 'TranslateMessageA', 'DispatchMessageA'] * len(messages)
    after = phase()
    final = capture()
    city = None
    if stop == CITY_PHASE:
        city = read(original, cpu.reg_read(UC_X86_REG_ESP) + 4)
        assert 0 <= city < regions[2][1] // 92
        assert read(original, regions[2][0] + 92 * city + 10, '<B') == after['active_civ']
    assert final[2:] == baseline[2:]  # cities, tiles and all seven knowledge caches
    civ_record = slice(STRIDE * player, STRIDE * (player + 1))
    assert final[0][civ_record] == baseline[0][civ_record]
    for slot in owned:
        unit_record = slice(40 * slot, 40 * (slot + 1))
        expected = bytearray(units[unit_record])
        if skip_selected:
            expected[10], expected[13] = 3, 255
        assert final[1][unit_record] == expected
    if city is None:
        assert final == baseline
    if city is not None and at_city_entry is not None:
        at_city_entry(original, player, city)
    return {
        'options': f'{options:08x}', 'before': before, 'after': after,
        'own_unit_slots': owned,
        'dispatched': messages, 'empty_filtered_polls': filtered_polls,
        'boundary': 'city-phase-entry' if city is not None else 'next-general-poll',
        'stop': f'{stop:08x}', 'city_slot': city, 'turn_starts': turn_starts,
        'viewer_civilization_and_map_knowledge_unchanged': True,
        'own_unit_record_change': 'spent movement and last direction' if skip_selected else 'none',
        'presentation_substitutes': substituted, 'host_substitutes': host,
    }


def case(home, binary, path, *, clear_preference_bit=False, skip_selected=False):
    result = {}

    def continuation(original, player, regions):
        result.update(trace(original, player, regions, clear_preference_bit=clear_preference_bit,
                            skip_selected=skip_selected))

    witness(home, binary, path, exhaust=not skip_selected, observation=skip_selected, after_idle=continuation)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True, type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    handoff = json.loads((root / 'docs/evidence/import-input-handoff.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert hashlib.sha256(binary).hexdigest() == handoff['binary_sha256']
    for role in ('source', 'rules'):
        source = handoff[role]
        assert hashlib.sha256((home / source['path']).read_bytes()).hexdigest() == source['sha256']
    path = handoff['source']['path']
    saved = case(home, binary, path, clear_preference_bit=False)
    advanced = case(home, binary, path, clear_preference_bit=True)
    actual = case(home, binary, path, skip_selected=True)
    assert saved['before'] == saved['after'] == advanced['before']
    assert saved['own_unit_slots'] == advanced['own_unit_slots'] == actual['own_unit_slots'] == [6, 9, 18, 26]
    assert saved['boundary'] == 'next-general-poll' and saved['turn_starts'] == []
    assert advanced['boundary'] == 'city-phase-entry' and advanced['city_slot'] == 6
    assert advanced['after'] == advanced['before'] | {
        'active_civ': 5, 'loop_civ': 5, 'resume_civ': 0, 'viewing_loop_running': 0,
    }
    assert advanced['turn_starts'] == [{'argument': 5, 'phase': advanced['after']}]
    assert actual['options'] == saved['options']
    assert actual['after'] == advanced['after']
    assert actual['turn_starts'] == advanced['turn_starts']
    assert actual['city_slot'] == advanced['city_slot']
    assert actual['dispatched'] == [
        {'character': 32, 'active_slot': slot, 'display_mode': 1} for slot in (6, 9, 26, 18)
    ]
    report = {
        'schema': 1,
        **{key: handoff[key] for key in ('binary_sha256', 'source', 'rules')},
        'message_kind': 'WM_CHAR',
        'prepared_window_globals': {f'{a:08x}': v for a, v in WINDOW_CONTEXT},
        'saved_options': saved,
        'cleared_preference_bit_0x40': advanced,
        'spend_saved_movement': actual,
        'scope': __doc__.strip(),
    }
    with args.out.open('x') as file:
        file.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
