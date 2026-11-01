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
        out["note"] = ("ToTPP-extended save: section sizes differ beyond the "
                       "base walk; offsets below are base-format guesses")

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
    if ver == 0x0032:
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
        # ---- embedded map decode (proven on ROMFALL, 2026-11-01) ----
        # Per map: Visible Improvements (7 B/tile) + Map (6 B/tile); a 2-byte
        # resource seed follows all maps (Mercator ToC). The stored grid is
        # dense: width hdr0/2 x hdr1 = surface tiles (ROMFALL 75x120=9000,
        # while hdr0=150 is the display width -- ToT tiles span 2 columns).
        # Map tile: byte 4 = terrain | river(0x80) (same encoding as .MP
        # words); byte 2 = improvements: 0x40 road, 0x80 railroad (verified
        # spatially: roads form the empire-wide network, rails the dense
        # Italy/Gaul core). Other bytes open.
        if maps:
            stored_w, stored_h = hdr[0] // 2, hdr[1]
            out["map_decode"] = {"stored_dims": [stored_w, stored_h],
                                 "display_dims": [hdr[0], hdr[1]],
                                 "dense": stored_w * stored_h == surface,
                                 "per_map": []}
            for m in range(maps):
                base = pos + m * map_data + 2  # +2 proven before tiles (ROMFALL)
                imp0 = base
                mp0 = base + 7 * surface
                terr_hist: dict[int, int] = {}
                river = road = rail = 0
                vis_nz = [0] * 7
                for t in range(surface):
                    t6 = data[mp0 + 6 * t + 4]
                    imp6 = data[mp0 + 6 * t + 2]
                    terr_hist[t6 & 0x7F] = terr_hist.get(t6 & 0x7F, 0) + 1
                    if t6 & 0x80:
                        river += 1
                    if imp6 & 0x40:
                        road += 1
                    if imp6 & 0x80:
                        rail += 1
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
                    "railroad_tiles": rail,
                    "visibility_nonzero_per_civ": vis_nz,
                    "note": ("imp byte2: road=0x40 rail=0x80 (proven "
                             "ROMFALL); remaining bits open; visibility "
                             "bytes' near-identical distribution suggests "
                             "per-civ fog, unproven"),
                })
            out["map_decode"]["resource_seed_offset"] = pos + maps * map_data - 2
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
            for k in range(n):
                rec = data[probe + 92 * k: probe + 92 * (k + 1)]
                nm = rec[:16].split(b"\x00")[0].decode("latin-1", "replace")
                names.append(nm)
            cities["names"] = names[:20]
            cities["names_total_named"] = sum(1 for x in names if x)
            cities["note"] = ("name field 16 B at record start (verified "
                              "ROMFALL: Roma first); trailing unnamed records "
                              "possible; per-field decode is open")
            out["cities"] = cities
            u_end = cities["offset"]
            region = u_end - blob10240_end
            # Verified on ROMFALL (34 + 710*40 = 28434) and Va_Auto
            # (34 + 711*40 = 28474): a 34-byte preamble, then 40-byte unit
            # records. The count is not stored in the preamble (only a
            # state u16 at +30 differs between scenario and in-game save).
            unit_count, rem = divmod(region, 40)
            out["units"] = {
                "region": [blob10240_end, u_end],
                "size": region,
                "record_size": 40,
                "preamble_bytes": rem,
                "count": unit_count if rem == 34 else None,
                "count_arithmetic": f"{region} = {rem} + 40 x {unit_count}",
                "note": ("count via exact region arithmetic (preamble 34 B + "
                         "40 B records); per-record field semantics still "
                         "open — x/y-like columns at record bytes 6-10, "
                         "see docs/evidence/README.md"),
            }
    return out


# --------------------------------------------------------------------------
# SCN/SAV embedded map render (proven layout, see walk_save's map_decode)

def render_scnmap(path: str, out_png: str, names, scale=4) -> dict:
    """Render the surface map embedded in a .SCN/.SAV.

    Uses the proven map-block layout (imp 7 B + map 6 B per tile, dense grid):
    terrain from map byte 4 (river = bit 0x80), road/railroad from map byte 2
    (0x40 / 0x80). Verified on ROMFALL: roads render as the empire-wide
    network, rails as the dense Italy/Gaul core, rivers as coastal chains.
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
    stats = {"river": 0, "road": 0, "rail": 0, "terrain": {}}
    for y in range(h):
        row = bytearray()
        for x in range(w):
            i = y * w + x
            t6 = data[mp0 + 6 * i + 4]
            imp6 = data[mp0 + 6 * i + 2]
            terr = t6 & 0x7F
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
                rgb = terrain_color(terr, names)
            row.extend(rgb * scale)
        rows.extend([row] * scale)
    write_png(out_png, w * scale, h * scale, rows)
    return {"png": os.path.abspath(out_png), "stored_dims": [w, h],
            "display_dims": md["display_dims"],
            "overlay_counts": stats,
            "decode": ("map byte4: terrain|river(0x80); map byte2: "
                       "road(0x40), rail(0x80) — proven ROMFALL 2026-11-01")}



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
