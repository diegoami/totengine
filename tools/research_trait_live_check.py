#!/usr/bin/env python3
"""Measure the leader-trait slot's research-valuation impact and what the
prepared-save native paths do with it.

The engine's built-in research selection derives its leader modifier from
Rules (`LEADERS` column 9) and the standalone native comparisons matched
with that basis: the harness writes the modifier into `0x606f4a +
leader*60`, and the research valuation's scores move measurably with that
byte.

This check measures, on the pinned save's tooling:

1. Standalone (proven harness): direct zero-trait versus Rules-modifier
   comparisons move 56/68/73 of 100 advance scores (leaders 0, 1, 2).
2. Nonzero control: a preloaded table (all 21 Rules modifiers) survives
   the observation/input handoff, city entry and the turn-44 annual
   unchanged - those paths neither clear nor rewrite the table.

An earlier revision read all-zero tables here and concluded the live
game presents zeroed traits; that interpretation was retracted on
review (#431): the prepared-save driver starts from zeroed emulation
memory, so its zero readings were harness defaults. The loaded-game
presentation itself (boot-time Rules population versus save
restoration) is not established by this probe. Recorded as evidence
only; no engine change is implied here.
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
    """Direct zero-trait vs Rules-modifier valuation comparison.

    Loads each context with a real leader, sweeps the native valuation
    with the leader-trait slot zeroed, then with the Rules modifier
    written, and counts the differing scores. That is the comparison the
    divergence question needs: live-zeroed basis against Rules basis.
    """
    native = OriginalValue(binary)
    leaders_rows = rules.get('LEADERS', {}).get('rows', [])
    rng = random.Random(11)
    report = []
    for case in range(cases):
        values = context(rng, case)
        civ = values[0]
        leader = case % 21
        values[4] = leader
        rules_modifier = None
        if leader < len(leaders_rows) and len(leaders_rows[leader]) > 9:
            rules_modifier = leaders_rows[leader][9]
        native.load(values)
        native.write(TRAIT_TABLE + leader * 60, 0, '<b')
        zeroed = [native.call(VALUATION, civ, i) for i in range(100)]
        if rules_modifier is not None:
            native.write(TRAIT_TABLE + leader * 60, int(rules_modifier), '<b')
        rules_scores = [native.call(VALUATION, civ, i) for i in range(100)]
        native.write(TRAIT_TABLE + leader * 60, 0, '<b')
        report.append({
            'case': case, 'civ': civ, 'leader': leader,
            'rules_modifier': rules_modifier,
            'zero_vs_rules_score_differences': sum(
                1 for a, b in zip(zeroed, rules_scores) if a != b),
        })
    return report


def live_presentation(home, binary, path, rules):
    """Nonzero control: a preloaded table survives the observed paths.

    The prepared-save driver starts from zeroed emulation memory; its
    earlier all-zero readings were harness defaults, not game behavior.
    This control instead preloads all 21 trait slots with the Rules
    modifiers, then runs the same observation/input handoff, city-entry
    sequence and turn-44 annual, and records the table at each point.
    The paths leave the preloaded values unchanged, so they do not clear
    the table; the loaded-game presentation itself (boot-time Rules
    population versus save restoration) is not established by this probe.
    """
    leaders_rows = rules.get('LEADERS', {}).get('rows', [])
    preload = []
    for leader in range(21):
        modifier = (leaders_rows[leader][9]
                    if leader < len(leaders_rows) and len(leaders_rows[leader]) > 9
                    else 0)
        preload.append(int(modifier) if modifier is not None else 0)
    state = {'preloaded': preload}

    def dump(original):
        return [read(original, TRAIT_TABLE + leader * 60, '<b')
                for leader in range(21)]

    def at_city(original, viewer, city):
        state['traits_at_city_entry'] = dump(original)
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
        state['traits_after_annual'] = dump(original)

    def after_idle(original, viewer, regions):
        state['turn'] = read(original, 0x607648, '<h')
        state['leaders'] = [read(original, CIV + STRIDE * seat + 14, '<h')
                            for seat in range(8)]
        for leader, modifier in enumerate(preload):
            original.write(TRAIT_TABLE + leader * 60, modifier, '<b')
        state['traits_at_idle'] = dump(original)
        trace(original, viewer, regions, clear_preference_bit=False,
              skip_selected=True, at_city_entry=at_city)
        state['traits_unchanged'] = (
            state['traits_at_idle'] == preload
            and state.get('traits_at_city_entry') == preload
            and state.get('traits_after_annual') == preload)

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
        'source': 'civ2.bak research valuation 0x49c95d; leader-trait slot '
                  'per the harness convention 0x606f4a + leader*60',
        'standalone': standalone_scores(binary, rules),
        'live': live_presentation(home, binary, 'Original/Ph_Auto2.SAV', rules),
        'rules_leaders_col9': [row[9] if len(row) > 9 else None
                               for row in rules.get('LEADERS', {}).get('rows', [])],
    }
    live = report['live']
    assert live['traits_unchanged'], live
    for entry in report['standalone']:
        assert entry['zero_vs_rules_score_differences'] > 0, entry
    args.out.write_text(json.dumps(report, indent=1) + '\n')
    print(json.dumps({'standalone': report['standalone'],
                      'live_traits_unchanged': live['traits_unchanged'],
                      'live_leaders': live['leaders']}, indent=1))


if __name__ == '__main__':
    main()
