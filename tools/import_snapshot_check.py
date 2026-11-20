#!/usr/bin/env python3
"""Measure classic snapshot records and current engine-only normalization.

Runs installed-input Rust checks; no native game execution or admission.
Original bytes remain private. Retained native reader/framing evidence is
separate from this conservative record policy and constructor measurement.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    home = Path(os.environ["CIVTOT_HOME"])
    pinned = json.loads((root / "docs/evidence/import-knowledge.json").read_text())["source"]
    sha = hashlib.sha256((home / pinned["path"]).read_bytes()).hexdigest()
    if sha != pinned["sha256"]:
        raise SystemExit("Pinned Original positive fixture differs")
    cargo = shutil.which("cargo") or str(Path.home() / ".cargo/bin/cargo")
    with tempfile.TemporaryDirectory(prefix="totengine-snapshot-") as folder:
        result = Path(folder) / "report.json"
        env = os.environ.copy()
        env["TOTENGINE_SNAPSHOT_EVIDENCE"] = str(result)
        subprocess.run(
            [cargo, "test", "-p", "engine", "--lib",
             "human::saved_view::tests::installed_snapshot_records_and_normalization_are_measured",
             "--", "--exact"], cwd=root, env=env, check=True,
        )
        report = json.loads(result.read_text())
    report["source"] = {"path": pinned["path"], "sha256": sha}
    report["inputs"] = [
        {"path": path, "sha256": hashlib.sha256((home / path).read_bytes()).hexdigest()}
        for path in ["Original/Rules.txt", "Original/Events.txt"]
    ]
    report["method"] = __doc__.strip()
    report["preserved_fields"] = [
        "unit identity/owner/type/coordinates/damage/orders/home/veteran/stack order",
        "city identity/coordinates/owner/size/food/shields/production/workers/specialists",
        "city output caches/flags/buildings/knowledge masks/remembered sizes",
        "treasury/rates/governments/research/techs/diplomacy/wonders/city counts",
        "map improvements/exploration and all seven remembered-improvement arrays",
        "all remaining booted world fields, checked by full Debug equality plus checksum",
    ]
    report["limits"] = (
        "Read-only record check, not called by current session factories or restore. "
        "Tail bytes and native pending-choice state remain unchecked. No stock "
        "dependency closure, faithful knowledge continuation, admitted initializer, "
        "native AI/RNG continuation, native UI or both-client entry is established."
    )
    args.out.write_text(json.dumps(report, indent=2) + "\n")
    print(f"snapshot: {report['live_units']} units, {report['live_cities']} cities; "
          f"movement-only world difference; {report['trailing_bytes_unchecked']} tail bytes unchecked")


if __name__ == "__main__":
    main()
