#!/usr/bin/env python3
"""Execute original Apollo map/city knowledge writes through original helpers.

Single synthetic surface map; stop before UI completion, no replacement callees.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct
from research_check import OriginalCost


def main():
    from unicorn.x86_const import UC_X86_REG_EBP
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold');p.add_argument('--vectors');a=p.parse_args()
    binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes();o=OriginalCost(binary);rng=random.Random(8825);lines=[]
    for k in range(32):
        w,h=8,4;n=w*h//2
        for address,value,fmt in ((0x607652,0,'<B'),(0x6762f8,0,'<I'),(0x6705a0,w,'<h'),
                                  (0x6705a2,h,'<h'),(0x6705a4,n,'<h'),(0x6705ae,0,'<h'),
                                  (0x607668,3,'<h'),(0x5deaa8,0x700000,'<I')):o.write(address,value,fmt)
        tiles=bytearray();inputs=[]
        for i in range(n):
            imp,visible=rng.randrange(256),rng.randrange(256);inputs.extend((imp,visible))
            tiles.extend((1,imp,0,1,visible,240))
        o.cpu.mem_write(0x700000,bytes(tiles))
        for civ in range(1,8):
            o.write(0x5dead0+4*civ,0x710000+0x1000*civ,'<I')
            o.cpu.mem_write(0x710000+0x1000*civ,bytes([13]*n))
        sizes=[]
        for i in range(3):
            c=bytearray(92);struct.pack_into('<hh',c,0,2*i,0);c[11]=rng.randrange(1,50)
            sizes.append(c[11]);struct.pack_into('<I',c,86,int(i!=2));o.cpu.mem_write(0x5fd150+92*i,bytes(c))
        frame=0x108100;o.cpu.reg_write(UC_X86_REG_EBP,frame);o.cpu.reg_write(o.sp,frame-256);o.write(frame-8,25,'<I')
        o.cpu.emu_start(0x4c8fcf,0x4c91d7,count=100000)
        assert o.cpu.reg_read(o.ip)==0x4c91d7
        visibility=[o.cpu.mem_read(0x700000+6*i+4,1)[0] for i in range(n)]
        caches=[list(o.cpu.mem_read(0x710000+0x1000*c,n)) for c in range(1,8)]
        cities=[list(o.cpu.mem_read(0x5fd150+92*i+14,9)) for i in range(3)]
        assert visibility==[255]*n
        assert all(cache==inputs[::2] for cache in caches)
        assert cities==[[255]+[size]*8 for size in sizes[:2]]+[[0]*9]
        lines.append(' '.join(map(str,inputs+sizes))+' | '+' '.join(map(str,visibility+caches[0]+[v for c in cities for v in c])))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'fragment':'0x4c8fcf..0x4c91d7',
            'cases':len(lines),'scope':'Original single-surface-map visibility, all seven remembered-improvement arrays, and live/dead city knowledge; stop before completion UI.'}
    if a.gold:Path(a.gold).write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:Path(a.vectors).write_text('# [improvements visible]*16 city_sizes*3 | visible*16 remembered_improvements*16 [known_mask known_sizes*8]*3\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
