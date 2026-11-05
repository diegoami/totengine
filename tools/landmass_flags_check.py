#!/usr/bin/env python3
"""Original hut-facing landmass strategy flags from military/city caches.

Executes 0x507a87..0x507ff4 plus 0x508275..0x5082be (the bit-16 write).
The intervening landmass build-strategy calculation does not change flags.
Original diplomacy/attitude callees execute; no replacements. Inputs model
surface admitted units and cities; per-civ military/attack accumulation is
checked separately using original 0x506e5e..0x506ec7 and 0x50733d..0x5073f7.
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
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold',type=Path);p.add_argument('--vectors',type=Path);a=p.parse_args()
    binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes();o=OriginalCost(binary);rng=random.Random(798);lines=[];frame=0x108100;land=1
    for case in range(512):
        civ=rng.randrange(1,8);humans=rng.randrange(256);rows=[]
        o.cpu.reg_write(UC_X86_REG_EBP,frame);o.write(frame+8,civ,'<I');o.write(frame-0x880,land,'<I');o.write(0x60765b,humans,'<B')
        o.cpu.mem_write(0x5f68a0,bytes(3348*8))
        for c in range(8):
            n=rng.randrange(5);attack=rng.choice((0,1,2,5,20,98));role=rng.randrange(8);cities=rng.randrange(4);treaty=rng.randrange(64)|rng.choice((0,8192));attitude=rng.randrange(101)
            rows.append((n,attack,role,cities,treaty,attitude));base=3348*c
            # Original accumulation for admitted units on landmass 1.
            o.write(frame+8,c,'<I');o.write(frame-0x87c,0,'<I');o.write(0x5f470c,attack,'<b');o.write(0x5f4712,role,'<b')
            for _ in range(n):
                o.cpu.reg_write(o.sp,frame-0x1000);o.cpu.emu_start(0x506e5e,0x506ec7,count=1000);assert o.cpu.reg_read(o.ip)==0x506ec7
                o.cpu.reg_write(o.sp,frame-0x1000);o.cpu.emu_start(0x50733d,0x5073f7,count=1000);assert o.cpu.reg_read(o.ip)==0x5073f7
            count=struct.unpack('<H',o.cpu.mem_read(0x5f6a70+base+2,2))[0];strength=struct.unpack('<H',o.cpu.mem_read(0x5f6c70+base+2,2))[0]
            assert count==(n if role<=4 else 0) and strength==n*attack
            o.write(0x5f6e70+base+land,cities,'<B');o.write(0x5f68c8+3348*civ+4*c,treaty,'<I');o.write(0x5f68e8+3348*civ+c,attitude,'<B')
        o.write(frame+8,civ,'<I');o.cpu.reg_write(o.sp,frame-0x1000);o.cpu.emu_start(0x507a87,0x507ff4,count=20000);assert o.cpu.reg_read(o.ip)==0x507ff4
        o.cpu.reg_write(o.sp,frame-0x1000);o.cpu.emu_start(0x508275,0x5082be,count=1000);assert o.cpu.reg_read(o.ip)==0x5082be
        flags=o.cpu.mem_read(0x5f7170+3348*civ+land,1)[0]
        lines.append(f'{civ} {humans} | '+' '.join(','.join(map(str,r)) for r in rows)+f' | {flags}')
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'cases':512,'replacement_callees':[],'scope':__doc__.strip()}
    if a.gold:a.gold.write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:a.vectors.write_text('# civ humans | 8 x n,attack,role,cities,treaty,attitude | flags\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
