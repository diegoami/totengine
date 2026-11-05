#!/usr/bin/env python3
"""Execute native shield-shortage/Republic-Democracy unrest unit selection.

Synthetic surface map and unit table. Unit deletion boundaries only mark the
slot dead; native selection, support accumulation and shield refunds execute.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct
from research_check import OriginalCost


class OriginalDisband(OriginalCost):
    def run(self,v,units,forts):
        from unicorn import UC_HOOK_CODE
        size,gov,human,turn,flags,shields,free2,free3,free4,rows=v
        for a,x,f in ((0x607666,8,'<h'),(0x607668,3,'<h'),(0x60765b,2 if human else 0,'<B'),
                      (0x607648,turn,'<h'),(0x607640,0,'<H'),(0x607652,0,'<B'),(0x6031b8,1,'<h'),
                      (0x5d75b0,0,'<I'),(0x5f68bd+3348,gov,'<B'),(0x6705a0,64,'<h'),(0x6705a2,16,'<h'),
                      (0x607638,0,'<H'),(0x6762f8,0,'<I'),(0x5deaa8,0x700000,'<I'),
                      (0x65384c,shields,'<i'),(0x6537fc,rows,'<i'),(0x5f5ec5,free2,'<B'),
                      (0x5f5ec6,free3,'<B'),(0x5f5ec7,free4,'<B')):self.write(a,x,f)
        tiles=bytearray([2,0,0,1,255,240]*(32*16))
        for x,y in forts:tiles[6*(y*32+x//2)+1]|=64
        for i,(x,y) in enumerate(((8,8),(16,8),(40,8))):
            c=bytearray(92);struct.pack_into('<hh',c,0,x,y);c[10]=1;c[11]=size
            if i==0:struct.pack_into('<I',c,6,flags)
            struct.pack_into('<I',c,86,i+1);self.cpu.mem_write(0x5fd150+92*i,bytes(c))
            tiles[6*(y*32+x//2)+1]|=2 | (64 if i==1 and turn&1 else 0);tiles[6*(y*32+x//2)+5]=16
        self.cpu.mem_write(0x700000,bytes(tiles))
        for i,(alive,role,owner,home,x,y,exempt) in enumerate(units):
            u=bytearray(40);struct.pack_into('<hh',u,0,x,y);u[8]=i;u[9]=owner;u[18]=home
            struct.pack_into('<I',u,30,alive);self.cpu.mem_write(0x607c58+40*i,bytes(u))
            t=bytearray(40);t[5]=8 if exempt else 0;t[16]=i+1;t[18]=role
            self.cpu.mem_write(0x5f4700+40*i,bytes(t))
        removed=[]
        boundaries=(0x41f4b8,0x4cbc30,0x467c89)
        def hook(cpu,address,length,data):
            if address in boundaries or address in (0x56c652,0x56a8c2):
                sp=cpu.reg_read(self.sp);ret=struct.unpack('<I',cpu.mem_read(sp,4))[0]
                if address in (0x56c652,0x56a8c2):
                    unit=struct.unpack('<I',cpu.mem_read(sp+4,4))[0];removed.append(unit)
                    self.write(0x607c76+40*unit,0,'<I')
                cpu.reg_write(self.sp,sp+4);cpu.reg_write(self.ip,ret);cpu.reg_write(self.ax,0)
        h=self.cpu.hook_add(UC_HOOK_CODE,hook)
        try:
            self.cpu.mem_write(0x108000,struct.pack('<II',0x100000,0));self.cpu.reg_write(self.sp,0x108000)
            self.cpu.emu_start(0x4c92a4,0x100000,count=200000)
            assert self.cpu.reg_read(self.ip)==0x100000,hex(self.cpu.reg_read(self.ip))
        finally:self.cpu.hook_del(h)
        boxes=[struct.unpack('<h',self.cpu.mem_read(0x5fd150+92*i+30,2))[0] for i in range(3)]
        flags=struct.unpack('<I',self.cpu.mem_read(0x5fd156,4))[0]
        return [flags,*boxes,len(removed),*removed]


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold');p.add_argument('--vectors');a=p.parse_args()
    binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes();o=OriginalDisband(binary);rng=random.Random(1386);lines=[]
    forts=[(10,8),(30,8),(54,8)]
    for _ in range(1024):
        v=[rng.randrange(1,8),rng.randrange(7),rng.randrange(2),rng.randrange(16),rng.choice((0,1,0x2001,0x102001)),rng.randrange(7),rng.randrange(5),rng.randrange(5),rng.randrange(5),rng.randrange(5,15)]
        units=[]
        for i in range(8):
            x,y=rng.choice(((8,8),(16,8),(40,8),(10,8),(30,8),(54,8),(4,6),(60,4)))
            units.append([rng.randrange(2),rng.randrange(8),rng.choice((1,1,2)),rng.choice((0,0,255)),x,y,rng.randrange(2)])
        result=o.run(v,units,forts)
        lines.append(' '.join(map(str,v+[x for u in units for x in u]))+' | '+' '.join(map(str,result)))
    # A lone military unit at a city carrying both city and fortress bits
    # distinguishes the owner helper from the containing-city lookup.
    for turn in (1,9):
        v=[2,5,0,turn,1,7,0,0,0,10]
        units=[[0,1,1,0,8,8,0] for _ in range(8)]
        units[7]=[1,1,1,0,16,8,0]
        result=o.run(v,units,forts)
        lines.append(' '.join(map(str,v+[x for u in units for x in u]))+' | '+' '.join(map(str,result)))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routine':'0x4c92a4','cases':len(lines),
            'forts':forts,'fortified_city_on_odd_turn':1,'scope':'Synthetic surface land units; native support accumulation, farthest-unit selection, shortage removal and AI Republic/Democracy unrest disband/refund; replacement deletion marks identity dead, presentation skipped; game options word 0; no sea cargo/civ AI-stat bookkeeping/full-turn parity.'}
    if a.gold:Path(a.gold).write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:Path(a.vectors).write_text('# size gov human turn flags shields free2 free3 free4 rows [alive role owner home x y exempt]*8 | flags boxes*3 removed_count removed_slots*count\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
