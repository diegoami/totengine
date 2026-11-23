#!/usr/bin/env python3
"""Measure map knowledge through the prepared native production commit.

Capture the city switch budget through the native helper, supply a legal selected
item, and execute the native commit through its build and shield writes. Stop
before follow-up presentation.
"""
import argparse
import json
import os
from pathlib import Path

from import_event_tail_check import digest
from import_loader_exit_check import rules_refresh
from import_phase_check import FRAME, frame, read, run
from import_unit_setup_check import prepare
from totctl import parse_rules

ENTRY = 0x434E58
STOP = 0x434EE4
CITY = 5


def case(home, binary, path, rules, item):
    original, player, _, _regions, _ = prepare(home, binary, path, exhaust=False)
    frame(original)
    original.write(FRAME - 0x1990, 1, '<I')
    original.write(FRAME - 0xE9C, 0, '<I')
    rules_refresh(original, rules)
    width, height = (read(original, at, '<h') for at in (0x6705A0, 0x6705A2))
    area = width * height // 2
    memory = read(original, 0x5DEAD0 + 4 * player, '<I')
    original.write(0x6705A4, area, '<h')

    def knowledge():
        return (bytes(original.cpu.mem_read(0x700000, 6 * area)),
                bytes(original.cpu.mem_read(memory, area)))

    record = 0x5FD150 + 92 * CITY
    before = knowledge()
    old_item = read(original, record + 59, '<b')
    old_shields = read(original, record + 30, '<H')
    original.call(0x4C0EE6, CITY)
    frame(original)
    original.write(FRAME + 8, CITY, '<I')
    original.write(FRAME + 12, item, '<i')
    original.write(FRAME - 0x6B8, player, '<I')
    run(original, ENTRY, {STOP}, calls=(0x4C0F37, 0x4C0EAC))
    assert before == knowledge()
    assert read(original, record + 59, '<b') == item
    return {'city': CITY, 'before_item': old_item, 'before_shields': old_shields,
            'item': item, 'shields': read(original, record + 30, '<H'),
            'explored_tiles': sum(bool(mask & (1 << player)) for mask in before[0][4::6]),
            'tile_bytes_changed': 0, 'remembered_bytes_changed': 0}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    prior = json.loads((root / 'docs/evidence/import-saved-rates.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(binary) == prior['binary_sha256']
    for key in ('source', 'rules'):
        item = prior[key]
        assert digest((home / item['path']).read_bytes()) == item['sha256']
    path = prior['source']['path']
    rules = parse_rules(home / prior['rules']['path'])
    cases = [case(home, binary, path, rules, item) for item in (3, -42)]
    assert all(item['before_item'] == 3 and item['before_shields'] == 12
               and item['explored_tiles'] == 59 for item in cases)
    assert [item['shields'] for item in cases] == [12, 6]
    report = {'schema': 1, **{key: prior[key] for key in ('source', 'rules', 'binary_sha256')},
              'entry': f'{ENTRY:08x}', 'stop': f'{STOP:08x}', 'cases': cases, 'scope': __doc__.strip()}
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
