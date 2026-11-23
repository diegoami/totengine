#!/usr/bin/env python3
"""Measure the first native human city pass after the pinned production purchase.

Prepare round 44 from the established opponent and annual trace. Supply the
measured city worker, specialist, shield and treasury results before the
human city pass. Run the native turn start and input preparation. Disable
autosave and substitute the redraw call, as in the quiet turn-44 control.
The surplus control starts with five more stored shields.
"""
import argparse
import json
import os
from pathlib import Path

from unicorn import UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from import_event_tail_check import digest
from import_first_order_check import NORTHWEST, execute as first_order
from import_phase_check import CIV, FRAME, STRIDE, frame, read
from import_round_close_check import TILES, TURN_START, measure_handoff, prepare_round
from totctl import parse_rules

CITY_SLOT = 5
UNIT_COUNT = 0x607666
UNIT_TABLE = 0x607C58
OBSERVE = 0x41F704
REDRAW = 0x467C89
ORDER_SCOPE = (
    "Prepare the paid human city production and reach the first input poll. "
    "Select the head or tail of the new city stack explicitly, then execute "
    "Wait, Skip or northwest movement through the existing prepared input probe. "
    "Set each Skip actor's prior direction to the corresponding checked engine value."
)


def city_stack_focus(original, newborn, units):
    anchors = ((51, 9), (53, 9), (54, 8), (61, 5), (0, 0))
    coordinates = (0x5F46FC, 0x5F46F8)
    previous = [read(original, address, '<h') for address in coordinates]
    active = read(original, 0x60764E, '<h')
    assert active == -1
    stacked = 26
    links = [(newborn, -1, stacked), (stacked, newborn, -1)]
    for slot, before, after in links:
        raw = units[slot]
        assert tuple(int.from_bytes(raw[i:i + 2], 'little', signed=True) for i in (26, 28)) == (before, after)

    def sweep():
        selections = []
        for anchor in anchors:
            for address, value in zip(coordinates, anchor, strict=True):
                original.write(address, value, '<h')
            selections.append(original.call(0x56CB3D, 0xffffffff, 1))
        return selections

    try:
        normal = sweep()
        for slot, before, after in links:
            original.write(UNIT_TABLE + slot * 40 + 26, after, '<h')
            original.write(UNIT_TABLE + slot * 40 + 28, before, '<h')
        reversed_head = sweep()
    finally:
        for slot, before, after in links:
            original.write(UNIT_TABLE + slot * 40 + 26, before, '<h')
            original.write(UNIT_TABLE + slot * 40 + 28, after, '<h')
        for address, value in zip(coordinates, previous, strict=True):
            original.write(address, value, '<h')
    assert normal == reversed_head == [18, newborn, newborn, 9, 6]
    return {'anchors': anchors, 'choices': normal, 'reversed_head_choices': reversed_head,
            'original_stack': [newborn, stacked]}


