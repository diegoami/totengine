"""Guard the numeric evidence boundary against invalid/reweighted cohorts."""
import csv
import json
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

    def test_coverage_gate_rejects_small_reference_and_shifted_biomes(self):
        protocol = json.loads((Path(__file__).resolve().parents[1] / "docs/evidence/worldgen-comparison-protocol.json").read_text())
        sources = [f"original-{i}" for i in range(10)]
        rows = [self.row(s) for s in sources + [f"engine-v2-{i}" for i in range(80)]]
        # Give all cohorts the protocol's 47% land setting; construct a fully
        # passing baseline so later failures cannot hide behind a count gate.
        for row in rows:
            row.update(logical_width=80, height=50, tiles=2000, plains=940, ocean=1060)
        with tempfile.TemporaryDirectory() as directory:
            loaded = compare.load_rows(self.write_rows(directory, rows))
        result = compare.report(loaded, sources, 16)
        self.assertTrue(compare.coverage_gate(result, protocol)["passed"])
        result["engine_holdout"]["percent_of_real_tiles"]["forest"]["mean"] = 5
        self.assertFalse(compare.coverage_gate(result, protocol)["passed"])
        result["engine_holdout"]["percent_of_real_tiles"]["forest"]["mean"] = 0
        result["original_reference"]["maps"] = 2
        self.assertFalse(compare.coverage_gate(result, protocol)["passed"])

    def test_spatial_diagnostics_reject_impossible_edges(self):
        row = self.row("original") | dict(neighbor_edges=100, coast_edges=60,
                                          same_terrain_edges=70, largest_land_tiles=90, land_components=2)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "spatial.tsv"
            with path.open("w") as stream:
                writer = csv.DictWriter(stream, fieldnames=row, delimiter="\t")
                writer.writeheader()
                writer.writerow(row)
            with self.assertRaisesRegex(ValueError, "spatial"):
                compare.load_rows(path)


if __name__ == "__main__":
    unittest.main()
