#!/usr/bin/env python3
"""Measure saved view through the prepared native city worker editor.

Supply the current-city accessor and the selected radius tile. The native
packed citizen edit and city totals run before the first redraw helper.
"""
import argparse
import json
import os
from pathlib import Path

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from import_city_science_entry_check import CALLS
from import_event_tail_check import digest
from import_loader_exit_check import rules_refresh
from import_phase_check import FRAME, frame, read, run
from import_unit_setup_check import prepare
from totctl import parse_rules

ENTRY = 0x4DDDC6
STOP = 0x4DD04B
CITY = 5


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    prior = json.loads((root / 'docs/evidence/import-saved-production.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(binary) == prior['binary_sha256']
    for key in ('source', 'rules'):
        item = prior[key]
        assert digest((home / item['path']).read_bytes()) == item['sha256']
    original, player, _, _regions, _ = prepare(home, binary, prior['source']['path'], exhaust=False)
    frame(original)
    original.write(FRAME - 0x1990, 1, '<I')
    original.write(FRAME - 0xE9C, 0, '<I')
    rules_refresh(original, parse_rules(home / prior['rules']['path']))
    width, height = (read(original, at, '<h') for at in (0x6705A0, 0x6705A2))
    area = width * height // 2
    original.write(0x6705A4, area, '<h')
    memory = read(original, 0x5DEAD0 + 4 * player, '<I')

    def knowledge():
        return (bytes(original.cpu.mem_read(0x700000, 6 * area)),
                bytes(original.cpu.mem_read(memory, area)))

    record = 0x5FD150 + 92 * CITY

    def city_state():
        return {'worked': read(original, record + 50, '<I'),
                'specialists': read(original, record + 24, '<I')}

    before = knowledge()
    original.write(0x657848 + 0x1A00, CITY, '<I')
    for offset in (0x1A04, 0x1A08, 0x1A0C):
        original.write(0x657848 + offset, 0, '<I')

    def accessor(cpu, ip, _size, _data):
        if ip == 0x411C50:
            stack = cpu.reg_read(UC_X86_REG_ESP)
            cpu.reg_write(UC_X86_REG_EAX, CITY)
            cpu.reg_write(UC_X86_REG_ESP, stack + 4)
            cpu.reg_write(UC_X86_REG_EIP, read(original, stack, '<I'))

    hook = original.cpu.hook_add(UC_HOOK_CODE, accessor)
    cases = []
    try:
        for tile in (4, 4, 20):
            frame(original)
            original.write(FRAME - 24, tile, '<i')
            state = city_state()
            original.write(0x653884, state['worked'] >> 26, '<I')
            run(original, ENTRY, {STOP}, calls=CALLS + (0x4DCFE9, 0x4C341B), instruction_limit=400000)
            assert before == knowledge()
            cases.append({'city': CITY, 'tile': tile, 'before': state, 'after': city_state(),
                          'explored_tiles': sum(bool(mask & (1 << player)) for mask in before[0][4::6]),
                          'tile_bytes_changed': 0, 'remembered_bytes_changed': 0})
    finally:
        original.cpu.hook_del(hook)
    assert [item['after'] for item in cases] == [
        {'worked': 68157568, 'specialists': 1},
        {'worked': 1048720, 'specialists': 0},
        {'worked': 1048720, 'specialists': 0},
    ]
    report = {'schema': 1, **{key: prior[key] for key in ('source', 'rules', 'binary_sha256')},
              'entry': f'{ENTRY:08x}', 'stop': f'{STOP:08x}', 'cases': cases,
              'scope': __doc__.strip()}
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
