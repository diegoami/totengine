#!/usr/bin/env python3
"""Summarize numeric worldgen_metrics TSV; never reads game assets.

The first 16 engine seeds are calibration data. Later seeds are a holdout
against seed-specific tuning, not an independent original-game reference.
This report deliberately does not award statistical-parity acceptance.
"""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import re
import statistics


TERRAINS = (
    "desert", "plains", "grassland", "forest", "hills", "mountains",
    "tundra", "glacier", "swamp", "jungle", "ocean",
)
METRICS = ("land", *TERRAINS, "rivers")


def load_rows(path):
    with path.open() as stream:
        reader = csv.DictReader(stream, delimiter="\t")
        expected = {"source", "logical_width", "height", "tiles", "rivers", *TERRAINS}
        if set(reader.fieldnames or []) != expected or len(reader.fieldnames) != len(expected):
            raise ValueError("unexpected metrics columns")
        rows = []
        labels = set()
        for row in reader:
            label = row["source"]
            if label in labels:
                raise ValueError(f"duplicate source {label}")
            labels.add(label)
            values = {k: int(v) for k, v in row.items() if k != "source"}
            n, width, height = (values[k] for k in ("tiles", "logical_width", "height"))
            if (n <= 0 or width <= 0 or height <= 0 or width % 2
                    or n != width // 2 * height or min(values.values()) < 0
                    or sum(values[t] for t in TERRAINS) != n
                    or values["rivers"] > n - values["ocean"]):
                raise ValueError(f"inconsistent tile counts for {label}")
            values["land"] = n - values["ocean"]
            rows.append({"source": label, **values})
    return rows


def cohort(rows):
    if not rows:
        raise ValueError("empty cohort")
    result = {"sources": [r["source"] for r in rows], "maps": len(rows)}
    # Equal weight per map, including if a future cohort uses different sizes.
    result["percent_of_real_tiles"] = {}
    for metric in METRICS:
        values = [100 * row[metric] / row["tiles"] for row in rows]
        result["percent_of_real_tiles"][metric] = {
            "mean": round(statistics.mean(values), 4),
            "sample_sd": round(statistics.stdev(values), 4) if len(values) > 1 else None,
            "min": round(min(values), 4),
            "max": round(max(values), 4),
        }
    return result


def report(rows, reference_labels, calibration_seeds):
    reference = [r for r in rows if r["source"] in reference_labels]
    if {r["source"] for r in reference} != set(reference_labels):
        raise ValueError("missing reference labels")
    calibration, holdout, versions = [], [], set()
    for row in rows:
        match = re.fullmatch(r"engine-v(\d+)-(\d+)", row["source"])
        if match:
            versions.add(int(match[1]))
            (calibration if int(match[2]) < calibration_seeds else holdout).append(row)
    if len(versions) != 1:
        raise ValueError("expected one engine algorithm version")
    # Comparisons require matched dimensions; controls are retained separately.
    dimensions = {(r["logical_width"], r["height"]) for r in reference + calibration + holdout}
    if len(dimensions) != 1:
        raise ValueError("comparison cohorts have different map dimensions")
    original = cohort(reference)
    result = {
        "status": "prototype; broader matched-settings original cohort still required",
        "algorithm": f"engine-designed v{versions.pop()}, not original generator",
        "dimensions": dict(zip(("logical_width", "height"), dimensions.pop())),
        "original_reference": original,
        "engine_calibration": cohort(calibration),
        "engine_holdout": cohort(holdout),
        "controls": [r for r in rows if r not in reference + calibration + holdout],
        "limits": [
            "Reference settings and provenance must be checked in the source manifest.",
            "Engine holdout seeds test robustness of calibration, not original-generator parity.",
            "No statistical acceptance awarded; the report describes counts, not a release gate.",
            "Biome counts do not measure spatial coherence or establish original-game MP import.",
        ],
    }
    for key in ("engine_calibration", "engine_holdout"):
        result[key]["mean_delta_from_reference_percentage_points"] = {
            metric: round(result[key]["percent_of_real_tiles"][metric]["mean"]
                          - original["percent_of_real_tiles"][metric]["mean"], 4)
            for metric in METRICS
        }
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("metrics", type=Path)
    parser.add_argument("--reference", action="append", required=True,
                        help="matched-settings original source label (repeatable)")
    parser.add_argument("--calibration-seeds", type=int, default=16)
    args = parser.parse_args()
    try:
        if args.calibration_seeds < 1:
            raise ValueError("calibration-seeds must be positive")
        result = report(load_rows(args.metrics), args.reference, args.calibration_seeds)
    except (ValueError, KeyError, TypeError) as error:
        parser.error(str(error))
    result["metrics_sha256"] = hashlib.sha256(args.metrics.read_bytes()).hexdigest()
    print(json.dumps(result, indent=2) + "\n", end="")


if __name__ == "__main__":
    main()
