#!/usr/bin/env python3
"""Execute original pikemen and barbarian Archer/Legion defence modifiers.

Original movement, hit-point and wonder callees execute. No replacements.
Other combat modifiers are outside these two explicit fragment boundaries.
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
    binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes();o=OriginalCost(binary);frame=0x108100;rng=random.Random(1515);lines=[]
    o.cpu.mem_write(0x607736,b'\xff\xff'*28);o.write(0x5f5eb8,3,'<B');o.write(0x6031be,0,'<h');o.write(0x60765b,16,'<B')
    for case_index in range(1024):
        base=rng.randrange(1,500);domain=rng.randrange(3);moves=rng.randrange(1,4);hp=rng.choice((10,20));flags=rng.choice((0,1024));barb=rng.randrange(2);typ=rng.choice((1,4,5));iron=rng.randrange(2)
        if case_index<4:base,domain,moves,hp,flags,barb,typ,iron=24,0,2,10,1024,int(case_index>0),4 if case_index==1 else 5,int(case_index==3)
        for address,value,fmt in ((frame+8,0,'<I'),(frame-8,1,'<I'),(frame-0x60,base,'<I'),(frame-0x80,0 if barb else 2,'<I'),(frame-0x68,1,'<I'),(frame-0xb0,24,'<I'),
            (0x607c60,0,'<B'),(0x607c58+40+8,typ,'<B'),(0x5f4709,domain,'<B'),(0x5f470a,3*moves,'<B'),(0x5f470e,hp,'<B'),(0x5f4700+40*typ+4,flags,'<H'),(0x6076f9,16*iron,'<B')):
            o.write(address,value,fmt)
        o.cpu.reg_write(UC_X86_REG_EBP,frame);o.cpu.reg_write(o.sp,frame-512)
        o.cpu.emu_start(0x5416fa,0x5417bc,count=5000);assert o.cpu.reg_read(o.ip)==0x5417bc
        o.cpu.emu_start(0x541cf4,0x541d82,count=5000);assert o.cpu.reg_read(o.ip)==0x541d82
        result=struct.unpack('<i',o.cpu.mem_read(frame-0x60,4))[0]
        lines.append(' '.join(map(str,[base,domain,moves,hp,flags,barb,typ,iron,result])))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routines':['0x5416fa..0x5417bc','0x541cf4..0x541d82'],'cases':len(lines),'replacement_callees':[],'scope':__doc__.strip()}
    if a.gold:a.gold.write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:a.vectors.write_text('# base att_domain att_move att_max_hp def_flags barbarian def_type humans_know_iron result\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
