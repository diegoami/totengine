"""Guard inspection/admission separation and private-input safety."""
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
import json
from pathlib import Path
import struct
import tempfile
import unittest

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
                  self.baseline | {'schema': True}, self.baseline | {'triggers': []}]:
            with self.assertRaises(ValueError): inv.load_baseline(json.dumps(b).encode())

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
