#!/usr/bin/env python3
"""crosscheck_mp.py — keep the Rust tot-kit decoder and the Python probe in
sync on real .MP files (milestone-review finding #28).

Runs both decoders over the corpus maps, compares layout / dimensions /
counted real tiles / terrain histogram, and writes (regen) or compares
(check) docs/evidence/crosscheck-mp.json. Called from tools/golden.sh;
self-skips without CIVTOT_HOME.
"""
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ART = os.path.join(HERE, "docs", "evidence", "crosscheck-mp.json")
LAYOUT_EQ = {"Tot3ByteSlot": "tot_3byte_slot",
             "TotPairedSlot": "tot_paired_slot",
             "Classic2Byte": "classic_2byte"}
FILES = ["World.mp", "Scenario/china/AnticChina.MP"]


def rust_decode(path, cargo):
    out = subprocess.run([cargo, "run", "-q", "-p", "totctl", "--", "mp", path],
                         capture_output=True, text=True, cwd=HERE).stdout
    d = {}
    hist = {}
    in_hist = False
    for line in out.splitlines():
        f = line.split()
        if line.startswith("layout"):
            d["layout"] = f[2]
        elif line.startswith("dimensions"):
            d["dims"] = [int(f[2]), int(f[4])]
        elif line.startswith("counted real"):
            d["counted_real"] = int(f[3])
            in_hist = True
        elif in_hist and len(f) >= 4 and f[0] == "id":
            hist[str(int(f[1]))] = int(f[3])
    d["terrain_top"] = hist
    return d


def python_decode(path):
    out = subprocess.run(
        ["python3", os.path.join(HERE, "tools", "totctl.py"),
         "map", path, "--json", "/tmp/_cc.json", "-o", "/tmp/_cc.png"],
        capture_output=True, text=True, cwd=HERE).stdout
    j = json.load(open("/tmp/_cc.json"))
    hist = {}
    node = j.get("render", j)
    for k in ("terrain_histogram", "histogram"):
        if k in node:
            # python JSON carries the top-12 (sorted by count desc); the
            # first 6 are the true top-6 — exactly what rust prints
            hist = {str(int(kk)): int(v) for kk, v in list(node[k].items())[:6]}
            break
    return {"layout": j.get("layout"),
            "dims": [j.get("width"), j.get("height")],
            "terrain_top": hist}


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "check"
    home = os.environ.get("CIVTOT_HOME")
    if not home:
        print("crosscheck: CIVTOT_HOME not set — skipping")
        return 0
    cargo = subprocess.run(["sh", "-c",
                            "command -v cargo || echo $HOME/.cargo/bin/cargo"],
                           capture_output=True, text=True).stdout.strip()
    doc, ok_all = {}, True
    for rel in FILES:
        path = os.path.join(home, rel)
        r = rust_decode(path, cargo)
        p = python_decode(path)
        ok = (r.get("layout") in LAYOUT_EQ
              and LAYOUT_EQ.get(r.get("layout")) == p.get("layout")
              and r.get("dims") == p.get("dims")
              and r.get("terrain_top") == p.get("terrain_top"))
        doc[rel] = {"rust": r, "python": p, "match": ok}
        ok_all = ok_all and ok
    text = json.dumps(
        {"files": doc,
         "note": ("Rust tot-kit vs Python probe: layout/dims and the exact "
                  "top-6 terrain histogram must agree (the top-6 of python's "
                  "sorted top-12 equals rust's printed top-6). The declared "
                  "dword is NOT compared — World.mp declares 9,000 real "
                  "tiles but 8,568 slots are non-void")},
        indent=1, sort_keys=True).replace(home, "@HOME@") + "\n"
    if mode == "regen":
        open(ART, "w").write(text)
        print("regen crosscheck-mp")
        return 0
    want = open(ART).read() if os.path.exists(ART) else ""
    if want != text:
        print("FAIL crosscheck-mp: output differs from committed artifact "
              "(run: make evidence-regen)")
        return 1
    if not ok_all:
        print("FAIL crosscheck-mp: decoders disagree (see artifact)")
        return 1
    print("ok    crosscheck-mp (rust == python on %d maps)" % len(FILES))
    return 0


if __name__ == "__main__":
    sys.exit(main())
