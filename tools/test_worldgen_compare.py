"""Guard the numeric evidence boundary against invalid/reweighted cohorts."""
import csv
from pathlib import Path
import tempfile
import unittest

import worldgen_compare as compare


class ComparisonTests(unittest.TestCase):
    def write_rows(self, directory, rows):
        path = Path(directory) / "metrics.tsv"
        fields = ("source", "logical_width", "height", "tiles", "rivers", *compare.TERRAINS)
        with path.open("w") as stream:
            writer = csv.DictWriter(stream, fieldnames=fields, delimiter="\t")
            writer.writeheader()
            writer.writerows(rows)
        return path

    def row(self, source):
        return dict(source=source, logical_width=32, height=16, tiles=256,
                    rivers=3, **{t: 0 for t in compare.TERRAINS}) | {"plains": 100, "ocean": 156}

    def test_controls_and_holdout_are_not_pooled_into_calibration(self):
        rows = [self.row(s) for s in ("original", "control", "engine-v2-0", "engine-v2-16")]
        rows[-1]["plains"], rows[-1]["ocean"] = 128, 128
        with tempfile.TemporaryDirectory() as directory:
            result = compare.report(compare.load_rows(self.write_rows(directory, rows)), ["original"], 16)
        self.assertEqual(result["controls"][0]["source"], "control")
        self.assertEqual(result["engine_calibration"]["percent_of_real_tiles"]["land"]["mean"], 39.0625)
        self.assertEqual(result["engine_holdout"]["percent_of_real_tiles"]["land"]["mean"], 50)

    def test_invalid_counts_and_duplicate_sources_fail(self):
        for change in ({"tiles": 255}, {"plains": 101}, {"rivers": 101}, {"ocean": -1}):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as directory:
                path = self.write_rows(directory, [self.row("original") | change])
                with self.assertRaises(ValueError):
                    compare.load_rows(path)
        with tempfile.TemporaryDirectory() as directory:
            path = self.write_rows(directory, [self.row("original")] * 2)
            with self.assertRaisesRegex(ValueError, "duplicate"):
                compare.load_rows(path)

    def test_missing_reference_or_holdout_fails(self):
        rows = [self.row(s) for s in ("original", "engine-v2-0")]
        with tempfile.TemporaryDirectory() as directory:
            rows = compare.load_rows(self.write_rows(directory, rows))
            for reference in (["missing"], ["original"]):
                with self.subTest(reference=reference), self.assertRaises(ValueError):
                    compare.report(rows, reference, 16)


if __name__ == "__main__":
    unittest.main()
