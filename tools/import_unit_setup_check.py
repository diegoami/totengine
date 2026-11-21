#!/usr/bin/env python3
"""Trace the resumed human turn through native active-unit selection.

The human map mode is prepared before entry. The traces stop at the selected
unit or no-unit branch before presentation, input, and any research prompt.
No callees are replaced. Only numeric observations and input hashes are saved.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path

from import_phase_check import CIV, FRAME, STRIDE, frame, loader, read, run
from yields_check import OriginalCity

ENTRY = 0x474B62
UNIT_RECORDS = 0x607C58
ACTIVE_SLOT = 0x60764E
SELECTED = 0x47297F
NO_UNIT = 0x472A8E
CALLS = (
    0x473BBE, 0x47355B, 0x472ADA, 0x4728C3, 0x56CB3D,
    0x56C960, 0x4030F0, 0x568F99, 0x568DA2, 0x568D14, 0x568D3E,
    0x566DAD, 0x566B9E, 0x566D26, 0x49C820, 0x444A61,
)


def witness(home, binary, path, *, exhaust, only_slot=None):
    original = OriginalCity(binary)
    data, walk, rules, _ = original.load_saved(home, path)
    player, active = data[680], data[681]
    assert player == active == 4 and data[678] == 0
    # Prepare native type movement and HP fields from the installed Rules,
    # as in the movement allowance probe. Other mapped fields come from
    # OriginalCity.load_saved.
    road_row = next(row for row, comment in zip(rules['COSMIC']['rows'],
                                               rules['COSMIC']['comments'], strict=True)
                    if comment.startswith('Road movement multiplier'))
    road = int(road_row[0])
    assert road == 3
    for index, row in enumerate(rules['UNITS']['rows'][:80]):
        if len(row) > 8:
            original.write(0x5F470A + index * 40, road * int(row[3].rstrip('.')), '<B')
            original.write(0x5F470E + index * 40, 10 * int(row[7].rstrip('hH')), '<B')
    original.write(0x5F5EB8, road, '<B')

    controlled_slots = []
    for slot in range(walk['units']['count']):
        record = UNIT_RECORDS + slot * 40
        if read(original, record + 9, '<B') != player or not read(original, record + 30, '<I'):
            continue
        if exhaust and slot != only_slot:
            original.write(record + 10, 255, '<B')
            controlled_slots.append(slot)

    loader(original, player, active, (0, 0, 0))
    frame(original)
    original.write(FRAME - 0x6AC, active, '<I')
    original.write(0x5D4FA0, 0, '<I')
    original.write(0x6762F8, 0, '<I')
    original.write(0x67A6E8, 1, '<I')  # already in unit-selection display mode
    original.write(0x5D50D4, 0, '<I')  # clean selection state
    raw_slot = read(original, ACTIVE_SLOT, '<h')
    regions = [(CIV, STRIDE * 8), (UNIT_RECORDS, walk['units']['count'] * 40),
               (0x5FD150, walk['cities']['count'] * 92)]
    before = [bytes(original.cpu.mem_read(addr, size)) for addr, size in regions]
    stop = run(original, ENTRY, {SELECTED, NO_UNIT}, calls=CALLS)
    assert before == [bytes(original.cpu.mem_read(addr, size)) for addr, size in regions]
    return {
        'stop': f'{stop:08x}',
        'saved_active_slot': raw_slot,
        'active_slot_at_boundary': read(original, ACTIVE_SLOT, '<h'),
        'controlled_spent_slots': controlled_slots,
        'player_research_target': read(original, CIV + STRIDE * player + 18, '<h'),
        'player_research_progress': read(original, CIV + STRIDE * player + 16, '<h'),
        'civ_unit_city_records_unchanged': True,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    phase = json.loads((root / 'docs/evidence/import-phase.json').read_text())
    dependencies = json.loads((root / 'docs/evidence/import-dependencies.json').read_text())
    rules = next(row for row in dependencies['inputs'] if row['role'] == 'rules')
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    path = phase['source']['path']
    assert hashlib.sha256(binary).hexdigest() == phase['binary_sha256']
    assert hashlib.sha256((home / path).read_bytes()).hexdigest() == phase['source']['sha256']
    assert hashlib.sha256((home / rules['path']).read_bytes()).hexdigest() == rules['sha256']
    cases = {
        'pinned': witness(home, binary, path, exhaust=False),
        'all_own_units_exhausted': witness(home, binary, path, exhaust=True),
        'only_slot_9_ready': witness(home, binary, path, exhaust=True, only_slot=9),
    }
    assert [(row['stop'], row['active_slot_at_boundary']) for row in cases.values()] == [
        ('0047297f', 6), ('00472a8e', -1), ('0047297f', 9),
    ]
    assert all(row['saved_active_slot'] == 0 and row['player_research_target'] == -1
               and row['player_research_progress'] == 0 for row in cases.values())
    report = {
        'schema': 1,
        'binary_sha256': phase['binary_sha256'],
        'source': phase['source'],
        'rules': {'path': rules['path'], 'sha256': rules['sha256']},
        'prepared_display_mode': 1,
        'entry': f'{ENTRY:08x}',
        'allowed_calls': [f'{addr:08x}' for addr in CALLS],
        'replaced_callees': [],
        'cases': cases,
        'scope': __doc__.strip(),
    }
    args.out.write_text(json.dumps(report, indent=2) + '\n')
    print('Native unit-selection boundaries: pinned 6, exhausted none, sole ready 9')


if __name__ == '__main__':
    main()