def city_pass(original, viewer, city_region, worker, purchase, stored, command):
    cpu = original.cpu
    city = city_region[0] + CITY_SLOT * 92
    area = read(original, 0x6705A4, '<h')
    memory = read(original, 0x5DEAD0 + 4 * viewer, '<I')

    def capture():
        tiles = bytes(cpu.mem_read(TILES, area * 6))
        known = tuple(bool(mask & (1 << viewer)) for mask in tiles[4::6])
        remembered = bytes(cpu.mem_read(memory, area))
        units = {
            slot: bytes(cpu.mem_read(UNIT_TABLE + slot * 40, 40))
            for slot in range(read(original, UNIT_COUNT, '<h'))
            if read(original, UNIT_TABLE + slot * 40 + 30, '<I')
        }
        return tiles, known, remembered, units

    assert read(original, city + 59, '<b') == purchase['item']
    original.write(city + 24, worker['specialists'], '<I')
    original.write(city + 50, worker['worked'], '<I')
    original.write(city + 30, stored, '<H')
    original.write(CIV + STRIDE * viewer + 10, purchase['treasury_after'], '<i')
    options = read(original, 0x60763A, '<I')
    original.write(0x60763A, options & ~0x2000, '<I')
    tiles_before, known_before, memory_before, units_before = capture()
    calls = []
    observations = []
    shield_writes = []

    def trace(cpu, address, _size, _data):
        if address in (REDRAW, OBSERVE):
            calls.append(f'{address:08x}')
        if address == OBSERVE:
            stack = cpu.reg_read(UC_X86_REG_ESP)
            observations.append([read(original, stack + offset, '<I') for offset in (4, 8)])
        if address == REDRAW:
            stack = cpu.reg_read(UC_X86_REG_ESP)
            cpu.reg_write(UC_X86_REG_EAX, 0)
            cpu.reg_write(UC_X86_REG_ESP, stack + 4)
            cpu.reg_write(UC_X86_REG_EIP, read(original, stack, '<I'))

    def write_shields(cpu, _access, _address, size, value, _data):
        assert size == 2
        shield_writes.append([f'{cpu.reg_read(UC_X86_REG_EIP):08x}',
                              read(original, city + 30, '<H'), value])

    shield_hook = cpu.hook_add(UC_HOOK_MEM_WRITE, write_shields, begin=city + 30, end=city + 31)
    hook = cpu.hook_add(UC_HOOK_CODE, trace)
    try:
        original.call(TURN_START, viewer)
    finally:
        cpu.hook_del(hook)
        cpu.hook_del(shield_hook)
    assert cpu.reg_read(UC_X86_REG_EIP) == 0x100000
    tiles_after, known_after, memory_after, units_after = capture()
    assert known_before == known_after and memory_before == memory_after
    assert calls == ['00467c89', '0041f704', '00467c89']
    slot, = units_after.keys() - units_before.keys()
    assert observations == [[slot, 1]]
    unit = units_after[slot]
    position = [int.from_bytes(unit[i:i + 2], 'little') for i in (0, 2)]
    shields_after = read(original, city + 30, '<H')
    assert (slot, unit[8], unit[9], position, unit[10], shields_after) == (22, 3, 4, [53, 9], 0, 0)
    assert shield_writes == [
        ['004c62e0', stored, stored + 2],
        ['004c6607', stored + 2, stored - 18],
        ['004c748e', stored - 18, 0],
    ]

    focus = city_stack_focus(original, slot, units_after)
    frame(original)
    original.write(FRAME - 0x6AC, viewer, '<I')
    count = read(original, UNIT_COUNT, '<h')
    handoff = measure_handoff(original, viewer,
                              [(CIV, STRIDE * 8), (UNIT_TABLE, count * 40), city_region],
                              observation=True, entry=0x474BC8, extra_calls=(0x570169,))
    _, known_poll, memory_poll, _ = capture()
    assert known_before == known_poll and memory_before == memory_poll
    assert handoff['active_slot'] == 18 and sum(known_poll) == 59
    if command is not None:
        actor, key, prior_direction = command
        assert actor in focus['original_stack']
        original.write(0x60764E, actor, '<h')
        if prior_direction is not None:
            original.write(UNIT_TABLE + actor * 40 + 13, prior_direction, '<B')
        regions = [(CIV, STRIDE * 8), (UNIT_TABLE, count * 40), city_region]
        return first_order(original, viewer, regions, key, return_address=0x100040)

    return {
        'turn': read(original, 0x607648, '<h'), 'city': CITY_SLOT,
        'applied_worker': worker, 'shields_before': stored, 'shield_writes': shield_writes,
        'shields_after': shields_after, 'unit': {
            'slot': slot, 'type': unit[8], 'owner': unit[9], 'position': position, 'spent': unit[10],
        },
        'city_pass_calls': calls, 'observation_arguments': observations,
        'tile_byte_changes': [(i, a, b) for i, (a, b) in enumerate(zip(tiles_before, tiles_after))
                              if a != b],
        'explored_before': sum(known_before), 'explored_after': sum(known_after),
        'explored_at_poll': sum(known_poll), 'remembered_bytes_changed': 0,
        'first_poll': handoff['observation_to_input']['first_poll'], 'focus': focus,
    }


