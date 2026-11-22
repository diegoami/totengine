#!/usr/bin/env python3
"""Reproduce the first Original dependency capture and shared-atlas check.

Runs Rust adapter/atlas tests and independently hashes the selected private
files with Python. No native game code/UI executes and no session is admitted.
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
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    home = Path(os.environ["CIVTOT_HOME"]).resolve()
    known = json.loads((root / "docs/evidence/import-knowledge.json").read_text())["source"]
    cargo = shutil.which("cargo") or str(Path.home() / ".cargo/bin/cargo")
    with tempfile.TemporaryDirectory(prefix="totengine-dependencies-") as folder:
        report_path = Path(folder) / "capture.json"
        atlas_path = Path(folder) / "atlas.json"
        env = os.environ.copy()
        env["TOTENGINE_DEPENDENCIES_EVIDENCE"] = str(report_path)
        env["TOTENGINE_ORIGINAL_ATLAS_EVIDENCE"] = str(atlas_path)
        for test in [
            "resources::original::tests::installed_original_capture_retains_the_complete_reference_dependency_set",
            "client::tests::installed_original_dependency_capture_builds_the_same_shared_atlas",
        ]:
            subprocess.run([cargo, "test", "-p", "totui", "--lib", test, "--", "--exact"],
                           cwd=root, env=env, check=True)
        report = json.loads(report_path.read_text())
        report["atlas"] = json.loads(atlas_path.read_text())
    for source in report["inputs"]:
        path = (home / source["path"]).resolve()
        path.relative_to(home)
        data = path.read_bytes()
        assert len(data) == source["length"], source["path"]
        assert hashlib.sha256(data).hexdigest() == source["sha256"], source["path"]
    source = next(x for x in report["inputs"] if x["role"] == "scenario")
    assert source["sha256"] == known["sha256"], "pinned Original fixture differs"
    report["source"] = {"path": source["path"], "sha256": source["sha256"]}
    report["method"] = __doc__.strip()
    report["independent_python_hashes_match"] = True
    report["limits"] = (
        "Only the known twelve-file Original dependency set and an owned SAV capture. "
        "No root/GIF/translation/alternate-art/custom/Lua dependency admission. "
        "The CLI and both clients use this capture for read-only Original SAV inspection. "
        "Experimental checkpoints can separately preserve their captured input in private storage. "
        "This dependency capture supplies neither a playable profile nor a playable entry. "
        "Pending-state and knowledge-continuation evidence and an admitted initializer are still required."
    )
    args.out.write_text(json.dumps(report, indent=2) + "\n")
    print(f"Original capture: {len(report['inputs']) - 1} dependencies plus save; "
          f"{report['atlas']['sprite_count']} shared sprites; no runtime admission")


if __name__ == "__main__":
    main()
