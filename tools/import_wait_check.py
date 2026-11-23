#!/usr/bin/env python3
"""Trace Wait and deferred unit records in the checked Original resume.

Run the native Wait action in the prepared input context. Repeat the
unit-table writer boundary and reader with virtual file buffers, then
compare the first selected actor with and without the deferred bit.
The mixed controls use Skip and the native selector after a move.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from import_first_order_check import NORTHWEST, execute
from import_input_handoff_check import measure_handoff
from import_phase_check import FRAME, frame, read, run
from import_round_handoff_check import trace
from import_unit_setup_check import prepare

UNITS = 0x607C58
OWN = (6, 9, 18, 26)
ANCHOR = (61, 5)


def install_anchor(original):
    for address, value in zip((0x5F46FC, 0x5F46F8), ANCHOR, strict=True):
        original.write(address, value, '<h')


def deferred(original, slot):
    return bool(read(original, UNITS + 40 * slot + 7, '<B') & 64)


def wait_series(original, player, regions, count):
    result = []
    for index in range(count):
        order = execute(original, player, regions, ord('w'), return_address=0x100040 + 16 * index)
        assert order['tile_byte_changes'] == order['cache_byte_changes'] == []
        assert order['explored_tiles'] == 59
        result.append({
            'actor': order['before']['active_slot'], 'selected': order['after']['active_slot'],
            'deferred': [slot for slot in OWN if deferred(original, slot)],
            'spent': [read(original, UNITS + 40 * slot + 10, '<B') for slot in OWN],
            'foreign_deferred': deferred(original, 1),
        })
    return result


def writer_units(original):
    frame(original)
    original.write(FRAME - 0x118, 1, '<I')
    run(original, 0x460029, {0x460054})
    src, width, count, handle = struct.unpack('<4I', original.cpu.mem_read(original.cpu.reg_read(original.sp), 16))
    assert src == UNITS and count == handle == 1
    return bytes(original.cpu.mem_read(src, width))


def read_tables(original, units, cities):
    frame(original)
    original.write(FRAME + 12, 1, '<I')
    original.write(0x5D1500, 1, '<I')
    original.write(0x5D1504, 1, '<I')
    payloads = iter((units, cities))
    reads = []

    def supply(cpu, ip, _size, _data):
        if ip != 0x5B3810:
            return
        sp = cpu.reg_read(UC_X86_REG_ESP)
        ret, destination, width, count, handle = struct.unpack('<5I', cpu.mem_read(sp, 20))
        payload = next(payloads)
        assert width == len(payload) and count == handle == 1
        cpu.mem_write(destination, bytes(payload))
        cpu.reg_write(UC_X86_REG_EAX, 1)
        cpu.reg_write(UC_X86_REG_ESP, sp + 4)
        cpu.reg_write(UC_X86_REG_EIP, ret)
        reads.append({'site': f'{ret - 5:08x}', 'bytes': width})

    hook = original.cpu.hook_add(UC_HOOK_CODE, supply)
    try:
        run(original, 0x45EFB6, {0x45F0E9}, calls=(0x5B3810,))
    finally:
        original.cpu.hook_del(hook)
    assert len(reads) == 2
    return reads


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True, type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    prior = json.loads((root / 'docs/evidence/import-saved-viewport.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    path = prior['source']['path']
    assert hashlib.sha256(binary).hexdigest() == prior['binary_sha256']
    for key in ('source', 'rules'):
        item = prior[key]
        assert hashlib.sha256((home / item['path']).read_bytes()).hexdigest() == item['sha256']

    def fresh(*, excluded=(), deferred_slots=()):
        original, player, _, regions, _ = prepare(home, binary, path, exhaust=False)
        install_anchor(original)
        for slot in excluded:
            original.write(UNITS + 40 * slot + 10, 3, '<B')
        for slot in deferred_slots:
            at = UNITS + 40 * slot + 7
            original.write(at, read(original, at, '<B') | 64, '<B')
        measure_handoff(original, player, regions, observation=True)
        return original, player, regions

    sequence = wait_series(*fresh(deferred_slots=(1,)), 6)
    assert [step['selected'] for step in sequence] == [6, 26, 18, 26, 9, 6]
    assert [step['deferred'] for step in sequence] == [[9], [6, 9], [6, 9, 26], [18], [18, 26], [9, 18, 26]]
    assert [step['foreign_deferred'] for step in sequence] == [True, True, True, False, False, False]
    assert all(step['spent'] == [0, 0, 0, 0] for step in sequence)
    solitary = wait_series(*fresh(excluded=(6, 18, 26)), 2)
    assert all(step['selected'] == 9 and not step['deferred'] and step['spent'] == [3, 0, 3, 3]
               for step in solitary)

    original, player, regions = fresh()
    wait_series(original, player, regions, 1)
    cities = bytes(original.cpu.mem_read(*regions[2]))
    units = writer_units(original)
    assert units[40 * 9 + 7] & 64
    assert units[40 * 9 + 10] == 0
    reload = {}
    for label in ('retained', 'cleared'):
        restored, seat, _, regions, _ = prepare(home, binary, path, exhaust=False)
        install_anchor(restored)
        payload = bytearray(units)
        if label == 'cleared':
            payload[40 * 9 + 7] &= ~64
        reads = read_tables(restored, payload, cities)
        assert deferred(restored, 9) == (label == 'retained')
        poll = measure_handoff(restored, seat, regions, observation=True)
        assert poll['explored_tiles_after'] == 59 and poll['human_memory_bytes_changed'] == 0
        reload[label] = {'selected': poll['active_slot'], 'reads': reads}
    assert [step['selected'] for step in reload.values()] == [6, 9]

    original, player, regions = fresh(deferred_slots=(9,))
    skipped = trace(original, player, regions, clear_preference_bit=False,
                    skip_selected=True, clear_deferred=True)
    skip_order = [entry['active_slot'] for entry in skipped['dispatched']]
    assert skip_order == [6, 26, 18, 9]
    assert not any(deferred(original, slot) for slot in OWN)

    original, player, regions = fresh()
    first = wait_series(original, player, regions, 1)[0]
    move = execute(original, player, regions, NORTHWEST, return_address=0x100050)
    actor = move['before']['active_slot']
    assert first['selected'] == actor == 6
    assert move['unit_position_after'] == (62, 4) and move['explored_tiles'] == 60
    selected = original.call(0x56CB3D, actor, 1)
    assert selected == 26 and deferred(original, 9)
    move_focus = {'actor': actor, 'selected': selected, 'deferred': [9],
                  'explored_tiles': move['explored_tiles'], 'position': move['unit_position_after']}

    report = {'schema': 1, **{key: prior[key] for key in ('binary_sha256', 'source', 'rules')},
              'anchor': ANCHOR, 'waits': sequence, 'one_ready': solitary,
              'writer_unit_bytes': len(units), 'reload': reload,
              'skip_with_slot_9_deferred': skip_order, 'move_after_wait': move_focus, 'scope': __doc__.strip()}
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
