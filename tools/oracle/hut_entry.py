#!/usr/bin/env python3
"""Controlled live hut entry, with reproducible gameplay CRT state.

Load Original/Ph_Auto2.SAV first (load_save.sh). This changes live RAM only;
never run on somebody else's oracle session. Entry/exit/RNG hooks and the
startup cave are restored in finally. Optional --far teleports the selected
unit to an empty site away from cities, and --fertility/--year/--level pin
branch inputs. The three proven presentation rand calls use a separate CRT stream;
otherwise reward dialogs change later gameplay rolls. Every call is logged.
No save, sprite, map bytes, city names or executable bytes are emitted.
"""
import argparse
import json
import os
from pathlib import Path
import re
import struct
import subprocess
import time

import randhook as rh
import unitram as u

CAVE, ENTRY, EXIT = 0x41562b, 0x54ab80, 0x54bddc
BUF, CRT = 0x607c60, 0x5e2590
RENDER_CALLS = (0x55852f, 0x581d96, 0x582b9b)


def p32(value):
    return struct.pack('<I', value)


def branch(opcode, source, target):
    return bytes([opcode]) + struct.pack('<i', target - source - 5)


def store(address):
    return b'\xa3' + p32(address)


def immediate(address, value):
    return b'\xc7\x05' + p32(address) + p32(value)


def wi(*args):
    env = {**os.environ, 'DISPLAY': ':99',
           'WINEPREFIX': os.environ.get('WINEPREFIX', os.path.expanduser('~/.wine_civ2tot'))}
    return subprocess.run(['timeout', '15', 'wine', str(Path(__file__).with_name('winject.exe')),
                           *map(str, args)], env=env, capture_output=True, text=True, check=True)


