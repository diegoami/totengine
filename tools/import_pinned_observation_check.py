#!/usr/bin/env python3
"""Execute the native observation loop on the pinned saved map and four prepared positions."""

import argparse
import json
import os
from pathlib import Path
import struct

from import_event_tail_check import digest
from import_input_handoff_check import AFTER_OBSERVATION, OBSERVATION_HELPERS, OBSERVE
from import_phase_check import read, run
from import_unit_setup_check import CALLS, UNIT_RECORDS, prepare

TILES = 0x700000
CASES = [
    (6, None), (9, None), (18, None), (26, None),
    (6, (62, 4)), (6, (62, 6)), (6, (63, 7)), (18, (50, 10)),
]


def relocate(original, width, slot, player, destination):
    record = UNIT_RECORDS + slot * 40
    x, y = (read(original, record + offset, '<h') for offset in (0, 2))
    tx, ty = destination
    assert (abs(tx - x), abs(ty - y)) in ((1, 1), (2, 0), (0, 2))
    assert read(original, record + 9, '<B') == player
    assert bytes(original.cpu.mem_read(record + 26, 4)) == b'\xff' * 4
    old = TILES + 6 * (y * (width // 2) + x // 2)
    new = TILES + 6 * (ty * (width // 2) + tx // 2)
    old_flags, new_flags = (read(original, tile + 1, '<B') for tile in (old, new))
    assert old_flags & 3 == 1 and new_flags & 3 == 0
    assert read(original, new, '<B') & 63 != 10
    original.write(old + 1, old_flags & ~1, '<B')
    original.write(old + 5, read(original, old + 5, '<B') | 0xf0, '<B')
    original.write(new + 1, new_flags | 1, '<B')
    original.write(new + 5, (read(original, new + 5, '<B') & 15) | (player << 4), '<B')
    original.write(record, tx, '<h')
    original.write(record + 2, ty, '<h')


def difference(before, after):
    return ','.join(f'{i}:{a}:{b}' for i, (a, b) in enumerate(zip(before, after, strict=True))
                    if a != b) or '-'


def observe(home, binary, path, slot, destination):
    original, player, _, records, _ = prepare(home, binary, path, exhaust=False)
    width, height = (read(original, at, '<h') for at in (0x6705A0, 0x6705A2))
    area = width * height // 2
    original.write(0x6705A4, area, '<h')
    original.write(0x5D156C, 1, '<B')
    if destination is not None:
        relocate(original, width, slot, player, destination)
    caches = [(read(original, 0x5DEAD0 + 4 * civ, '<I'), area) for civ in range(1, 8)]
    cpu = original.cpu
    before_records = [bytes(cpu.mem_read(addr, size)) for addr, size in records]
    before_tiles = cpu.mem_read(TILES, 6 * area)
    before_cache = b''.join(cpu.mem_read(addr, size) for addr, size in caches)
    cpu.mem_write(0x108000, struct.pack('<III', 0x100000, slot, 0))
    cpu.reg_write(original.sp, 0x108000)
    run(original, OBSERVE, {AFTER_OBSERVATION}, calls=OBSERVATION_HELPERS + CALLS + (0x570169,))
    assert [bytes(cpu.mem_read(addr, size)) for addr, size in records] == before_records
    after_tiles = cpu.mem_read(TILES, 6 * area)
    after_cache = b''.join(cpu.mem_read(addr, size) for addr, size in caches)
    delta_tiles = difference(before_tiles, after_tiles)
    delta_cache = difference(before_cache, after_cache)
    assert (delta_tiles == delta_cache == '-') == (destination is None)
    x, y = destination or ('-', '-')
    return f'{slot} {x} {y} {delta_tiles} {delta_cache}'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    pin = json.loads((root / 'docs/evidence/import-input-handoff.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    path = pin['source']['path']
    assert digest(binary) == pin['binary_sha256']
    assert digest((home / path).read_bytes()) == pin['source']['sha256']
    assert digest((home / pin['rules']['path']).read_bytes()) == pin['rules']['sha256']
    lines = [
        f"# binary_sha256 {pin['binary_sha256']}",
        f"# save_sha256 {pin['source']['sha256']}",
        f"# rules_sha256 {pin['rules']['sha256']}",
        '# native 0041f704 to 004205a7; active_contacts=0; no replaced callees',
        '# unit_slot destination_x destination_y tile_byte_deltas cache_byte_deltas',
        '# deltas are offset:before:after; civ/unit/city records remain unchanged',
        *(observe(home, binary, path, slot, destination) for slot, destination in CASES),
    ]
    with args.out.open('x') as stream:
        stream.write('\n'.join(lines) + '\n')


if __name__ == '__main__':
    main()
