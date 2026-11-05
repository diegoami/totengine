#!/usr/bin/env python3
"""Execute native end-of-city unrest, income and celebration transitions.

Presentation, AI build selection and revolution dispatch are explicit call
boundaries. Growth-limit/building checks execute original code.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct
from research_check import OriginalCost


class OriginalMoodTransition(OriginalCost):
    def run(self,v):
        from unicorn import UC_HOOK_CODE
        size,owner,gov,human,flags,happy,unhappy,food,eaten,shields,support,surplus,taxes,scenario,scenario_flags,buildings=v
        c=bytearray(92);c[10]=owner;c[11]=size;struct.pack_into('<I',c,6,flags)
        for k,b in ((9,1),(23,2)):
            if buildings&b:c[54+k//8]|=1<<(k%8)
        self.cpu.mem_write(0x5fd150,bytes(c));self.cpu.mem_write(0x5f68a0,bytes(3348*8))
        for a,x,f in ((0x60765b,(1<<owner) if human else 0,'<B'),(0x5f68bd+3348*owner,gov,'<B'),
                      (0x607640,128 if scenario else 0,'<H'),(0x5f5e50,scenario_flags,'<H'),
                      (0x67a6e0,-1,'<i'),(0x6031b8,1,'<h'),(0x5f5eba,eaten,'<B'),
                      (0x5f5ec1,8,'<B'),(0x5f5ec2,12,'<B'),(0x6537d0,happy,'<i'),(0x653828,unhappy,'<i'),
                      (0x653848,food,'<i'),(0x653858,0,'<i'),(0x653888,0,'<i'),
                      (0x65384c,shields,'<i'),(0x6537e8,support,'<i'),(0x65389c,surplus,'<i'),
                      (0x6537d4,taxes,'<i')):self.write(a,x,f)
        events=[]
        boundaries={0x434d81:2,0x52588e:3,0x479f98:0,0x41b050:0,0x4c52ec:0,0x4c558f:0,
                    0x4cbc30:0,0x467c89:0,0x45a4b0:0,0x45aa97:0}
        def hook(cpu,address,length,data):
            if address in boundaries:
                event=boundaries[address]
                if event:events.append(event)
                sp=cpu.reg_read(self.sp);ret=struct.unpack('<I',cpu.mem_read(sp,4))[0]
                cpu.reg_write(self.sp,sp+4);cpu.reg_write(self.ip,ret);cpu.reg_write(self.ax,0)
        h=self.cpu.hook_add(UC_HOOK_CODE,hook)
        try:
            self.cpu.mem_write(0x108000,struct.pack('<II',0x100000,0));self.cpu.reg_write(self.sp,0x108000)
            self.cpu.emu_start(0x4c998e,0x100000,count=100000)
            assert self.cpu.reg_read(self.ip)==0x100000,hex(self.cpu.reg_read(self.ip))
        finally:self.cpu.hook_del(h)
        flags=struct.unpack('<I',self.cpu.mem_read(0x5fd156,4))[0];size=self.cpu.mem_read(0x5fd15b,1)[0]
        income=struct.unpack('<i',self.cpu.mem_read(0x5f68aa+3348*owner,4))[0]
        civ_flags=struct.unpack('<H',self.cpu.mem_read(0x5f68a0+3348*owner,2))[0]
        return [size,flags,income,civ_flags,events.count(2),events.count(3)]


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold');p.add_argument('--vectors');a=p.parse_args()
    binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes();o=OriginalMoodTransition(binary);rng=random.Random(8613);lines=[]
    for _ in range(2048):
        size=rng.randrange(1,30)
        v=[size,rng.randrange(8),rng.randrange(7),rng.randrange(2),rng.choice((0,1,2,3,0x4000,0x4001,0x2001,0x102003)),
           rng.randrange(size+1),rng.choice((0,0,rng.randrange(size+1))),rng.randrange(100),rng.randrange(4),
           rng.randrange(10),rng.randrange(6),rng.randrange(-6,7),rng.randrange(50),rng.randrange(2),rng.choice((0,16)),rng.randrange(4)]
        result=o.run(v);lines.append(' '.join(map(str,v+result)))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routine':'0x4c998e','cases':len(lines),
            'scope':'Unrest flags, treasury credit, celebration start/end and extra population with native growth limits; supplied totals; replace presentation/AI production-choice/revolution-dispatch callees, record latter two requests; excludes their side effects and full turns.'}
    if a.gold:Path(a.gold).write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:Path(a.vectors).write_text('# size owner government human flags happy unhappy food food_each shields support surplus taxes scenario scenario_flags buildings | size flags income civ_flags build_requests revolution_requests\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
