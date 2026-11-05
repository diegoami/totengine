#!/usr/bin/env python3
"""Execute original Eiffel Tower and Manhattan mutations, with notifications suppressed."""
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
    binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes();o=OriginalCost(binary);rng=random.Random(8823);lines=[]
    for i in range(256):
        kind=i%2;civ=rng.randrange(1,8);flags=rng.randrange(65536);reputations=[rng.randrange(256) for _ in range(8)]
        attitudes=[rng.randrange(256) for _ in range(8)];treaties=[rng.randrange(65536) for _ in range(8)]
        for c in range(8):
            b=0x5f68a0+3348*c
            o.write(b,flags if c==civ else 0,'<H');o.write(b+38,reputations[c],'<B')
            o.write(b+72+civ,attitudes[c],'<B');o.write(b+40+4*civ,treaties[c],'<I')
        if kind==0:
            o.cpu.mem_write(0x108000,struct.pack('<II',0x100000,civ));o.cpu.reg_write(o.sp,0x108000)
            o.cpu.emu_start(0x4c6121,0x100000,count=10000)
            assert o.cpu.reg_read(o.ip)==0x100000
        else:
            frame=0x108100;o.cpu.reg_write(UC_X86_REG_EBP,frame);o.cpu.reg_write(o.sp,frame-256)
            o.write(frame-8,23,'<I');o.write(0x6031b8,1,'<h')
            o.cpu.emu_start(0x4c8ddf,0x4c8f7d,count=10000)
            assert o.cpu.reg_read(o.ip)==0x4c8f7d
        outflags=struct.unpack('<H',o.cpu.mem_read(0x5f68a0+3348*civ,2))[0]
        outrep=[o.cpu.mem_read(0x5f68a0+3348*c+38,1)[0] for c in range(8)]
        outatt=[o.cpu.mem_read(0x5f68a0+3348*c+72+civ,1)[0] for c in range(8)]
        outtreaties=[struct.unpack('<I',o.cpu.mem_read(0x5f68a0+3348*c+40+4*civ,4))[0] for c in range(8)]
        values=[kind,civ,flags]+reputations+attitudes+treaties
        lines.append(' '.join(map(str,values))+' | '+' '.join(map(str,[outflags]+outrep+outatt+outtreaties)))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routines':['0x4c6121','0x4c8ddf..0x4c8f7d'],
            'cases':len(lines),'scope':'Original Eiffel Tower/Manhattan civ flags, reputation, attitudes and treaty mutations, no notification/UI parity.'}
    if a.gold:Path(a.gold).write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:Path(a.vectors).write_text('# kind civ flags reputations*8 incoming_attitudes*8 incoming_treaties*8 | flags reputations*8 incoming_attitudes*8 incoming_treaties*8\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
