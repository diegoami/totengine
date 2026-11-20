#!/usr/bin/env python3
"""Execute native stack reset, order wakes and the contact-call boundary.

Normal non-invisible surface contexts execute 0x41f8a7..0x4205a7 with
original callees. A contact case stops at 0x526aca entry and records its five
arguments; no conversation callee is replaced or executed. Completed cases
compare knowledge fields and orders. Stack resets execute 0x56adfe in full.
The native redraw-enable byte is zero, so original redraw callees return.
Neither movement as a whole nor diplomacy outcomes are asserted.
"""
import argparse
import hashlib
import itertools
import json
import os
from pathlib import Path
import struct

from unicorn import UC_HOOK_CODE
from import_visibility_check import prepare, capture, OFFSETS
from import_knowledge_check import OriginalCity, FRAME, UNIT, CITY, read


def run(o, domain, other_domain, k, city_bits, ocean_bits, order, role,
        human_mask, treaty, active, special, no_units, reset_first, *, configure=None):
    args = [4,domain,1,other_domain,1,k,city_bits,1,1,0,6,6,255,2 if no_units else 0]
    tiles,units,cities = prepare(o,*args)
    positions = [(6,6),(6+OFFSETS[k][0],6+OFFSETS[k][1])]
    for group, (x,y) in enumerate(positions):
        i = y*8+x//2
        tiles[6*i] = 10 if ocean_bits & (1<<group) else 1
        o.write(0x700000+6*i,tiles[6*i],'<B')
        o.write(0x5F4700+40*group+18,role,'<B')
    for slot,u in enumerate(units):
        if struct.unpack_from('<I',u,30)[0]:
            u[17] = order if slot%2==0 else 3
            if special and slot < 2:
                u[8] = 9
            o.cpu.mem_write(UNIT+40*slot,bytes(u))
    if special:
        o.cpu.mem_write(0x5F4700+40*9,bytes(o.cpu.mem_read(0x5F4700,40)))
    for a,b in [(4,5),(5,4)]:
        o.write(0x5F68A0+3348*a+40+4*b,treaty,'<I')
    o.write(0x60765B,human_mask,'<B')
    o.write(0x5D156C,0,'<B')
    o.write(FRAME-0x50,human_mask & 16,'<I')
    o.write(FRAME-0x24,int(bool(ocean_bits & 1)),'<I')
    o.write(FRAME+12,active,'<I')
    if configure is not None:
        configure(o,tiles,units,cities)
    if reset_first:
        o.call(0x56ADFE,0)
    contact = []
    def stop(cpu,address,size,data):
        if address == 0x526ACA:
            sp = cpu.reg_read(o.sp)
            contact.extend(struct.unpack('<5i',cpu.mem_read(sp+4,20)))
            cpu.emu_stop()
    hook = o.cpu.hook_add(UC_HOOK_CODE,stop)
    try:
        o.cpu.emu_start(0x41F8A7,0x4205A7,count=300000)
    finally:
        o.cpu.hook_del(hook)
    if contact:
        assert o.cpu.reg_read(o.ip) == 0x526ACA
        return 'C',contact
    assert o.cpu.reg_read(o.ip) == 0x4205A7,hex(o.cpu.reg_read(o.ip))
    return 'U',capture(o,4,tiles,units,cities,orders=True)


def reset(o,slot,mask):
    _,units,_ = prepare(o,4,0,0,0,0,1,0,0,1,0,6,6,mask,0)
    before = bytes(o.cpu.mem_read(UNIT,160))
    o.call(0x56ADFE,slot)
    after = bytes(o.cpu.mem_read(UNIT,160))
    expected = bytearray(before)
    for j in range(2*(slot//2),2*(slot//2)+2):
        expected[40*j+11] = 0
    assert after == expected
    return [after[40*j+11] for j in range(4)]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--out',type=Path,required=True)
    p.add_argument('--vectors',type=Path,required=True)
    args = p.parse_args()
    binary = (Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes()
    o = OriginalCity(binary)
    call_sites = [(0x54FD72,0x56ADFE),(0x5500E2,0x41F704),(0x56AE2B,0x56ADD6)]
    for site,target in call_sites:
        instruction = bytes(o.cpu.mem_read(site,5))
        assert instruction[0] == 0xE8
        assert site+5+struct.unpack('<i',instruction[1:])[0] == target
    rows = ['# R slot mask | four resulting unit masks',
            '# W/B domain other_domain offset city_bits ocean_bits order role human_mask treaty active type9 no_neighbor_units reset_first | U knowledge/orders or C contact arguments',
            '# U masks[4] orders[4] city_masks[2] remembered_sizes[2] count explored_indices...']
    counts = {'reset':0,'completed':0,'contact_boundary':0}
    for slot,mask in itertools.product(range(4),(0,1,16,32,128,255)):
        rows.append(f'R {slot} {mask} | '+' '.join(map(str,reset(o,slot,mask))))
        counts['reset'] += 1
    for j,(d,od,order,human,allied) in enumerate(itertools.product(range(4),range(4),(0,3,4,12,28,255),(0,16,32,48),range(2))):
        inputs = [d,od,(1,8,21)[j%3],(j//3)%4,(j//5)%4,order,7 if j%2 else 0,
                  human,5|(8 if allied else 0),0,0,0,(j//11)%2]
        kind,out = run(o,*inputs)
        assert kind == 'U'
        rows.append('W '+' '.join(map(str,inputs))+' | '+kind+' '+' '.join(map(str,out)))
        counts['completed'] += 1
    for j,(d,od,city,active,met,special) in enumerate(itertools.product(range(4),range(4),range(4),range(2),range(2),range(2))):
        inputs = [d,od,1,city,(j//3)%4,12,0,16,4|met,active,special,(j//7)%2,(j//13)%2]
        kind,out = run(o,*inputs)
        rows.append('B '+' '.join(map(str,inputs))+' | '+kind+' '+' '.join(map(str,out)))
        counts['completed' if kind=='U' else 'contact_boundary'] += 1
    vectors = '\n'.join(rows)+'\n'
    args.vectors.write_text(vectors)
    report = {'schema':1,'binary_sha256':hashlib.sha256(binary).hexdigest(),
              'method':__doc__.strip(),'cases':counts,
              'checked_call_sites':[{'site':hex(site),'target':hex(target)} for site,target in call_sites],
              'vectors_sha256':hashlib.sha256(vectors.encode()).hexdigest(),
              'limits':'Normal single-player non-invisible surface contexts. No full movement, conversation outcomes, AI sighting bookkeeping, loader or faithful runtime activation.'}
    args.out.write_text(json.dumps(report,indent=2)+'\n')
    print(counts)


if __name__ == '__main__':
    main()