def distance(width, flat, x, y, tx, ty):
    dx, dy = abs(x - tx), abs(y - ty)
    if not flat:
        dx = min(dx, width - dx)
    middle = (dx + dy) // 2
    return max(dx, dy) - ((middle - min(dx, dy) + 1) // 2)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--seed', required=True, type=lambda s: int(s, 0))
    parser.add_argument('--unit', default=9, type=int)
    parser.add_argument('--far', action='store_true')
    parser.add_argument('--near-own', action='store_true')
    parser.add_argument('--home-settlers', action='store_true')
    parser.add_argument('--turn', type=int)
    parser.add_argument('--fertility', type=int, choices=range(16))
    parser.add_argument('--year', type=int)
    parser.add_argument('--level', type=int, choices=range(4))
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    windows = wi('wins').stdout
    window = next((re.match(r'hwnd=(\w+)', line).group(1) for line in windows.splitlines()
                   if 'vis=1' in line and 'Civilization II: Test of Time' in line), None)
    assert window, 'game window missing'
    wi('focus', '0x' + window)
    subprocess.run(['python3', str(Path(__file__).with_name('unitram.py')), 'select',
                    str(args.unit), '30'], check=True)
    units, types = u.bases()
    assert units != 0x607c58, 'scratch buffer needs ToTPP-relocated units'
    assert u.read(0x54afb2, 3) == bytes.fromhex('88048d'), 'city-base instruction changed'
    cities_base = u.u32(0x54afb2 + 3) - 11
    assert u.read(0x54bb88, 3) == bytes.fromhex('0fbe05'), 'global tech-mask load changed'
    tech_masks = u.u32(0x54bb88 + 3) - 55
    def knowledge(owner):
        packed = [0] * 13
        for tech, mask in enumerate(u.read(tech_masks, 100)):
            if mask & (1 << owner):
                packed[tech // 8] |= 1 << (tech % 8)
        return packed
    n_units, n_cities = u.u16(0x607666), u.u16(0x607668)
    raw_units, raw_cities = u.read(units, 40 * n_units), u.read(cities_base, 92 * n_cities)
    records = [raw_units[40 * i:40 * (i + 1)] for i in range(n_units)]
    city_records = [raw_cities[92 * i:92 * (i + 1)] for i in range(n_cities)]
    r = records[args.unit]
    civ = r[9] & 15
    assert civ and u.read(types + 40 * r[8] + 9, 1) == b'\x00', 'land player unit required'
    width, height, seed = u.u16(0x6705a0), u.u16(0x6705a2), u.u16(0x6705a8)
    map_base = u.u32(0x5deaa8)
    tiles = u.read(map_base, 6 * width * height // 2)
    flags = u.u16(0x607638)
    old_x, old_y = struct.unpack_from('<hh', r)
    alive = [(struct.unpack_from('<hh', v), struct.unpack_from('<I', v, 30)[0]) for v in records]
    city_positions = [struct.unpack_from('<hh', c) for c in city_records if struct.unpack_from('<I', c, 86)[0]]

    def tile(x, y):
        i = 6 * (y * (width // 2) + x // 2)
        return tiles[i:i + 6]

    def hut(x, y):
        a = (x + y) // 2
        b = x - a
        return (a & 3) + 4 * (b & 3) == (seed + 13 * (b >> 2) + 11 * (a >> 2) + 8) & 31

    if args.far:
        candidates = []
        for y in range(4, height - 4):
            for x in range(y & 1, width, 2):
                sy = y - 2
                if not hut(x, y) or tile(x, y)[0] & 15 == 10 or tile(x, sy)[0] & 15 == 10:
                    continue
                if tile(x, y)[1] & 3 or tile(x, sy)[1] & 3:
                    continue
                near = min((distance(width, bool(flags & 0x8000), x, y, *c) for c in city_positions), default=9999)
                own = [struct.unpack_from('<hh', c) for c in city_records
                       if struct.unpack_from('<I', c, 86)[0] and c[10] == civ]
                own_near = min((distance(width, bool(flags & 0x8000), x, y, *c) for c in own), default=9999)
                if near >= 4 and (not args.near_own or (own_near < 24 and u.read(0x5f6e70 + 3348 * civ + tile(x, y)[3], 1)[0] != 0)):
                    candidates.append((near, x, y))
        assert candidates, 'no empty distant hut/source pair'
        _, tx, ty = max(candidates)
        sx, sy = tx, ty - 2
        assert sum(1 for xy, uid in alive if uid and xy == (old_x, old_y)) == 1, 'teleport source has a stack'
        old_tile = map_base + 6 * (old_y * (width // 2) + old_x // 2)
        u.write(old_tile + 1, bytes([tile(old_x, old_y)[1] & ~1]))
        u.write(units + 40 * args.unit, struct.pack('<hh', sx, sy))
        u.write(units + 40 * args.unit + 26, b'\xff' * 4)
        source_tile = map_base + 6 * (sy * (width // 2) + sx // 2)
        u.write(source_tile + 1, bytes([tile(sx, sy)[1] | 1]))
        u.write(source_tile + 5, bytes([(civ << 4) | (tile(sx, sy)[5] & 15)]))
    else:
        sx, sy = old_x, old_y
        tx, ty = sx, sy + 2
        assert hut(tx, ty), 'south neighbour is not selected by the hut hash'
    assert 0 <= ty < height and tile(tx, ty)[1] & 3 == 0
    if args.turn is not None:
        u.write(0x607648, struct.pack('<h', args.turn))
    if args.year is not None:
        u.write(0x60764a, struct.pack('<h', args.year))
    if args.level is not None:
        u.write(0x607659, bytes([args.level]))
    target_tile = map_base + 6 * (ty * (width // 2) + tx // 2)
    fertility = args.fertility if args.fertility is not None else tile(tx, ty)[5] & 15
    u.write(target_tile + 5, bytes([0xf0 | fertility]))
    homed = []
    if args.home_settlers:
        home = next(i for i, c in enumerate(city_records) if c[10] == civ and struct.unpack_from('<I', c, 86)[0])
        for i, v in enumerate(records):
            if v[9] & 15 == civ and struct.unpack_from('<I', v, 30)[0] and v[18] == 255 and u.read(types + 40 * v[8] + 18, 1)[0] == 5:
                u.write(units + 40 * i + 18, bytes([home]))
                homed.append([i, home])
    u.ready(args.unit, units)
    before = {'unit': args.unit, 'source': [sx, sy], 'original_source': [old_x, old_y],
              'target': [tx, ty], 'type': r[8], 'civ': civ, 'seed': args.seed,
              'homed_settlers': homed, 'turn': u.u16(0x607648), 'year': struct.unpack('<h', u.read(0x60764a, 2))[0],
              'difficulty': u.read(0x607658, 1)[0], 'level': u.read(0x607659, 1)[0],
              'fertility': fertility, 'site_bits': tile(tx, ty)[2] >> 5,
              'landmass': tile(tx, ty)[3], 'treasury': struct.unpack('<i', u.read(0x5f68aa + 3348 * civ, 4))[0],
              'techs': knowledge(civ), 'units': n_units, 'cities': n_cities}
    entry_old, exit_old, rand_old = u.read(ENTRY, 6), u.read(EXIT, 5), u.read(rh.RAND, 5)
    assert entry_old == bytes.fromhex('558bec83ec74') and exit_old == bytes.fromhex('5f5e5bc9c3')
    assert u.read(rh.RAND, 6) == rh.RAND_PROLOGUE
    call_old = [u.read(a, 5) for a in RENDER_CALLS]
    assert all(v == branch(0xe8, a, rh.RAND) for a, v in zip(RENDER_CALLS, call_old))
    code = immediate(BUF + 32, 0) + b'\x50\x8b\x44\x24\x08' + store(BUF)
    code += immediate(BUF + 4, args.seed) + immediate(CRT, args.seed) + b'\x58' + entry_old
    code += branch(0xe9, CAVE + len(code), ENTRY + 6)
    exit_addr = CAVE + len(code)
    exitcode = b'\x50\xa1' + p32(CRT) + store(BUF + 8)
    for offset, address in ((-0x58, BUF + 12), (8, BUF + 16), (-0x5c, BUF + 20), (-0x20, BUF + 24)):
        exitcode += b'\x8b\x45' + bytes([offset & 255]) + store(address)
    exitcode += b'\xa1' + p32(BUF + 32) + store(BUF + 36) + immediate(BUF + 28, 1) + b'\x58' + exit_old
    code += exitcode
    # Swap a separate stream around the identified presentation calls.
    # The actual original rand() executes; mechanics state is restored.
    graphics_addr = CAVE + len(code)
    graphics = b'\x51\x52\xff\x35' + p32(CRT) + b'\x8b\x15' + p32(BUF + 40)
    graphics += b'\x89\x15' + p32(CRT)
    graphics_call_return = graphics_addr + len(graphics) + 5
    graphics += branch(0xe8, graphics_addr + len(graphics), rh.RAND)
    graphics += b'\x8b\x15' + p32(CRT) + b'\x89\x15' + p32(BUF + 40)
    graphics += b'\x5a\x89\x15' + p32(CRT) + b'\x5a\x59\xc3'
    code += graphics
    rh.CAVE, rh.IDX, rh.BUF, rh.CAP = CAVE + len(code), BUF + 32, BUF + 64, 1000
    code += rh.cave_code(state=True)
    assert len(code) < 512, 'startup cave overflow'
    cave_old, buf_old = u.read(CAVE, len(code)), u.read(BUF, 8064)
    try:
        u.write(BUF, bytes(8064))
        u.write(BUF + 40, p32(0x7351))
        u.write(CAVE, code)
        u.write(rh.RAND, branch(0xe9, rh.RAND, rh.CAVE))
        for address in RENDER_CALLS:
            u.write(address, branch(0xe8, address, graphics_addr))
        u.write(EXIT, branch(0xe9, EXIT, exit_addr))
        u.write(ENTRY, branch(0xe9, ENTRY, CAVE) + b'\x90')
        wi('key', 0x62)
        deadline = time.monotonic() + 40
        while not u.u32(BUF + 28) and time.monotonic() < deadline:
            time.sleep(.3)
            if not u.u32(BUF + 28):
                wi('key', 13)
        words = list(struct.unpack('<8I', u.read(BUF, 32)))
        assert words[7] == 1 and words[0] == args.unit, f'hut did not complete: {words}'
        draws = list(struct.iter_unpack('<II', u.read(BUF + 64, 8 * min(u.u32(BUF + 36), 1000))))
        gameplay = [[hex(a), s] for a, s in draws if a != graphics_call_return]
        new_units = []
        for i in range(u.u16(0x607666)):
            v = u.read(units + 40 * i, 40)
            uid = struct.unpack_from('<I', v, 30)[0]
            old_id = struct.unpack_from('<I', records[i], 30)[0] if i < n_units else 0
            if uid and uid != old_id:
                x, y = struct.unpack_from('<hh', v)
                new_units.append({'index': i, 'type': v[8], 'owner': v[9] & 15, 'x': x, 'y': y,
                                  'moves_spent': v[10], 'home': v[18], 'stack_links': list(struct.unpack_from('<hh', v, 26)), 'veteran': bool(struct.unpack_from('<H', v, 6)[0] & 0x2000)})
        assert u.u16(0x607648) == before['turn'], 'turn advanced during fixture'
        after = {'hook_words': words, 'gameplay_draws': gameplay,
                 'renderer_draws': sum(a == graphics_call_return for a, _ in draws),
                 'treasury': struct.unpack('<i', u.read(0x5f68aa + 3348 * civ, 4))[0],
                 'techs': knowledge(civ),
                 'units': u.u16(0x607666), 'cities': u.u16(0x607668), 'new_units': new_units,
                 'explorer_home': u.read(units + 40 * args.unit + 18, 1)[0], 'explorer_stack_links': list(struct.unpack('<hh', u.read(units + 40 * args.unit + 26, 4)))}
        if words[3] == 0:
            c = u.read(cities_base + 92 * words[5], 92)
            after['new_city'] = {'index': words[5], 'size': c[11], 'owner': c[10], 'buildings': list(c[54:59])}
        result = {'scope': 'live ToTPP oracle; CRT pinned at hut entry; three presentation callers use a separate CRT stream',
                  'before': before, 'after': after}
        args.output.write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps({'case': words[3], 'gameplay_draws': len(gameplay), 'renderer_draws': after['renderer_draws'],
                          'crt_after': words[2], 'new_units': new_units, 'output': str(args.output)}))
    finally:
        u.write(ENTRY, entry_old)
        u.write(EXIT, exit_old)
        u.write(rh.RAND, rand_old)
        for address, data in zip(RENDER_CALLS, call_old):
            u.write(address, data)
        u.write(CAVE, cave_old)
        u.write(BUF, buf_old)


if __name__ == '__main__':
    main()
