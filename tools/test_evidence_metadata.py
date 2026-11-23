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
