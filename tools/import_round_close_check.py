#!/usr/bin/env python3
"""Measure saved human knowledge across a prepared native annual update.

Advance the pinned save through the checked opponent moves, then run the
unpatched annual update called by the single-player outer loop. Windows
rectangle and directory calls operate on emulator memory. The supplied
end-of-round position and host directory failures are controlled inputs.
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
from import_input_handoff_check import witness
from import_opponent_combat_view_check import (
    CRT, execute, partial_movement, singleton_movement, unconnected_road,
)
from import_phase_check import read
from import_round_handoff_check import trace
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

    def at_city(original, viewer, city):
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

    def after_idle(original, viewer, regions):
        trace(original, viewer, regions, clear_preference_bit=False, skip_selected=True,
              at_city_entry=at_city)

    witness(home, binary, prior['source']['path'], exhaust=False,
            observation=True, after_idle=after_idle)
    report = {'schema': 1, **{key: prior[key] for key in ('binary_sha256', 'source', 'rules')},
              **prepared, 'scope': __doc__.strip()}
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
