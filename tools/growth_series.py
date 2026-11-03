#!/usr/bin/env python3
"""growth_series.py — derive the T1-4 city growth series from oracle saves.

    python3 tools/growth_series.py <dir-of-turnNN.sav> -o docs/evidence/t1-4-growth-series.tsv

Reads every `turnNN.sav` in the directory (a peaceful standard game driven
turn by turn through the original, e.g. the std1 lab of evidence §T1-4)
with the game-framing walker (evidence §Record framing FIXED) and writes,
per save: the turn, and per city its size (u8@11), food box (u16@28),
gross food (u8@82) and supported settlers (units homed there with
type 0 = Settlers or 1 = Engineers, the standard @UNITS rows). Derived
numbers only. `crates/engine/src/cities.rs` replays the series.
"""
import argparse
import glob
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import totctl  # noqa: E402


def series(path):
    w = totctl.walk_save(path)
    d = open(path, "rb").read()
    s0, n_units = w["units"]["region"][0], w["units"]["count"]
    c0, n_cities = w["cities"]["offset"], w["cities"]["count"]
    homes = [d[s0 + 40 * j + 18] for j in range(n_units)
             if d[s0 + 40 * j + 8] in (0, 1)]
    cities = []
    for k in range(n_cities):
        r = d[c0 + 92 * k: c0 + 92 * (k + 1)]
        cities.append({
            "name": r[34:50].split(b"\x00")[0].decode("latin-1"),
            "size": r[11],
            "food_box": struct.unpack_from("<H", r, 28)[0],
            "gross_food": r[82],
            "settlers": homes.count(k),
        })
    return {"turn": struct.unpack_from("<h", d, 668)[0], "cities": cities}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("dir")
    ap.add_argument("-o", "--out", required=True)
    a = ap.parse_args()
    saves = sorted(glob.glob(os.path.join(a.dir, "turn[0-9][0-9].sav")))
    rows = [series(p) for p in saves]
    with open(a.out, "w") as f:
        f.write("# T1-4 growth series: standard-game lab (std1), one block per\n"
                "# harvested autosave, game record framing; derived numbers only.\n"
                "# generator: tools/growth_series.py\n"
                "# turn\tcity\tsize\tfood_box\tgross_food\tsettlers\n")
        for r in rows:
            for c in r["cities"]:
                f.write(f"{r['turn']}\t{c['name']}\t{c['size']}\t{c['food_box']}"
                        f"\t{c['gross_food']}\t{c['settlers']}\n")

if __name__ == "__main__":
    main()
