#!/usr/bin/env python3
"""Re-derive road/rail connection behaviour from the user's original executable.

CIVTOT_HOME required. --emulate requires Unicorn installed outside this repo.
No original map or executable bytes are emitted.
"""
import argparse
from collections import deque
import hashlib
import json
import os
from pathlib import Path
import struct

from research_check import OriginalCost
from totctl import parse_rules, walk_save

DX = (1, 2, 1, 0, -1, -2, -1, 0)
DY = (-1, 0, 1, 2, 1, 0, -1, -2)


class Connections:
    def __init__(self, width, height, flat, tiles, diplomacy, blocked=()):
        self.width, self.height, self.flat = width, height, flat
        self.tiles, self.diplomacy = tiles, diplomacy
        self.blocked = set(blocked)

    def tile(self, x, y):
        if 0 <= x < self.width and 0 <= y < self.height:
            i = 6 * (y * (self.width // 2) + x // 2)
            return self.tiles[i:i + 6]
        return None

    def wrap(self, x):
        return x if self.flat else x % self.width

    def distance(self, x, y, tx, ty):
        dx, dy = abs(x - tx), abs(y - ty)
        if not self.flat and dx > self.width // 2:
            dx = self.width - dx
        m = (dx + dy) >> 1
        return max(dx, dy) - ((m - min(dx, dy) + 1) >> 1)

    def connection(self, civ, x, y, tx, ty):
        a, b = self.tile(x, y), self.tile(tx, ty)
        if a is None or b is None:
            return 0
        # 0x5701fc returns -1 on ocean; otherwise tile byte +3.
        if (-1 if a[0] & 15 == 10 else a[3]) != (-1 if b[0] & 15 == 10 else b[3]):
            return 0
        dx = abs(x - tx)
        if not self.flat and dx > self.width // 2:
            dx = self.width - dx
        if (dx + abs(y - ty)) // 2 >= 23:
            return 0
        # Reverse uniform-cost search, NOT a search restricted to roads.
        # The original stores offsets in a 48x48 diamond around the target.
        sx = tx + ((x - tx + self.width // 2) % self.width - self.width // 2) if not self.flat else x
        costs = {(tx, ty): 1}
        queue = deque([(tx, ty)])
        while queue:
            px, py = queue.popleft()
            for ox, oy in zip(DX, DY):
                nx, ny = px + ox, py + oy
                tile = self.tile(self.wrap(nx), ny)
                if abs(nx - tx) + abs(ny - ty) >= 48 or tile is None:
                    continue
                if tile[0] & 15 in self.blocked:
                    continue
                if tile[0] & 15 == 10 and (self.wrap(nx), ny) not in ((x, y), (tx, ty)):
                    continue
                if (nx, ny) not in costs:
                    costs[nx, ny] = costs[px, py] + 1
                    queue.append((nx, ny))
        rail = True
        for _ in range(51):
            current = costs.get((sx, y), 0)
            if not current or current >= 99:
                return 0
            best = None
            for direction, (ox, oy) in enumerate(zip(DX, DY)):
                nx, ny = sx + ox, y + oy
                # The selector bounds its raw (pre-wrap) neighbour even
                # though cost lookup uses the nearest wrapped coordinate.
                if abs(x + ox - tx) + abs(ny - ty) >= 48:
                    continue
                tile = self.tile(self.wrap(nx), ny)
                if tile is None or tile[0] & 15 in self.blocked:
                    continue
                cost = costs.get((nx, ny), 0)
                if not cost or cost >= current:
                    continue
                # 0x48d960 prefers the smaller geometric distance on ties.
                rank = (cost + 1, self.distance(self.wrap(nx), ny, tx, ty), direction)
                if best is None or rank < best[0]:
                    best = (rank, nx, ny)
            if best is None:
                return 0
            _, sx, y = best
            x = self.wrap(sx)
            if (x, y) == (tx, ty):
                return 2 if rail else 1
            tile = self.tile(x, y)
            owner = tile[5] >> 4
            city_owner = owner if tile[1] & 0xc2 == 2 and owner != 15 else -1
            if city_owner == civ:
                return 2 if rail else 1
            unit_owner = owner if tile[1] & 1 and owner != 15 else -1
            occupied = city_owner if city_owner >= 0 else unit_owner
            if city_owner >= 0 and not self.diplomacy[civ][city_owner] & 8:
                return 0
            if occupied > 0 and occupied != civ and not self.diplomacy[civ][occupied] & 8:
                return 0
            if not tile[1] & 16:
                return 0
            rail &= bool(tile[1] & 32)
        return 0


class OriginalConnection(OriginalCost):
    def load(self, model):
        self.write(0x6705a0, model.width, '<h')
        self.write(0x6705a2, model.height, '<h')
        self.write(0x607638, 0x8000 if model.flat else 0, '<H')
        self.write(0x6762f8, 0, '<I')
        self.write(0x5deaa8, 0x700000, '<I')
        self.cpu.mem_write(0x700000, bytes(model.tiles))
        # This probe covers land unit type 2. Terrain passability is an
        # explicit input; the original callees execute without stubs.
        self.cpu.mem_write(0x5f4700 + 80, bytes(40))
        self.cpu.mem_write(0x66f8c0, bytes(11 * 24))
        for terrain in model.blocked:
            self.write(0x66f8c6 + 24 * terrain, -2, '<b')
        for civ, row in enumerate(model.diplomacy):
            self.cpu.mem_write(0x5f68c8 + 3348 * civ, struct.pack('<8I', *row))
        self.write(0x62d3ec, -1, '<i')
        self.write(0x628bd8, -1, '<i')
        self.write(0x5d5858, 0, '<I')

    def connection(self, *args):
        self.cpu.mem_write(0x108000, struct.pack('<6I', 0x100000, *args))
        self.cpu.reg_write(self.sp, 0x108000)
        self.cpu.emu_start(0x471a13, 0x100000, count=20_000_000)
        assert self.cpu.reg_read(self.ip) == 0x100000, 'original connection did not return'
        return self.cpu.reg_read(self.ax)


def saved_model(path, rules_path):
    data = path.read_bytes()
    walk = walk_save(str(path))
    width, height = walk['map_decode']['display_dims']
    surface = width * height // 2
    start = walk['sections_derived']['map_data']['offset'] + 7 * surface
    diplomacy = [[struct.unpack_from('<I', data, 2944 + 3348 * c + 40 + 4 * o)[0]
                  for o in range(8)] for c in range(8)]
    model = Connections(width, height, bool(struct.unpack_from('<H', data, 652)[0] & 0x8000),
                        data[start:start + 6 * surface], diplomacy)
    rules = parse_rules(str(rules_path))
    assert int(rules['UNITS']['rows'][2][2]) == 0, 'probe requires land unit type 2'
    advanced = rules.get('UNITS_ADVANCED', {}).get('rows', [])
    ignores = len(advanced) > 2 and int(advanced[2][6], 2) & 16
    if not ignores:
        model.blocked = {i for i, row in enumerate(rules['TERRAIN']['rows'][:11])
                         if len(row) > 15 and row[15].strip().lower() == 'yes'}
    start = walk['cities']['offset']
    cities = [data[start + 92 * i:start + 92 * (i + 1)] for i in range(walk['cities']['count'])]
    return model, cities


def synthetic_cases():
    """Invented maps: exercise branches without copying game assets."""
    specs = [
        ('road', (1, 8, 8, 16, 8), False, [], []),
        ('rail', (1, 8, 8, 16, 8), True, [], []),
        ('broken-road', (1, 8, 8, 16, 8), False, [(12, 8, 1, 0)], []),
        ('foreign-unit', (1, 8, 8, 16, 8), False, [(12, 8, 1, 17), (12, 8, 5, 32)], []),
        ('foreign-unit-permitted', (1, 8, 8, 16, 8), False, [(12, 8, 1, 17), (12, 8, 5, 32)], [(1, 2, 8)]),
        ('friendly-intermediate-city', (1, 8, 8, 16, 8), False, [(10, 8, 1, 2), (10, 8, 5, 16), (12, 8, 1, 0)], []),
        ('foreign-city', (1, 8, 8, 16, 8), False, [(10, 8, 1, 2), (10, 8, 5, 32)], []),
        ('different-landmass', (1, 8, 8, 16, 8), False, [(16, 8, 3, 2)], []),
        ('endpoint-needs-no-road', (1, 8, 8, 10, 8), False, [(10, 8, 1, 0)], []),
        ('wrap', (1, 2, 8, 62, 8), False, [], []),
        ('wrap-after-detour', (1, 4, 8, 50, 8), False, [], []),
        ('sea-barrier', (1, 8, 8, 16, 8), False,
         [(12 + (y & 1), y, 0, 10) for y in range(16)], []),
    ]
    for name, inputs, rail, edits, treaties in specs:
        tiles = bytearray([1, 48 if rail else 16, 0, 1, 255, 240] * (64 * 16 // 2))
        for x, y, field, value in edits:
            tiles[6 * (y * 32 + x // 2) + field] = value
        diplomacy = [[0] * 8 for _ in range(8)]
        for civ, other, flags in treaties:
            diplomacy[civ][other] = flags
        yield name, inputs, Connections(64, 16, False, tiles, diplomacy)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--emulate', action='store_true')
    parser.add_argument('--vectors', type=Path, help='write derived save/city connection cases')
    args = parser.parse_args()
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    emulator = OriginalConnection(binary) if args.emulate else None
    if args.vectors and not emulator:
        parser.error('--vectors requires --emulate')
    vectors = []
    result = {'source': 'civ2.bak 0x471a13, path search 0x48c9c5',
              'source_sha256': hashlib.sha256(binary).hexdigest(),
              'scope': 'surface map; land unit type 2; Rules terrain passability; no Wine run',
              'synthetic': [], 'saves': []}
    for name, inputs, model in synthetic_cases():
        value = model.connection(*inputs)
        if emulator:
            emulator.load(model)
            assert value == emulator.connection(*inputs), name
        result['synthetic'].append({'case': name, 'connection': value})
    for name in ('Original/Ph_Auto2.SAV', 'FallRome/ROMFALL.scn'):
        model, cities = saved_model(home / name, home / Path(name).parent / 'Rules.txt')
        if emulator:
            emulator.load(model)
        rows = []
        for i, city in enumerate(cities):
            x, y, map_id = struct.unpack_from('<hhh', city)
            if map_id:
                continue
            capitals = [(k, c) for k, c in enumerate(cities) if c[10] == city[10] and c[54] & 2]
            if not capitals:
                continue
            k, capital = min(capitals, key=lambda kc: model.distance(x, y, *struct.unpack_from('<hh', kc[1])))
            tx, ty = struct.unpack_from('<hh', capital)
            if (x, y) == (tx, ty):
                continue
            inputs = (city[10], x, y, tx, ty)
            value = model.connection(*inputs)
            if emulator:
                original = emulator.connection(*inputs)
                assert value == original, (name, i, k, inputs, value, original)
            rows.append([i, k, value])
            vectors.append(f'{name} {i} {k} {value}')
        result['saves'].append({'save': name, 'blocked_terrain': sorted(model.blocked),
                                'columns': ['city', 'capital', 'connection'], 'rows': rows})
    if args.vectors:
        args.vectors.write_text('# save city capital connection\n' + '\n'.join(vectors) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
