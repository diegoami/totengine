"""Evidence metadata must match the files it describes (#278 regression).

The campaign golden metadata (`docs/evidence/generated-campaign-v1.json`)
records the SHA-256 of its golden checkpoint file; a refresh that forgets the
hash (as in the first v0.19.0 candidate) is caught here.
"""
import hashlib
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


class GoldenMetadata(unittest.TestCase):
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
