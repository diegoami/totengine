#!/usr/bin/env python3
"""Execute the original game's Wait, Skip and northwest move after prepared resume.

The resume witness reaches the input loop and completes an empty poll. This
probe supplies characters or an internal key code directly to the game
callbacks. It prepares the running-window guards and substitutes display,
audio and host boundaries. Windows message dispatch and live input are outside
this trace.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from import_input_handoff_check import (
    DISPLAY_HELPERS, IDLE_FILTER_CALL, OBSERVATION_HELPERS, OBSERVATION_REDRAW, OBSERVE, witness,
)
from import_phase_check import CIV, STRIDE, read, run
from import_unit_setup_check import ACTIVE_SLOT, CALLS, UNIT_RECORDS

CHARACTER_CALLBACK = 0x40F592
KEY_CALLBACK = 0x40FBD4
MOVE = 0x54C69B
NORTHWEST = 0xA7
HOST_CALLS = {0x5B0A50: 'XD_InFlushSendBuffer', 0x54F34F: 'GetAsyncKeyState',
              IDLE_FILTER_CALL: 'PeekMessageA'}
PRESENTATION = DISPLAY_HELPERS + (OBSERVATION_REDRAW, 0x5373EB)
RETURN = 0x100000
TILES = 0x700000
ALLOWED_CALLS = CALLS + OBSERVATION_HELPERS + DISPLAY_HELPERS + (
    0x411C70, 0x40DD3E, 0x40E10C, 0x5B1770, 0x5B1630, 0x40F1C4,
    0x5475D6, 0x547530, 0x54C69B, 0x4029F0, 0x57704E, 0x576FF6,
    0x57145C, 0x570169, 0x56CDDC, 0x4EDE27, 0x490851, 0x5719C8, OBSERVATION_REDRAW,
    0x5B0A50, 0x40E1E6, 0x5706E4, 0x56B307, 0x56B1D3, 0x56C174,
    0x569AFE, 0x569547, 0x569856, 0x569113, 0x5373EB, 0x56ADFE, 0x56ADD6,
    0x568FDF, 0x56AE5B, 0x57093F, 0x569F41,
)
# These are native globals representing the already-open, quiet window.
WINDOW_CONTEXT = ((0x5CED04, 1), (0x5CEB1C, -1), (0x5CEB20, -1))


def changes(before, after):
    return [(i, old, new) for i, (old, new) in enumerate(zip(before, after, strict=True))
            if old != new]


def execute(original, player, regions, key, *, return_address=RETURN):
    move = key == NORTHWEST
    entry = KEY_CALLBACK if move else CHARACTER_CALLBACK
    cpu = original.cpu
    for address, value in WINDOW_CONTEXT:
        original.write(address, value, '<i')
    area = read(original, 0x6705A4, '<h')
    regions = regions + [(TILES, 6 * area)] + [
        (read(original, 0x5DEAD0 + 4 * civ, '<I'), area) for civ in range(1, 8)
    ]

    def snapshot():
        return [bytes(cpu.mem_read(address, length)) for address, length in regions]

    def selection():
        research = CIV + STRIDE * player
        return {
            'active_slot': read(original, ACTIVE_SLOT, '<h'),
            'display_mode': read(original, 0x67A6E8),
            'research_target': read(original, research + 18, '<h'),
            'research_progress': read(original, research + 16, '<h'),
        }

    before = snapshot()
    before_selection = selection()
    presentation = []
    host = []
    handlers = []
    move_args = []
    observation_args = []

    def substitute(emu, address, _size, _data):
        sp = emu.reg_read(UC_X86_REG_ESP)
        if address in (0x5475D6, 0x547530):
            handlers.append(f'{address:08x}')
        if address == OBSERVE:
            observation_args.append([read(original, sp + off, '<i') for off in (4, 8)])
        if address == MOVE and move:
            move_args.append([read(original, sp + off, '<i') for off in (4, 8, 12)])
        if address not in PRESENTATION and address not in HOST_CALLS:
            return
        emu.reg_write(UC_X86_REG_EAX, 0)
        if address == IDLE_FILTER_CALL:
            filters = [read(original, sp + off, '<I') for off in (4, 8, 12, 16)]
            assert filters == [0, 0x3BD, 0x3BD, 1]
            host.append({'import': HOST_CALLS[address], 'filter': filters[1:3], 'result': 0})
            emu.reg_write(UC_X86_REG_ESP, sp + 20)
            emu.reg_write(UC_X86_REG_EIP, address + 6)
        elif address == 0x54F34F:
            assert read(original, sp, '<I') == 0x10
            host.append({'import': HOST_CALLS[address], 'key': 0x10, 'result': 0})
            emu.reg_write(UC_X86_REG_ESP, sp + 4)
            emu.reg_write(UC_X86_REG_EIP, address + 6)
        else:
            if address in HOST_CALLS:
                host.append({'import': HOST_CALLS[address], 'result': 0})
            else:
                presentation.append(f'{address:08x}')
            emu.reg_write(UC_X86_REG_ESP, sp + 4)
            emu.reg_write(UC_X86_REG_EIP, read(original, sp, '<I'))

    hook = cpu.hook_add(UC_HOOK_CODE, substitute)
    try:
        cpu.mem_write(0x108000, struct.pack('<II', return_address, key))
        cpu.reg_write(original.sp, 0x108000)
        run(original, entry, {return_address}, calls=ALLOWED_CALLS, instruction_limit=200000)
    finally:
        cpu.hook_del(hook)

    after = snapshot()
    after_selection = selection()
    assert before[0] == after[0] and before[2] == after[2]
    assert read(original, 0x65BE34) == 0  # input callback's in-progress guard cleared
    unit_idx = before_selection['active_slot']
    unit = UNIT_RECORDS + 40 * unit_idx
    record = slice(40 * unit_idx, 40 * (unit_idx + 1))
    cost = read(original, 0x5F470A + 40 * read(original, unit + 8, '<B'), '<B')
    return {
        'entry': f'{entry:08x}',
        'input_code': key,
        'handlers': handlers,
        'move_arguments': move_args,
        'observation_arguments': observation_args,
        'before': before_selection,
        'after': after_selection,
        'unit_position_before': struct.unpack_from('<hh', before[1][record]),
        'unit_position_after': struct.unpack_from('<hh', after[1][record]),
        'active_unit_type_allowance': cost,
        'unit_byte_changes': [
            {'slot': offset // 40, 'byte': offset % 40, 'before': old, 'after': new}
            for offset, old, new in changes(before[1], after[1])
        ],
        'civ_city_bytes_unchanged': True,
        'tile_byte_changes': changes(before[3], after[3]),
        'cache_byte_changes': [
            [civ, index, old, new]
            for civ, (before_cache, after_cache) in enumerate(zip(before[4:], after[4:], strict=True), 1)
            for index, old, new in changes(before_cache, after_cache)
        ],
        'explored_tiles': sum(bool(mask & (1 << player)) for mask in after[3][4::6]),
        'presentation_substitutes': presentation,
        'host_substitutes': host,
    }


def case(home, binary, path, key):
    result = {}

    def trace(original, player, regions):
        result.update(execute(original, player, regions, key))

    witness(home, binary, path, exhaust=False, observation=True, after_idle=trace)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True, type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    handoff = json.loads((root / 'docs/evidence/import-input-handoff.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    path = handoff['source']['path']
    binary = (home / 'civ2.bak').read_bytes()
    assert hashlib.sha256(binary).hexdigest() == handoff['binary_sha256']
    for role in ('source', 'rules'):
        source = handoff[role]
        assert hashlib.sha256((home / source['path']).read_bytes()).hexdigest() == source['sha256']
    inputs = [('wait', ord('w')), ('skip', ord(' ')), ('move_northwest', NORTHWEST)]
    cases = {name: case(home, binary, path, key) for name, key in inputs}
    for name, row in cases.items():
        assert row['before'] == {
            'active_slot': 6, 'display_mode': 1, 'research_target': -1, 'research_progress': 0,
        }
        assert row['after'] == row['before'] | {'active_slot': 9 if name == 'wait' else 6}
        assert row['unit_position_before'] == (63, 5)
        assert row['active_unit_type_allowance'] == 3
        if name != 'move_northwest':
            assert row['unit_position_after'] == (63, 5)
            assert row['tile_byte_changes'] == row['cache_byte_changes'] == []
            assert row['explored_tiles'] == 59
    assert cases['wait']['handlers'] == ['005475d6']
    assert cases['wait']['unit_byte_changes'] == [
        {'slot': 6, 'byte': 7, 'before': 2, 'after': 66},
    ]
    assert cases['skip']['handlers'] == ['00547530']
    assert cases['skip']['unit_byte_changes'] == [
        {'slot': 6, 'byte': 10, 'before': 0, 'after': 3},
        {'slot': 6, 'byte': 13, 'before': 1, 'after': 255},
    ]
    moved = cases['move_northwest']
    assert moved['move_arguments'] == [[6, 6, 3]]
    assert moved['observation_arguments'] == [[6, 1]]
    assert moved['unit_position_after'] == (62, 4)
    assert moved['unit_byte_changes'] == [
        {'slot': 6, 'byte': index, 'before': old, 'after': new}
        for index, old, new in ((0, 63, 62), (2, 5, 4), (6, 0, 64), (10, 0, 6), (13, 1, 6))
    ]
    assert moved['explored_tiles'] == 60
    assert moved['tile_byte_changes'] == [(670, 0, 16), (1147, 0, 1),
                                         (1151, 240, 64), (1387, 1, 0)]
    assert moved['presentation_substitutes'].count('005373eb') == 1
    assert moved['cache_byte_changes'] == [[4, 191, 0, 1], [4, 231, 1, 0]]
    report = {
        'schema': 1,
        **{key: handoff[key] for key in ('binary_sha256', 'source', 'rules')},
        'prepared_window_globals': {f'{address:08x}': value for address, value in WINDOW_CONTEXT},
        'cases': cases,
        'scope': __doc__.strip(),
    }
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
