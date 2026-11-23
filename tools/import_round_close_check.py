#!/usr/bin/env python3
"""Measure saved human knowledge across the annual update and fresh input.

Advance the pinned save through the checked opponent moves, then run the
unpatched annual update called by the single-player outer loop. Windows
rectangle and directory calls operate on emulator memory. The supplied
end-of-round position and host directory failures are controlled inputs.
Each fresh input control starts from the same turn-44 handoff.
"""
from collections import Counter
import argparse
import hashlib
import json
import os
from pathlib import Path

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from import_city_view_check import TILES, move_opponent
from import_first_order_check import NORTHWEST, execute as first_order
from import_input_handoff_check import measure_handoff, witness
from import_opponent_combat_view_check import (
    CRT, execute, partial_movement, singleton_movement, unconnected_road,
)
from import_phase_check import CIV, FRAME, STRIDE, frame, read
from import_round_handoff_check import TURN_START, trace
from totctl import parse_rules

ENTRY = 0x4702FC
HOST_IMPORTS = {
    0x691A84: 'SetRect',
    0x69175C: 'GetCurrentDirectoryA',
    0x691758: 'SetCurrentDirectoryA',
    0x6917A4: 'GetLastError',
    0x691764: 'SetEnvironmentVariableA',
}


def c_string(original, pointer):
    data = bytes(original.cpu.mem_read(pointer, 260))
    value, end, _ = data.partition(b'\0')
    assert end
    return value


def annual_update(original, viewer):
    area = read(original, 0x6705A4, '<h')
    pointer = read(original, 0x5DEAD0 + 4 * viewer, '<I')
    original.write(0x607655, viewer, '<B')  # outer loop restores the human owner
    assert read(original, 0x5D1570, '<B') == 0

    def capture():
        tiles = original.cpu.mem_read(TILES, 6 * area)
        explored = tuple(bool(mask & (1 << viewer)) for mask in tiles[4::6])
        memory = bytes(original.cpu.mem_read(pointer, area))
        return {'turn': read(original, 0x607648, '<h'),
                'crt': read(original, CRT, '<I'), 'explored': sum(explored)}, explored, memory

    cpu = original.cpu
    before, explored_before, memory_before = capture()
    directory = b'C:\\CIVTOT'
    hosts = []
    observations = []
    draws = []

    def boundary(cpu, at, size, _):
        nonlocal directory
        if at == 0x41F704:
            observations.append(at)
        elif at == 0x5B11D0:
            draws.append(read(original, CRT, '<I'))
        instruction = bytes(cpu.mem_read(at, size))
        if not instruction.startswith(b'\xff\x15'):
            return
        host = HOST_IMPORTS[int.from_bytes(instruction[2:], 'little')]
        hosts.append(host)
        sp = cpu.reg_read(UC_X86_REG_ESP)

        def argument(index):
            return read(original, sp + 4 * index, '<I')

        if host == 'SetRect':
            cpu.mem_write(argument(0), bytes(cpu.mem_read(sp + 4, 16)))
            argc, result = 5, 1
        elif host == 'GetCurrentDirectoryA':
            assert argument(0) > len(directory)
            cpu.mem_write(argument(1), directory + b'\0')
            argc, result = 2, len(directory)
        elif host == 'SetCurrentDirectoryA':
            requested = c_string(original, argument(0))
            argc, result = 1, int(bool(requested))
            if requested:
                directory = requested
        elif host == 'GetLastError':
            argc, result = 0, 2
        else:
            assert host == 'SetEnvironmentVariableA'
            argc, result = 2, 1
        cpu.reg_write(UC_X86_REG_ESP, sp + 4 * argc)
        cpu.reg_write(UC_X86_REG_EAX, result)
        cpu.reg_write(UC_X86_REG_EIP, at + size)

    hook = cpu.hook_add(UC_HOOK_CODE, boundary)
    try:
        original.call(ENTRY, 0xffffffff)
    finally:
        cpu.hook_del(hook)
    assert cpu.reg_read(UC_X86_REG_EIP) == 0x100000
    after, explored_after, memory_after = capture()
    assert before == {'turn': 43, 'crt': 0x6f277c6c, 'explored': 59}
    assert after == {'turn': 44, 'crt': 0xd766ce46, 'explored': 59}
    assert explored_before == explored_after and memory_before == memory_after
    assert not observations and len(draws) == 2
    return {'entry': f'{ENTRY:08x}', 'argument': -1, 'before': before, 'after': after,
            'viewer_memory_changed': False, 'observation_calls': observations,
            'crt_draws': len(draws), 'host_calls': dict(Counter(hosts))}


