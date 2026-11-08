"""Guard inspection/admission separation and private-input safety."""
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
import json
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

import scenario_inventory as inv
import totctl


class InventoryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.home = self.root/'game'
        self.home.mkdir()
        self.baseline = json.loads((inv.ROOT/'docs/scenario-engine-baseline.json').read_text())
        self.entry = dict(id='sample', kind='classic', save='sample.scn', rules='Rules.txt',
                          events='Events.txt', lua=[], art_roots=['.'],
                          declared_features=['surface'], first_entry_candidate=True)
        self.manifest = dict(schema=1, first_entry_profile='classic-surface-032-v1', entries=[self.entry])
        # Authored structural fixture in the previously proven probe framing.
        data = bytearray(40246)
        data[:8] = b'CIVILIZE'
        struct.pack_into('<H', data, 10, 0x32)
        struct.pack_into('<8h', data, 29900, 4, 2, 4, 1, 0, 1, 1, 0)
        (self.home/'sample.scn').write_bytes(data)
        (self.home/'Rules.txt').write_bytes(b'@UNITS\r\nUnit\r\n@TERRAIN\r\nDesert\r\n@CIVILIZE\r\nStudy\r\n')
        (self.home/'Events.txt').write_text('@IF\nTURN\nturn=1\n@THEN\nCHANGEMONEY\nreceiver=Anyone\namount=1\n@ENDIF\n')

    def tearDown(self):
        self.temp.cleanup()

    def report(self):
        return inv.report(self.home, json.dumps(self.manifest).encode(), json.dumps(self.baseline).encode())

    def cli_report(self):
        manifest = self.root/'manifest.json'
        manifest.write_text(json.dumps(self.manifest))
        output = self.root/'report.json'
        with redirect_stderr(StringIO()):
            code = inv.main(['--home', str(self.home), '--manifest', str(manifest), '--out', str(output)])
        result = json.loads(output.read_text())
        output.unlink()
        return code, result

    def test_structural_macro_errors_fail_inspection_but_keep_reports(self):
        valid = (self.home/'Events.txt').read_text()
        for src, reason in [
            (valid.replace('@ENDIF', ''), 'unclosed_event'),
            ('@IF\nTURN\n@THEN\nTEXT\nunclosed', 'unclosed_payload'),
            ('@IF\n@THEN\n@ENDIF', 'malformed_then'),
            ('@IF\nTURN\n@ENDIF', 'malformed_endif'),
            (valid.replace('CHANGEMONEY', 'CREATEUNIT\nLOCATIONS\nx,y\nENDLOCATIONS'), 'malformed_location'),
            (valid.replace('TURN', '@BEGINEVENTS\nTURN'), 'misplaced_directive'),
        ]:
            with self.subTest(reason=reason):
                (self.home/'Events.txt').write_text(src)
                code, report = self.cli_report()
                self.assertEqual(code, 1)
                self.assertEqual(report['summary']['inspection_errors'], 1)
                entry = report['entries'][0]
                self.assertEqual(entry['support']['inspection'], 'incomplete')
                self.assertIn(reason, entry['diagnostics'][0]['reasons'])
                self.assertIn(reason, {d['code'] for d in entry['macro']['diagnostics']})
        # Unsupported semantics remain inspectable; they never grant simulation.
        (self.home/'Events.txt').write_text(valid.replace('CHANGEMONEY', 'UNKNOWN'))
        code, report = self.cli_report()
        self.assertEqual(code, 0)
        self.assertEqual(report['entries'][0]['support']['simulation'], 'not_admitted')

    def test_oversized_coordinates_leave_other_entries_reportable(self):
        valid = (self.home/'Events.txt').read_text()
        (self.home/'valid.txt').write_text(valid)
        self.manifest['entries'].append(self.entry | {'id': 'other', 'events': 'valid.txt', 'first_entry_candidate': False})
        for position in range(3):
            fields = ['1', '2', '0']
            fields[position] = '9'*5000
            (self.home/'Events.txt').write_text('@IF\nTURN\nturn=1\n@THEN\nCREATEUNIT\nLOCATIONS\n'+','.join(fields)+'\nENDLOCATIONS\n@ENDIF')
            with self.subTest(position=position):
                code, report = self.cli_report()
                self.assertEqual(code, 1)
                self.assertEqual(report['summary']['inspection_errors'], 1)
                a, b = report['entries']
                self.assertIn('malformed_location', a['diagnostics'][0]['reasons'])
                self.assertEqual(b['support']['inspection'], 'report_available')

    def test_bad_magic_is_malformed_not_an_unsupported_version(self):
        for data, reason in [(b'not a save header', 'malformed_or_truncated'),
                             (b'CIVILIZE\0\0\x99\0', 'unsupported_version')]:
            (self.home/'sample.scn').write_bytes(data)
            code, report = self.cli_report()
            self.assertEqual(code, 1)
            self.assertEqual(report['entries'][0]['diagnostics'][0]['reason'], reason)

    def test_begin_directive_has_an_outer_position_and_is_not_payload_syntax(self):
        valid = (self.home/'Events.txt').read_text()
        self.assertEqual(inv.scan_events('@BEGINEVENTS\n'+valid+'@ENDEVENTS', self.baseline)['diagnostics'], [])
        for source in [valid.replace('TURN', '@BEGINEVENTS\nTURN'), valid+'@BEGINEVENTS', '@BEGINEVENTS\n@BEGINEVENTS\n'+valid]:
            self.assertIn('misplaced_directive', {d['code'] for d in inv.scan_events(source, self.baseline)['diagnostics']})
        payload = valid.replace('CHANGEMONEY', 'TEXT\n@BEGINEVENTS\nENDTEXT\nCHANGEMONEY')
        self.assertNotIn('misplaced_directive', {d['code'] for d in inv.scan_events(payload, self.baseline)['diagnostics']})

    def test_scanner_stops_before_unbounded_vocabulary_or_diagnostics_are_retained(self):
        for actions, category in [('\n'.join('UNKNOWN'+str(i) for i in range(1000)), 'vocabulary'),
                                  ('UNKNOWN\n'*(inv.SCAN_ENTRY_LIMITS['diagnostics']+10), 'diagnostics')]:
            (self.home/'Events.txt').write_text('@IF\nTURN\nturn=1\n@THEN\n'+actions+'\n@ENDIF')
            code, full = self.cli_report()
            self.assertEqual(code, 1)
            macro = full['entries'][0]['macro']
            self.assertFalse(macro['scan_complete'])
            self.assertEqual(macro['overflow'], {'scope': 'entry', 'category': category})
            self.assertLessEqual(sum(len(macro[k]) for k in ['actions', 'triggers', 'logical']), inv.SCAN_ENTRY_LIMITS['vocabulary'])
            self.assertLessEqual(len(macro['diagnostics']), inv.SCAN_ENTRY_LIMITS['diagnostics']+1)
            compact = inv.compact_report(full)['entries'][0]['macro']
            self.assertLessEqual(len(compact['diagnostics']), 8)
            self.assertIn('scan_limit_exceeded', compact['diagnostic_counts'])
            self.assertFalse(compact['scan_complete'])
            self.assertLessEqual(len(compact['audited_baseline_coverage']['actions']), inv.SCAN_ENTRY_LIMITS['vocabulary'])

    def test_scanner_shares_report_budgets_and_keeps_later_entry_reports(self):
        for category, source in [
            ('diagnostics', '@IF\nTURN\n@THEN\n'+'UNKNOWN\n'*10+'@ENDIF'),
            ('vocabulary', '@IF\nTURN\n@THEN\n'+'\n'.join('UNKNOWN'+str(i) for i in range(10))+'\n@ENDIF'),
            ('lines', '\n'*20),
        ]:
            with self.subTest(category=category), patch.dict(inv.SCAN_REPORT_LIMITS, {category: 15}):
                (self.home/'Events.txt').write_text(source)
                self.manifest['entries'] = [self.entry | {'id': 'entry-'+str(i), 'first_entry_candidate': False} for i in range(3)]
                code, report = self.cli_report()
                self.assertEqual(code, 1)
                self.assertEqual(len(report['entries']), 3)
                macros = [e['macro'] for e in report['entries']]
                self.assertTrue(any(m['overflow'] == {'scope': 'report', 'category': category} for m in macros))
                for e in report['entries']:
                    self.assertIn('save_probe', e)
                    if not e['macro']['scan_complete']:
                        self.assertEqual(e['support']['inspection'], 'incomplete')
                if category == 'diagnostics':
                    self.assertLessEqual(sum(len(m['diagnostics']) for m in macros), 15+len(macros))
                if category == 'vocabulary':
                    self.assertLessEqual(sum(len(m[k]) for m in macros for k in ['triggers', 'actions', 'logical']), 15)

    def test_scanner_line_work_is_bounded_even_without_diagnostics(self):
        d = inv.scan_events('\n'*(inv.SCAN_ENTRY_LIMITS['lines']+10), self.baseline)
        self.assertEqual(d['overflow'], {'scope': 'entry', 'category': 'lines'})
        self.assertEqual(d['diagnostics'][0]['code'], 'scan_limit_exceeded')

    def test_snapshot_helpers_preserve_existing_probe_results_and_newlines(self):
        p = self.home/'sample.scn'
        self.assertEqual(totctl.walk_save(str(p)), totctl.walk_save_bytes(p.read_bytes(), str(p)))
        p = self.home/'Rules.txt'
        p.write_bytes(b'@UNITS\rName\x85ellipsis\r\nSecond, 1 ; note\n')
        a = totctl.parse_rules(str(p))
        self.assertEqual(a, totctl.parse_rules_text(p.read_bytes().decode('latin-1')))
        self.assertEqual(a['UNITS']['rows'], [['Name\x85ellipsis'], ['Second', '1']])

    def test_recognised_macros_do_not_admit_runtime_or_human_play(self):
        d = self.report(); e = d['entries'][0]
        self.assertEqual(d['summary']['inspection_errors'], 0)
        self.assertEqual(e['save_probe']['unit_count'], 0)
        self.assertEqual(e['macro']['triggers'], {'TURN': 1})
        self.assertEqual(e['macro']['actions'], {'CHANGEMONEY': 1})
        self.assertEqual(e['support']['simulation'], 'not_admitted')
        self.assertEqual(e['support']['engine_boot'], 'not_run')
        self.assertEqual(e['support']['human_entry'], 'not_implemented')
        self.assertEqual(e['support']['native_export'], 'not_admitted')
        self.assertFalse(e['art']['decoded'])

    def test_text_and_location_payloads_are_not_command_counts(self):
        src = '@IF\nTURN\nturn=1\n@THEN\nTEXT\n@IF\nGIVETECHNOLOGY\n@OR\nENDTEXT\nCREATEUNIT\nunit=Unit\nRANDOMIZE\nLOCATIONS\n1,2,0\n2,3,1\nENDLOCATIONS\n@ENDIF\n'
        d = inv.scan_events(src, self.baseline)
        self.assertEqual(d['event_blocks'], 1)
        self.assertNotIn('GIVETECHNOLOGY', d['actions'])
        self.assertEqual(d['logical'], {})
        self.assertNotIn('ENDLOCATIONS', d['actions'])
        self.assertNotIn('LOCATIONS', d['actions'])
        self.assertIn('non_surface_location', {x['code'] for x in d['diagnostics']})
        self.assertIn('RANDOMIZE', d['actions'])

    def test_compounds_skipped_technology_and_unknown_actions_remain_visible(self):
        src = '@IF\nTURN\nturn=1\n@AND\nRANDOMTURN\ndenominator=3\n@THEN\nGIVETECHNOLOGY\ntechnology=1\nreceiver=Anyone\nTAKETECHNOLOGY\ntechnology=2\n@ENDIF\n'
        d = inv.scan_events(src, self.baseline)
        codes = {x['code'] for x in d['diagnostics']}
        self.assertIn('unsupported_compound_condition', codes)
        self.assertIn('emitted_but_skipped', codes)
        self.assertIn('unimplemented_keyword', codes)
        self.assertEqual(d['actions']['GIVETECHNOLOGY'], 1)
        self.assertEqual(d['actions']['TAKETECHNOLOGY'], 1)

    def test_malformed_payloads_and_parameters_do_not_disappear(self):
        for src, code in [('@IF\nTURN\n@THEN\nTEXT\nnever closed', 'unclosed_payload'),
                          ('@IF\nTURN\nturn=1\n@THEN\nCREATEUNIT\nLOCATIONS\nx,y\n', 'malformed_location'),
                          ('@IF\nTURN\nfutureflag=1\n@THEN\n@ENDIF', 'unmodelled_parameter')]:
            with self.subTest(code=code):
                d = inv.scan_events(src, self.baseline)
                self.assertIn(code, {x['code'] for x in d['diagnostics']})

    def test_unknown_prose_is_not_copied_into_metadata(self):
        secret = 'private descriptive content '*100
        src = '@IF\n'+secret+'\n@THEN\n@ENDIF'
        d = inv.scan_events(src, self.baseline)
        self.assertNotIn('private descriptive', json.dumps(d))
        self.assertIn('<unrecognized>', d['triggers'])

    def test_missing_truncated_and_unknown_format_inputs_are_reports_not_success(self):
        for data in [b'', b'CIVILIZE\0\0', b'CIVILIZE\0\0\x32\0', b'CIVILIZE\0\0\x99\0']:
            with self.subTest(data=data):
                (self.home/'sample.scn').write_bytes(data)
                d = self.report()
                self.assertEqual(d['summary']['inspection_errors'], 1)
                self.assertEqual(d['entries'][0]['save_probe']['status'], 'failed')
        (self.home/'sample.scn').unlink()
        d = self.report()
        self.assertEqual(d['entries'][0]['diagnostics'][0]['code'], 'source_unavailable')

    def test_manifest_paths_lists_and_ids_are_bounded(self):
        for path in ['../outside', '/absolute', 'a/../b', 'a//b', 'a\\b', 'C:/x']:
            with self.subTest(path=path):
                m = self.manifest | {'entries': [self.entry | {'save': path}]}
                with self.assertRaises(ValueError): inv.load_manifest(json.dumps(m).encode())
        for m in [[], {}, self.manifest | {'schema':True}, self.manifest | {'entries':[self.entry | {'id':1}]}, self.manifest | {'entries': [self.entry, self.entry]},
                  self.manifest | {'entries': [self.entry]*65},
                  self.manifest | {'entries': [self.entry | {'lua':'not-list'}]}]:
            with self.assertRaises(ValueError): inv.load_manifest(json.dumps(m).encode())

    def test_symlink_escape_and_oversized_source_are_rejected_without_writes(self):
        outside = self.root/'outside'; outside.write_bytes(b'private')
        (self.home/'escape').symlink_to(outside)
        with self.assertRaises(ValueError): inv.snapshot(self.home, 'escape')
        self.assertEqual(outside.read_bytes(), b'private')
        with (self.home/'large').open('wb') as stream: stream.truncate(inv.MAX_FILE+1)
        with self.assertRaises(ValueError): inv.snapshot(self.home, 'large')

    def test_symlink_loops_are_structured_input_errors(self):
        (self.home/'a').symlink_to('b')
        (self.home/'b').symlink_to('a')
        self.entry['save'] = 'a'
        d = self.report()
        self.assertEqual(d['summary']['inspection_errors'], 1)
        self.assertEqual(d['entries'][0]['diagnostics'][0]['code'], 'source_unavailable')

    def test_art_precedence_and_ambiguity_do_not_silently_choose_another_root(self):
        p = self.home/'Scenario'; p.mkdir()
        (p/'Terrain1.GIF').write_bytes(b'local')
        (self.home/'terrain1.bmp').write_bytes(b'base')
        d = inv.inventory_art(self.home, ['Scenario','.'])
        self.assertEqual(d['files']['terrain1']['path'], 'Scenario/Terrain1.GIF')
        (p/'Terrain1.BMP').write_bytes(b'one'); (p/'terrain1.bmp').write_bytes(b'two')
        if len(list(p.glob('*'))) < 3: self.skipTest('case-insensitive filesystem')
        d = inv.inventory_art(self.home, ['Scenario','.'])
        self.assertNotIn('terrain1', d['files'])
        self.assertIn('ambiguous_art', {x['code'] for x in d['diagnostics']})

    def test_malformed_engine_baseline_is_configuration_failure(self):
        for b in [[], {}, self.baseline | {'source_sha256': {}},
                  {k: v for k, v in self.baseline.items() if k != 'meaning'},
                  self.baseline | {'meaning': None}, self.baseline | {'meaning': 1},
                  self.baseline | {'schema': True}, self.baseline | {'triggers': []}]:
            with self.assertRaises(ValueError): inv.load_baseline(json.dumps(b).encode())
        manifest = self.root/'manifest.json'; manifest.write_text(json.dumps(self.manifest))
        broken = self.root/'baseline.json'
        broken.write_text(json.dumps({k: v for k, v in self.baseline.items() if k != 'meaning'}))
        output = self.root/'missing.json'
        original = inv.read_document
        def read(path):
            return broken.read_bytes() if path.name == 'scenario-engine-baseline.json' else original(path)
        with patch.object(inv, 'read_document', side_effect=read), redirect_stderr(StringIO()) as errors:
            self.assertEqual(inv.main(['--home', str(self.home), '--manifest', str(manifest), '--out', str(output)]), 2)
        self.assertFalse(output.exists())
        self.assertIn('meaning must be a string', errors.getvalue())

    def test_unframed_record_count_is_not_a_valid_profile_candidate(self):
        data=bytearray((self.home/'sample.scn').read_bytes())
        struct.pack_into('<h',data,698,-1)
        (self.home/'sample.scn').write_bytes(data)
        d=self.report()
        self.assertEqual(d['entries'][0]['save_probe']['status'],'partial_layout')
        self.assertFalse(d['entries'][0]['first_profile_structure']['classic_counts_probed'])

    def test_art_root_escape_fails_inspection_without_reading_outside(self):
        outside=self.root/'outside-dir';outside.mkdir()
        (outside/'terrain1.bmp').write_bytes(b'private')
        (self.home/'external').symlink_to(outside,target_is_directory=True)
        self.entry['art_roots']=['external']
        d=self.report()
        self.assertEqual(d['summary']['inspection_errors'],1)
        self.assertEqual(d['entries'][0]['art']['files'],{})
        self.assertIn('resource_root_escape',{x['code'] for x in d['entries'][0]['diagnostics']})

    def test_source_audit_change_invalidates_import_status(self):
        self.baseline['source_sha256']['crates/engine/src/events.rs'] = '0'*64
        d = self.report()
        self.assertFalse(d['engine_audit']['matches_current_source'])
        self.assertEqual(d['entries'][0]['support']['human_entry'], 'audit_required')
        self.assertIn('engine_source_audit_stale', d['entries'][0]['blockers'])

    def test_lua_and_multiple_maps_are_declared_not_executed_or_inferred(self):
        path=self.home/'events.lua'; path.write_text('error("MUST NOT RUN")')
        self.entry.update(lua=['events.lua'], declared_features=['multiple_maps','lua'])
        d=self.report()['entries'][0]
        self.assertFalse(d['lua']['executed'])
        self.assertIn('lua_execution_not_implemented', d['blockers'])
        self.assertIn('multiple_map_simulation_not_implemented', d['blockers'])
        self.assertNotIn('map_count', d['save_probe'])
        self.assertEqual(path.read_text(), 'error("MUST NOT RUN")')

    def test_report_is_deterministic_and_has_no_absolute_installation_path(self):
        a = json.dumps(self.report(), sort_keys=True)
        self.assertEqual(a, json.dumps(self.report(), sort_keys=True))
        self.assertNotIn(str(self.home), a)

    def test_compact_view_bounds_examples_without_losing_counts_or_admission_state(self):
        (self.home/'Events.txt').write_text('@IF\nTURN\nturn=1\n@THEN\n'+'UNSUPPORTED\n'*30+'@ENDIF\n')
        full = self.report(); compact = inv.compact_report(full)
        macro = compact['entries'][0]['macro']
        self.assertEqual(compact['view'], 'compact')
        self.assertEqual(len(macro['diagnostics']), 8)
        self.assertEqual(macro['diagnostics_total'], 30)
        self.assertEqual(macro['diagnostic_counts'], {'unimplemented_keyword':30})
        self.assertTrue(macro['diagnostics_truncated'])
        self.assertEqual(compact['summary'], full['summary'])
        self.assertEqual(compact['entries'][0]['support'], full['entries'][0]['support'])
        self.assertEqual(compact['full_report_sha256'], inv.digest((json.dumps(full,indent=2,sort_keys=True)+'\n').encode()))
        self.assertEqual(len(full['entries'][0]['macro']['diagnostics']), 30)

    def test_cli_require_and_exclusive_output_do_not_overwrite_inputs(self):
        manifest = self.root/'manifest.json'; manifest.write_text(json.dumps(self.manifest))
        output = self.root/'report.json'
        args=['--home',str(self.home),'--manifest',str(manifest),'--out',str(output)]
        with redirect_stderr(StringIO()):
            self.assertEqual(inv.main(args+['--require','simulation']), 1)
            before=output.read_bytes()
            self.assertEqual(inv.main(args), 2)
            self.assertEqual(output.read_bytes(), before)
            self.assertEqual(inv.main(args[:-1]+[str(self.home/'sample.scn')]), 2)
        self.assertEqual((self.home/'sample.scn').read_bytes()[:8], b'CIVILIZE')


if __name__ == '__main__':
    unittest.main()
