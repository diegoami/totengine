#!/usr/bin/env python3
"""Compare human map knowledge before civ 5's city work and at production selection.

Starting with the same saved turn as the round-handoff witness, run four
native Skip orders. Continue from the first opponent city-phase entry to
the call into the production selector. The probe supplies host messages
and substitutes presentation. From the selector boundary, three controlled
native moves use the first spatial engine orders as inputs. The selector
and native opponent policy are outside the trace.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from import_input_handoff_check import IDLE_FILTER_CALL, witness
from import_phase_check import read
from import_round_handoff_check import CITY_PHASE, trace

PRODUCTION_CHOICE = 0x434D81
TILES = 0x700000
UNIT_COUNT = 0x607666
UNITS = 0x607C58
PRESENTATION = {0x467C89, 0x467D19}


def measure(original, viewer, city):
    cpu = original.cpu
    cpu.mem_map(0, 4096)  # native exception-registration chain
    area = read(original, 0x6705A4, '<h')
    width = read(original, 0x6705A0, '<h')
    cache = read(original, 0x5DEAD0 + 4 * viewer, '<I')

    def view():
        raw = cpu.mem_read(TILES, area * 6)
        known = [bool(mask & (1 << viewer)) for mask in raw[4::6]]
        memory = bytes(cpu.mem_read(cache, area))
        records = {}
        for slot in range(read(original, UNIT_COUNT, '<h')):
            address = UNITS + 40 * slot
            identity = read(original, address + 30, '<I')
            if identity:
                x, y = (read(original, address + off, '<h') for off in (0, 2))
                owner = read(original, address + 9, '<B')
                visible = read(original, address + 11, '<B') & (1 << viewer)
                records[slot] = (identity, x, y, owner, bool(visible))
        return known, memory, records

    before = view()
    substitutes = []

    def boundary(emu, at, _size, _data):
        if at == PRODUCTION_CHOICE:
            emu.emu_stop()
        elif at in PRESENTATION:
            sp = emu.reg_read(UC_X86_REG_ESP)
            substitutes.append(f'{at:08x}')
            emu.reg_write(UC_X86_REG_EAX, 0)
            emu.reg_write(UC_X86_REG_ESP, sp + 4)
            emu.reg_write(UC_X86_REG_EIP, read(original, sp, '<I'))

    hook = cpu.hook_add(UC_HOOK_CODE, boundary)
    try:
        cpu.emu_start(CITY_PHASE, 0, count=4_000_000)
    finally:
        cpu.hook_del(hook)
    assert cpu.reg_read(UC_X86_REG_EIP) == PRODUCTION_CHOICE
    assert read(original, cpu.reg_read(UC_X86_REG_ESP) + 4) == city
    after = view()
    created = [
        {'slot': slot, 'position': [x, y], 'owner': owner,
         'tile_known': after[0][y * (width // 2) + x // 2], 'viewer_mask': visible,
         'visibility_byte': read(original, UNITS + 40 * slot + 11, '<B'),
         'orders': read(original, UNITS + 40 * slot + 17, '<B')}
        for slot, (identity, x, y, owner, visible) in after[2].items()
        if slot not in before[2] or before[2][slot][0] != identity
    ]
    assert before[:2] == after[:2]
    assert all(after[2].get(slot) == unit for slot, unit in before[2].items())
    assert len(created) == 1 and not created[0]['tile_known']
    assert (created[0]['visibility_byte'], created[0]['orders']) == (0, 255)
    return {'city_slot': city, 'stop': f'{PRODUCTION_CHOICE:08x}',
            'viewer_explored_tiles': sum(after[0]), 'viewer_memory_changed': False,
            'previous_unit_knowledge_changed': False,
            'created_units': created, 'presentation_substitutes': substitutes}



def move_into_unseen_city(original, viewer):
    return move_opponent(original, viewer, (2, 1, 3), (2, 19, 34), (41, 37), (43, 37),
                         ((19, -1), (34, 2), (-1, 19)))


def move_opponent(original, viewer, arguments, slots, origin, destination, links):
    """Supply an engine spatial order to native movement."""
    cpu = original.cpu
    area = read(original, 0x6705A4, '<h')
    cache = read(original, 0x5DEAD0 + 4 * viewer, '<I')

    def knowledge():
        raw = cpu.mem_read(TILES, area * 6)
        return [bool(b & (1 << viewer)) for b in raw[4::6]], bytes(cpu.mem_read(cache, area))

    def unit(slot):
        address = UNITS + 40 * slot
        return {
            'slot': slot,
            'position': [read(original, address + off, '<h') for off in (0, 2)],
            'owner': read(original, address + 9, '<B'),
            'spent': read(original, address + 10, '<B'),
            'previous': read(original, address + 26, '<h'),
            'next': read(original, address + 28, '<h'),
        }

    before = [unit(slot) for slot in slots]
    saved = knowledge()
    substitutes = []

    def boundary(emu, at, _size, _data):
        if at not in (IDLE_FILTER_CALL, 0x467D19):
            return
        substitutes.append(f'{at:08x}')
        sp = emu.reg_read(UC_X86_REG_ESP)
        emu.reg_write(UC_X86_REG_EAX, 0)
        if at == IDLE_FILTER_CALL:
            assert [read(original, sp + off) for off in (4, 8, 12, 16)] == [0, 0x3BD, 0x3BD, 1]
            emu.reg_write(UC_X86_REG_ESP, sp + 20)
            emu.reg_write(UC_X86_REG_EIP, at + 6)
        else:
            emu.reg_write(UC_X86_REG_ESP, sp + 4)
            emu.reg_write(UC_X86_REG_EIP, read(original, sp, '<I'))

    hook = cpu.hook_add(UC_HOOK_CODE, boundary)
    try:
        result = original.call(0x54C69B, *arguments)
    finally:
        cpu.hook_del(hook)
    after = [unit(slot) for slot in slots]
    assert saved == knowledge()
    assert result == 1 and before[0]['position'] == list(origin) and after[0]['position'] == list(destination)
    assert tuple((record['previous'], record['next']) for record in after) == links
    return {
        'entry': '0054c69b', 'arguments': list(arguments), 'return': result,
        'explored_tiles': sum(saved[0]), 'viewer_memory_changed': False,
        'before': before, 'after': after, 'substitutes': substitutes,
    }

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True, type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    evidence = json.loads((root / 'docs/evidence/import-round-handoff.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert hashlib.sha256(binary).hexdigest() == evidence['binary_sha256']
    for role in ('source', 'rules'):
        source = evidence[role]
        assert hashlib.sha256((home / source['path']).read_bytes()).hexdigest() == source['sha256']
    observed = {}

    def entry(original, player, city):
        observed.update(measure(original, player, city))
        observed['prepared_opponent_move'] = move_into_unseen_city(original, player)
        observed['prepared_opponent_empty_move'] = move_opponent(
            original, player, (10, 7, 3), (10,), (41, 39), (41, 37), ((-1, -1),)
        )
        observed['prepared_opponent_departure'] = move_opponent(
            original, player, (19, 6, 3), (19, 34, 2), (43, 37), (42, 36),
            ((-1, -1), (-1, 2), (34, -1)),
        )

    def continue_round(original, player, regions):
        trace(original, player, regions, clear_preference_bit=False, skip_selected=True,
              at_city_entry=entry)

    witness(home, binary, evidence['source']['path'], exhaust=False,
            observation=True, after_idle=continue_round)
    assert observed['city_slot'] == 6 and observed['viewer_explored_tiles'] == 59
    report = {'schema': 1, **{key: evidence[key] for key in ('binary_sha256', 'source', 'rules')},
              'observed': observed, 'scope': __doc__.strip()}
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
