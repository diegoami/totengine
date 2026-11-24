#!/usr/bin/env python3
"""Measure the live leader-trait presentation against research valuation.

The engine's built-in research selection derives its leader modifier from
Rules (`LEADERS` column 9) and the standalone native comparisons matched
with that basis: the harness wrote the modifier into `0x606f4a +
leader*60`, and the research valuation's scores move measurably with that
byte. The live original zeroes the same table during play (recorded for
the trade-turn valuation in import-opponent-trade-contact.json), so the
loaded game presents modifier 0 wherever Rules carry a nonzero value.

This check measures both halves on the pinned save:

1. Standalone (proven harness): the trait slot moves 56-77 of 100 advance
   scores between the Rules modifier and zero.
2. Live: the pinned save's leader ids and the presented trait table at the
   turn-43 idle and after the turn-44 annual - all zero at both points.

Together: the engine's Rules-derived research valuation diverges from live
original behavior for every civ whose leader carries a nonzero Rules
modifier. Recorded as a finding; no engine change is implied here.
"""
import argparse
import json
import os
import random
import sys
from pathlib import Path


sys.path.insert(0, str(Path(__file__).resolve().parent))

from ai_research_value_check import OriginalValue, context
from import_city_view_check import move_opponent
from import_input_handoff_check import witness
from import_opponent_combat_view_check import (
    execute, partial_movement, singleton_movement, unconnected_road,
)
from import_phase_check import CIV, STRIDE, read
from import_round_handoff_check import trace
from totctl import parse_rules

TRAIT_TABLE = 0x606F4A
VALUATION = 0x49C95D


def standalone_scores(binary, rules, cases=3):
    """Score impact of the trait slot on the proven standalone harness."""
    native = OriginalValue(binary)
    leaders_rows = rules.get('LEADERS', {}).get('rows', [])
    rng = random.Random(11)
    report = []
    for case in range(cases):
        values = context(rng, case)
        civ, leader, trait = values[0], values[4], values[5]
        # Force a real leader so the slot arithmetic is direct; the random
        # contexts include the sentinel -1, whose slot is legitimately
        # below the table but is not a civ the engine ever scores.
        values[4] = leader = case % 21
        rules_modifier = None
        if leader < len(leaders_rows) and len(leaders_rows[leader]) > 9:
            rules_modifier = leaders_rows[leader][9]
        native.load(values)
        baseline = [native.call(VALUATION, civ, i) for i in range(100)]
        native.write(TRAIT_TABLE + leader * 60, 0, '<b')
        zeroed = [native.call(VALUATION, civ, i) for i in range(100)]
        native.write(TRAIT_TABLE + leader * 60, trait, '<b')
        entry = {
            'case': case, 'civ': civ, 'leader': leader, 'trait': trait,
            'rules_modifier': rules_modifier,
            'zeroed_score_differences': sum(
                1 for a, b in zip(baseline, zeroed) if a != b),
        }
        if rules_modifier is not None and str(rules_modifier) != str(trait):
            native.write(TRAIT_TABLE + leader * 60, int(rules_modifier), '<b')
            rules_scores = [native.call(VALUATION, civ, i) for i in range(100)]
            native.write(TRAIT_TABLE + leader * 60, trait, '<b')
            entry['rules_score_differences'] = sum(
                1 for a, b in zip(baseline, rules_scores) if a != b)
        report.append(entry)
    return report


def live_presentation(home, binary, path, rules):
    """The pinned save's trait table at the idle and after one annual."""
    state = {}

    def at_city(original, viewer, city):
        state['traits_at_city_entry'] = [
            read(original, TRAIT_TABLE + leader * 60, '<b') for leader in range(21)]
        # The proven round-close driver: the opponent movements and city
        # operations initialize the state the annual update reads.
        execute(original, viewer, city, rules, 7)
        move_opponent(original, viewer, (4, 5, 3), (4, 7), (11, 45), (9, 45),
                      ((7, -1), (-1, 4)))
        move_opponent(original, viewer, (7, 4, 3), (7, 4, 24), (9, 45), (8, 46),
                      ((-1, 24), (-1, -1), (7, -1)))
        move_opponent(original, viewer, (24, 0, 3), (24, 7, 4), (8, 46), (9, 45),
                      ((-1, 4), (-1, -1), (24, -1)))
        partial_movement(original, viewer, None)
        singleton_movement(original, viewer, None)
        unconnected_road(original, viewer)
        from import_round_close_check import annual_update
        state['annual'] = annual_update(original, viewer)
        state['traits_after_annual'] = [
            read(original, TRAIT_TABLE + leader * 60, '<b') for leader in range(21)]

    def after_idle(original, viewer, regions):
        state['turn'] = read(original, 0x607648, '<h')
        state['leaders'] = [read(original, CIV + STRIDE * seat + 14, '<h')
                            for seat in range(8)]
        state['traits_at_idle'] = [
            read(original, TRAIT_TABLE + leader * 60, '<b') for leader in range(21)]
        trace(original, viewer, regions, clear_preference_bit=False,
              skip_selected=True, at_city_entry=at_city)

    witness(home, binary, path, exhaust=False, observation=True, after_idle=after_idle)
    return state


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True, type=Path)
    args = parser.parse_args()
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    rules = parse_rules(home / 'Original/Rules.txt')
    report = {
        'source': 'civ2.bak research valuation 0x49c95d; trait slot '
                  '0x606f4a + leader*60',
        'standalone': standalone_scores(binary, rules),
        'live': live_presentation(home, binary, 'Original/Ph_Auto2.SAV', rules),
        'rules_leaders_col9': [row[9] if len(row) > 9 else None
                               for row in rules.get('LEADERS', {}).get('rows', [])],
    }
    live = report['live']
    assert live['traits_at_idle'] == [0] * 21, live['traits_at_idle']
    assert live['traits_after_annual'] == [0] * 21, live['traits_after_annual']
    for entry in report['standalone']:
        assert entry['zeroed_score_differences'] > 0, entry
    args.out.write_text(json.dumps(report, indent=1) + '\n')
    print(json.dumps({'standalone': report['standalone'],
                      'live_leaders': live['leaders'],
                      'live_traits_zero': True}, indent=1))


if __name__ == '__main__':
    main()
