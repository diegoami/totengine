#!/usr/bin/env python3
"""Execute original round-end barbarian arrivals with explicit spawn/UI boundaries.

Original location helpers, dispatch, technology gates and CRT run unchanged.
Spawn requests replace unit allocation (successful synthetic records); UI calls
are suppressed. No executable bytes, saves, names or map records are emitted.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct
from research_check import OriginalCost

TECHS = (11, 34, 35, 37, 39, 45, 50, 51, 57, 81)


class OriginalBarbarians(OriginalCost):
    def integer(self, address, fmt='<I'):
        return struct.unpack(fmt, self.cpu.mem_read(address, struct.calcsize(fmt)))[0]

    def run(self, case):
        from unicorn import UC_HOOK_CODE
        seed, turn, level, difficulty, mode, flags, mask, water, size, money, population, human, government, original = case
        for a,v,f in ((0x5e2590,seed,'<I'), (0x607648,turn,'<h'),
                      (0x607659,level,'<B'), (0x607658,difficulty,'<B'),
                      (0x607782,mode,'<B'), (0x607638,flags,'<H'),
                      (0x60765b,human,'<B'), (0x607668,2,'<h'),
                      (0x607666,0,'<h'), (0x607770,1,'<B'),
                      (0x5f68bd+3348,government,'<B'),
                      (0x5f68aa+3348,money,'<i'), (0x5f6914+3348,population,'<h'),
                      (0x6705a0,64,'<h'),(0x6705a2,48,'<h'),
                      (0x6762f8,0,'<I'),(0x5deaa8,0x700000,'<I'),
                      (0x6031b8,1,'<I'), (0x5f4a83,50,'<b'),
                      (0x5d16b0,0x710000,'<I')):
            self.write(a,v,f)
        self.cpu.mem_write(0x6076d2,bytes(100))
        for i,tech in enumerate(TECHS):
            self.write(0x6076d2+tech,2 if mask & (1<<i) else 0,'<B')
        tiles=bytearray()
        for y in range(48):
            for x in range(32):
                ocean = water == 1 or (water == 2 and y < 20)
                tiles.extend((10 if ocean else 1,0,0,1,0,240))
        self.cpu.mem_write(0x700000,bytes(tiles))
        self.write(0x61bcac+40,100,'<h')
        self.cpu.mem_write(0x66f8c0,bytes(11*24))
        self.cpu.mem_write(0x5f68c8,bytes(32))
        for i in range(2):
            c=bytearray(92)
            struct.pack_into('<hhh',c,0,20+12*i,24,0)
            c[10:13]=bytes((1,size,original))
            struct.pack_into('<I',c,86,i+1)
            self.cpu.mem_write(0x5fd150+92*i,bytes(c))
        requests=[]
        def boundary(cpu,address,length,data):
            if address not in (0x56a1c0,0x5692f7,0x569c88,0x40cef0,0x42095e,0x467d19):
                return
            sp=cpu.reg_read(self.sp); ret=self.integer(sp)
            result=0
            if address==0x56a1c0:
                t,civ,x,y,map_id=struct.unpack('<5i',cpu.mem_read(sp+4,20))
                assert (civ,map_id)==(0,0)
                result=len(requests)
                requests.append([t,x,y,255,False])
                record=bytearray(40)
                struct.pack_into('<hh',record,0,x,y)
                record[8]=t; record[17]=255
                struct.pack_into('<I',record,30,result+1)
                cpu.mem_write(0x607c58+40*result,bytes(record))
                self.write(0x607666,result+1,'<h')
            elif address==0x5692f7:
                idx=self.integer(sp+4)
                requests[idx][3]=3
                self.write(0x607c58+40*idx+17,3,'<B')
            cpu.reg_write(self.ax,result)
            cpu.reg_write(self.sp,sp+4); cpu.reg_write(self.ip,ret)
        hook=self.cpu.hook_add(UC_HOOK_CODE,boundary)
        try:
            self.cpu.mem_write(0x108000,struct.pack('<I',0x100000))
            self.cpu.reg_write(self.sp,0x108000)
            self.cpu.emu_start(0x46e6cd,0x100000,count=5000000)
            assert self.cpu.reg_read(self.ip)==0x100000,hex(self.cpu.reg_read(self.ip))
        finally:
            self.cpu.hook_del(hook)
        for i,request in enumerate(requests):
            request[4]=bool(self.integer(0x607c58+40*i+6,'<H') & 0x2000)
        return self.integer(0x5e2590), requests


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--gold'); p.add_argument('--vectors'); p.add_argument('--cases',type=int,default=1024)
    args=p.parse_args()
    binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes()
    oracle=OriginalBarbarians(binary)
    rng=random.Random(83)
    results=[]
    for i in range(args.cases):
        case=[rng.getrandbits(32),rng.choice((0,14,30,47,79,127,150,199,255)),
              rng.randrange(4),rng.randrange(6),rng.choice((0,2)),rng.choice((0,1,32768,32769)),
              rng.randrange(1024),rng.randrange(3),rng.randrange(1,40),rng.choice((0,25,200,32000)),
              rng.randrange(1,200),rng.choice((0,2)),rng.randrange(7),rng.choice((1,2))]
        state,requests=oracle.run(case)
        results.append((case,state,requests))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routine':'0x46e6cd',
            'cases':len(results),'arrivals':sum(bool(r[2]) for r in results),
            'units':sum(len(r[2]) for r in results),
            'scope':'Synthetic 64x48 maps; native gameplay and CRT, successful spawn-record boundary, no UI or full-turn parity.'}
    if args.gold: Path(args.gold).write_text(json.dumps(report,indent=2)+'\n')
    if args.vectors:
        lines=['# seed turn level difficulty mode flags techmask water size money population human government original_owner | final_CRT | type,x,y,orders,veteran ...']
        for case,state,requests in results:
            lines.append(' '.join(map(str,case))+' | '+str(state)+' | '+' '.join(','.join(str(int(v)) for v in r) for r in requests))
        Path(args.vectors).write_text('\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__': main()
