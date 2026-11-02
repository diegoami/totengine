#!/usr/bin/env python3
"""crosscheck_rules.py — keep the Rust tot-kit Rules parser and the Python
probe in sync on the real Rules.txt (T1-1, mirroring crosscheck_mp.py).

Compares section names/order, per-section row counts, terrain semantics,
unit count + domain histogram, governments, and the COSMIC road multiplier.
Writes (regen) or compares (check) docs/evidence/crosscheck-rules.json.
"""
import json
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ART = os.path.join(HERE, "docs", "evidence", "crosscheck-rules.json")


def rust_decode(path, cargo):
    out = subprocess.run([cargo, "run", "-q", "-p", "totctl", "--",
                          "rules", path],
                         capture_output=True, text=True, cwd=HERE).stdout
    d = {}
    for line in out.splitlines():
        key, _, val = line.partition(":")
        d[key.strip()] = val.strip()
    return {
        "sections": d.get("sections", "").split(","),
        "row_counts": {k: int(v) for k, v in
                       re.findall(r"(\w+)=(\d+)", d.get("row counts", ""))},
        "semantic_ok": d.get("semantic ok") == "true",
        "terrain0_10": d.get("terrain 0/10"),
        "units": int(d.get("units", "0") or 0),
        "unit_domains": d.get("unit domains"),
        "governments": d.get("governments", "").split(", "),
        "transport_relationships": d.get("transport rel", ""),
        "cosmic_road": d.get("cosmic road"),
    }


def python_decode(path):
    sys.path.insert(0, os.path.join(HERE, "tools"))
    import importlib.util
    spec = importlib.util.spec_from_file_location(
        "t", os.path.join(HERE, "tools", "totctl.py"))
    t = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(t)
    secs = t.parse_rules(path)
    summ = t.rules_summary(secs)
    domains = [0, 0, 0]
    for r in secs.get("UNITS", {}).get("rows", []):
        try:
            domains[min(int(r[2]), 2)] += 1
        except (ValueError, IndexError):
            pass
    sc = summ["semantic_checks"]
    terr = [r[0] for r in secs.get("TERRAIN", {}).get("rows", [])][:11]
    return {
        "sections": [s for s in summ["sections"]],
        "row_counts": {k: v["row_count"] for k, v in summ["sections"].items()},
        "semantic_ok": bool(sc.get("pass")),
        "terrain0_10": f"{terr[0]} / {terr[10]}" if len(terr) > 10 else "?",
        "units": summ["sections"].get("UNITS", {}).get("row_count", 0),
        "unit_domains": "/".join(str(c) for c in domains),
        "governments": summ["sections"].get("GOVERNMENTS", {})
                        .get("first_column", [])[:7],
        "transport_relationships": str(
            [(int(r[0]), int(r[1]))
             for r in secs.get("MAP_TRANSPORT_RELATIONSHIPS", {})
             .get("rows", []) if len(r) > 1]),
        "cosmic_road": summ["sections"].get("COSMIC", {})
                        .get("values", {}).get("Road movement multiplier"),
    }


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "check"
    home = os.environ.get("CIVTOT_HOME")
    if not home:
        print("crosscheck-rules: CIVTOT_HOME not set — skipping")
        return 0
    path = os.path.join(home, "Rules.txt")
    cargo = subprocess.run(["sh", "-c",
                            "command -v cargo || echo $HOME/.cargo/bin/cargo"],
                           capture_output=True, text=True).stdout.strip()
    r = rust_decode(path, cargo)
    p = python_decode(path)
    ok = (r["sections"] == p["sections"]
          and r["row_counts"] == p["row_counts"]
          and r["semantic_ok"] == p["semantic_ok"] is True
          and r["terrain0_10"] == p["terrain0_10"] == "Desert / Ocean"
          and r["units"] == p["units"]
          and r["unit_domains"] == p["unit_domains"]
          and r["governments"] == p["governments"]
          and r["cosmic_road"] == p["cosmic_road"] == "3")
    doc = {"rust": r, "python": p, "match": ok,
           "note": ("Rust tot-kit vs Python probe on Rules.txt: section "
                    "order/row counts, terrain semantics (Desert=0/Ocean=10), "
                    "unit count + domain histogram, governments, COSMIC road "
                    "multiplier must all agree")}
    text = json.dumps(doc, indent=1, sort_keys=True).replace(home, "@HOME@") + "\n"
    if mode == "regen":
        open(ART, "w").write(text)
        print("regen crosscheck-rules")
        return 0
    want = open(ART).read() if os.path.exists(ART) else ""
    if want != text:
        print("FAIL crosscheck-rules: output differs from committed artifact "
              "(run: make evidence-regen)")
        return 1
    if not ok:
        print("FAIL crosscheck-rules: decoders disagree (see artifact)")
        return 1
    print("ok    crosscheck-rules (rust == python on Rules.txt)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