def stack_orders(prepared_birth, paid):
    cases = {}
    for actor, other in [(22, 26), (26, 22)]:
        wait = prepared_birth(paid, (actor, ord('w'), None))
        assert wait['before']['active_slot'] == actor and wait['after']['active_slot'] == other
        assert wait['observation_arguments'] == [[other, 1]]
        assert wait['unit_position_after'] == wait['unit_position_before'] == (53, 9)
        assert wait['tile_byte_changes'] == wait['cache_byte_changes'] == []
        change, = wait['unit_byte_changes']
        assert change['slot'] == actor and change['byte'] == 7
        assert change['after'] == change['before'] | 64

        move = prepared_birth(paid, (actor, NORTHWEST, None))
        assert move['before']['active_slot'] == actor
        assert move['move_arguments'] == [[actor, 6, 3]]
        assert move['observation_arguments'] == [[actor, 1]]
        assert move['unit_position_before'] == (53, 9) and move['unit_position_after'] == (52, 8)
        assert move['tile_byte_changes'] == [(2077, 0, 1), (2081, 240, 64)]
        assert move['cache_byte_changes'] == [[4, 346, 0, 1]]
        assert {'slot': actor, 'byte': 10, 'before': 0, 'after': 6} in move['unit_byte_changes']
        for slot, offset in [(22, 28), (26, 26)]:
            assert all({'slot': slot, 'byte': offset + i, 'before': value, 'after': 255}
                       in move['unit_byte_changes']
                       for i, value in enumerate((26 if slot == 22 else 22, 0)))
        direction = 0 if actor == 22 else 5
        skip = prepared_birth(paid, (actor, ord(' '), direction))
        assert skip['before']['active_slot'] == actor
        assert skip['unit_position_before'] == skip['unit_position_after'] == (53, 9)
        assert skip['observation_arguments'] == [[actor, 1]]
        assert skip['tile_byte_changes'] == skip['cache_byte_changes'] == []
        assert skip['unit_byte_changes'] == [
            {'slot': actor, 'byte': 10, 'before': 0, 'after': 3},
            {'slot': actor, 'byte': 13, 'before': direction, 'after': 255},
        ]
        assert wait['explored_tiles'] == move['explored_tiles'] == skip['explored_tiles'] == 59
        cases[str(actor)] = {'wait': wait, 'move_northwest': move, 'skip': skip}
    return cases


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--orders', action='store_true', help='Record Wait, movement and Skip after production')
    args = parser.parse_args()
    evidence = Path(__file__).resolve().parent.parent / 'docs/evidence'
    purchase = json.loads((evidence / 'import-saved-purchase.json').read_text())
    workers = json.loads((evidence / 'import-saved-citizens.json').read_text())
    round_close = json.loads((evidence / 'import-round-close.json').read_text())
    keys = ('binary_sha256', 'source', 'rules')
    assert all(purchase[key] == workers[key] == round_close[key] for key in keys)
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(binary) == purchase['binary_sha256']
    for key in ('source', 'rules'):
        item = purchase[key]
        assert digest((home / item['path']).read_bytes()) == item['sha256']
    rules = parse_rules(home / purchase['rules']['path'])
    def prepared_birth(stored, command=None):
        annual, birth = prepare_round(
            home, binary, purchase['source']['path'], rules,
            lambda original, viewer, city_region: city_pass(
                original, viewer, city_region, workers['cases'][0]['after'],
                purchase['pinned'], stored, command,
            ),
        )
        assert annual['annual_update'] == round_close['annual_update']
        return birth

    paid = purchase['pinned']['shields_after']
    report = {'schema': 1, **{key: purchase[key] for key in keys}}
    if args.orders:
        report.update(cases=stack_orders(prepared_birth, paid), scope=ORDER_SCOPE)
    else:
        report.update(
            human_city_birth=prepared_birth(paid),
            surplus_control=prepared_birth(paid + 5), scope=__doc__.strip(),
        )
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
