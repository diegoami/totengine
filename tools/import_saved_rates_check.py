#!/usr/bin/env python3
"""Measure the native rate commit against saved map knowledge.

Supply the already chosen legal rates to the native dialog commit
fragment. Native city refresh runs through its normal calls. Stop
before window cleanup and keep display input outside this probe.
"""
import argparse
import json
import os
from pathlib import Path

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP

from import_city_science_entry_check import CALLS
from import_event_tail_check import digest
from import_loader_exit_check import rules_refresh
from import_phase_check import CIV, FRAME, STRIDE, frame, read, run
from import_unit_setup_check import prepare
from totctl import parse_rules

ENTRY = 0x40B4FA
STOP = 0x40B6E3
CITY_TOTALS = 0x4C5268
DIALOG = 0x104000


def case(home, binary, path, rules, science, tax):
    original, player, _, _regions, _ = prepare(home, binary, path, exhaust=False)
    frame(original)
    original.write(FRAME - 0x1990, 1, '<I')
    original.write(FRAME - 0xE9C, 0, '<I')
    rules_refresh(original, rules)
    width, height = (read(original, at, '<h') for at in (0x6705A0, 0x6705A2))
    area = width * height // 2
    old = read(original, 0x5DEAD0 + 4 * player, '<I')
    original.write(0x6705A4, area, '<h')

    def knowledge():
        return (bytes(original.cpu.mem_read(0x700000, 6 * area)),
                bytes(original.cpu.mem_read(old, area)))

    original.write(FRAME + 8, player, '<I')
    original.write(FRAME - 0x460, 0, '<I')
    original.write(FRAME - 0x4A0, DIALOG, '<I')
    original.write(DIALOG + 0x744, tax, '<I')
    original.write(DIALOG + 0x74C, science, '<I')
    seat = CIV + STRIDE * player
    before = knowledge()
    rates_before = [read(original, seat + offset, '<B') for offset in (27, 28)]
    cities = []

    def observe(cpu, ip, _size, _data):
        if ip == CITY_TOTALS:
            cities.append(read(original, cpu.reg_read(UC_X86_REG_ESP) + 4, '<I'))

    hook = original.cpu.hook_add(UC_HOOK_CODE, observe)
    try:
        run(original, ENTRY, {STOP}, calls=CALLS, instruction_limit=400000)
    finally:
        original.cpu.hook_del(hook)
    assert before == knowledge()
    assert [read(original, seat + offset, '<B') for offset in (27, 28)] == [science, tax]
    assert cities == ([] if rates_before == [science, tax] else [5])
    return {'before': rates_before, 'science': science, 'tax': tax, 'refreshed_cities': cities,
            'explored_tiles': sum(bool(mask & (1 << player)) for mask in before[0][4::6]),
            'tile_bytes_changed': 0, 'remembered_bytes_changed': 0}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    prior = json.loads((root / 'docs/evidence/import-pending-choice.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(binary) == prior['binary_sha256']
    for key in ('source', 'rules'):
        item = prior[key]
        assert digest((home / item['path']).read_bytes()) == item['sha256']
    path = prior['source']['path']
    rules = parse_rules(home / prior['rules']['path'])
    cases = [case(home, binary, path, rules, science, tax) for science, tax in [(6, 4), (4, 4), (4, 6)]]
    assert all(item['before'] == [6, 4] and item['explored_tiles'] == 59 for item in cases)
    report = {'schema': 1, **{key: prior[key] for key in ('source', 'rules', 'binary_sha256')},
              'entry': f'{ENTRY:08x}', 'stop': f'{STOP:08x}', 'cases': cases, 'scope': __doc__.strip()}
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
