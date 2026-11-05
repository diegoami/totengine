#!/usr/bin/env python3
"""Validate expanded seeded_start captures; emit numeric/hash provenance only.

Usage: worldgen_capture_manifest.py CAPTURE_ROOT LEGACY_RG2_ROOT
Requires CIVTOT_HOME for executable/Rules identity. Raw saves, screenshots
and RAM dumps remain outside Git. This does not run or change the game.
"""
import hashlib
from collections import defaultdict
import json
import os
from pathlib import Path
import struct
import sys

HERE = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save_fields(path):
    data = path.read_bytes()
    count = struct.unpack_from("<I", data, 29728 + 168)[0]
    offset = 29728 + 172 + count * 14
    header = struct.unpack_from("<8h", data, offset)
    width, height, area = header[:3]
    assert width == 80 and height == 50 and area == 2000, (path, header)
    tiles = data[offset + 16 + area * 7:offset + 16 + area * 13]
    assert len(tiles) == area * 6
    return dict(turn=struct.unpack_from("<h", data, 668)[0],
                year=struct.unpack_from("<h", data, 670)[0], difficulty=data[684],
                rivals=data[689], barbarian_level=data[685], family=data[982],
                resource_seed=header[4], advance_count=struct.unpack_from("<h", data, 702)[0]), tiles


def main(root, legacy):
    root, legacy = Path(root), Path(legacy)
    home = Path(os.environ["CIVTOT_HOME"])
    protocol_path = HERE / "docs/evidence/worldgen-comparison-protocol.json"
    protocol = json.loads(protocol_path.read_text())
    rows = []
    for i, seed in enumerate(protocol["original_settings"]["new_crt_seeds"]):
        directory = root / ("original-0b" if i == 0 else f"original-{i}")
        log = directory.with_suffix(".log").read_text()
        assert f"pinned NR=0x1234 CRT={int(seed, 0):#x} params=[0, 0, 0, 0, 0] mapseed=0x1357" in log
        assert "intro dump: 80x50, 2000 tiles" in log
        assert "no fresh save" not in log and "timed out" not in log
        candidates = sorted(p for p in directory.glob("Original_*") if p.suffix.lower() == ".sav")
        assert candidates, directory
        source = candidates[0]
        fields, tiles = save_fields(source)
        assert fields == dict(turn=1, year=-4000, difficulty=4, rivals=4,
                              barbarian_level=2, family=0, resource_seed=0x1357, advance_count=89), fields
        assert source.stat().st_mtime_ns >= (directory / ".marker").stat().st_mtime_ns
        intro = (directory / "intro_tiles.bin").read_bytes()
        assert len(intro) == len(tiles)
        assert bytes(v & 0x8f for v in intro[::6]) == bytes(v & 0x8f for v in tiles[::6])
        # Native region IDs may include both land and associated water. Check
        # words +0/+2/+4 against every tile carrying each observed region ID.
        table = (directory / "intro_landmass.bin").read_bytes()
        counts = defaultdict(lambda: [0, 0, 0])
        for at in range(0, len(intro), 6):
            terrain, region = intro[at] & 15, intro[at + 3]
            counts[region][0] += terrain != 10
            counts[region][1] += terrain in (1, 2)
            counts[region][2] += terrain == 10
        regions = []
        for region, count in sorted(counts.items()):
            assert tuple(count) == struct.unpack_from("<3h", table, region * 40)
            regions.append(dict(id=region, land=count[0], plains_grassland=count[1], water=count[2]))
        rows.append(dict(label=f"original-{i}", directory=directory.name, file=source.name,
                         sha256=sha(source), intro_tiles_sha256=sha(directory / "intro_tiles.bin"),
                         crt=seed, regions=regions, **fields))
    old = json.loads((HERE / "docs/evidence/worldgen-v1.json").read_text())
    for record in old["original_inputs"]:
        if record["label"] not in protocol["original_settings"]["retained_reference_labels"]:
            continue
        source = legacy / record["label"] / record["file"]
        assert sha(source) == record["sha256"]
        fields, _ = save_fields(source)
        rows.append(dict(label=record["label"], directory=record["label"], file=record["file"],
                         sha256=record["sha256"], crt=record["crt"], **fields))
    assert len({row["sha256"] for row in rows}) == len(rows), "duplicate original saves"
    print(json.dumps(dict(protocol_sha256=sha(protocol_path),
                          executable_sha256=sha(home / "civ2.exe"),
                          unpatched_executable_sha256=sha(home / "civ2.bak"),
                          rules_sha256=sha(home / "Original/Rules.txt"), sources=rows,
                          checks="Matched profile, fresh turn-1 saves; all new terrain/river cells equal pre-AI RAM capture; legacy hashes rechecked."), indent=2))


if __name__ == "__main__":
    main(*sys.argv[1:])
