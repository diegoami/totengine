#!/usr/bin/env python3
"""Trace the saved viewport into the prepared native resume input poll.

The plain single-player post-city prefix reads into the two native
selector coordinates. Repeat the existing prepared resume with those
coordinates and with the zero-anchor control. Then send Space to each
selected unit until the first opponent city phase. The probe supplies
a display context and controlled message queues.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct

from import_event_tail_check import trace_prefix
from import_input_handoff_check import measure_handoff
from import_phase_check import read
from import_round_handoff_check import trace
from import_unit_setup_check import prepare
from totctl import walk_save

VIEWPORT_READS = {
    '00461836': 0x5F46FC,
    '00461860': 0x5F46F8,
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    prior = json.loads((root / 'docs/evidence/import-input-handoff.json').read_text())
    tail_report = json.loads((root / 'docs/evidence/import-event-tail.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    path = prior['source']['path']
    binary = (home / 'civ2.bak').read_bytes()
    data = (home / path).read_bytes()
    assert hashlib.sha256(binary).hexdigest() == prior['binary_sha256'] == tail_report['binary_sha256']
    assert hashlib.sha256(data).hexdigest() == prior['source']['sha256'] == tail_report['source']['sha256']
    rules = prior['rules']
    assert hashlib.sha256((home / rules['path']).read_bytes()).hexdigest() == rules['sha256']
    cities = walk_save(str(home / path))['cities']
    city_end = cities['offset'] + cities['count'] * cities['record_size']
    tail = data[city_end:]
    prefix = trace_prefix(binary, data, tail)
    assert prefix == tail_report['prefix_trace']

    offset = 0
    fields = {}
    anchor = []
    for site, info in prefix['reads'].items():
        if site in VIEWPORT_READS:
            assert info == {'width': 2, 'calls': 1, 'bytes_read': 2}
            value, = struct.unpack_from('<h', tail, offset)
            fields[site] = {'target': f'{VIEWPORT_READS[site]:08x}', 'post_city_offset': offset}
            anchor.append(value)
        offset += info['bytes_read']
    assert anchor == [61, 5]
    assert [item['post_city_offset'] for item in fields.values()] == [63, 65]

    cases = {}
    for label, coordinates, slot in [('zero_anchor', (0, 0), 6), ('saved_anchor', anchor, 9)]:
        original, player, _, regions, _ = prepare(home, binary, path, exhaust=False)
        for address, value in zip(VIEWPORT_READS.values(), coordinates, strict=True):
            original.write(address, value, '<h')
        assert [read(original, address, '<h') for address in VIEWPORT_READS.values()] == list(coordinates)
        handoff = measure_handoff(original, player, regions, observation=True)
        assert handoff['active_slot'] == slot
        assert handoff['explored_tiles_after'] == 59
        assert handoff['human_memory_bytes_changed'] == 0
        assert handoff['research_target'] == -1 and handoff['research_progress'] == 0
        spent = trace(original, player, regions, clear_preference_bit=False, skip_selected=True)
        skip_order = [step['active_slot'] for step in spent['dispatched']]
        assert skip_order == ([9, 6, 26, 18] if label == 'saved_anchor' else [6, 9, 26, 18])
        assert [step['character'] for step in spent['dispatched']] == [32] * 4
        assert spent['boundary'] == 'city-phase-entry' and spent['city_slot'] == 6
        final_anchor = [read(original, address, '<h') for address in VIEWPORT_READS.values()]
        assert final_anchor == [51, 9]
        cases[label] = {'anchor': coordinates, 'handoff': handoff, 'skip_order': skip_order,
                        'after_skip_anchor': final_anchor, 'opponent_city_slot': spent['city_slot']}

    report = {'schema': 1, **{key: prior[key] for key in ('binary_sha256', 'source', 'rules')},
              'fields': fields, 'cases': cases, 'scope': __doc__.strip()}
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
