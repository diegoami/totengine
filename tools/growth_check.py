#!/usr/bin/env python3
"""Execute original food rows and positive-box growth before food collection.

Native growth fragment includes real aqueduct/sewer, granary and Pyramids
helpers, stopping before growth UI/worker assignment. Starvation is separate.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct
from research_check import OriginalCost
from production_check import rows as shield_rows


class OriginalGrowth(OriginalCost):
    def food_rows(self, case):
        cosmic,difficulty,human,civ,turn,leader,w23,w25,m73,m58,m76,government,low,high=case
        for a,v,f in ((0x5f5ebb,cosmic,'<B'),(0x607658,difficulty,'<B'),(0x60765b,human,'<B'),
                      (0x607648,turn,'<h'),(0x607781,leader,'<B'),(0x607764,w23,'<h'),
                      (0x607768,w25,'<h'),(0x60771b,m73,'<B'),(0x60770c,m58,'<B'),(0x60771e,m76,'<B'),
                      (0x5f68bd+3348*civ,government,'<B'),(0x5f5ebd,low,'<B'),(0x5f5ebe,high,'<B')):
            self.write(a,v,f)
        self.cpu.mem_write(0x108000,struct.pack('<III',0x100000,0,civ));self.cpu.reg_write(self.sp,0x108000)
        self.cpu.emu_start(0x4c19e0,0x100000,count=2000)
        assert self.cpu.reg_read(self.ip)==0x100000
        return struct.unpack('<i',self.cpu.mem_read(0x6537e0,4))[0],struct.unpack('<i',self.cpu.mem_read(0x653888,4))[0]

    def growth(self, size, box, rows, cosmic, buildings, pyramid, aqueduct, sewer):
        from unicorn import UC_HOOK_CODE
        from unicorn.x86_const import UC_X86_REG_EBP
        frame=0x108100
        self.write(frame+8,0,'<I');self.write(frame-0x34,1,'<I')
        for a,v,f in ((0x6537e0,rows,'<i'),(0x5f5ebb,cosmic,'<B'),(0x5f5ec1,aqueduct,'<B'),
                      (0x5f5ec2,sewer,'<B'),(0x5d75b0,0,'<I'),(0x5f5c18,-1,'<b')):self.write(a,v,f)
        city=bytearray(92);city[10]=1;city[11]=size
        struct.pack_into('<h',city,28,box);struct.pack_into('<I',city,86,1)
        for i in (3,9,23):
            if buildings & (1<<{3:0,9:1,23:2}[i]):city[54+i//8]|=1<<(i%8)
        self.cpu.mem_write(0x5fd150,bytes(city));self.write(0x607736,0 if pyramid else -1,'<h')
        stops={0x4c5f59,0x4c5ee3,0x4c5df0}
        def stop(cpu,address,length,data):
            if address in stops:cpu.emu_stop()
        hook=self.cpu.hook_add(UC_HOOK_CODE,stop)
        try:
            self.cpu.reg_write(UC_X86_REG_EBP,frame);self.cpu.reg_write(self.sp,frame-256)
            self.cpu.emu_start(0x4c5cdb,0x100000,count=10000)
            assert self.cpu.reg_read(self.ip) in stops,hex(self.cpu.reg_read(self.ip))
        finally:self.cpu.hook_del(hook)
        outsize=self.cpu.mem_read(0x5fd15b,1)[0];outbox=struct.unpack('<h',self.cpu.mem_read(0x5fd16c,2))[0]
        blocked=struct.unpack('<i',self.cpu.mem_read(frame-0x28,4))[0] if self.cpu.reg_read(self.ip)!=0x4c5f59 else 0
        return outsize,outbox,blocked


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold');p.add_argument('--rows-vectors');p.add_argument('--growth-vectors');a=p.parse_args()
    binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes();o=OriginalGrowth(binary);rng=random.Random(13)
    row_lines=[];growth_lines=[]
    for i in range(1024):
        case=[rng.choice((7,10,13)),rng.randrange(6),rng.randrange(256),rng.randrange(1,8),rng.choice((0,199,200,201,350)),rng.randrange(8),
              rng.choice((-2,-1,0)),rng.choice((-2,-1,0)),rng.randrange(256),rng.randrange(256),rng.randrange(256),rng.randrange(7),rng.randrange(4),rng.randrange(4)]
        result=o.food_rows(case)
        adjusted=case[:11];adjusted[4]-=1
        assert result==(shield_rows(adjusted),case[13] if case[11]>2 else case[12]),(case,result)
        row_lines.append(' '.join(map(str,case+list(result))))
        size=rng.randrange(1,40);rows=rng.randrange(5,16);cosmic=rng.choice((7,10,13))
        box=max(0,(size+1)*rows+rng.choice((-3,-1,0,1,3)))
        values=[size,box,rows,cosmic,rng.randrange(8),rng.randrange(2),rng.randrange(6,13),rng.randrange(13,25)]
        result=o.growth(*values)
        growth_lines.append(' '.join(map(str,values+list(result))))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routines':['0x4c19e0','0x4c5cdb..0x4c5f59'],
            'row_cases':1024,'growth_cases':1024,'scope':'Native food rows/settler upkeep and positive-box growth with original granary/Pyramids/aqueduct/sewer helpers, before collection/worker assignment/UI; no starvation or full-turn claim.'}
    if a.gold:Path(a.gold).write_text(json.dumps(report,indent=2)+'\n')
    if a.rows_vectors:Path(a.rows_vectors).write_text('# cosmic difficulty human civ turn leader w23 w25 tech73 tech58 tech76 government low high | rows settler_food\n'+'\n'.join(row_lines)+'\n')
    if a.growth_vectors:Path(a.growth_vectors).write_text('# size box rows cosmic building_mask pyramid aqueduct_limit sewer_limit | size box blocked_improvement\n'+'\n'.join(growth_lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
