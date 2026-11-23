#!/usr/bin/env python3
"""Compose the prepared native loader and human-input boundaries.

Use the pinned event trailer, supply stock Rules rows and process preferences,
and carry the same prepared memory into resumed input. Two controls vary only
the player's research progress or government. The probe substitutes display and
host queue operations and stops at the first input poll and an idle iteration.
Two event controls retain a single active or spent mark through that boundary.
"""
import argparse
import json
import os
from pathlib import Path

from import_event_tail_check import digest, trace
from import_input_handoff_check import measure_handoff
from import_loader_exit_check import caller_options, post_event
from import_phase_check import CIV, FRAME, STRIDE, frame, read
from import_unit_setup_check import prepare
from totctl import parse_rules, walk_save_bytes


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    viewport = json.loads((root / 'docs/evidence/import-saved-viewport.json').read_text())
    event = json.loads((root / 'docs/evidence/import-event-tail.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    path = viewport['source']['path']
    data = (home / path).read_bytes()
    binary = (home / 'civ2.bak').read_bytes()
    rules_path = home / viewport['rules']['path']
    assert digest(data) == viewport['source']['sha256'] == event['source']['sha256']
    assert digest(binary) == viewport['binary_sha256'] == event['binary_sha256']
    assert digest(rules_path.read_bytes()) == viewport['rules']['sha256']
    rules = parse_rules(rules_path)
    cities = walk_save_bytes(data)['cities']
    city_end = cities['offset'] + cities['size']
    tail = data[city_end + event['event_offset_after_cities']:]
    options = int.from_bytes(data[654:658], 'little')
    anchor = viewport['cases']['saved_anchor']['anchor']

    def case(*, progress=None, government=None, marked_word=None):
        trailer = bytearray(tail)
        if marked_word is not None:
            offset = 8 + marked_word
            word = int.from_bytes(trailer[offset:offset + 4], 'little')
            trailer[offset:offset + 4] = (word | 0x20000000).to_bytes(4, 'little')
        original, player, _, regions, _ = prepare(home, binary, path, exhaust=False)
        seat = CIV + STRIDE * player
        if progress is not None:
            original.write(seat + 16, progress, '<h')
        if government is not None:
            original.write(seat + 29, government, '<B')
        width, height = (read(original, at, '<h') for at in (0x6705A0, 0x6705A2))
        area = width * height // 2
        original.write(0x6705A4, area, '<h')
        ranges = regions + [(0x700000, 6 * area)] + [
            (read(original, 0x5DEAD0 + 4 * civ, '<I'), area) for civ in range(1, 8)
        ]

        def finish(original, records):
            loader = post_event(original, ranges, player, records, rules)
            caller = caller_options(original, options, options, 0)
            frame(original)
            original.write(FRAME - 0x6AC, player, '<I')
            for address, coordinate in zip((0x5F46FC, 0x5F46F8), anchor, strict=True):
                original.write(address, coordinate, '<h')
            assert read(original, 0x5D4FA0) == 0
            result = measure_handoff(original, player, regions, observation=True)
            poll = result['observation_to_input']['first_poll']
            assert read(original, seat + 29, '<B') == (1 if government is None else government)
            assert poll['research_target'] == -1 and poll['research_progress'] == (progress or 0)
            assert poll['active_slot'] == 9 and result['explored_tiles_after'] == 59
            assert result['human_memory_bytes_changed'] == 0
            assert len(records) == 2
            marks = {
                'active': [bool(read(original, record + 8, '<I') & mask)
                           for record, mask in zip(records, (0x20000000, 0x80000000), strict=True)],
                'spent': [bool(read(original, record + 4, '<I') & 0x20000000) for record in records],
            }
            assert marks == {'active': [marked_word == 8, False], 'spent': [marked_word == 4, False]}
            return {'loader_phase': loader['phase'], 'caller': caller, 'event_marks_at_poll': marks,
                    'government_at_poll': read(original, seat + 29, '<B'),
                    'input': poll, 'idle': result['observation_to_input']['idle_iteration']}

        loaded = trace(binary, bytes(trailer), original=original, record_base=0x7F0000, after_load=finish)
        assert loaded['remaining'] == 0 and loaded['round_flag_at_stop'] == 1
        guards = loaded['round_start_gate']
        assert guards == [
            {'resume_civ': 4, 'restart': 0, 'dispatches': False},
            {'resume_civ': 4, 'restart': 1, 'dispatches': True},
            {'resume_civ': 0, 'restart': 0, 'dispatches': True},
        ]
        return {'round_start_controls': guards, **loaded['after_load']}

    report = {
        'schema': 1, **{key: viewport[key] for key in ('source', 'rules', 'binary_sha256')},
        'anchor': anchor, 'cases': {'pinned': case(), 'stored_progress': case(progress=1),
                                    'anarchy': case(government=0), 'active_event': case(marked_word=8),
                                    'spent_event': case(marked_word=4)}, 'scope': __doc__.strip(),
    }
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
