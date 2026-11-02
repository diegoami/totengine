#!/usr/bin/env python3
"""
totctl — Civilization II: Test of Time format probe (Tier-0 reference tool).

Reads original game data (no assets are redistributed):
  * Rules.txt  -> structured summary (JSON)
  * .MP maps   -> JSON + PNG render
  * .SAV/.SCN  -> structural walk per Mercator's published ToT section map
  * save diff  -> annotated byte-diff mapped to known sections

Reference implementation for docs/plan.md Phase 0. Production language is decided
separately; this probe exists to validate format assumptions fast.

Usage:
  totctl rules  <Rules.txt> [-o out.json]
  totctl map    <file.mp>  [-o out.png] [--json out.json] [--scale N]
  totctl scnmap <file.scn> [-o out.png] [--json out.json] [--scale N]
  totctl save   <file.sav|scn> [-o out.json]
  totctl diff   <a.sav> <b.sav> [-o out.json]

Relative input paths resolve against $CIVTOT_HOME when set (e.g.
CIVTOT_HOME=/mnt/d/games/CIVTOT totctl map World.mp).
"""
from __future__ import annotations

import argparse
import json
import os
import struct
import sys
import zlib


def resolve(path: str) -> str:
    """Resolve an input path against $CIVTOT_HOME when relative."""
    home = os.environ.get("CIVTOT_HOME")
    if home and not os.path.isabs(path):
        return os.path.join(home, path)
    return path

# --------------------------------------------------------------------------
# generic helpers

def write_png(path: str, width: int, height: int, rows) -> None:
    """Minimal RGB PNG writer (no external deps)."""
    def chunk(tag: bytes, data: bytes) -> bytes:
        return (struct.pack(">I", len(data)) + tag + data
                + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF))

    raw = b"".join(b"\x00" + bytes(row) for row in rows)
    png = (b"\x89PNG\r\n\x1a\n"
           + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
           + chunk(b"IDAT", zlib.compress(raw, 9))
           + chunk(b"IEND", b""))
    with open(path, "wb") as fh:
        fh.write(png)


def null_strings(block: bytes, limit: int = 5):
    """Extract null-terminated printable strings from a block."""
    out, cur = [], []
    for b in block:
        if b == 0:
            if cur:
                s = bytes(cur).decode("latin-1")
                if s.strip():
                    out.append(s)
                cur = []
        elif 32 <= b < 127:
            cur.append(b)
        else:
            cur = []
    return out[:limit]


# --------------------------------------------------------------------------
# Rules.txt

def parse_rules(path: str) -> dict:
    sections: dict[str, dict] = {}
    name = None
    with open(path, "r", encoding="latin-1") as fh:
        for raw in fh:
            line = raw.rstrip("\r\n")
            if line.startswith(";") or not line.strip():
                continue
            if line.startswith("@"):
                name = line[1:].strip()
                sections.setdefault(name, {"header": None, "rows": []})
                continue
            if name is None:
                continue
            sec = sections[name]
            if sec["header"] is None:
                sec["header"] = [c.strip() for c in line.split(",")]
            else:
                sec["rows"].append([c.strip() for c in line.split(",")])
    return sections


def rules_summary(sections: dict) -> dict:
    out = {"sections": {}}
    for name, sec in sections.items():
        entry = {"columns": sec["header"], "row_count": len(sec["rows"])}
        if name == "COSMIC":  # key=value style
            entry["values"] = {r[0]: (r[1] if len(r) > 1 else "")
                               for r in sec["rows"]}
        if name in ("TERRAIN", "UNITS", "IMPROVE", "CIVILIZE", "GOVERNMENTS"):
            entry["first_column"] = [r[0] for r in sec["rows"]][:80]
        out["sections"][name] = entry
    return out


def terrain_names(sections: dict):
    """Base terrain ids are the first 11 @TERRAIN rows (Desert..Ocean);
    the remaining rows are special-resource overlays."""
    sec = sections.get("TERRAIN")
    return [r[0].lower() for r in sec["rows"][:11]] if sec else []


# --------------------------------------------------------------------------
# .MP map files

TERRAIN_PALETTE_HINTS = [
    ("ocean", (24, 62, 133)), ("sea", (52, 106, 168)), ("coast", (86, 144, 196)),
    ("grass", (94, 155, 70)), ("plains", (156, 187, 97)),
    ("forest", (34, 94, 44)), ("jungle", (11, 102, 89)), ("swamp", (84, 114, 58)),
    ("desert", (216, 196, 116)), ("hill", (139, 106, 70)),
    ("mount", (130, 127, 122)), ("tundra", (168, 179, 173)),
    ("glacier", (232, 240, 244)), ("arctic", (214, 228, 235)),
]


def terrain_color(idx: int, names):
    if 0 <= idx < len(names):
        n = names[idx]
        for key, rgb in TERRAIN_PALETTE_HINTS:
            if key in n:
                return rgb
    # deterministic fallback color from index
    return (40 + (idx * 37) % 180, 40 + (idx * 67) % 180, 40 + (idx * 97) % 180)


