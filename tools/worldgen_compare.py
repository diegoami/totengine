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
SPATIAL = {"neighbor_edges", "coast_edges", "same_terrain_edges", "largest_land_tiles", "land_components"}


def load_rows(path):
    with path.open() as stream:
        reader = csv.DictReader(stream, delimiter="\t")
        expected = {"source", "logical_width", "height", "tiles", "rivers", *TERRAINS}
        columns = set(reader.fieldnames or [])
        if columns not in (expected, expected | SPATIAL) or len(reader.fieldnames) != len(columns):
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
            if SPATIAL <= columns and (
                    values["neighbor_edges"] <= 0
                    or values["coast_edges"] + values["same_terrain_edges"] > values["neighbor_edges"]
                    or values["largest_land_tiles"] > values["land"]
                    or values["land_components"] > values["land"]):
                raise ValueError(f"inconsistent spatial counts for {label}")
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
    if all(SPATIAL <= row.keys() for row in rows):
        result["spatial"] = {}
        for metric, denominator in (("coast_edges", "neighbor_edges"),
                                    ("same_terrain_edges", "neighbor_edges"),
                                    ("largest_land_tiles", "land")):
            values = [100 * row[metric] / max(1, row[denominator]) for row in rows]
            result["spatial"][metric + "_percent"] = {
                "mean": round(statistics.mean(values), 4),
                "sample_sd": round(statistics.stdev(values), 4) if len(values) > 1 else None,
                "min": round(min(values), 4), "max": round(max(values), 4),
            }
        result["spatial"]["land_components_mean"] = round(statistics.mean(r["land_components"] for r in rows), 4)
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


def coverage_gate(result, protocol):
    """Predeclared practical mean-coverage budgets, not a distribution test."""
    gate = protocol["gate"]
    original = result["original_reference"]
    engine = result["engine_holdout"]
    checks = {
        "original_sample_count": original["maps"] >= gate["minimum_original_maps"],
        "engine_sample_count": engine["maps"] >= gate["minimum_engine_holdout_maps"],
        "dimensions": result["dimensions"] == {
            k: protocol["original_settings"][k] for k in ("logical_width", "height")},
    }
    # The engine holdout must be precisely the declared seeds, without alias
    # labels reweighting one seed. Original source/settings are validated by
    # the capture manifest separately; counts cannot prove their provenance.
    lo, hi = protocol["engine_settings"]["holdout_seeds"]
    matches = [re.fullmatch(r"engine-v(\d+)-(\d+)", s) for s in engine["sources"]]
    checks["declared_holdout"] = (all(matches) and len(matches) == hi - lo + 1
                                  and {int(m[2]) for m in matches} == set(range(lo, hi + 1))
                                  and {int(m[1]) for m in matches} == {protocol["algorithm_version"]})
    checks["land_setting"] = (engine["percent_of_real_tiles"]["land"]["min"]
                              == engine["percent_of_real_tiles"]["land"]["max"]
                              == protocol["engine_settings"]["land_percent"])
    measurements = {}
    for metric in METRICS:
        if metric == "ocean":
            continue  # complement of the land check
        reference = original["percent_of_real_tiles"][metric]["mean"]
        delta = abs(engine["percent_of_real_tiles"][metric]["mean"] - reference)
        if metric == "land":
            budget = gate["land_mean_absolute_tolerance_percentage_points"]
        elif metric == "rivers":
            budget = gate["river_mean_absolute_tolerance_percentage_points"]
        else:
            budget = max(gate["biome_minimum_absolute_tolerance_percentage_points"],
                         reference * gate["biome_relative_tolerance"])
        measurements[metric] = dict(absolute_delta_pp=round(delta, 4), budget_pp=round(budget, 4),
                                    passed=delta <= budget)
    return dict(passed=all(checks.values()) and all(m["passed"] for m in measurements.values()),
                checks=checks, measurements=measurements,
                scope="Declared mean-coverage budgets only; provenance and render checks are separate.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("metrics", type=Path)
    parser.add_argument("--reference", action="append", required=True,
                        help="matched-settings original source label (repeatable)")
    parser.add_argument("--calibration-seeds", type=int, default=16)
    parser.add_argument("--protocol", type=Path, help="apply a predeclared mean-coverage gate")
    args = parser.parse_args()
    try:
        if args.calibration_seeds < 1:
            raise ValueError("calibration-seeds must be positive")
        result = report(load_rows(args.metrics), args.reference, args.calibration_seeds)
        if args.protocol:
            result["status"] = "comparison measurements; coverage gate and provenance/render checks are separate"
            result["coverage_gate"] = coverage_gate(result, json.loads(args.protocol.read_text()))
            result["protocol_sha256"] = hashlib.sha256(args.protocol.read_bytes()).hexdigest()
    except (ValueError, KeyError, TypeError) as error:
        parser.error(str(error))
    result["metrics_sha256"] = hashlib.sha256(args.metrics.read_bytes()).hexdigest()
    print(json.dumps(result, indent=2) + "\n", end="")


if __name__ == "__main__":
    main()
