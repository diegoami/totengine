"""Evidence metadata must match the files it describes (#278 regression).

The campaign golden metadata (`docs/evidence/generated-campaign-v1.json`)
records the SHA-256 of its golden checkpoint file; a refresh that forgets the
hash (as in the first v0.19.0 candidate) is caught here.
"""
import hashlib
import json
import unittest
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


class GoldenMetadata(unittest.TestCase):
    def test_native_history_block_matches_trailer_extent(self):
        history = json.loads((ROOT / 'docs/evidence/import-history.json').read_text())
        tail = json.loads((ROOT / 'docs/evidence/import-event-tail.json').read_text())
        self.assertEqual(history['source'], tail['source'])
        self.assertEqual(history['binary_sha256'], tail['binary_sha256'])
        witness = history['source_witness']
        self.assertEqual(witness['offset_after_cities'] + witness['bytes'],
                         tail['event_offset_after_cities'])
        self.assertEqual(witness['used_entries'], 0)
        self.assertEqual([(row['initial_count'], row['final_count'])
                          for row in history['append']['cases']], [(0, 1), (11, 12), (12, 12)])
        self.assertEqual(history['append']['cases'][-1]['changed_offsets'], [])

    def test_input_handoff_report_matches_the_unit_setup_witness(self):
        report = json.loads((ROOT / 'docs/evidence/import-input-handoff.json').read_text())
        setup = json.loads((ROOT / 'docs/evidence/import-unit-setup.json').read_text())
        for key in ('binary_sha256', 'source', 'rules'):
            self.assertEqual(report[key], setup[key])
        cases = report['cases']
        observation = cases['native_observation']
        self.assertEqual(observation['active_slot'], setup['cases']['pinned']['active_slot_at_boundary'])
        self.assertEqual(observation['explored_tiles_before'], 59)
        self.assertEqual(observation['explored_tiles_after'], 59)
        self.assertEqual(observation['human_memory_bytes_changed'], 0)
        self.assertEqual(observation['unit_masks_changed'], 0)
        self.assertEqual(observation['city_knowledge_records_changed'], 0)
        self.assertNotIn('0041f704', observation['substituted_calls'])
        self.assertEqual(observation['observation_calls'], [[6, 1], [6, 1]])
        continued = observation['observation_to_input']
        self.assertNotIn('0041f704', continued['substituted_calls'])
        self.assertEqual(continued['first_poll']['active_slot'], 6)
        selected = cases['selected_input_control']
        exhausted = cases['exhausted_input_control']
        self.assertEqual((selected['return_address'], exhausted['return_address']),
                         ('0047342e', '00473522'))
        self.assertIn('0041f704', selected['substituted_calls'])
        for poll in (selected['first_poll'], exhausted['first_poll'], continued['first_poll']):
            self.assertEqual(poll['stop_at_call'], '00576ee4')
            self.assertEqual(poll['import'], 'PeekMessageA')
            self.assertEqual(poll['message_filter'], [0, 0])
            self.assertEqual((poll['research_target'], poll['research_progress']), (-1, 0))
        for path in (selected, exhausted, continued):
            idle = path['idle_iteration']
            self.assertEqual(idle['initial_queue_result'], 'empty')
            self.assertEqual(idle['filtered_polls'],
                             [{'call': '00577011', 'message_filter': [0x3bd, 0x3bd]}])
            self.assertEqual(idle['next_general_poll'], path['first_poll']['stop_at_call'])
            self.assertEqual(idle['active_slot'], path['first_poll']['active_slot'])
            self.assertTrue(idle['civ_unit_city_records_unchanged'])

    def test_first_orders_extend_the_prepared_input_witness(self):
        first = json.loads((ROOT / 'docs/evidence/import-first-order.json').read_text())
        handoff = json.loads((ROOT / 'docs/evidence/import-input-handoff.json').read_text())
        for key in ('binary_sha256', 'source', 'rules'):
            self.assertEqual(first[key], handoff[key])
        poll = handoff['cases']['native_observation']['observation_to_input']['first_poll']
        wait, skip, move = (first['cases'][name] for name in ('wait', 'skip', 'move_northwest'))
        for case in (wait, skip, move):
            for key in ('active_slot', 'research_target', 'research_progress'):
                self.assertEqual(case['before'][key], poll[key])
            self.assertEqual(case['after']['research_target'], poll['research_target'])
            self.assertEqual(case['after']['research_progress'], poll['research_progress'])
            self.assertTrue(case['civ_city_bytes_unchanged'])
        self.assertEqual(wait['after']['active_slot'], 9)
        self.assertEqual(skip['unit_byte_changes'][0],
                         {'slot': 6, 'byte': 10, 'before': 0, 'after': 3})
        self.assertEqual([case['explored_tiles'] for case in (wait, skip, move)], [59, 59, 60])
        self.assertEqual(move['move_arguments'], [[6, 6, 3]])
        self.assertEqual(move['unit_position_after'], [62, 4])

    def test_city_science_witness_uses_the_same_pinned_state(self):
        science = json.loads((ROOT / 'docs/evidence/import-city-science-entry.json').read_text())
        handoff = json.loads((ROOT / 'docs/evidence/import-input-handoff.json').read_text())
        for key in ('binary_sha256', 'source', 'rules'):
            self.assertEqual(science[key], handoff[key])
        cases = science['cases']
        for name, proposed in [('pinned', 0), ('one_science', 1), ('one_science_with_advice', 1)]:
            row = cases[name]
            self.assertEqual(row['city_slot'], 5)
            self.assertEqual(row['computed_science'], 0)
            self.assertEqual(row['accumulation'], {'civ': 4, 'points': proposed})
            self.assertEqual(row['after']['progress'], proposed)
            self.assertEqual(row['after']['target'], -1)
        self.assertEqual([row['stop'] for row in cases.values()],
                         ['00100000', '004a1436', '00477030'])

    def test_round_handoff_preserves_the_pinned_human_context(self):
        report = json.loads((ROOT / 'docs/evidence/import-round-handoff.json').read_text())
        handoff = json.loads((ROOT / 'docs/evidence/import-input-handoff.json').read_text())
        for key in ('binary_sha256', 'source', 'rules'):
            self.assertEqual(report[key], handoff[key])
        saved = report['saved_options']
        advanced = report['cleared_preference_bit_0x40']
        self.assertEqual(int(saved['options'], 16) ^ int(advanced['options'], 16), 0x40)
        self.assertEqual(saved['before'], saved['after'])
        self.assertEqual(saved['after'], advanced['before'])
        self.assertEqual(advanced['after']['turn'], advanced['before']['turn'])
        self.assertEqual((advanced['after']['active_civ'], advanced['after']['resume_civ']), (5, 0))
        self.assertEqual(advanced['after']['research_target'], -1)
        self.assertEqual(advanced['city_slot'], 6)
        selected = report['spend_saved_movement']
        self.assertEqual(selected['options'], saved['options'])
        self.assertEqual(selected['after'], advanced['after'])
        self.assertEqual(selected['city_slot'], advanced['city_slot'])
        self.assertEqual([(m['character'], m['active_slot']) for m in selected['dispatched']],
                         [(32, 6), (32, 9), (32, 26), (32, 18)])
        for case in (saved, advanced, selected):
            self.assertTrue(case['viewer_civilization_and_map_knowledge_unchanged'])

    def test_native_city_work_preserves_view_despite_an_unseen_birth(self):
        city = json.loads((ROOT / 'docs/evidence/import-city-view.json').read_text())
        handoff = json.loads((ROOT / 'docs/evidence/import-round-handoff.json').read_text())
        for key in ('binary_sha256', 'source', 'rules'):
            self.assertEqual(city[key], handoff[key])
        observed = city['observed']
        self.assertEqual(observed['city_slot'], handoff['spend_saved_movement']['city_slot'])
        self.assertEqual(observed['viewer_explored_tiles'], 59)
        self.assertFalse(observed['viewer_memory_changed'])
        self.assertFalse(observed['previous_unit_knowledge_changed'])
        self.assertEqual(observed['created_units'],
                         [{'slot': 34, 'position': [43, 37], 'owner': 5,
                           'tile_known': False, 'viewer_mask': False,
                           'visibility_byte': 0, 'orders': 255}])
        move = observed['prepared_opponent_move']
        self.assertEqual(move['arguments'], [2, 1, 3])
        self.assertEqual(move['explored_tiles'], 59)
        self.assertFalse(move['viewer_memory_changed'])
        self.assertEqual([unit['next'] for unit in move['after']], [-1, 2, 19])
        empty = observed['prepared_opponent_empty_move']
        self.assertEqual(empty['arguments'], [10, 7, 3])
        self.assertEqual(empty['explored_tiles'], 59)
        self.assertFalse(empty['viewer_memory_changed'])
        self.assertEqual(empty['after'][0]['position'], [41, 37])
        departure = observed['prepared_opponent_departure']
        self.assertEqual(departure['arguments'], [19, 6, 3])
        self.assertEqual(departure['explored_tiles'], 59)
        self.assertFalse(departure['viewer_memory_changed'])
        self.assertEqual([unit['next'] for unit in departure['after']], [-1, 2, -1])

    def test_native_opponent_combat_preserves_saved_knowledge(self):
        report = json.loads((ROOT / 'docs/evidence/import-opponent-combat-view.json').read_text())
        city = json.loads((ROOT / 'docs/evidence/import-city-view.json').read_text())
        for key in ('binary_sha256', 'source', 'rules'):
            self.assertEqual(report[key], city[key])
        for seed, damage in [('7', 0), ('1', 5)]:
            case = report['cases'][seed]
            self.assertEqual(case['crt_before'], int(seed))
            self.assertEqual(case['observation_calls'], [[20, 1]])
            self.assertEqual(case['rules_refresh']['rows_by_section']['UNITS'], 80)
            self.assertEqual(case['viewer_explored_tiles'], 59)
            self.assertFalse(case['viewer_memory_changed'])
            self.assertEqual(case['after'][0]['damage'], damage)
            self.assertEqual([u['alive'] for u in case['after']], [True, False])
        stack = report['cases']['7']['following_field_stack']
        self.assertEqual(stack['explored_tiles'], 59)
        self.assertEqual(stack['after'][0]['position'], [9, 45])
        self.assertEqual([unit['next'] for unit in stack['after']], [-1, 4])
        self.assertFalse(stack['viewer_memory_changed'])
        self.assertEqual(stack['after'][0]['spent'], 6)
        self.assertEqual(report['cases']['7']['refused_departure']['return'], 0)
        arrival = report['cases']['7']['following_city_arrival']
        self.assertEqual(arrival['explored_tiles'], 59)
        self.assertEqual(arrival['after'][0]['spent'], 1)
        self.assertEqual([unit['next'] for unit in arrival['after']], [24, -1, -1])
        partial = report['cases']['7']['following_partial_move']
        self.assertEqual(partial['crt_before'], 0x0a14e6e2)
        self.assertEqual(partial['crt_after'], 0x23e1f21d)
        self.assertEqual(partial['roll'], 5)
        self.assertFalse(partial['moved'])
        self.assertEqual(partial['after'][0]['position'], [9, 45])
        self.assertEqual(partial['after'][0]['spent'], 3)
        self.assertEqual(
            [(case['roll'], case['moved'], case['after'][0]['spent'])
             for case in report['partial_movement_controls'].values()],
            [(2, False, 3), (5, False, 3), (0, True, 7), (1, True, 7)],
        )
        singleton = report['cases']['7']['following_singleton_move']
        self.assertEqual(singleton['crt_before'], partial['crt_after'])
        self.assertEqual(singleton['crt_after'], 0x6f277c6c)
        self.assertEqual(singleton['roll'], 6)
        self.assertFalse(singleton['moved'])
        self.assertEqual(singleton['after'][0]['position'], [13, 43])
        self.assertEqual(singleton['after'][0]['spent'], 3)
        self.assertEqual(
            [(case['roll'], case['moved'], case['after'][0]['spent'])
             for case in report['singleton_movement_controls'].values()],
            [(2, False, 3), (0, True, 10)],
        )
        road = report['cases']['7']['following_unconnected_road']
        self.assertEqual(road['after'][0]['position'], [9, 43])
        self.assertEqual(road['after'][0]['spent'], 3)
        self.assertEqual(road['crt_after'], singleton['crt_after'])
        self.assertEqual(road['refused_followup']['return'], 0)
        self.assertEqual(road['refused_followup']['before'], road['refused_followup']['after'])
        departure = report['cases']['7']['following_city_departure']
        self.assertEqual(departure['explored_tiles'], 59)
        self.assertEqual(departure['after'][0]['spent'], 1)
        self.assertEqual([unit['next'] for unit in departure['after']], [4, -1, -1])
        self.assertEqual(
            [tuple(case[key] for key in ('attack', 'destination_flags', 'spent',
                                         'mover_previous', 'mover_next'))
             for case in report['city_departure_controls']],
            [(0, 16, 1, 4, -1), (1, 0, 6, -1, 4), (1, 48, 0, -1, 4)],
        )
        self.assertEqual(
            [tuple(case[key] for key in ('attack', 'origin_flags', 'destination_flags',
                                         'spent', 'mover_previous', 'mover_next'))
             for case in report['city_arrival_controls']],
            [(0, 16, 0, 1, 24, -1), (1, 0, 0, 3, -1, 24), (1, 48, 0, 0, -1, 24),
             (1, 48, 48, 0, -1, 24), (1, 0, 48, 3, -1, 24)],
        )
        self.assertEqual(
            [(case['origin_flags'], case['destination_flags'], case['spent'])
             for case in report['route_controls']],
            [(0, 0, 6), (0, 16, 6), (0, 48, 6), (16, 0, 6), (16, 16, 1),
             (16, 48, 1), (48, 0, 6), (48, 16, 1), (48, 48, 0)],
        )

    def test_native_first_opponent_contact_is_not_the_between_trade_turn_return(self):
        report = json.loads((ROOT / 'docs/evidence/import-opponent-contact.json').read_text())
        reference = json.loads((ROOT / 'docs/evidence/import-human-city-stack-returns.json').read_text())
        for key in ('source', 'rules', 'binary_sha256'):
            self.assertEqual(report[key], reference[key])
        self.assertEqual(report['pair'], [5, 6])
        self.assertEqual(report['contact'], [48, 28, 0])
        first, repeat, due = report['cases'].values()
        self.assertEqual(first['prior_treaty_words'], [0, 0])
        self.assertEqual(first['treaty_words'], [0x4401, 0x4401])
        self.assertEqual(first['first_grant_arguments'], [5, 39, 6, 0, 0])
        self.assertEqual((first['stopped_at'], repeat['stopped_at'], due['stopped_at']),
                         ('0049dedf', '00100000', '0049dedf'))
        self.assertEqual(repeat['treaty_words'], [0x401, 0x401])
        self.assertEqual([row['trade_calls'] for row in (first, repeat, due)], [1, 0, 1])
        self.assertTrue(all(row['explored_tiles'] == 59 and
                            row['unit_city_map_and_viewer_memory_unchanged']
                            for row in (first, repeat, due)))

        for key, seed, advances in (
            ('complete', 1, [39, 46]),
            ('engine_seed_control', 0xd766ce46, [39, 1]),
        ):
            control = report[key]
            self.assertEqual(control['crt_before'], seed)
            self.assertEqual(control['crt_draws'], 9)
            self.assertEqual(control['treaty_words'], [0x4405, 0x4405])
            self.assertEqual(control['stopped_at'], '00100000')
            self.assertEqual(control['host_calls'], {'SetRect': 162})
            self.assertEqual(control['explored_tiles'], 59)
            self.assertTrue(control['unit_city_map_and_viewer_memory_unchanged'])
            self.assertEqual([grant[1] for grant in control['grant_arguments']], advances)
            state = seed
            best = {}
            candidates = control['candidate_scores']
            self.assertEqual(len(candidates), 9)
            self.assertEqual(sorted(row[1] for row in candidates), [row[1] for row in candidates])
            for recipient, advance, value, score in candidates:
                state = (state * 214013 + 2531011) & 0xffffffff
                self.assertEqual(score, value + ((state >> 16) & 0x7fff) % 3)
                if recipient not in best or score > best[recipient][0]:
                    best[recipient] = (score, advance)
            self.assertEqual(state, control['crt_after'])
            self.assertEqual([best[civ][1] for civ in (5, 6)], advances)

    def test_native_human_city_stack_returns_link_the_arriving_unit_first(self):
        report = json.loads((ROOT / 'docs/evidence/import-human-city-stack-returns.json').read_text())
        departure = json.loads((ROOT / 'docs/evidence/import-human-city-stack-orders.json').read_text())
        for key in ('source', 'rules', 'binary_sha256'):
            self.assertEqual(report[key], departure[key])
        self.assertEqual(set(report['cases']), {'22', '26'})
        for actor, resident in ((22, 26), (26, 22)):
            case = report['cases'][str(actor)]
            self.assertEqual(case['city_stack'], [actor, resident])
            self.assertEqual(case['move_arguments'], [[actor, 2, 3]])
            self.assertEqual(case['observation_arguments'], [[actor, 1]])
            self.assertEqual(case['unit_position_before'], [52, 8])
            self.assertEqual(case['unit_position_after'], [53, 9])
            self.assertIn({'slot': actor, 'byte': 10, 'before': 0, 'after': 6},
                          case['unit_byte_changes'])
            self.assertEqual(case['tile_byte_changes'], [[2077, 1, 0]])
            self.assertEqual(case['cache_byte_changes'], [[4, 346, 1, 0]])
            self.assertEqual(case['explored_tiles'], 59)

    def test_native_human_city_stack_orders_have_measured_deltas(self):
        report = json.loads((ROOT / 'docs/evidence/import-human-city-stack-orders.json').read_text())
        city = json.loads((ROOT / 'docs/evidence/import-human-city-birth.json').read_text())
        for key in ('source', 'rules', 'binary_sha256'):
            self.assertEqual(report[key], city[key])
        head, tail = city['human_city_birth']['focus']['original_stack']
        self.assertEqual(set(report['cases']), {str(head), str(tail)})
        for actor, other in ((head, tail), (tail, head)):
            case = report['cases'][str(actor)]
            wait, move = case['wait'], case['move_northwest']
            self.assertEqual((wait['before']['active_slot'], wait['after']['active_slot']),
                             (actor, other))
            self.assertEqual(wait['observation_arguments'], [[other, 1]])
            self.assertEqual(wait['tile_byte_changes'], [])
            self.assertEqual(wait['cache_byte_changes'], [])
            self.assertEqual(move['move_arguments'], [[actor, 6, 3]])
            self.assertEqual(move['unit_position_after'], [52, 8])
            self.assertIn({'slot': actor, 'byte': 10, 'before': 0, 'after': 6},
                          move['unit_byte_changes'])
            self.assertEqual(move['tile_byte_changes'], [[2077, 0, 1], [2081, 240, 64]])
            self.assertEqual(move['cache_byte_changes'], [[4, 346, 0, 1]])
            self.assertEqual((wait['explored_tiles'], move['explored_tiles']), (59, 59))
            skip = case['skip']
            self.assertEqual(skip['observation_arguments'], [[actor, 1]])
            self.assertEqual(skip['unit_byte_changes'],
                             [{'slot': actor, 'byte': 10, 'before': 0, 'after': 3},
                              {'slot': actor, 'byte': 13, 'before': 0 if actor == head else 5,
                               'after': 255}])
            self.assertEqual((skip['tile_byte_changes'], skip['cache_byte_changes'],
                              skip['explored_tiles']), ([], [], 59))

    def test_native_human_city_birth_reaches_input_with_saved_knowledge(self):
        report = json.loads((ROOT / 'docs/evidence/import-human-city-birth.json').read_text())
        purchase = json.loads((ROOT / 'docs/evidence/import-saved-purchase.json').read_text())
        workers = json.loads((ROOT / 'docs/evidence/import-saved-citizens.json').read_text())
        for key in ('source', 'rules', 'binary_sha256'):
            self.assertEqual(report[key], purchase[key])
            self.assertEqual(report[key], workers[key])
        birth = report['human_city_birth']
        self.assertEqual(birth['applied_worker'], workers['cases'][0]['after'])
        self.assertEqual(birth['shields_before'], purchase['pinned']['shields_after'])
        self.assertEqual(birth['shields_after'], 0)
        self.assertEqual(birth['shield_writes'], [
            ['004c62e0', 20, 22], ['004c6607', 22, 2], ['004c748e', 2, 0],
        ])
        self.assertEqual(report['surplus_control']['shield_writes'], [
            ['004c62e0', 25, 27], ['004c6607', 27, 7], ['004c748e', 7, 0],
        ])
        self.assertEqual([birth[key] for key in ('explored_before', 'explored_after',
                                                 'explored_at_poll')], [59, 59, 59])
        self.assertEqual((birth['tile_byte_changes'], birth['remembered_bytes_changed']), ([], 0))
        self.assertEqual(birth['unit'],
                         {'slot': 22, 'type': 3, 'owner': 4, 'position': [53, 9], 'spent': 0})
        self.assertEqual(birth['observation_arguments'], [[22, 1]])
        self.assertEqual(birth['focus'], {
            'anchors': [[51, 9], [53, 9], [54, 8], [61, 5], [0, 0]],
            'choices': [18, 22, 22, 9, 6],
            'reversed_head_choices': [18, 22, 22, 9, 6],
            'original_stack': [22, 26],
        })
        self.assertEqual(birth['first_poll']['active_slot'], 18)

    def test_native_production_purchase_preserves_saved_knowledge(self):
        report = json.loads((ROOT / 'docs/evidence/import-saved-purchase.json').read_text())
        production = json.loads((ROOT / 'docs/evidence/import-saved-production.json').read_text())
        for key in ('source', 'rules', 'binary_sha256'):
            self.assertEqual(report[key], production[key])
        fields = ('city', 'item', 'price', 'admitted', 'treasury_before', 'treasury_after',
                  'shields_before', 'shields_after', 'explored_tiles', 'tile_bytes_changed',
                  'remembered_bytes_changed')
        self.assertEqual(tuple(report['pinned'][key] for key in fields),
                         (5, 3, 19, True, 25, 6, 12, 20, 59, 0, 0))
        self.assertEqual(report['pinned']['rebased_shields'], 20)
        self.assertEqual(tuple(report['short_treasury'][key] for key in fields),
                         (5, 3, 19, False, 18, 18, 12, 12, 59, 0, 0))

    def test_native_worker_edits_preserve_saved_knowledge(self):
        report = json.loads((ROOT / 'docs/evidence/import-saved-citizens.json').read_text())
        production = json.loads((ROOT / 'docs/evidence/import-saved-production.json').read_text())
        for key in ('source', 'rules', 'binary_sha256'):
            self.assertEqual(report[key], production[key])
        self.assertEqual((report['entry'], report['stop']), ('004dddc6', '004dd04b'))
        self.assertEqual([case['tile'] for case in report['cases']], [4, 4, 20])
        self.assertEqual([case['after'] for case in report['cases']], [
            {'worked': 68157568, 'specialists': 1}, {'worked': 1048720, 'specialists': 0},
            {'worked': 1048720, 'specialists': 0},
        ])
        self.assertTrue(all(case['explored_tiles'] == 59 and case['tile_bytes_changed'] == 0
                            and case['remembered_bytes_changed'] == 0 for case in report['cases']))

    def test_native_production_commit_preserves_saved_knowledge(self):
        report = json.loads((ROOT / 'docs/evidence/import-saved-production.json').read_text())
        rates = json.loads((ROOT / 'docs/evidence/import-saved-rates.json').read_text())
        for key in ('source', 'rules', 'binary_sha256'):
            self.assertEqual(report[key], rates[key])
        self.assertEqual((report['entry'], report['stop']), ('00434e58', '00434ee4'))
        self.assertEqual([(case['city'], case['item'], case['shields'])
                          for case in report['cases']], [(5, 3, 12), (5, -42, 6)])
        self.assertTrue(all(case['explored_tiles'] == 59 and case['tile_bytes_changed'] == 0
                            and case['remembered_bytes_changed'] == 0 for case in report['cases']))

    def test_native_rate_commit_preserves_saved_knowledge(self):
        report = json.loads((ROOT / 'docs/evidence/import-saved-rates.json').read_text())
        pinned = json.loads((ROOT / 'docs/evidence/import-pending-choice.json').read_text())
        for key in ('source', 'rules', 'binary_sha256'):
            self.assertEqual(report[key], pinned[key])
        self.assertEqual((report['entry'], report['stop']), ('0040b4fa', '0040b6e3'))
        self.assertEqual([(case['science'], case['tax'], case['refreshed_cities'])
                          for case in report['cases']], [(6, 4, []), (4, 4, [5]), (4, 6, [5])])
        self.assertTrue(all(case['explored_tiles'] == 59 and case['tile_bytes_changed'] == 0
                            and case['remembered_bytes_changed'] == 0 for case in report['cases']))

    def test_pending_choice_controls_reach_input_without_a_new_project(self):
        report = json.loads((ROOT / 'docs/evidence/import-pending-choice.json').read_text())
        viewport = json.loads((ROOT / 'docs/evidence/import-saved-viewport.json').read_text())
        for key in ('source', 'rules', 'binary_sha256'):
            self.assertEqual(report[key], viewport[key])
        self.assertEqual(report['anchor'], [61, 5])
        for name, government, progress in [('pinned', 1, 0), ('stored_progress', 1, 1), ('anarchy', 0, 0)]:
            case = report['cases'][name]
            self.assertEqual(case['government_at_poll'], government)
            self.assertEqual(case['input']['research_target'], -1)
            self.assertEqual(case['input']['research_progress'], progress)
            self.assertEqual(case['input']['active_slot'], 9)
            self.assertEqual(case['input']['stop_at_call'], case['idle']['next_general_poll'])
            self.assertEqual([branch['dispatches'] for branch in case['round_start_controls']], [False, True, True])

    def test_prepared_resume_keeps_native_victory_marks(self):
        report = json.loads((ROOT / 'docs/evidence/import-pending-choice.json').read_text())
        for name, active, spent in [('pinned', False, False), ('active_event', True, False),
                                    ('spent_event', False, True)]:
            case = report['cases'][name]
            self.assertEqual(case['event_marks_at_poll'], {
                'active': [active, False], 'spent': [spent, False],
            })
            self.assertEqual(case['input']['active_slot'], 9)
            self.assertFalse(case['round_start_controls'][0]['dispatches'])

    def test_native_wait_records_deferral_without_spending_movement(self):
        report = json.loads((ROOT / 'docs/evidence/import-wait.json').read_text())
        viewport = json.loads((ROOT / 'docs/evidence/import-saved-viewport.json').read_text())
        for key in ('source', 'rules', 'binary_sha256'):
            self.assertEqual(report[key], viewport[key])
        self.assertEqual(report['anchor'], [61, 5])
        self.assertEqual([step['selected'] for step in report['waits']], [6, 26, 18, 26, 9, 6])
        self.assertEqual(report['waits'][3]['deferred'], [18])
        self.assertFalse(report['waits'][3]['foreign_deferred'])
        self.assertTrue(all(step['spent'] == [0, 0, 0, 0] for step in report['waits']))
        self.assertTrue(all(step['selected'] == 9 and not step['deferred'] for step in report['one_ready']))
        self.assertEqual(report['writer_unit_bytes'], 1520)
        self.assertEqual([case['selected'] for case in report['reload'].values()], [6, 9])
        self.assertEqual(report['skip_with_slot_9_deferred'], [6, 26, 18, 9])
        self.assertEqual(report['move_after_wait'], {
            'actor': 6, 'selected': 26, 'deferred': [9], 'explored_tiles': 60, 'position': [62, 4],
        })

    def test_native_annual_update_retains_the_saved_projection(self):
        close = json.loads((ROOT / 'docs/evidence/import-round-close.json').read_text())
        combat = json.loads((ROOT / 'docs/evidence/import-opponent-combat-view.json').read_text())
        for key in ('source', 'rules', 'binary_sha256'):
            self.assertEqual(close[key], combat[key])
        self.assertEqual(close['last_move']['after'][0]['position'], [10, 42])
        self.assertEqual(close['last_move']['explored_tiles'], 59)
        step = close['annual_update']
        self.assertEqual(step['before']['crt'], combat['cases']['7']['following_unconnected_road']['crt_after'])
        self.assertEqual(step['before']['turn'], 43)
        self.assertEqual(step['after'], {'turn': 44, 'crt': 0xd766ce46, 'explored': 59})
        self.assertTrue(step['cleared_wait_flag'])
        self.assertEqual(step['crt_draws'], 2)
        self.assertEqual(step['observation_calls'], [])
        self.assertFalse(step['viewer_memory_changed'])

    def test_fresh_human_input_keeps_saved_knowledge_and_ready_units(self):
        report = json.loads((ROOT / 'docs/evidence/import-round-close.json').read_text())
        fresh = report['fresh_human_input']
        self.assertTrue(fresh['autosave_disabled'])
        self.assertEqual(fresh['spent_after_annual_update'], dict.fromkeys(('6', '9', '18', '26'), 0))
        handoff = fresh['handoff']
        self.assertEqual(handoff['explored_tiles_after'], 59)
        self.assertEqual(handoff['human_memory_bytes_changed'], 0)
        self.assertEqual(handoff['observation_calls'], [[18, 1], [18, 1]])
        first_poll = handoff['observation_to_input']['first_poll']
        self.assertEqual(first_poll['active_slot'], 18)
        self.assertEqual((first_poll['research_target'], first_poll['research_progress']), (-1, 0))

    def test_saved_viewport_changes_the_prepared_resume_selection(self):
        report = json.loads((ROOT / 'docs/evidence/import-saved-viewport.json').read_text())
        prefix = json.loads((ROOT / 'docs/evidence/import-event-tail.json').read_text())
        self.assertEqual(report['source'], prefix['source'])
        self.assertEqual(report['binary_sha256'], prefix['binary_sha256'])
        self.assertEqual(report['fields'], {
            '00461836': {'target': '005f46fc', 'post_city_offset': 63},
            '00461860': {'target': '005f46f8', 'post_city_offset': 65},
        })
        for label, anchor, slot in [('zero_anchor', [0, 0], 6), ('saved_anchor', [61, 5], 9)]:
            case = report['cases'][label]
            self.assertEqual(case['anchor'], anchor)
            handoff = case['handoff']
            self.assertEqual(handoff['active_slot'], slot)
            self.assertEqual(handoff['explored_tiles_after'], 59)
            self.assertEqual(handoff['human_memory_bytes_changed'], 0)
            self.assertEqual(handoff['observation_to_input']['first_poll']['active_slot'], slot)
            self.assertEqual(case['skip_order'], [9, 6, 26, 18] if label == 'saved_anchor' else [6, 9, 26, 18])
            self.assertEqual(case['after_skip_anchor'], [51, 9])
            self.assertEqual(case['opponent_city_slot'], 6)

    def test_fresh_human_focus_raster_covers_every_tile(self):
        report = json.loads((ROOT / 'docs/evidence/import-round-close.json').read_text())
        focus = report['fresh_human_input']['focus']
        self.assertEqual(focus['entry'], '0056cb3d')
        self.assertEqual(focus['anchor'], [51, 9])
        self.assertEqual(focus['selected'], 18)
        self.assertEqual((focus['width'], focus['height']), (80, 50))
        self.assertEqual(set(focus['selected_counts']), {'6', '9', '18', '26'})
        self.assertEqual(sum(focus['selected_counts'].values()), 2000)
        self.assertRegex(focus['raster_fnv64'], r'^[0-9a-f]{16}$')

    def test_fresh_human_orders_match_native_budget_and_view_deltas(self):
        report = json.loads((ROOT / 'docs/evidence/import-round-close.json').read_text())
        skip = report['fresh_orders']['skip']
        self.assertEqual(skip['explored_tiles'], 59)
        self.assertEqual(skip['tile_byte_changes'], [])
        self.assertEqual(skip['cache_byte_changes'], [])
        self.assertEqual(skip['unit_byte_changes'], [{'slot': 18, 'byte': 10, 'before': 0, 'after': 3}])
        move = report['fresh_orders']['move_northwest']
        self.assertEqual(move['observation_arguments'], [[18, 1]])
        self.assertEqual(move['unit_position_after'], [50, 8])
        self.assertEqual(move['explored_tiles'], 60)
        self.assertEqual(move['tile_byte_changes'], [[2068, 0, 16], [2071, 0, 1], [2075, 240, 64], [2311, 1, 0]])
        self.assertEqual(move['cache_byte_changes'], [[4, 345, 0, 1], [4, 385, 1, 0]])

    def test_unit_setup_witness_extends_the_saved_phase_evidence(self):
        setup = json.loads((ROOT / 'docs/evidence/import-unit-setup.json').read_text())
        phase = json.loads((ROOT / 'docs/evidence/import-phase.json').read_text())
        self.assertEqual(setup['source'], phase['source'])
        self.assertEqual(setup['binary_sha256'], phase['binary_sha256'])
        self.assertEqual(setup['replaced_callees'], [])
        cases = setup['cases']
        self.assertEqual(cases['pinned']['active_slot_at_boundary'], 6)
        self.assertEqual(cases['only_slot_9_ready']['active_slot_at_boundary'], 9)
        self.assertEqual(cases['all_own_units_exhausted']['active_slot_at_boundary'], -1)
        for case in cases.values():
            self.assertTrue(case['civ_unit_city_records_unchanged'])
            self.assertEqual(case['player_research_target'], phase['source_witness']['target'])
            self.assertEqual(case['player_research_progress'], phase['source_witness']['progress'])

    def test_event_state_report_has_all_distinct_round_trips(self):
        report = json.loads((ROOT / 'docs/evidence/import-event-state.json').read_text())
        tail = json.loads((ROOT / 'docs/evidence/import-event-tail.json').read_text())
        self.assertEqual(report['source'], tail['source'])
        self.assertEqual(report['binary_sha256'], tail['binary_sha256'])
        self.assertEqual(report['pinned_action_and_modifier_words'],
                         [tail['victory_fields'][f'{half}_header_words'][1:]
                          for half in ('first', 'second')])
        cases = report['cases']
        self.assertEqual(len(cases), 16)
        self.assertEqual(len({tuple(map(tuple, case['input'])) for case in cases}), 16)
        for case in cases:
            self.assertEqual(case['input'], case['written'])
            self.assertEqual(case['written'], case['read'])
            self.assertTrue(case['startup_preserved_records'])
            self.assertTrue(case['resumed_round_preserved_records'])
            self.assertTrue(case['fresh_round_cleared_only_active_marks'])
        self.assertEqual(report['writer']['bytes_with_round_flag_clear'], 0)
        startup = report['startup']
        self.assertEqual(startup['synthetic_cases'], len(cases))
        witness = startup['pinned_witness']
        self.assertEqual(witness['trigger_words'], [0x100, 0x40])
        self.assertEqual(witness['round_flag_set_stop'], '00463fbe')
        self.assertEqual(witness['round_flag_clear_stop'], '00463fc8')
        self.assertEqual(witness['scenario_loaded_control_stop'], '004d3ae8')
        self.assertTrue(witness['records_preserved'])
        self.assertEqual(report['round_boundary']['resume_stop'], '00474574')
        self.assertEqual(report['round_boundary']['fresh_stop'], '00474520')

    def test_event_tail_report_matches_snapshot_extent(self):
        report = json.loads((ROOT / 'docs/evidence/import-event-tail.json').read_text())
        snapshot = json.loads((ROOT / 'docs/evidence/import-snapshot.json').read_text())
        self.assertEqual(report['source'], snapshot['source'])
        self.assertEqual(report['post_city_bytes'], snapshot['trailing_bytes_unchecked'])
        prefix = report['prefix_trace']
        self.assertEqual(prefix['consumed'], report['event_offset_after_cities'])
        self.assertEqual(sum(group['bytes_read'] for group in prefix['reads'].values()),
                         prefix['consumed'])
        self.assertEqual(prefix['stop'], '00461d7b')
        self.assertEqual(report['truncated_prefix']['stop'], '00463052')
        native = report['native']
        self.assertEqual(report['event_offset_after_cities'] + native['consumed'],
                         report['post_city_bytes'])
        self.assertEqual(native['remaining'], 0)
        comparison = report['victory_string_comparison']
        self.assertEqual(comparison['total_strings_matched'], len(native['strings']))
        self.assertEqual(comparison['text_lines_matched_per_record'], [6, 6])
        self.assertTrue(comparison['rejects_same_length_text_change'])
        victory = json.loads((ROOT / 'docs/evidence/original-victory.json').read_text())
        self.assertEqual(report['events_txt_sha256'], victory['events_sha256'])
        self.assertEqual(native['round_flag_at_stop'], 1)
        self.assertEqual([row['dispatches'] for row in native['round_start_gate']],
                         [False, True, True])

    def test_native_phase_boundary_metadata_and_pinned_witness(self):
        report = json.loads((ROOT / 'docs/evidence/import-phase.json').read_text())
        vectors = (ROOT / 'docs/evidence/import-phase-vectors.txt').read_bytes()
        self.assertEqual(hashlib.sha256(vectors).hexdigest(), report['vectors_sha256'])
        rows = [line for line in vectors.decode().splitlines() if line and not line.startswith('#')]
        self.assertEqual(len(rows), len(set(rows)))
        counts = Counter(line.split()[0] for line in rows)
        self.assertEqual(dict(counts), {'L': report['cases']['loader'],
                                      'R': report['cases']['round_guard'],
                                      'S': report['cases']['science'],
                                      'P': report['cases']['source_prefix']})
        native = json.loads((ROOT / 'docs/evidence/import-knowledge.json').read_text())
        self.assertEqual(report['source']['sha256'], native['source']['sha256'])
        self.assertEqual(report['binary_sha256'], native['binary_sha256'])
        self.assertEqual(report['replaced_callees'], [])
        self.assertFalse(report['runtime_admitted'])
        witness = report['source_witness']
        self.assertEqual((witness['player'], witness['active'], witness['human_mask']), (4, 4, 16))
        self.assertEqual((witness['progress'], witness['target'], witness['round_boundary']), (0, -1, 2))
        self.assertEqual(witness['science_0'], [0, 0])
        self.assertEqual(witness['science_1'], [1, 3])
        self.assertEqual(witness['unit_setup_prefix']['stop'], '00472ada')
        self.assertEqual(witness['unit_setup_prefix']['active_unit_at_boundary'], -1)
        self.assertTrue(witness['unit_setup_prefix']['civ_unit_city_records_unchanged'])

    def test_original_dependency_capture_metadata(self):
        report = json.loads((ROOT / 'docs/evidence/import-dependencies.json').read_text())
        saved = json.loads((ROOT / 'docs/evidence/import-saved-view.json').read_text())
        self.assertEqual(report['source'], saved['source'])
        self.assertEqual(report['source_fnv64'], saved['source_fnv64'])
        self.assertFalse(report['runtime_admitted'])
        self.assertTrue(report['independent_python_hashes_match'])
        self.assertTrue(report['atlas']['same_as_existing_original_atlas'])
        self.assertEqual(report['dependency_set'], 'original-reference-dependencies-v1')
        roles = {'rules', 'events', 'labels', 'describe', 'scenario',
                 'terrain1', 'terrain2', 'units', 'cities', 'city', 'icons', 'people', 'dialog'}
        self.assertEqual({item['role'] for item in report['inputs']}, roles)
        self.assertEqual(len(report['inputs']), len(roles))
        self.assertEqual(sum(item['dimensions'] is not None for item in report['inputs']), 8)
        self.assertEqual(hashlib.sha256(report['identity'].encode()).hexdigest(), report['identity_sha256'])
        self.assertIn(f"dependency-set {report['dependency_set']}\n", report['identity'])
        self.assertIn(f"presentation {report['presentation']}\n", report['identity'])
        for item in report['inputs']:
            self.assertEqual(len(bytes.fromhex(item['sha256'])), 32)
            self.assertIn(f"sha256 {item['role']} {item['sha256']}\n", report['identity'])
            self.assertTrue(item['path'].lower().startswith('original/'))
            self.assertGreater(item['length'], 0)

    def test_snapshot_record_and_normalization_measurement(self):
        report = json.loads((ROOT / 'docs/evidence/import-snapshot.json').read_text())
        saved = json.loads((ROOT / 'docs/evidence/import-saved-view.json').read_text())
        self.assertEqual(report['source'], saved['source'])
        self.assertEqual(report['initial_world'], saved['initial_world'])
        self.assertEqual(report['source_fnv64'], saved['source_fnv64'])
        self.assertFalse(report['runtime_admitted'])
        self.assertEqual(report['only_world_difference_from_boot'], 'movement')
        self.assertEqual(report['rng_draws'], [0, 0])
        self.assertGreater(report['trailing_bytes_unchecked'], 0)
        positions = report['stack_positions']
        self.assertEqual(len(positions), report['unit_slots'])
        rows = report['unit_rows']
        self.assertEqual(len(rows), report['live_units'])
        self.assertEqual(sum(p >= 0 for p in positions), len(rows))
        self.assertEqual(len({row[0] for row in rows}), len(rows))
        self.assertEqual(len({row[1] for row in rows}), len(rows))
        for slot, identity, owner, spent, allowance, remaining in rows:
            self.assertGreaterEqual(positions[slot], 0)
            self.assertGreater(identity, 0)
            self.assertIn(owner, range(8))
            self.assertEqual(remaining, max(0, allowance - spent))
        self.assertTrue(any(row[3] > row[4] for row in rows))

    def test_saved_view_integration_and_legacy_baseline_metadata(self):
        report=json.loads((ROOT/'docs/evidence/import-saved-view.json').read_text())
        native=json.loads((ROOT/'docs/evidence/import-knowledge.json').read_text())
        self.assertEqual(report['source']['sha256'],native['source']['sha256'])
        self.assertEqual(report['initializer_schema'],2)
        self.assertEqual(report['initial_seen_tiles'],native['source']['explored_tiles'])
        self.assertEqual(report['initial_unit_slots'],native['source']['shown_units'])
        baseline=json.loads((ROOT/'docs/evidence/import-saved-view-legacy.json').read_text())
        checkpoint=baseline['checkpoint'];data=(ROOT/checkpoint['path']).read_bytes()
        self.assertEqual(len(data),checkpoint['length'])
        self.assertEqual(hashlib.sha256(data).hexdigest(),checkpoint['sha256'])
        self.assertIn(b'initializer 1 profile experimental-scenario-v1 ',data)
        for source in baseline['rules']:
            self.assertEqual(hashlib.sha256((ROOT/source['path']).read_bytes()).hexdigest(),source['sha256'])
        clients=json.loads((ROOT/'docs/evidence/import-saved-view-clients.json').read_text())
        self.assertEqual(clients['initial_view']['initial_seen_tiles'],report['initial_seen_tiles'])
        self.assertEqual(clients['initial_view']['initial_units'],report['initial_unit_slots'])
        self.assertEqual(clients['initializer']['schema'],2)
        self.assertEqual(clients['initializer']['profile'],report['profile'])
        self.assertEqual(len(clients['real_input']['checkpoint_comparisons']),11)
        old=json.loads((ROOT/'docs/evidence/import-knowledge-clients.json').read_text())
        new={c['checkpoint']:c['sha256'] for c in clients['real_input']['checkpoint_comparisons']}
        for checkpoint in old['real_input']['checkpoint_comparisons']:
            self.assertEqual(new[checkpoint['checkpoint']],checkpoint['sha256'])

    def test_city_phase_memory_vectors_match_native_report(self):
        report = json.loads((ROOT / 'docs/evidence/import-city-memory.json').read_text())
        vectors = (ROOT / 'docs/evidence/import-city-memory-vectors.txt').read_bytes()
        self.assertEqual(hashlib.sha256(vectors).hexdigest(),report['vectors_sha256'])
        rows = [line for line in vectors.decode().splitlines()
                if line and not line.startswith('#')]
        self.assertEqual(len(rows),report['cases'])
        self.assertEqual(len(rows),len(set(rows)))

    def test_submarine_vectors_match_native_report(self):
        report = json.loads((ROOT / 'docs/evidence/import-submarine.json').read_text())
        vectors = (ROOT / 'docs/evidence/import-submarine-vectors.txt').read_bytes()
        self.assertEqual(hashlib.sha256(vectors).hexdigest(), report['vectors_sha256'])
        rows = [line for line in vectors.decode().splitlines()
                if line and not line.startswith('#')]
        self.assertEqual(len(rows),len(set(rows)))
        counts = Counter('contact_boundary' if line.split('|')[1].strip().startswith('C ') else
                         'completed' for line in rows)
        self.assertEqual(dict(counts),report['cases'])
        self.assertEqual(sum(line.startswith('F ') for line in rows),12)
        self.assertEqual(set(report['numeric_witnesses']),{
            'paired_submarine_seen_without_detector',
            'single_submarine_hidden_without_detector',
            'single_submarine_seen_by_detector'})
        self.assertTrue(all(n>0 for n in report['numeric_witnesses'].values()))

    def test_stack_reset_and_contact_vectors_match_native_report(self):
        report = json.loads((ROOT / 'docs/evidence/import-contact.json').read_text())
        vectors = (ROOT / 'docs/evidence/import-contact-vectors.txt').read_bytes()
        self.assertEqual(hashlib.sha256(vectors).hexdigest(), report['vectors_sha256'])
        rows = [line for line in vectors.decode().splitlines()
                if line and not line.startswith('#')]
        self.assertEqual(len(rows), len(set(rows)))
        counts = Counter('reset' if line.startswith('R ') else
                         'contact_boundary' if line.split('|')[1].strip().startswith('C ') else
                         'completed' for line in rows)
        self.assertEqual(dict(counts), report['cases'])

    def test_unit_knowledge_update_vectors_match_native_report(self):
        report = json.loads((ROOT / 'docs/evidence/import-visibility.json').read_text())
        vectors = (ROOT / 'docs/evidence/import-visibility-vectors.txt').read_bytes()
        self.assertEqual(hashlib.sha256(vectors).hexdigest(), report['vectors_sha256'])
        cases = [line for line in vectors.decode().splitlines()
                 if line and not line.startswith('#')]
        self.assertEqual(len(cases), report['cases'])
        self.assertEqual(len(set(cases)), len(cases))

    def test_saved_knowledge_vectors_match_native_report(self):
        report = json.loads((ROOT / 'docs/evidence/import-knowledge.json').read_text())
        vectors = (ROOT / 'docs/evidence/import-knowledge-vectors.txt').read_bytes()
        self.assertEqual(hashlib.sha256(vectors).hexdigest(), report['vectors_sha256'])
        counts = Counter(line.split()[0] for line in vectors.decode().splitlines()
                         if line and not line.startswith('#'))
        for kind, key in [('G','city_unit_gates'),('S','city_size'),
                          ('R','remembered_improvements'),('E','exploration'),('T','tiny_exploration')]:
            self.assertEqual(counts[kind], report['cases'][key])

    def test_campaign_golden_hash_matches_the_recorded_file(self):
        meta = json.loads((ROOT / 'docs/evidence/generated-campaign-v1.json').read_text())
        golden = ROOT / 'docs/evidence/generated-campaign-v1.ctc'
        actual = hashlib.sha256(golden.read_bytes()).hexdigest()
        self.assertEqual(
            meta['golden']['sha256'], actual,
            'generated-campaign-v1.json golden.sha256 does not match the .ctc file')
        major, minor, _ = meta['golden']['engine_version'].split('.')
        self.assertIn(
            f'v{major}.{minor} candidate', meta['golden']['refresh'],
            'the refresh note should name the current candidate version')
        self.assertIn(
            f"engine {meta['golden']['engine_version']}\n", golden.read_text(),
            'the golden header should carry the recorded engine version')


if __name__ == '__main__':
    unittest.main()
