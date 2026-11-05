#!/usr/bin/env python3
"""Execute original worker allocation with supplied yields and availability.

Only tile-yield calls (0x4c22a7) are replaced by synthetic triples. Original
allocation, bit operations, collection and comparisons execute unchanged.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct
from research_check import OriginalCost


class OriginalWorkers(OriginalCost):
    def run(self, v, blocked, yields):
        from unicorn import UC_HOOK_CODE
        size,worked,flags,human,turn,processing,food_each,settlers,settler_food,support,box=v
        for a,value,fmt in ((0x60765b,2 if human else 0,'<B'),(0x607648,turn,'<h'),
                            (0x5d75ac,processing,'<I'),(0x5f5eba,food_each,'<B'),
                            (0x653858,settlers,'<i'),(0x653888,settler_food,'<i'),
                            (0x6537e8,support,'<i')):self.write(a,value,fmt)
        c=bytearray(92);c[10]=1;c[11]=size;struct.pack_into('<I',c,6,flags)
        struct.pack_into('<h',c,28,box);struct.pack_into('<I',c,50,worked)
        self.cpu.mem_write(0x5fd150,bytes(c));self.cpu.mem_write(0x6537b0,bytes(blocked))
        def tile(cpu,address,length,data):
            if address!=0x4c22a7:return
            sp=cpu.reg_read(self.sp);ret,city,k,kind=struct.unpack('<4I',cpu.mem_read(sp,16));assert city==0
            cpu.reg_write(self.ax,yields[k][kind]);cpu.reg_write(self.sp,sp+4);cpu.reg_write(self.ip,ret)
        hook=self.cpu.hook_add(UC_HOOK_CODE,tile)
        try:
            self.cpu.mem_write(0x108000,struct.pack('<II',0x100000,0));self.cpu.reg_write(self.sp,0x108000)
            self.cpu.emu_start(0x4c2c14,0x100000,count=1000000)
            assert self.cpu.reg_read(self.ip)==0x100000
        finally:self.cpu.hook_del(hook)
        worked=struct.unpack('<I',self.cpu.mem_read(0x5fd182,4))[0]
        flags=struct.unpack('<I',self.cpu.mem_read(0x5fd156,4))[0]
        box=struct.unpack('<h',self.cpu.mem_read(0x5fd16c,2))[0]
        totals=struct.unpack('<3i',self.cpu.mem_read(0x653848,12))
        warning=struct.unpack('<i',self.cpu.mem_read(0x65385c,4))[0]
        return [worked,flags,box,*totals,warning]


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold');p.add_argument('--vectors');a=p.parse_args()
    binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes();o=OriginalWorkers(binary);rng=random.Random(1313);lines=[]
    for i in range(1024):
        size=rng.randrange(1,40);wanted=rng.randrange(min(size,16)+1)
        worked=rng.randrange(1<<21) | (wanted<<26)
        v=[size,worked,rng.choice((0,1,0x4000,0x4001,0x8000)),rng.randrange(2),rng.randrange(8),rng.randrange(2),
           rng.randrange(5),rng.randrange(4),rng.randrange(3),rng.randrange(5),rng.choice((-2,-1,0,1,20))]
        blocked=[rng.choice((0,0,0,0,1,2,4,8,16,32)) for _ in range(20)]
        yields=[[rng.randrange(7),rng.randrange(5),rng.randrange(5)] for _ in range(21)]
        result=o.run(v,blocked,yields)
        values=v+blocked+[n for row in yields for n in row]
        lines.append(' '.join(map(str,values))+' | '+' '.join(map(str,result)))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routine':'0x4c2c14','cases':len(lines),
            'scope':'Original worker allocation, trim/reselection, specialist count, flags, food-debt normalization and gross totals; synthetic tile-yield boundary and supplied availability bytes; no full city phase parity.'}
    if a.gold:Path(a.gold).write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:Path(a.vectors).write_text('# size worked flags human turn processing food_each settlers settler_food support box blocked*20 [food shields trade]*21 | worked flags box food shields trade warning\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
