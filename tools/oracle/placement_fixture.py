"""placement_fixture.py <run-dir> <draw-log> <label> — RG-2 placement fixture.

Inputs come from one `seeded_start.sh` run with
`DUMP_INTRO=1 HOOK_OUT=<draw-log> HOOK_ARGS="--skip 0x403000:0x40a000 --state"`:
  <run-dir>/intro_tiles.bin, intro_landmass.bin, intro_maphdr.bin (RAM at
  "In the Beginning": after placement, before any AI move) and the turn-1
  autosave (settings bytes 684..689); <draw-log> = randhook's
  (return address, CRT state before the draw) lines.

It replays start placement with the decoded algorithm (evidence §RG-2
start placement), checks every simulated draw (call site and CRT state)
against the log, and prints a fixture block: setup, CRT states, starts
and the tiles placement examined (derived data: a few hundred tile
samples, not a map). Exits non-zero if the replay differs from the log.
"""
import os
import struct
import sys
from collections import Counter

SITE_SEED50 = "0048c5d3"  # rand()%50 before the civ loop (0x48c5ce)
SITE_ATT_AI, SITE_ATT_HUMAN = "004888f7", "0048895e"
SITE_TRIBE, SITE_GENDER = "00488fea", "004890d6"
SITE_X, SITE_Y = "00489330", "004893dc"


class Crt:
    def __init__(self, s):
        self.s = s

    def rand(self):
        self.s = (self.s * 214013 + 2531011) & 0xFFFFFFFF
        return (self.s >> 16) & 0x7FFF


def main(run, log_path, label):
    rows = [(ln.split()[0], int(ln.split()[1], 16)) for ln in open(log_path)]
    i0 = next(i for i, (s, _) in enumerate(rows) if s == SITE_SEED50)
    tiles = bytearray(open(os.path.join(run, "intro_tiles.bin"), "rb").read())
    lm = open(os.path.join(run, "intro_landmass.bin"), "rb").read()
    w2, h, _, _, seed = struct.unpack_from("<hhhhH", open(os.path.join(run, "intro_maphdr.bin"), "rb").read())
    w = w2 // 2
    sav = next(open(os.path.join(run, f), "rb").read() for f in sorted(os.listdir(run))
               if f.lower().endswith("_auto.sav"))
    mask, human, mask_after = sav[686], sav[687], sav[688]
    assert mask == mask_after, (mask, mask_after)
    for k in range(w * h):  # pre-placement: drop the settlers' marks
        tiles[6 * k + 1] &= ~1
        tiles[6 * k + 5] |= 0xF0

    def T(x, y):
        return 6 * (y * w + (x >> 1))

    def site_value(x, y):
        v = tiles[T(x, y) + 2] >> 5
        if v:
            return v
        f = tiles[T(x, y) + 5] & 15
        return max(f, 8) if f else 0

    def lm_value(x, y):
        return struct.unpack_from("<h", lm, 40 * tiles[T(x, y) + 3] + 2)[0]

    def owner(x, y):
        o = tiles[T(x, y) + 5] >> 4
        return -1 if o == 15 else o

    def hut(x, y):
        if tiles[T(x, y)] & 15 == 10 or owner(x, y) >= 0:
            return False
        a = (x + y) >> 1
        b = x - a
        return ((a & 3) + 4 * (b & 3)) == ((seed + 13 * (b >> 2) + 11 * (a >> 2) + 8) & 31)

    def occupied(x, y):
        f = tiles[T(x, y) + 1]
        return bool(((f & 0xC2) == 2 or f & 1) and owner(x, y) >= 0)

    def dist(x1, y1, x2, y2):
        dx = abs(x1 - x2)
        if w2 // 2 < dx:
            dx = w2 - dx
        dx, dy = abs(dx), abs(y1 - y2)
        m = (dx + dy) >> 1
        return dy - ((m - dx + 1) >> 1) if dy >= dx else dx - ((m - dy + 1) >> 1)

    why = Counter()

    def accept(x, y, n, is_human, units):
        def rej(r):
            why[r] += 1
            return False
        if n > 2000 and not is_human:
            return True
        if tiles[T(x, y)] & 15 == 10:
            return rej("ocean")
        if site_value(x, y) < 12 - (n >> 5):
            return rej("site")
        md = min([30] + [dist(x, y, a, b) for a, b in units])
        if md < 16 - (n >> 7):
            return rej("dist")
        if md < 2 and n <= 2000:
            return rej("dist<2")
        if md < 1:
            return rej("dist<1")
        if lm_value(x, y) < 32 - (n >> 6):
            return rej("landmass")
        if hut(x, y) and n <= 2000:
            return rej("hut")
        if occupied(x, y):
            return rej("occupied")
        return True

    crt = Crt(rows[i0][1])
    sim, seen, units, starts = [], {}, [], []

    def draw(site):
        sim.append((site, crt.s))
        return crt.rand()

    draw(SITE_SEED50)
    for c in range(1, 8):
        is_human = bool(human >> c & 1)
        for k in range(1, 8):
            draw(SITE_ATT_HUMAN if human >> k & 1 else SITE_ATT_AI)
        if not is_human:
            draw(SITE_TRIBE)
            draw(SITE_GENDER)
        if not mask >> c & 1:
            continue
        n = 0
        while True:
            x = draw(SITE_X) % w2
            y = draw(SITE_Y) % h
            if x & 1:
                x -= 1
            if y & 1:
                x += 1
            n += 1
            o = T(x, y)
            seen.setdefault((x, y), (tiles[o], tiles[o + 2], tiles[o + 5], lm_value(x, y)))
            if accept(x, y, n, is_human, units):
                break
        units.append((x, y))
        starts.append((c, x, y, n))
        o = T(x, y)
        tiles[o + 1] |= 1
        tiles[o + 5] = (tiles[o + 5] & 15) | (c << 4)

    log = rows[i0:i0 + len(sim)]
    bad = next((j for j, (a, b) in enumerate(zip(sim, log)) if a != b), None)
    if bad is not None or i0 + len(sim) >= len(rows):
        sys.exit(f"{label}: replay differs from the log at draw {bad}")
    print(f"world {label}")
    print(f"# {len(sim)} draws replayed, every call site and CRT state equal to the log; rejections {dict(why)}")
    print(f"setup w2={w2} h={h} seed={seed:#06x} human={human:#04x} mask={mask:#04x}")
    print(f"crt_start {rows[i0][1]:#010x}")
    print(f"crt_end {rows[i0 + len(sim)][1]:#010x}")
    print(f"draws {len(sim)}")
    for c, x, y, n in starts:
        print(f"start {c} {x} {y} {n}")
    print("# tile x y byte0 byte2 byte5 landmass_value")
    for (x, y), (b0, b2, b5, lv) in sorted(seen.items()):
        print(f"tile {x} {y} {b0} {b2} {b5} {lv}")
    print("end")


if __name__ == "__main__":
    main(*sys.argv[1:4])
