#!/usr/bin/env python3
"""Check route arithmetic by executing the original route loop in civ2.bak.

CIVTOT_HOME required; --emulate requires Unicorn installed outside the repo.
The loop 0x4c46bf..0x4c4890 executes with original building/connection callees.
City refresh and final corruption are outside this isolated arithmetic check.
No original assets are emitted.
"""
import argparse
import hashlib
import itertools
import json
import os
from pathlib import Path
import struct

from connections_check import OriginalConnection, saved_model, synthetic_cases


def income(source, partner, commodity, domestic, connection, airports, highways):
    factor = max(connection, int(airports)) + int(highways)
    base = (source + partner + 4) >> 3
    value = base + ((base * factor) >> 1)
    if commodity < 0:
        value = 0
    return value >> 1 if domestic else value


class OriginalRoutes(OriginalConnection):
    def routes(self, cities, index, tile_trade):
        from unicorn.x86_const import UC_X86_REG_EBP
        for i, city in enumerate(cities):
            self.cpu.mem_write(0x5fd150 + 92 * i, bytes(city))
        frame = 0x108100
        self.write(frame + 8, index, '<I')
        self.write(frame - 0x28, cities[index][10], '<I')
        self.write(0x653850, tile_trade, '<i')
        self.cpu.mem_write(0x653810, bytes(12))
        self.cpu.reg_write(UC_X86_REG_EBP, frame)
        self.cpu.reg_write(self.sp, frame - 0x100)
        self.cpu.emu_start(0x4c46bf, 0x4c4890, count=20_000_000)
        assert self.cpu.reg_read(self.ip) == 0x4c4890, 'route loop did not finish'
        return list(struct.unpack('<3i', self.cpu.mem_read(0x653810, 12)))


def city(x, y, owner, trade, airport=False, highways=False):
    result = bytearray(92)
    struct.pack_into('<hh', result, 0, x, y)
    result[10] = owner
    struct.pack_into('<h', result, 32, trade)
    for building, present in ((32, airport), (25, highways)):
        if present:
            result[54 + building // 8] |= 1 << (building % 8)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--emulate', action='store_true')
    parser.add_argument('--vectors', type=Path)
    args = parser.parse_args()
    if args.vectors and not args.emulate:
        parser.error('--vectors requires --emulate')
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    emulator = OriginalRoutes(binary) if args.emulate else None
    models = {name: model for name, _, model in synthetic_cases()}
    vectors = []
    for connection, airports, highways, domestic, commodity, trades in itertools.product(
            range(3), (False, True), (False, True), (False, True), (-1, 0, 15),
            ((0, 0), (3, 8), (12, 21), (41, 50))):
        model = models[{0: 'broken-road', 1: 'road', 2: 'rail'}[connection]]
        source, partner = trades
        cities = [city(8, 8, 1, source, airports, highways),
                  city(16, 8, 1 if domestic else 2, partner, airports)]
        cities[0][60] = 1
        struct.pack_into('<b', cities[0], 67, commodity)
        struct.pack_into('<h', cities[0], 70, 1)
        value = income(source, partner, commodity, domestic, connection, airports, highways)
        if emulator:
            emulator.load(model)
            assert emulator.routes(cities, 0, 0) == [value, 0, 0]
        vectors.append([source, partner, commodity, int(domestic), connection,
                        int(airports), int(highways), value])
    saves = []
    for name in ('Original/Ph_Auto2.SAV', 'FallRome/ROMFALL.scn'):
        model, cities = saved_model(home / name, home / Path(name).parent / 'Rules.txt')
        if emulator:
            emulator.load(model)
        routes = []
        for i, c in enumerate(cities):
            count = struct.unpack_from('<b', c, 60)[0]
            assert 0 <= count <= 3
            if not count:
                continue
            values, slots = [], []
            for k in range(count):
                partner = struct.unpack_from('<h', c, 70 + 2 * k)[0]
                p = cities[partner]
                connection = model.connection(c[10], *struct.unpack_from('<hh', c),
                                              *struct.unpack_from('<hh', p))
                airports = bool(c[58] & 1 and p[58] & 1)
                values.append(income(struct.unpack_from('<h', c, 32)[0],
                                     struct.unpack_from('<h', p, 32)[0],
                                     struct.unpack_from('<b', c, 67 + k)[0],
                                     c[10] == p[10], connection, airports,
                                     bool(c[57] & 2)))
                slots.append({'partner_index': partner,
                              'commodity': struct.unpack_from('<b', c, 67 + k)[0],
                              'source_cache': struct.unpack_from('<h', c, 32)[0],
                              'partner_cache': struct.unpack_from('<h', p, 32)[0],
                              'income': values[-1]})
            if emulator:
                assert emulator.routes(cities, i, 0)[:count] == values
            routes.append({'city_index': i, 'slots': slots})
        saves.append({'save': name, 'cities': len(cities), 'routes': routes})
    result = {'source': 'civ2.bak route loop 0x4c46bf..0x4c4890',
              'source_sha256': hashlib.sha256(binary).hexdigest(),
              'scope': 'route income only; refresh/final corruption excluded; surface map',
              'original_x86_checked': bool(emulator), 'synthetic_cases': len(vectors),
              'saves': saves}
    if args.vectors:
        args.vectors.write_text('# source partner commodity domestic connection airports highways income\n'
                                + ''.join(' '.join(map(str, v)) + '\n' for v in vectors))
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
