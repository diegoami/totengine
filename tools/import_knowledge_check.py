#!/usr/bin/env python3
"""Execute classic saved-knowledge readers and unit exploration in civ2.bak.

Renderer fragments stop before drawing: normal map-city admission, map-unit
admission and city-size selection execute original instructions without callee
replacements. Unit exploration executes 0x41f8a7..0x41fa2a and original map
helpers on controlled surface maps, with the single-player presentation context
disabled. No live loader/UI or complete movement/contact update is asserted.
Only numeric derived outputs and input hashes are emitted.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct

from unicorn.x86_const import UC_X86_REG_EBP

from yields_check import OriginalCity
from totctl import walk_save

FRAME = 0x108800
CITY = 0x5FD150
UNIT = 0x607C58


def frame(o):
    o.cpu.mem_write(FRAME - 0x100, bytes(0x200))
    o.cpu.reg_write(UC_X86_REG_EBP, FRAME)
    o.cpu.reg_write(o.sp, FRAME - 0x200)


def fragment(o, start, stop):
    o.cpu.emu_start(start, stop, count=200000)
    assert o.cpu.reg_read(o.ip) == stop, hex(o.cpu.reg_read(o.ip))


def read(o, address, fmt='<I'):
    return struct.unpack(fmt, o.cpu.mem_read(address, struct.calcsize(fmt)))[0]


def city_shown(o, human, owner, explored, known, size):
    frame(o)
    o.write(0x67A6E0, human, '<I')
    o.write(CITY + 10, owner, '<B')
    o.write(CITY + 14, known, '<B')
    o.write(CITY + 15 + human, size, '<B')
    o.write(FRAME - 8, 0, '<I')
    o.write(FRAME - 4, int(explored), '<I')
    fragment(o, 0x4664B9, 0x46656C)
    return read(o, FRAME - 4)


def unit_shown(o, human, owner, explored, visible):
    frame(o)
    o.write(0x67A6E0, human, '<I')
    o.write(UNIT + 9, owner, '<B')
    o.write(UNIT + 11, visible, '<B')
    o.write(FRAME - 8, 0, '<I')
    o.write(FRAME - 4, int(explored), '<I')
    fragment(o, 0x466AB8, 0x466B35)
    return read(o, FRAME - 4)


def displayed_size(o, human, owner, actual, remembered):
    frame(o)
    o.write(0x67A6E0, human, '<I')
    o.write(0x607657, 0, '<B')
    o.write(0x607640, 0, '<H')
    o.write(0x5F5E50, 0, '<H')
    o.write(FRAME - 0x58, owner, '<I')
    o.write(FRAME + 12, 0, '<I')
    o.write(CITY + 11, actual, '<B')
    o.write(CITY + 15 + human, remembered, '<B')
    fragment(o, 0x538304, 0x538395)
    return read(o, FRAME - 4, '<i')


def exploration(o, human, domain, extended, x, y, flat, width=16, height=12):
    n = width * height // 2
    tiles = bytearray()
    for i in range(n):
        tiles.extend((10 if i % 3 == 0 else 1, 0x10 | (i % 4) * 0x20, 0, 1, 1, 240))
    o.cpu.mem_write(0x700000, bytes(tiles))
    for address, value, fmt in [
        (0x6705A0, width, '<h'), (0x6705A2, height, '<h'),
        (0x6705A4, n, '<h'), (0x6762F8, 0, '<I'),
        (0x607638, 0x8000 if flat else 0, '<H'),
        (0x5DEAA8, 0x700000, '<I'), (0x607652, 0, '<B'),
    ]:
        o.write(address, value, fmt)
    memory = 0x720000
    for civ in range(1, 8):
        o.write(0x5DEAD0 + 4 * civ, memory + 0x1000 * civ, '<I')
        o.cpu.mem_write(memory + 0x1000 * civ, bytes([0x80] * n))
    frame(o)
    for off, value in [(-0x34, x), (-0x40, y), (-0x4C, 0),
                       (-0x68, human), (-0x54, 1 << human),
                       (-0x3C, int(extended)), (-0x64, domain),
                       (-0x28, 0), (-8, 0)]:
        o.write(FRAME + off, value, '<I')
    fragment(o, 0x41F8A7, 0x41FA2A)
    changed = [i for i in range(n) if o.cpu.mem_read(0x700000 + 6 * i + 4, 1)[0] != 1]
    cache = o.cpu.mem_read(memory + 0x1000 * human, n)
    assert all(cache[i] == (tiles[6*i+1] if i in changed else 0x80) for i in range(n))
    return changed


def remembered_improvements(o, human, current, remembered):
    # Renderer 0x46513e uses the supplied civ's cache for normal human views.
    frame(o)
    index = 6 * 8 + 3
    o.write(0x700000 + 6*index + 1, current, '<B')
    o.write(0x720000 + 0x1000 * human + index, remembered, '<B')
    o.write(FRAME - 0x84, 0x700000 + 6*index, '<I')
    o.write(FRAME + 0x14, 6, '<I')
    o.write(FRAME + 0x18, 6, '<I')
    o.write(FRAME + 0x1C, human, '<I')
    fragment(o, 0x46513E, 0x465182)
    return read(o, FRAME - 0x8C)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--vectors', type=Path, required=True)
    args = parser.parse_args()
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    o = OriginalCity(binary)
    gates = []
    sizes = []
    for human in range(1, 8):
        bit = 1 << human
        for owner in (human, 0, human % 7 + 1):
            for explored in (False, True):
                for mask in (0, bit, 255 ^ bit):
                    for size in (0, 1, 5):
                        city = city_shown(o, human, owner, explored, mask, size)
                        unit = unit_shown(o, human, owner, explored, mask)
                        assert city == int(explored and (owner == human or (mask & bit and size != 0))), (human, owner, explored, mask, size, city)
                        assert unit == int(explored and bool(owner == human or mask & bit)), (human, owner, explored, mask, unit)
                        gates.append([human, owner, explored, mask, size, city, unit])
            for actual in (1, 9, 40):
                for remembered in (0, 2, 7):
                    value = displayed_size(o, human, owner, actual, remembered)
                    assert value == (actual if owner == human else remembered)
                    sizes.append([human, owner, actual, remembered, value])
    scouts = []
    for human in (1, 4, 7):
        for domain in range(4):
            for extended in (False, True):
                for x, y in ((6, 6), (0, 6), (0, 0)):
                    for flat in (False, True):
                        scouts.append([human, domain, extended, x, y, flat,
                                       exploration(o, human, domain, extended, x, y, flat)])
    tiny = []
    for width in (2, 4, 6):
        for domain in range(4):
            for extended in (False, True):
                for x, y in ((0, 0), (width-2, 2)):
                    for flat in (False, True):
                        tiny.append([4,domain,extended,x,y,flat,width,6,exploration(o,4,domain,extended,x,y,flat,width,6)])
    exploration(o,4,0,False,6,6,False)  # restore standard map context for readers
    memory = []
    for human in range(1, 8):
        for current in (0, 16, 32, 255):
            for remembered in (0, 16, 32, 49, 255):
                value = remembered_improvements(o, human, current, remembered)
                assert value == remembered
                memory.append([human, current, remembered, value])
    data = (home / 'Original/Ph_Auto2.SAV').read_bytes()
    walk = walk_save(str(home / 'Original/Ph_Auto2.SAV'))
    source = {'path': 'Original/Ph_Auto2.SAV', 'sha256': hashlib.sha256(data).hexdigest(),
              'player': data[680], 'units': walk['units']['count'], 'cities': walk['cities']['count']}
    width, height = walk['map_decode']['display_dims']
    n = width * height // 2
    start = walk['sections_derived']['map_data']['offset']
    tiles = data[start + 7*n:start + 13*n]
    human = data[680]
    bit = 1 << human
    cities, units = [], []
    for slot in range(walk['cities']['count']):
        c = data[walk['cities']['offset'] + 92*slot:][:92]
        if not struct.unpack_from('<I', c, 86)[0]:
            continue
        x, y = struct.unpack_from('<HH', c)
        explored = bool(tiles[6*(y*(width//2)+x//2)+4] & bit)
        if city_shown(o, human, c[10], explored, c[14], c[15+human]):
            cities.append([slot, displayed_size(o, human, c[10], c[11], c[15+human])])
    for slot in range(walk['units']['count']):
        u = data[walk['units']['region'][0] + 40*slot:][:40]
        if not struct.unpack_from('<I', u, 30)[0]:
            continue
        x, y = struct.unpack_from('<HH', u)
        explored = bool(tiles[6*(y*(width//2)+x//2)+4] & bit)
        if unit_shown(o, human, u[9], explored, u[11]):
            units.append(slot)
    source['explored_tiles'] = sum(bool(tiles[6*i+4] & bit) for i in range(n))
    source['shown_units'] = units
    source['shown_cities_and_sizes'] = cities
    source['madrid_extra_engine_tile'] = {'x':54,'y':12,'explored_in_save':bool(tiles[6*(12*(width//2)+27)+4] & bit)}
    rows = ['# G human owner explored mask remembered_size city_shown unit_shown',
            '# S human owner actual_size remembered_size displayed_size',
            '# R human current_improvements remembered_improvements displayed_improvements',
            '# T human domain extended x y flat width height count explored_indices...',
            '# E human domain extended x y flat count explored_indices...']
    rows.extend('G '+' '.join(str(int(x)) for x in row) for row in gates)
    rows.extend('S '+' '.join(map(str, row)) for row in sizes)
    rows.extend('R '+' '.join(map(str, row)) for row in memory)
    rows.extend('E '+' '.join(str(int(x)) for x in row[:6])+f' {len(row[6])} '+' '.join(map(str,row[6])) for row in scouts)
    rows.extend('T '+' '.join(str(int(x)) for x in row[:8])+f' {len(row[8])} '+' '.join(map(str,row[8])) for row in tiny)
    vectors = '\n'.join(rows)+'\n'
    args.vectors.write_text(vectors)
    offsets = [list(pair) for pair in zip(struct.unpack('25b',o.cpu.mem_read(0x5D1620,25)),struct.unpack('25b',o.cpu.mem_read(0x5D1650,25)))]
    report = {'schema': 1, 'binary_sha256': hashlib.sha256(binary).hexdigest(),
              'method': __doc__.strip(), 'source': source,
              'cases': {'city_unit_gates':len(gates),'city_size':len(sizes),'remembered_improvements':len(memory),'exploration':len(scouts),'tiny_exploration':len(tiny)},
              'vectors_sha256':hashlib.sha256(vectors.encode()).hexdigest(),
              'unit_exploration_offsets':offsets,
              'limits': 'Normal saved-map reader gates and controlled unit-exploration fragment only. No live SAV loader, city/stack drawing, invisible-unit/contact/diplomacy lifecycle, or full continuing knowledge parity.'}
    args.out.write_text(json.dumps(report, indent=2) + '\n')
    print(f'{len(gates)} city/unit gates, {len(sizes)} city-size selections, {len(memory)} remembered-improvement reads, {len(scouts)+len(tiny)} exploration contexts passed')


if __name__ == '__main__':
    main()
