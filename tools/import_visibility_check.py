#!/usr/bin/env python3
"""Execute the normal unit knowledge-update loops in unpatched civ2.bak.

0x41f8a7..0x4205a7 executes exploration, adjacent city/stack observation,
and outer-ring reciprocal sight with original callees, without replacements.
Prepared locals represent the normal single-player, non-cheat context. All
units are awake, non-invisible; foreign relationships have treaty bits 1/4/8.
The fragment stops before presentation. First-contact calls are forbidden.
AI sighting bookkeeping may execute but is not part of the compared output.
"""
import argparse
import hashlib
import itertools
import json
import os
from pathlib import Path
import struct

from unicorn import UC_HOOK_CODE
from import_knowledge_check import OriginalCity, FRAME, CITY, UNIT, frame, fragment, read

OFFSETS = [(1,-1),(2,0),(1,1),(0,2),(-1,1),(-2,0),(-1,-1),(0,-2),
           (2,-2),(2,2),(-2,2),(-2,-2),(1,-3),(3,-1),(3,1),(1,3),
           (-1,3),(-3,1),(-3,-1),(-1,-3),(0,0),(4,0),(-4,0),(0,4),(0,-4)]


def prepare(o, civ, domain, extended, other_domain, other_extended, k,
            city_bits, seen, attack, flat, x, y, mask, neighbor_mode):
    w, h, n = 16, 12, 96
    other = civ if neighbor_mode & 1 else civ % 7 + 1
    nx, ny = x + OFFSETS[k][0], y + OFFSETS[k][1]
    if not flat:
        nx = nx+w if nx < 0 else nx-w if nx >= w else nx
    valid = 0 <= nx < w and 0 <= ny < h
    tiles = bytearray(v for i in range(n) for v in
                      (10 if i % 3 == 0 else 1, 0x10 | (i % 4)*0x20, 0, 1, 1, 240))
    locations = [(x,y,civ),(nx,ny,other)]
    units, cities = [], []
    for group,(ux,uy,owner) in enumerate(locations):
        present = group == 0 or valid
        for j in range(2):
            slot = 2*group+j
            u = bytearray(40)
            if present and (group == 0 or neighbor_mode < 2):
                struct.pack_into('<hh',u,0,ux,uy)
                u[8],u[9],u[11],u[17] = group,owner,mask,255
                struct.pack_into('<hhI',u,26,slot-1 if j else -1,slot+1 if not j else -1,slot+1)
            units.append(u)
        c = bytearray(92)
        if present:
            i = uy*8+ux//2
            if group == 0 or neighbor_mode < 2:
                tiles[6*i+1] |= 1
            tiles[6*i+5] = owner << 4
            if city_bits & (1 << group):
                tiles[6*i+1] = (tiles[6*i+1] & ~0xC0) | 2
                struct.pack_into('<hh',c,0,ux,uy)
                c[10],c[11],c[14] = owner,5+2*group,mask
                c[15:23] = bytes(range(40,48))
                struct.pack_into('<I',c,86,group+1)
        cities.append(c)
    tiles[6*(y*8+x//2)+4] |= (1 << other) if seen else 0
    o.cpu.mem_write(0x700000,bytes(tiles))
    o.cpu.mem_write(UNIT,b''.join(units))
    o.cpu.mem_write(CITY,b''.join(cities))
    o.cpu.mem_write(0x5F4700,bytes(80))
    for group, (d,e) in enumerate(((domain,extended),(other_domain,other_extended))):
        o.write(0x5F4700+40*group+4,e,'<I')
        o.write(0x5F4700+40*group+9,d,'<B')
        o.write(0x5F4700+40*group+12,attack if group==0 else 1,'<B')
    for addr,val,fmt in [
        (0x6705A0,w,'<h'),(0x6705A2,h,'<h'),(0x6705A4,n,'<h'),
        (0x6762F8,0,'<I'),(0x607638,0x8000 if flat else 0,'<H'),
        (0x607652,0,'<B'),(0x607655,0,'<B'),(0x60765B,0,'<B'),
        (0x5DEAA8,0x700000,'<I'),(0x607668,2,'<h'),(0x607666,4,'<h'),
        (0x5DE648,0,'<I'),(0x62D3EC,-1,'<i'),(0x628BD8,-1,'<i')]:
        o.write(addr,val,fmt)
    for a in range(8):
        for b in range(8):
            o.write(0x5F68A0+3348*a+40+4*b,13,'<I')
    for c in range(1,8):
        o.write(0x5DEAD0+4*c,0x720000+0x1000*c,'<I')
        o.cpu.mem_write(0x720000+0x1000*c,bytes([128]*n))
    frame(o)
    for off,val in [(8,0),(12,0),(-0x34,x),(-0x40,y),(-0x4C,0),
                    (-0x68,civ),(-0x54,1<<civ),(-0x3C,extended),
                    (-0x64,domain),(-0x24,int(tiles[6*(y*8+x//2)]==10)),
                    (-0x28,0),(-8,0),(-0x20,0),(-0x10,0)]:
        o.write(FRAME+off,val,'<I')
    return tiles, units, cities


def capture(o, civ, tiles, units, cities, orders=False):
    n = 96
    after = bytes(o.cpu.mem_read(0x700000,6*n))
    explored = [i for i in range(n) if after[6*i+4] & (1<<civ)]
    assert all(after[6*i+4] == (tiles[6*i+4] | ((1<<civ) if i in explored else 0)) for i in range(n))
    assert all(after[6*i:6*i+4] == tiles[6*i:6*i+4] and after[6*i+5] == tiles[6*i+5] for i in range(n))
    for c in range(1,8):
        cache = bytes(o.cpu.mem_read(0x720000+0x1000*c,n))
        assert all(cache[i] == (tiles[6*i+1] if c==civ and i in explored else 128) for i in range(n))
    masks, final_orders = [], []
    for slot,before in enumerate(units):
        u = bytearray(o.cpu.mem_read(UNIT+40*slot,40))
        masks.append(u[11]); u[11] = before[11]
        if orders:
            final_orders.append(u[17]); u[17] = before[17]
        assert u == before
    known, sizes = [], []
    for slot,before in enumerate(cities):
        c = bytearray(o.cpu.mem_read(CITY+92*slot,92))
        known.append(c[14]); sizes.append(c[15+civ])
        c[14],c[15+civ] = before[14],before[15+civ]
        assert c == before
    return masks+final_orders+known+sizes+[len(explored)]+explored


def context(o, *values):
    initial = prepare(o, *values)
    fragment(o,0x41F8A7,0x4205A7)
    return capture(o,values[0],*initial)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--out',type=Path,required=True)
    p.add_argument('--vectors',type=Path,required=True)
    args = p.parse_args()
    binary = (Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes()
    o = OriginalCity(binary)
    def forbid(cpu,address,size,data):
        if address in (0x526ACA,0x467D19):
            raise AssertionError(f'unexpected contact/presentation: {address:x}')
    o.cpu.hook_add(UC_HOOK_CODE,forbid)
    rows = ['# civ domain extended other_domain other_extended offset city_bits seen attack flat x y initial_mask neighbor_mode | unit_masks[4] city_masks[2] remembered_sizes_for_civ[2] count explored_indices...',
            '# neighbor_mode: 0 foreign stack, 1 friendly stack, 2 foreign no units, 3 friendly no units; city_bits still selects cities']
    offsets = [k for k in range(25) if k != 20]
    for j,values in enumerate(itertools.product((1,4,7),range(4),range(2),range(4),range(2),range(4))):
        civ,d,e,od,oe,city = values
        inputs = [civ,d,e,od,oe,offsets[(j//4)%24],city,(j//3)%2,(j//5)%2,(j//7)%2,
                  0 if j%3==0 else 6,0 if j%5==0 else 6,(0,1,128,255)[(j//11)%4],(j//19)%4]
        output = context(o,*inputs)
        rows.append(' '.join(map(str,inputs))+' | '+' '.join(map(str,output)))
    vectors = '\n'.join(rows)+'\n'
    args.vectors.write_text(vectors)
    report = {'schema':1,'binary_sha256':hashlib.sha256(binary).hexdigest(),
              'method':__doc__.strip(),'cases':len(rows)-2,
              'vectors_sha256':hashlib.sha256(vectors.encode()).hexdigest(),
              'limits':'Controlled single surface map, normal awake non-invisible units, established treaty bits 1/4/8. Knowledge fields only; no live loader/movement, contact, AI sighting bookkeeping, visibility reset, city phase or faithful runtime admission.'}
    args.out.write_text(json.dumps(report,indent=2)+'\n')
    print(f'{len(rows)-2} native unit knowledge transitions passed')


if __name__ == '__main__':
    main()