def fresh_human_input(original, viewer, city_region, key):
    cpu = original.cpu
    area = read(original, 0x6705A4, '<h')
    pointer = read(original, 0x5DEAD0 + 4 * viewer, '<I')

    def knowledge():
        tiles = cpu.mem_read(TILES, 6 * area)
        return (tuple(bool(mask & (1 << viewer)) for mask in tiles[4::6]),
                bytes(cpu.mem_read(pointer, area)))

    def budgets():
        return {str(slot): read(original, 0x607C58 + 40 * slot + 10, '<B')
                for slot in (6, 9, 18, 26)}

    initial = knowledge()
    before = budgets()
    options = read(original, 0x60763A, '<I')
    original.write(0x60763A, options & ~0x2000, '<I')  # disable per-civ autosave in this control
    substitutes = []

    def redraw(cpu, address, _size, _data):
        if address != 0x467C89:
            return
        sp = cpu.reg_read(UC_X86_REG_ESP)
        substitutes.append(f'{address:08x}')
        cpu.reg_write(UC_X86_REG_EAX, 0)
        cpu.reg_write(UC_X86_REG_ESP, sp + 4)
        cpu.reg_write(UC_X86_REG_EIP, read(original, sp, '<I'))

    hook = cpu.hook_add(UC_HOOK_CODE, redraw)
    try:
        original.call(TURN_START, viewer)
    finally:
        cpu.hook_del(hook)
    assert cpu.reg_read(UC_X86_REG_EIP) == 0x100000
    assert substitutes == ['00467c89']
    assert before == budgets() == {'6': 0, '9': 0, '18': 0, '26': 0}
    assert initial == knowledge()

    frame(original)
    original.write(FRAME - 0x6AC, viewer, '<I')
    count = read(original, 0x607666, '<h')
    regions = [(CIV, STRIDE * 8), (0x607C58, count * 40), city_region]
    handoff = measure_handoff(original, viewer, regions, observation=True,
                              entry=0x474BC8, extra_calls=(0x570169,))
    assert initial == knowledge()
    assert handoff['active_slot'] == 18
    assert handoff['observation_to_input']['first_poll']['active_slot'] == 18
    assert handoff['research_target'] == -1 and handoff['research_progress'] == 0
    order = first_order(original, viewer, regions, key, return_address=0x100040)
    assert order['before']['active_slot'] == 18
    assert order['after']['research_target'] == -1
    assert order['after']['research_progress'] == 0
    assert order['observation_arguments'] == [[18, 1]]
    return {'autosave_disabled': bool(options & 0x2000), 'spent_after_annual_update': before,
            'turn_start_substitutes': substitutes, 'handoff': handoff}, order


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True, type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    prior = json.loads((root / 'docs/evidence/import-opponent-combat-view.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert hashlib.sha256(binary).hexdigest() == prior['binary_sha256']
    for key in ('source', 'rules'):
        item = prior[key]
        assert hashlib.sha256((home / item['path']).read_bytes()).hexdigest() == item['sha256']
    rules = parse_rules(str(home / prior['rules']['path']))
    prepared = {}
    city_region = None
    next_order = None

    def at_city(original, viewer, city):
        nonlocal next_order
        execute(original, viewer, city, rules, 7)
        move_opponent(original, viewer, (4, 5, 3), (4, 7), (11, 45), (9, 45),
                      ((7, -1), (-1, 4)))
        move_opponent(original, viewer, (7, 4, 3), (7, 4, 24), (9, 45), (8, 46),
                      ((-1, 24), (-1, -1), (7, -1)))
        move_opponent(original, viewer, (24, 0, 3), (24, 7, 4), (8, 46), (9, 45),
                      ((-1, 4), (-1, -1), (24, -1)))
        partial_movement(original, viewer, None)
        singleton_movement(original, viewer, None)
        unconnected_road(original, viewer)
        prepared['last_move'] = move_opponent(original, viewer, (33, 4, 3), (33,),
                                              (11, 41), (10, 42), ((-1, -1),))
        prepared['annual_update'] = annual_update(original, viewer)
        prepared['fresh_human_input'], next_order = fresh_human_input(original, viewer, city_region, key)

    def after_idle(original, viewer, regions):
        nonlocal city_region
        city_region = regions[2]
        trace(original, viewer, regions, clear_preference_bit=False, skip_selected=True,
              at_city_entry=at_city)

    controls = {}
    for name, key in [('skip', ord(' ')), ('move_northwest', NORTHWEST)]:
        witness(home, binary, prior['source']['path'], exhaust=False,
                observation=True, after_idle=after_idle)
        controls[name] = next_order
    assert controls['skip']['unit_byte_changes'] == [{'slot': 18, 'byte': 10, 'before': 0, 'after': 3}]
    assert controls['skip']['explored_tiles'] == 59
    move = controls['move_northwest']
    assert move['explored_tiles'] == 60 and move['unit_position_after'] == (50, 8)
    assert move['tile_byte_changes'] == [(2068, 0, 16), (2071, 0, 1), (2075, 240, 64), (2311, 1, 0)]
    assert move['cache_byte_changes'] == [[4, 345, 0, 1], [4, 385, 1, 0]]
    report = {'schema': 1, **{key: prior[key] for key in ('binary_sha256', 'source', 'rules')},
              **prepared, 'fresh_orders': controls, 'scope': __doc__.strip()}
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