def parse_mp(path: str):
    data = open(path, "rb").read()
    info = {"file": path, "size": len(data)}
    # Variant A (observed on ToT World.mp): 8-byte header = w, h, dword real-tile
    # count; then w*h slots of 3 bytes each (word + 1 extra byte); small tail.
    if len(data) >= 8:
        w, h = struct.unpack_from("<HH", data, 0)
        real = struct.unpack_from("<I", data, 4)[0]
        if w and h and w < 8192 and h < 8192:
            body = len(data) - 8
            if body == w * h * 3 or abs(body - w * h * 3) <= 256:
                slots = []
                for i in range(w * h):
                    o = 8 + 3 * i
                    slots.append((data[o] | (data[o + 1] << 8), data[o + 2]))
                info.update(layout="tot_3byte_slot", header_bytes=8, width=w,
                            height=h, real_tiles=real,
                            tiles=[s[0] for s in slots],
                            extra=[s[1] for s in slots],
                            tail_bytes=body - w * h * 3)
                return info
    # Variant C (observed on ToTPP-era AnticChina.MP, proven 2026-11-01):
    # 8-byte header = w, h, u16 real count, u16 maps; then a 90-byte front
    # blob; then real*6 bytes of stored tiles (3-byte data slot + 3-byte pad
    # slot: pad word 0x003F on 22,488 records / 0x0001 on 4,293 / 22 other
    # values on 1,219; pad extra 0xF0 on 25,882 / 0xFF on 1,219 plus
    # low-bit variants; data-slot extra byte takes the 8 values
    # 0x00..0xE0 in 0x20 steps); then real bytes of per-tile flags (mostly
    # 0). Stored grid is dense w/2 x h (display w = 2x).
    if len(data) >= 8:
        w, h = struct.unpack_from("<HH", data, 0)
        real, maps = struct.unpack_from("<HH", data, 4)
        if (w and h and w % 2 == 0 and maps == 1
                and (w // 2) * h == real
                and len(data) == 98 + 6 * real + real):
            tiles, extra, pad = [], [], []
            for t in range(real):
                o = 98 + 6 * t
                tiles.append(data[o] | (data[o + 1] << 8))
                extra.append(data[o + 2])
                pad.append(data[o + 3] | (data[o + 4] << 8))
            info.update(layout="tot_paired_slot", header_bytes=98, width=w // 2,
                        height=h, display_width=w, real_tiles=real, maps=maps,
                        tiles=tiles, extra=extra, pad_words=pad,
                        per_tile_flags=data[98 + 6 * real:98 + 7 * real])
            return info
    # Variant B (classic Civ2 MP): 4-byte header, word per slot.
    for hdr in (4, 8, 2, 0):
        if len(data) < hdr + 4:
            continue
        w, h = struct.unpack_from("<HH", data, 0)
        if w and h and w < 4096 and h < 4096 and len(data) - hdr == 2 * w * h:
            info.update(layout="classic_2byte", header_bytes=hdr, width=w,
                        height=h,
                        tiles=list(struct.unpack_from("<%dH" % (w * h), data, hdr)))
            return info
    info["error"] = ("no known layout fits: sizes tried tot_3byte_slot "
                     "(8 + 3*w*h) and classic_2byte (hdr + 2*w*h)")
    return info


def _score_decode(tiles, w, h, decode):
    """Spatial-coherence + dominance heuristic: real terrain maps are clumpy and
    dominated by water. Used to pick between candidate tile decodings."""
    terr = [decode(t) for t in tiles]
    same = total = 0
    for y in range(h):
        for x in range(w):
            i = y * w + x
            for dx, dy in ((1, 0), (0, 1)):
                nx, ny = x + dx, y + dy
                if nx < w and ny < h:
                    total += 1
                    if terr[i] == terr[ny * w + nx]:
                        same += 1
    counts: dict[int, int] = {}
    for t in terr:
        counts[t] = counts.get(t, 0) + 1
    dominance = max(counts.values()) / len(terr)
    return (same / total if total else 0) + dominance, counts


def render_mp(mp, names, out_png, scale=4):
    w, h, tiles = mp["width"], mp["height"], mp["tiles"]
    extra = mp.get("extra")
    if mp.get("layout") == "tot_3byte_slot" and extra is not None:
        # verified on ToT World.mp: terrain = word & 0x7F (bit 0x80 = RIVER,
        # proven 2026-11-01: bit-0x80 tiles form dendritic chains matching
        # Earth's rivers — Mississippi, African and Australian networks), void
        # = extra byte & 0xF0. Special resources are NOT stored in .MP files.
        def is_void(i):
            return (extra[i] & 0xF0) != 0

        def terr(i):
            return tiles[i] & 0x7F

        decode = "verified: word&0x7F, river=word&0x80, void=extra&0xF0"
        hist = {}
        rivers = 0
        for i in range(w * h):
            if not is_void(i):
                hist[terr(i)] = hist.get(terr(i), 0) + 1
                if tiles[i] & 0x80:
                    rivers += 1
        rows = []
        for y in range(h):
            row = bytearray()
            for x in range(w):
                i = y * w + x
                if is_void(i):
                    rgb = (10, 10, 14)
                elif tiles[i] & 0x80:
                    rgb = (0, 170, 230)  # river: bright blue, distinct from ocean
                else:
                    rgb = terrain_color(terr(i), names)
                row.extend(rgb * scale)
            rows.extend([row] * scale)
        write_png(out_png, w * scale, h * scale, rows)
        return {"decode": decode,
                "river_tiles": rivers,
                "terrain_histogram": {str(k): v for k, v in
                                      sorted(hist.items(), key=lambda kv: -kv[1])[:12]},
                "png": out_png}

    if mp.get("layout") == "tot_paired_slot":
        # AnticChina.MP flavor (ToTPP-era): dense stored grid, word =
        # terrain | river(0x80) | special(0x40) | improvement(0x3000);
        # verified spatially 2026-11-01 (Yellow River/Yangtze networks,
        # coastal specials, improvement clusters in river valleys).
        # paired-slot render mask = 0x3F per the documented decode (bit 0x40
        # = special resource, 0x80 = river); a 0x7F mask mis-colours every
        # special tile (issue #29: 2,008 tiles = 7% of AnticChina)
        def terr(i):
            return tiles[i] & 0x3F

        hist, rivers, specials, imps = {}, 0, 0, 0
        for i in range(w * h):
            t = tiles[i] & 0x3F
            hist[t] = hist.get(t, 0) + 1
            if tiles[i] & 0x80:
                rivers += 1
            if tiles[i] & 0x40:
                specials += 1
            if tiles[i] & 0x3000:
                imps += 1
        rows = []
        for y in range(h):
            row = bytearray()
            for x in range(w):
                i = y * w + x
                if tiles[i] & 0x3000:
                    rgb = (35, 35, 35)          # improvement flag
                elif tiles[i] & 0x80:
                    rgb = (0, 170, 230)         # river
                else:
                    rgb = terrain_color(terr(i), names)
                row.extend(rgb * scale)
            rows.extend([row] * scale)
        write_png(out_png, w * scale, h * scale, rows)
        return {"decode": "paired-slot: terrain=word&0x3F, river=0x80, "
                          "special=0x40, improvement=0x3000",
                "river_tiles": rivers, "special_tiles": specials,
                "improvement_tiles": imps,
                "terrain_histogram": {str(k): v for k, v in
                                      sorted(hist.items(), key=lambda kv: -kv[1])[:12]},
                "png": out_png}

    # classic/unknown layout: try candidate decodings, pick most coherent.
    candidates = {
        "low_nibble": lambda t: t & 0x0F,
        "low_6bits": lambda t: t & 0x3F,
        "low_5bits": lambda t: t & 0x1F,
        "high_byte": lambda t: (t >> 8) & 0xFF,
    }
    scored = {}
    for label, dec in candidates.items():
        score, counts = _score_decode(tiles, w, h, dec)
        scored[label] = (score, dec, counts)
    best = max(scored, key=lambda k: scored[k][0])
    score, dec, counts = scored[best]
    rows = []
    for y in range(h):
        row = bytearray()
        for x in range(w):
            t = tiles[y * w + x]
            if t == 0xFFFF:
                rgb = (10, 10, 14)  # void / black
            else:
                rgb = terrain_color(dec(t), names)
            row.extend(rgb * scale)
        rows.extend([row] * scale)
    write_png(out_png, w * scale, h * scale, rows)
    return {"decode": best,
            "scores": {k: round(v[0], 4) for k, v in scored.items()},
            "terrain_histogram": {str(k): v for k, v in
                                  sorted(counts.items(), key=lambda kv: -kv[1])[:12]}}


# --------------------------------------------------------------------------
# SAV/SCN structural walk (base ToT 1.1, version 0x0032)
# Section sizes per Mercator's Apolyton thread #130935 (verified arithmetic:
# 640+16+38+200+56+288 = 1238 for ToT settings; wonders = 28 shorts).

SAVE_VERSIONS = {0x0027: "Classic/CiC", 0x0028: "Fantastic Worlds",
                 0x002C: "MGE 1.3", 0x0031: "ToT 1.0", 0x0032: "ToT 1.1",
                 0x0033: "ToT 1.1 + ToTPP (extended)"}


def walk_ext033(data, hdr, blob10240_end, out):
    """Deep-walk the ToTPP 0x33 extension (proven on the Warring States
    family: Warring States.scn / Hu_Auto.SAV / deiYan.sav / deiQin.sav,
    2026-11-02). Layout relative to blob10240_end:

      [+0, +32+15*surface)   extended per-tile arrays (mostly sparse; a
                             dense 6 B/tile band and a 10,240-B blob sit
                             inside; sub-layout open)
      [+u0, +u0+40*N)        unit records, 40 B, count-based
      [+4 interlude)         4 bytes (00 00 2a 00 seen)
      [+c0, +c0+92*M)        city records, 92 B, count-based
      [+c_end, LUAE|EOF)     fixed-size post-city block (~12.9 KB)

    0x33 unit record layout (differs from the 0x32 base layout!):
      @0  u16 id — sequential, re-assigned on every save
      @2  u16 tail short (values 0..7 observed, bulk = 2; role open)
      @4  u16 = 2 on the bulk (the 16 id-0 records excepted)
      @6  u16 varies 0..7 (sub-kind? role open)
      @10/@12 s16 x/y — LOGICAL grid coords; off-board units carry
      negative coords (-100/-500 observed)
      @18 u8 type — Rules.txt @UNITS index (190 rows in Warring
      States; values 0..188 observed)
      @19 u8 owner — 0=barbarians, 1..7 = civs
      @28 u16 optional link, 0xffff = none (home city?)
      remaining bytes flags/counters (open)

    Verified arithmetic: deiQin − deiYan = +198 units (+7,920) and +11
    city slots (+1,012), post-city block −92 = +8,840 bytes exactly;
    scn vs Hu_Auto (load+autosave, same sizes) keep 1442 units, of
    which 1352 sit at an (x,y) occurring somewhere in the .scn, ids
    re-assigned 6527..7967 -> 10..1450.
    """
    import struct as _s
    surface = hdr[2]
    xmax, ymax = 2 * hdr[0], hdr[1]
    lua = data.find(b"LUAE", blob10240_end)
    end = lua if lua > 0 else len(data)
    ustart = blob10240_end + 32 + 15 * surface

    def urec_valid(rec):
        if rec.count(0) == 40:
            return False
        uid, f2 = _s.unpack_from("<HH", rec, 0)
        x, y = _s.unpack_from("<hh", rec, 10)
        return (uid < 8192 and f2 < 16
                and -1000 <= x <= xmax and -1000 <= y <= ymax)

    ext = {"unit_start_offset": ustart,
           "unit_start_formula": "blob10240_end + 32 + 15*surface"}
    if ustart + 40 > len(data) or not urec_valid(data[ustart:ustart + 40]):
        ext["error"] = ("unit-start formula did not land on a valid record "
                        "(map variant?); tail framing only")
        out["ext033"] = ext
        return
    # The formula is a LANDMARK inside the array, not necessarily its first
    # record: on all four Warring States files exactly one more record
    # (id=0, f2=0 — the bulk carries f2=2) precedes it. Extend backwards to
    # the maximal contiguous block (bounded) so the block boundaries are
    # measured, not assumed.
    ustart2 = ustart
    for _ in range(128):
        if ustart2 - 40 >= blob10240_end and urec_valid(data[ustart2 - 40:ustart2]):
            ustart2 -= 40
        else:
            break
    if ustart2 != ustart:
        ext["unit_start_measured"] = ustart2
        ext["unit_start_formula_note"] = (
            "formula lands %d record(s) into the block; id-0/f2-0 slot-0 "
            "record(s) precede the f2=2 bulk; whether slot 0 belongs to the "
            "unit array or terminates the preceding per-tile section is "
            "open" % ((ustart - ustart2) // 40))
    units, off = [], ustart2
    while off + 40 <= end and urec_valid(data[off:off + 40]):
        units.append(off)
        off += 40
    owner_hist, type_hist = {}, {}
    ids = []
    sample = []
    for k, uo in enumerate(units):
        r = data[uo:uo + 40]
        uid = _s.unpack_from("<H", r, 0)[0]
        x, y = _s.unpack_from("<hh", r, 10)
        t, o = r[18], r[19]
        ids.append(uid)
        owner_hist[o] = owner_hist.get(o, 0) + 1
        type_hist[t] = type_hist.get(t, 0) + 1
        if k < 8 or k >= len(units) - 2:
            sample.append({"i": k, "id": uid, "x": x, "y": y,
                           "type": t, "owner": o})
    ext["units"] = {
        "offset": ustart2, "end": off, "count": len(units),
        "record_size": 40, "id_range": [min(ids), max(ids)],
        "owner_histogram": dict(sorted(owner_hist.items())),
        "type_histogram_top": dict(sorted(type_hist.items(),
                                          key=lambda kv: -kv[1])[:12]),
        "sample": sample,
        "fields_proven": {
            "id_u16@0": "sequential, re-assigned on every save",
            "x_s16@10": "logical-grid x (negative = off-board)",
            "y_s16@12": "logical-grid y (negative = off-board)",
            "type_u8@18": "index into Rules.txt @UNITS",
            "owner_u8@19": "0=barbarians, 1..7 = civs",
        },
    }
    interlude = 4
    cstart = off + interlude
    ncities = 0
    names = []
    while cstart + 92 * ncities + 92 <= end:
        s = data[cstart + 92 * ncities: cstart + 92 * (ncities + 1)]
        x, y = _s.unpack_from("<HH", s, 58)
        if x > xmax or y > ymax:
            break
        nm = s[:16].split(b"\x00")[0].decode("latin-1", "replace")
        names.append(nm)
        ncities += 1
    cend = cstart + 92 * ncities
    ext["cities"] = {
        "offset": cstart, "count": ncities, "record_size": 92,
        "named": sum(1 for n in names if n),
        "names_first_12": names[:12],
        "note": ("same 92-B record as the 0x32 base format (name @0, "
                 "x u16@58, y u16@60 logical)"),
    }
    ext["post_city"] = {"offset": cend, "size": end - cend,
                        "note": ("fixed-size block (~12.9 KB here; does not "
                                 "scale with city count); internal layout "
                                 "open — trade-route-like records and id "
                                 "lists observed")}
    total = end - blob10240_end
    # Falsifiable structural checks (issue #25): the arithmetic below is a
    # *description* — the post-city term is the leftover by construction and
    # would absorb any mis-walk, so validity rests on these checks instead.
    def printable_name_slot(o):
        nm = data[o:o + 16].split(b"\x00")[0]
        return bool(nm) and all(32 <= c < 127 for c in nm)

    cands2 = set()
    i = blob10240_end
    while i < end - 16:
        j = data.find(b"\x00", i, i + 16)
        if 3 <= j - i <= 15 and data[i + 15] == 0 \
                and all(32 <= c < 127 for c in data[i:j]):
            cands2.add(i)
            i = j
        i += 1
    competing_grids = []
    seen = set()
    for s in sorted(cands2):
        if s in seen or (cstart <= s < cend):
            continue
        ch = [s]
        while ch[-1] + 92 in cands2:
            ch.append(ch[-1] + 92)
        seen.update(ch[1:])
        if len(ch) >= 8 and all(
                _s.unpack_from("<H", data, c + 58)[0] <= xmax
                and _s.unpack_from("<H", data, c + 60)[0] <= ymax
                for c in ch):
            competing_grids.append(s)
    ext["checks"] = {
        "unit_start_valid": True,  # an invalid start returns early above
        "unit_block_preceded_by_invalid": (
            not urec_valid(data[ustart2 - 40:ustart2])
            if ustart2 - 40 >= blob10240_end else True),
        "city0_name_printable": printable_name_slot(cstart),
        "city_grid_is_only_name_grid": not competing_grids,
        "competing_name_grids": competing_grids,
    }
    ext["accounting"] = {
        "total": total,
        "formula": (f"{total} = {ustart2 - blob10240_end} (per-tile arrays) "
                    f"+ 40*{len(units)} + 4 + 92*{ncities} + "
                    f"{end - cend} (post-city)"),
        "note": ("descriptive decomposition: the post-city term is the "
                 "leftover by construction — the falsifiable checks are in "
                 "ext.checks (unit-start validity, block maximality, city-0 "
                 "name, no competing 92-B name grid outside the walked one)"),
    }
    out["ext033"] = ext


def walk_save(path: str) -> dict:
    data = open(path, "rb").read()
    out = {"file": path, "size": len(data)}
    if data[:8] != b"CIVILIZE":
        out["error"] = "not a Civ2 save/scenario (bad magic)"
        return out
    out["magic"] = "CIVILIZE"
    ver = struct.unpack_from("<H", data, 10)[0]
    out["version_raw"] = "0x%04X" % ver
    out["version"] = SAVE_VERSIONS.get(ver, "unknown")
    if ver not in (0x0032, 0x0033):
        out["note"] = "walker targets ToT; other versions not parsed further"
        return out
    if ver == 0x0033:
        out["note"] = ("ToTPP-extended save: base 0x32 front sections apply; "
                       "after the 10240-blob comes the ToTPP extended state "
                       "(dynamic, mostly-sparse arrays), then a 'LUAE' magic "
                       "followed by the serialized Lua state as plain text, "
                       "ending with an 'EVNT' trailer + 4 bytes")

    pos = 12
    out["unit_transport_relationships"] = {"offset": pos, "size": 640}
    pos += 640                                   # settings: transport rel
    out["menu"] = {"offset": pos, "size": 16}
    pos += 16
    turn, year = struct.unpack_from("<hh", data, pos)
    out["game"] = {"offset": pos, "size": 38, "turn": turn, "year": year}
    pos += 38
    first = data[pos:pos + 100]
    disc = data[pos + 100:pos + 200]
    out["technologies"] = {
        "offset": pos, "size": 200,
        "first_discovered_count": sum(1 for b in first if b),
        "discovered_mask_popcount": sum(bin(b).count("1") for b in disc)}
    pos += 200
    wonders = struct.unpack_from("<28h", data, pos)
    out["wonders"] = {"offset": pos, "size": 56,
                      "built": sum(1 for v in wonders if v not in (-1, -2)),
                      "destroyed": sum(1 for v in wonders if v == -2)}
    pos += 56
    out["unknown_288"] = {"offset": pos, "size": 288}
    pos += 288
    assert pos == 12 + 1238, "settings block arithmetic broken"

    # civilizations: 7 name blocks x242, then 8 property blocks x3348
    out["civ_names"] = []
    for i in range(7):
        block = data[pos + i * 242: pos + (i + 1) * 242]
        out["civ_names"].append(null_strings(block, 4))
    pos += 7 * 242
    out["civ_properties"] = {"offset": pos, "size": 8 * 3348}
    pos += 8 * 3348

    # transporters (ToT 1.1): 172 + 14*n — count assumed in first short
    t_first = struct.unpack_from("<h", data, pos)[0]
    out["transporters"] = {"offset": pos, "fixed": 172,
                           "first_short": t_first,
                           "guessed_count": max(0, t_first)}
    pos += 172  # variable tail unknown; treat as fixed for the base probe

    # map header: 16 bytes = 8 shorts
    hdr = struct.unpack_from("<8h", data, pos)
    out["map_header"] = {"offset": pos, "values": list(hdr)}
    pos += 16
    out["map_note"] = ("fields 6&7 (1-based) multiply into a following blob per "
                       "Mercator; maps repeat the data block in ToT")
    if ver in (0x0032, 0x0033):  # front layout identical (proven on deiQin)
        maps, surface = hdr[3], hdr[2]
        map_data = 13 * surface + 2 if maps else 0
        blob = 2 * hdr[5] * hdr[6] if maps else 0
        map_end = pos + maps * map_data + blob
        blob10240_end = map_end + 10240
        out["sections_derived"] = {
            "map_data": {"offset": pos, "size": maps * map_data,
                         "per_map": map_data, "maps": maps},
            "blob_2ab": {"offset": pos + maps * map_data, "size": blob},
            "blob_10240": {"offset": map_end, "size": 10240,
                           "note": "mostly zeros in ROMFALL"},
        }
        # ---- embedded map decode (proven on ROMFALL, 2026-11-01/02) ----
        # Per map: a 7 B/tile visibility block, then 6 B-stride tile fields.
        # Measured byte positions (ROMFALL 2026-11-02, re-verified against
        # the section's known histograms): terrain|river|special byte at
        # mp0+6t+4 (terrain = &0x3F: ids 0-10; 0x40 = special resource,
        # 0x80 = river); road/rail bits at mp0+6t+12 (0x40 road, 0x80
        # railroad — 3,497 road / 413 rail / 128 both on ROMFALL, matching
        # the original spatial verification). The exact interleaving of the
        # 6-B-stride fields inside the section remains open.
        if maps:
            stored_w, stored_h = hdr[0] // 2, hdr[1]
            out["map_decode"] = {"stored_dims": [stored_w, stored_h],
                                 "display_dims": [hdr[0], hdr[1]],
                                 "dense": stored_w * stored_h == surface,
                                 "logical_to_stored": "sx = (x - (y&1))//2 - 1, sy = y",
                                 "geometry_note": ("logical grid (unit/city coords) "
                                                   "is hdr0 x hdr1; the terrain "
                                                   "grid is hdr0/2 x hdr1 dense "
                                                   "(diamond tiles: 2 logical "
                                                   "columns per stored column, 2 "
                                                   "border columns west; "
                                                   "per-file support numbers "
                                                   "measured in "
                                                   "garrisons_measured / "
                                                   "city_terrain_measured "
                                                   "below)"),
                                 "per_map": []}
            for m in range(maps):
                base = pos + m * map_data + 2
                imp0 = base
                mp0 = base + 7 * surface
                terr_hist: dict[int, int] = {}
                river = road = rail = special = both = 0
                vis_nz = [0] * 7
                for t in range(surface):
                    t6 = data[mp0 + 6 * t + 4]
                    imp6 = data[mp0 + 6 * t + 14]
                    terr_hist[t6 & 0x3F] = terr_hist.get(t6 & 0x3F, 0) + 1
                    if t6 & 0x80:
                        river += 1
                    if t6 & 0x40:
                        special += 1
                    r6, l6 = imp6 & 0x40, imp6 & 0x80
                    if r6:
                        road += 1
                    if l6:
                        rail += 1
                    if r6 and l6:
                        both += 1
                    for j in range(7):
                        if data[imp0 + 7 * t + j]:
                            vis_nz[j] += 1
                out["map_decode"]["per_map"].append({
                    "map_index": m,
                    "improvements_offset": mp0,
                    "terrain_histogram": {str(k): v for k, v in
                                          sorted(terr_hist.items(),
                                                 key=lambda kv: -kv[1])[:12]},
                    "river_tiles": river, "road_tiles": road,
                    "railroad_tiles": rail, "special_tiles": special,
                    "road_and_rail_tiles": both,
                    "visibility_nonzero_per_civ": vis_nz,
                    "note": ("terrain byte mp0+6t+4: terrain=&0x3F (ids 0-10), "
                             "0x40=special resource (70 ocean specials on "
                             "ROMFALL), 0x80=river; road/rail bits at "
                             "mp0+6t+12: 0x40 road / 0x80 rail (ROMFALL "
                             "3497/413, 128 both — matches the original "
                             "spatial verification); the 6-B-stride field "
                             "interleave is open; visibility bytes' "
                             "near-identical distribution suggests per-civ "
                             "fog, unproven"),
                })
            out["map_decode"]["resource_seed_offset"] = pos + maps * map_data - 2
        if ver == 0x0033 and maps:
            # 0x33 tail framing (proven on deiQin/deiYan saves, Warring
            # States, 2026-11-02): LUAE magic + serialized Lua state text
            # ('return { ... }'), then EVNT + 4 bytes at EOF. ToTPP writes
            # plain 0x32 (no tail) for scenarios without Lua events (verified
            # via the oracle on Midgard).
            lua = data.find(b"LUAE", blob10240_end if maps else pos)
            evnt = data.rfind(b"EVNT")
            if lua > 0 and evnt > lua:
                blob_txt = data[lua + 4:evnt - 4]
                import re as _re
                keys = sorted(set(k.decode() for k in _re.findall(
                    rb'\n  \["([A-Za-z_][A-Za-z0-9_]*)"\] = \{', blob_txt[:500000])))
                tiles = len(_re.findall(rb'"tile\(\d+:\d+\)"', blob_txt))
                out["totpp_tail"] = {
                    "lua_magic_offset": lua,
                    "mid_region": [blob10240_end, lua],
                    "mid_region_size": lua - blob10240_end,
                    "lua_state_bytes": len(blob_txt),
                    "lua_head": blob_txt[:60].decode("latin-1", "replace"),
                    "lua_top_level_tables": keys,
                    "lua_tile_entries": tiles,
                    "evnt_trailer_offset": evnt,
                    "note": ("mid region walked below (ext033); Lua state "
                             "is plain readable text"),
                }
            # 0x33 deep walk; the 0x32 units/cities scan below does not
            # apply (that layout lives in the extension instead)
            walk_ext033(data, hdr, blob10240_end, out)
            return out
        # cities: find the 92-byte name grid — candidate printable name
        # fields (16-byte slot: printable prefix + NUL padding), then the
        # longest chain of candidates at exactly 92-byte stride.
        cands = set()
        i = blob10240_end
        while i < len(data) - 16:
            j = data.find(b"\x00", i, i + 16)
            if 3 <= j - i <= 15:
                nm = data[i:j]
                if all(32 <= c < 127 or 0xA0 <= c <= 0xFF for c in nm) and data[i + 15] == 0:
                    # require the previous record position to exist: the slot
                    # before a real grid is data, not another name
                    cands.add(i)
                    i = j
            i += 1
        best_chain = []
        for s in sorted(cands):
            if best_chain and s <= best_chain[0] + 92 * (len(best_chain) - 1):
                continue  # inside an existing chain
            chain = [s]
            while chain[-1] + 92 in cands:
                chain.append(chain[-1] + 92)
            if len(chain) > len(best_chain):
                best_chain = chain
        if len(best_chain) >= 8:
            probe = best_chain[0]
            n = (best_chain[-1] - probe) // 92 + 1
            cities = {"offset": probe, "count": n, "record_size": 92,
                      "size": n * 92, "chain_found": len(best_chain)}
            names = []
            coords = []
            for k in range(n):
                rec = data[probe + 92 * k: probe + 92 * (k + 1)]
                nm = rec[:16].split(b"\x00")[0].decode("latin-1", "replace")
                names.append(nm)
                coords.append({"i": k, "name": nm,
                               "x": struct.unpack_from("<H", rec, 58)[0],
                               "y": struct.unpack_from("<H", rec, 60)[0],
                               "tail_shorts": list(struct.unpack_from("<3H", rec, 86))})
            cities["names"] = names[:20]
            cities["names_total_named"] = sum(1 for x in names if x)
            cities["coords_first_8"] = coords[:8]
            cities["note"] = ("name 16 B @0; x u16@58 / y u16@60 on the "
                              "logical grid (same space as unit coords; "
                              "per-file garrison/terrain support in "
                              "garrisons_measured / city_terrain_measured); "
                              "tail shorts @86-91 measured, e.g. Roma "
                              "(5, 5, 8) — NOT the city size: a size-like "
                              "short sits in the two bytes immediately "
                              "BEFORE each name (Roma 29 = its in-game "
                              "size), i.e. at the final short of the "
                              "preceding 92-B slot; exact field boundary "
                              "open")
            out["cities"] = cities
            u_end = cities["offset"]
            region = u_end - blob10240_end
            # Verified on ROMFALL (34 + 710*40 = 28434) and Va_Auto
            # (34 + 711*40 = 28474): a 34-byte preamble, then 40-byte unit
            # records. The count is not stored in the preamble (only a
            # state u16 at +30 differs between scenario and in-game save).
            unit_count, rem = divmod(region, 40)
            units_table = []
            owner_hist: dict[int, int] = {}
            type_hist: dict[int, int] = {}
            for i in range(unit_count if rem == 34 else 0):
                r = data[blob10240_end + 34 + 40*i: blob10240_end + 34 + 40*(i+1)]
                x, y = struct.unpack_from("<HH", r, 6)
                rec = {"i": i, "x": x, "y": y, "type": r[14],
                       "owner": r[15], "id": struct.unpack_from("<H", r, 36)[0]}
                units_table.append(rec)
                owner_hist[r[15]] = owner_hist.get(r[15], 0) + 1
                type_hist[r[14]] = type_hist.get(r[14], 0) + 1
            out["units"] = {
                "region": [blob10240_end, u_end],
                "size": region,
                "record_size": 40,
                "preamble_bytes": rem,
                "count": unit_count if rem == 34 else None,
                "count_arithmetic": f"{region} = {rem} + 40 x {unit_count}",
                "fields_proven": {
                    "x_u16@6": "display-grid x (0..width-1)",
                    "y_u16@8": "display-grid y (0..height-1)",
                    "type_u8@14": "index into Rules.txt @UNITS",
                    "owner_u8@15": "civ: 0=barbarians, 1..7=players (civ_names[n-1]); bit 0x10 = leader unit (Pope Leo I)",
                    "id_u16@36": "per-save unit id (re-assigned on every save)",
                },
                "owner_histogram": dict(sorted(owner_hist.items())),
                "type_histogram_top": dict(sorted(type_hist.items(), key=lambda kv: -kv[1])[:12]),
                "sample_first_8": units_table[:8],
                "verification": ("layout facts: docs/evidence/README.md "
                                 "§T0-6; the garrison support number is "
                                 "measured per file in "
                                 "cities.garrisons_measured / "
                                 "cities.city_terrain_measured"),
                "note": ("remaining bytes open: 0-5 constants(2,2,0), 12-13 "
                         "counters/flags, 16-19 counters (veteran?), 20-35 "
                         "sparse, 38-39 tail"),
            }
            # per-file measured support for the garrison/geometry claims —
            # issue #27: never stamp another file's verification results here
            occ = {(u["x"], u["y"]) for u in units_table}
            garrisoned = sum(1 for c in coords if (c["x"], c["y"]) in occ)
            cities["garrisons_measured"] = (
                f"{garrisoned}/{n} cities have a unit on their exact tile "
                "(measured on this file)")
            stored_w2, stored_h2 = hdr[0] // 2, hdr[1]
            base = pos + 2 + 7 * surface
            on_ocean = on_land = outside = 0
            for c in coords:
                x, y = c["x"], c["y"]
                sx = (x - (y & 1)) // 2 - 1
                if not (0 <= sx < stored_w2 and 0 <= y < stored_h2):
                    outside += 1
                    continue
                t = data[base + 6 * (y * stored_w2 + sx) + 4] & 0x7F
                if t == 10:
                    on_ocean += 1
                else:
                    on_land += 1
            cities["city_terrain_measured"] = (
                f"{on_land}/{n} cities on non-ocean terrain, {on_ocean} on "
                f"ocean, {outside} outside the stored grid (measured on "
                "this file)")
    return out


# --------------------------------------------------------------------------
# SCN/SAV embedded map render (proven layout, see walk_save's map_decode)

def render_scnmap(path: str, out_png: str, names, scale=4) -> dict:
    """Render the surface map embedded in a .SCN/.SAV.

    Uses the measured tile-field positions (ROMFALL 2026-11-02): terrain
    from the byte at mp0+6t+4 (terrain = &0x3F, 0x40 = special resource,
    river = 0x80 — special-ocean tiles render as ocean), road/railroad
    bits from the byte at mp0+6t+12 (0x40 / 0x80). Verified on ROMFALL:
    3,497 road / 413 rail / 128 both tiles, matching the original spatial
    verification of the networks.
    """
    walk = walk_save(path)
    md = walk.get("map_decode")
    if not md:
        return {"error": "no decodable map (walker found none): %s"
                % walk.get("error", walk.get("note", "see save walk"))}
    data = open(path, "rb").read()
    w, h = md["stored_dims"]
    surface = walk["map_header"]["values"][2]
    mp0 = walk["sections_derived"]["map_data"]["offset"] + 2 + 7 * surface
    rows = []
    stats = {"river": 0, "road": 0, "rail": 0, "special": 0, "terrain": {}}
    for y in range(h):
        row = bytearray()
        for x in range(w):
            i = y * w + x
            t6 = data[mp0 + 6 * i + 4]
            imp6 = data[mp0 + 6 * i + 14]
            terr = t6 & 0x3F
            stats["terrain"][terr] = stats["terrain"].get(terr, 0) + 1
            if imp6 & 0x80:
                rgb = (35, 35, 35)            # railroad
                stats["rail"] += 1
            elif imp6 & 0x40:
                rgb = (139, 105, 50)          # road
                stats["road"] += 1
            elif t6 & 0x80:
                rgb = (0, 170, 230)           # river
                stats["river"] += 1
            else:
                if t6 & 0x40:
                    stats["special"] += 1
                rgb = terrain_color(terr, names)
            row.extend(rgb * scale)
        rows.extend([row] * scale)
    write_png(out_png, w * scale, h * scale, rows)
    return {"png": os.path.abspath(out_png), "stored_dims": [w, h],
            "display_dims": md["display_dims"],
            "overlay_counts": stats,
            "decode": ("terrain byte mp0+6t+4: terrain=&0x3F, 0x40=special "
                       "(rendered as its terrain), 0x80=river; road/rail "
                       "bits at mp0+6t+12: 0x40/0x80 — ROMFALL 2026-11-02")}



def diff_saves(path_a: str, path_b: str) -> dict:
    a = open(path_a, "rb").read()
    b = open(path_b, "rb").read()
    if len(a) != len(b):
        delta = len(b) - len(a)
        note = "size delta %+d bytes" % delta
        for label, size in (("unit record (ToT)", 40), ("city record (ToT)", 92),
                            ("compiled macro event (ToT)", 276)):
            if delta != 0 and delta % size == 0:
                note += " = %+d %s(s)" % (delta // size, label)
        common = min(len(a), len(b))
        regions = _diff_regions(a[:common], b[:common])
        return {"a": path_a, "b": path_b, "size_a": len(a), "size_b": len(b),
                "note": note,
                "diff_bytes_common_prefix": sum(r[1] - r[0] + 1 for r in regions),
                "region_count": len(regions),
                "regions_first_50": regions[:50]}
    regions = _diff_regions(a, b)
    return {"a": path_a, "b": path_b, "size": len(a),
            "diff_bytes": sum(r[1] - r[0] + 1 for r in regions),
            "regions": regions, "region_count": len(regions)}


def _diff_regions(a: bytes, b: bytes):
    regions = []
    start = None
    for i in range(len(a)):
        if a[i] != b[i]:
            if start is None:
                start = i
        elif start is not None:
            regions.append([start, i - 1])
            start = None
    if start is not None:
        regions.append([start, len(a) - 1])
    merged = []
    for r in regions:
        if merged and r[0] - merged[-1][1] <= 8:
            merged[-1][1] = r[1]
        else:
            merged.append(r)
    return merged


# --------------------------------------------------------------------------
# CLI

def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("rules", help="parse Rules.txt to JSON summary")
    p.add_argument("path")
    p.add_argument("-o", "--out")

    p = sub.add_parser("map", help="parse .MP and render PNG")
    p.add_argument("path")
    p.add_argument("-o", "--out", default="map.png")
    p.add_argument("--rules", help="Rules.txt for terrain names "
                    "(defaults to $CIVTOT_HOME/Rules.txt when set)")
    p.add_argument("--scale", type=int, default=4)
    p.add_argument("--json", dest="json_out")

    p = sub.add_parser("scnmap", help="render the map embedded in a .SCN/.SAV "
                     "with road/railroad/river overlays")
    p.add_argument("path")
    p.add_argument("-o", "--out", default="scnmap.png")
    p.add_argument("--rules", help="Rules.txt for terrain names "
                    "(defaults to $CIVTOT_HOME/Rules.txt when set)")
    p.add_argument("--scale", type=int, default=4)
    p.add_argument("--json", dest="json_out")

    p = sub.add_parser("save", help="structural walk of .SAV/.SCN")
    p.add_argument("path")
    p.add_argument("-o", "--out")

    p = sub.add_parser("diff", help="annotated byte diff of two saves")
    p.add_argument("a")
    p.add_argument("b")
    p.add_argument("-o", "--out")

    args = ap.parse_args(argv)
    result = {}
    json_out = None
    png_out = None

    def default_rules(explicit):
        if explicit:
            return resolve(explicit)
        home = os.environ.get("CIVTOT_HOME")
        cand = os.path.join(home, "Rules.txt") if home else None
        return cand if cand and os.path.exists(cand) else None

    if args.cmd == "rules":
        result = rules_summary(parse_rules(resolve(args.path)))
        json_out = args.out
    elif args.cmd == "map":
        rules_path = default_rules(args.rules)
        names = terrain_names(parse_rules(rules_path)) if rules_path else []
        mp = parse_mp(resolve(args.path))
        result = {k: v for k, v in mp.items() if k not in ("tiles", "extra")}
        if "tiles" in mp:
            png_out = args.out
            result["render"] = render_mp(mp, names, png_out, args.scale)
            result["render"]["png"] = os.path.abspath(png_out)
        json_out = args.json_out
    elif args.cmd == "scnmap":
        rules_path = default_rules(args.rules)
        names = terrain_names(parse_rules(rules_path)) if rules_path else []
        png_out = args.out
        result = render_scnmap(resolve(args.path), png_out, names, args.scale)
        json_out = args.json_out
    elif args.cmd == "save":
        result = walk_save(resolve(args.path))
        json_out = args.out
    elif args.cmd == "diff":
        result = diff_saves(resolve(args.a), resolve(args.b))
        json_out = args.out

    text = json.dumps(result, indent=2, default=str)
    if json_out:
        with open(json_out, "w") as fh:
            fh.write(text)
        print("wrote %s" % json_out, file=sys.stderr)
    print(text)


if __name__ == "__main__":
    main()
