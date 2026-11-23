#!/usr/bin/env python3
"""Execute a controlled native battle after the three unseen opponent moves.

The native Rules refresh loads the combat tables before the battle. Each
control supplies an explicit CRT seed and the engine's spatial order.
The seed-7 control then crosses between a field stack and a friendly city.
Windows timing and presentation are substituted. Native opponent policy,
live input, and macro dispatch are outside this witness.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from import_city_view_check import TILES, measure, move_into_unseen_city, move_opponent
from import_input_handoff_check import IDLE_FILTER_CALL, witness
from import_loader_exit_check import rules_refresh
from import_phase_check import FRAME, frame, read
from import_round_handoff_check import trace
from totctl import parse_rules

CRT = 0x5E2590
PRESENTATION = {0x40D510, 0x467C89, 0x467D19}
IMPORTS = {
    b'\xff\x15\xbc\x1a\x69\x00': ('timeGetTime', 0),
    b'\xff\x15\xe8\x17\x69\x00': ('GetTickCount', 0),
    b'\xff\x15\xa0\x17\x69\x00': ('OutputDebugStringA', 4),
}


def execute(original, viewer, city, rules, seed):
    measure(original, viewer, city)
    frame(original)
    original.write(FRAME - 0x1990, 1, '<I')  # successful virtual save close
    original.write(FRAME - 0xe9c, 0, '<I')
    loaded = rules_refresh(original, rules)
    move_into_unseen_city(original, viewer)
    move_opponent(original, viewer, (10, 7, 3), (10,), (41, 39), (41, 37), ((-1, -1),))
    move_opponent(original, viewer, (19, 6, 3), (19, 34, 2), (43, 37), (42, 36),
                  ((-1, -1), (-1, 2), (34, -1)))

    cpu = original.cpu
    area = read(original, 0x6705A4, '<h')
    cache = read(original, 0x5DEAD0 + 4 * viewer, '<I')

    def knowledge():
        tiles = cpu.mem_read(0x700000, area * 6)
        return ([bool(mask & (1 << viewer)) for mask in tiles[4::6]],
                bytes(cpu.mem_read(cache, area)))

    def unit(slot):
        base = 0x607C58 + 40 * slot
        return {'slot': slot, 'owner': read(original, base + 9, '<B'),
                'position': [read(original, base + off, '<h') for off in (0, 2)],
                'alive': read(original, base + 30, '<I') != 0,
                'spent': read(original, base + 10, '<B'),
                'damage': read(original, base + 12, '<B')}

    original.write(CRT, seed, '<I')
    before = [unit(slot) for slot in (20, 22)]
    saved = knowledge()
    tick = 0
    hosts = []
    display = []
    observes = []

    def boundary(emu, at, size, _data):
        nonlocal tick
        sp = emu.reg_read(UC_X86_REG_ESP)
        if at == 0x41F704:
            observes.append([read(original, sp + off) for off in (4, 8)])
        instruction = bytes(emu.mem_read(at, size))
        if instruction in IMPORTS:
            name, arguments = IMPORTS[instruction]
            hosts.append(name)
            if name != 'OutputDebugStringA':
                tick += 1000
                emu.reg_write(UC_X86_REG_EAX, tick)
            emu.reg_write(UC_X86_REG_ESP, sp + arguments)
            emu.reg_write(UC_X86_REG_EIP, at + size)
        elif at == IDLE_FILTER_CALL:
            hosts.append('PeekMessageA')
            assert [read(original, sp + off) for off in (4, 8, 12, 16)] == [0, 0x3BD, 0x3BD, 1]
            emu.reg_write(UC_X86_REG_EAX, 0)
            emu.reg_write(UC_X86_REG_ESP, sp + 20)
            emu.reg_write(UC_X86_REG_EIP, at + size)
        elif at in PRESENTATION:
            display.append(f'{at:08x}')
            emu.reg_write(UC_X86_REG_EAX, 0)
            emu.reg_write(UC_X86_REG_ESP, sp + 4)
            emu.reg_write(UC_X86_REG_EIP, read(original, sp, '<I'))

    hook = cpu.hook_add(UC_HOOK_CODE, boundary)
    try:
        result = original.call(0x54C69B, 20, 4, 3)
    finally:
        cpu.hook_del(hook)
    after = [unit(slot) for slot in (20, 22)]
    assert result == 1 and [u['alive'] for u in before] == [True, True]
    assert [u['alive'] for u in after] == [True, False]
    assert before[0]['position'] == after[0]['position'] == [23, 45]
    assert before[1]['position'] == [22, 46]
    assert observes == [[20, 1]]
    assert saved == knowledge()
    return {
        'crt_before': seed, 'crt_after': read(original, CRT, '<I'),
        'arguments': [20, 4, 3], 'return': result,
        'before': before, 'after': after,
        'viewer_explored_tiles': sum(saved[0]), 'viewer_memory_changed': False,
        'observation_calls': observes, 'rules_refresh': loaded,
        'host_substitutes': hosts, 'display_substitutes': display,
    }


def set_route(original, x, y, flags):
    width = read(original, 0x6705A0, '<h') // 2
    address = TILES + 6 * (y * width + x // 2) + 1
    original.write(address, (read(original, address, '<B') & ~0x30) | flags, '<B')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    evidence = json.loads((root / 'docs/evidence/import-city-view.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert hashlib.sha256(binary).hexdigest() == evidence['binary_sha256']
    for role in ('source', 'rules'):
        source = evidence[role]
        assert hashlib.sha256((home / source['path']).read_bytes()).hexdigest() == source['sha256']
    rules = parse_rules(str(home / evidence['rules']['path']))

    def run_case(seed, field_routes=None, city_control=None, departure_control=None):
        def entry(original, player, city):
            nonlocal result
            result = execute(original, player, city, rules, seed)
            if seed != 7:
                return
            if field_routes is not None:
                for x, flags in zip((11, 9), field_routes):
                    set_route(original, x, 45, flags)
            stack = move_opponent(original, player, (4, 5, 3), (4, 7), (11, 45), (9, 45),
                                  ((7, -1), (-1, 4)))
            result['following_field_stack'] = stack
            if field_routes is None and city_control is None and departure_control is None:
                refused = move_opponent(original, player, (4, 3, 3), (4, 7), (9, 45), (9, 45),
                                        ((7, -1), (-1, 4)), expected_return=0)
                assert refused['after'] == stack['after'] and refused['after'][0]['spent'] == 6
                result['refused_departure'] = refused
            if field_routes is None:
                if city_control is None:
                    attack = 1
                else:
                    attack, origin, destination = city_control
                    set_route(original, 9, 45, origin)
                    set_route(original, 8, 46, destination)
                    original.write(0x5F470C + 40 * 2, attack, '<B')  # Warrior attack
                links = ((-1, 24), (-1, -1), (7, -1)) if attack else ((24, -1), (-1, -1), (-1, 7))
                result['following_city_arrival'] = move_opponent(
                    original, player, (7, 4, 3), (7, 4, 24), (9, 45), (8, 46), links,
                )
            if field_routes is None and city_control is None:
                if departure_control is None:
                    attack = 1
                else:
                    attack, flags = departure_control
                    set_route(original, 9, 45, flags)
                    original.write(0x5F470C + 40 * 2, attack, '<B')  # Warrior attack
                links = ((-1, 4), (-1, -1), (24, -1)) if attack else ((4, -1), (-1, -1), (-1, 24))
                result['following_city_departure'] = move_opponent(
                    original, player, (24, 0, 3), (24, 7, 4), (8, 46), (9, 45), links,
                )

        def prepared(original, player, regions):
            trace(original, player, regions, clear_preference_bit=False, skip_selected=True,
                  at_city_entry=entry)

        result = None
        witness(home, binary, evidence['source']['path'], exhaust=False,
                observation=True, after_idle=prepared)
        assert result is not None
        return result

    cases = {str(seed): run_case(seed) for seed in (7, 1)}
    assert [cases[str(seed)]['after'][0]['damage'] for seed in (7, 1)] == [0, 5]
    assert all(case['viewer_explored_tiles'] == 59 for case in cases.values())
    controls = []
    for origin in (0, 0x10, 0x30):
        for destination in (0, 0x10, 0x30):
            stack = run_case(7, (origin, destination))['following_field_stack']
            assert stack['before'][0]['spent'] == 0 and stack['explored_tiles'] == 59
            controls.append({'origin_flags': origin, 'destination_flags': destination,
                             'spent': stack['after'][0]['spent']})
    assert [case['spent'] for case in controls] == [6, 6, 6, 6, 1, 1, 6, 1, 0]
    assert cases['7']['following_city_arrival']['after'][0]['spent'] == 1
    city_controls = []
    for attack, origin, destination, spent in [(0, 16, 0, 1), (1, 0, 0, 3), (1, 48, 0, 0),
                                               (1, 48, 48, 0), (1, 0, 48, 3)]:
        arrival = run_case(7, city_control=(attack, origin, destination))['following_city_arrival']
        mover = arrival['after'][0]
        assert mover['spent'] == spent and arrival['explored_tiles'] == 59
        city_controls.append({'attack': attack, 'origin_flags': origin, 'destination_flags': destination,
                              'spent': spent, 'mover_previous': mover['previous'], 'mover_next': mover['next']})
    assert cases['7']['following_city_departure']['after'][0]['spent'] == 1
    departures = []
    for attack, flags, spent in [(0, 16, 1), (1, 0, 6), (1, 48, 0)]:
        departure = run_case(7, departure_control=(attack, flags))['following_city_departure']
        mover = departure['after'][0]
        assert mover['spent'] == spent and departure['explored_tiles'] == 59
        departures.append({'attack': attack, 'destination_flags': flags, 'spent': spent,
                           'mover_previous': mover['previous'], 'mover_next': mover['next']})
    report = {'schema': 1, **{key: evidence[key] for key in ('binary_sha256', 'source', 'rules')},
              'cases': cases, 'route_controls': controls, 'city_arrival_controls': city_controls,
              'city_departure_controls': departures, 'scope': __doc__.strip()}
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
