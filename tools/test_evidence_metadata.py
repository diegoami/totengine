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
    def test_city_phase_memory_vectors_match_native_report(self):
        report = json.loads((ROOT / 'docs/evidence/import-city-memory.json').read_text())
        vectors = (ROOT / 'docs/evidence/import-city-memory-vectors.txt').read_bytes()
        self.assertEqual(hashlib.sha256(vectors).hexdigest(),report['vectors_sha256'])
        rows = [line for line in vectors.decode().splitlines()
                if line and not line.startswith('#')]
        self.assertEqual(len(rows),report['cases'])
        self.assertEqual(len(rows),len(set(rows)))

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
