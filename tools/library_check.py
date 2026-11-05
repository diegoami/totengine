#!/usr/bin/env python3
"""Original Great Library follow-on grant eligibility after a discovery.

Execute 0x49ebb5..0x49ec5c with original ownership/obsolescence, knowledge
and technology-group helpers. Stop at grant entry; no grant/UI is replaced.
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
    from unicorn import UC_HOOK_CODE
    from unicorn.x86_const import UC_X86_REG_EBP
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold',type=Path);p.add_argument('--vectors',type=Path);a=p.parse_args()
    binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes();o=OriginalCost(binary);rng=random.Random(8804);frame=0x108100;lines=[]
    for _ in range(512):
        owner=rng.randrange(8);hosted=rng.randrange(2);mask=rng.randrange(256);expired=rng.randrange(2);group=rng.choice((-1,0));flag=rng.randrange(3);tech=rng.choice((0,0,0,89))
        city=bytearray(92);city[10]=owner;struct.pack_into('<I',city,86,1);o.cpu.mem_write(0x5fd150,bytes(city))
        o.write(0x607736+8,0 if hosted else -1,'<h');o.write(0x5f5c18+4,1,'<b')
        o.cpu.mem_write(0x6076d2,bytes(100));o.write(0x6076d2+tech,mask,'<B');o.write(0x6076d3,128 if expired else 0,'<B')
        for civ in range(8):
            o.write(0x5f68a0+3348*civ+14,0,'<h');o.cpu.mem_write(0x5f68a0+3348*civ+96,bytes(13))
            o.write(0x5f68a0+3348*civ+96+tech//8,(1<<(tech%8)) if mask&(1<<civ) else 0,'<B')
        o.write(0x5f68a0+3348*7+96,((mask>>7)&1 if tech==0 else 0)|(2 if expired else 0),'<B')
        o.write(0x5d0a1c+20*tech,group,'<i');o.write(0x606f78,flag,'<B')
        o.write(frame+8,3,'<I');o.write(frame+12,tech,'<I')
        requested=[]
        def boundary(cpu,address,length,data):
            if address==0x49dedf:
                sp=cpu.reg_read(o.sp);requested.extend(struct.unpack('<2i',cpu.mem_read(sp+4,8)));cpu.emu_stop()
        hook=o.cpu.hook_add(UC_HOOK_CODE,boundary)
        try:
            o.cpu.reg_write(UC_X86_REG_EBP,frame);o.cpu.reg_write(o.sp,frame-512)
            o.cpu.emu_start(0x49ebb5,0x49ec5c,count=20000)
            assert o.cpu.reg_read(o.ip) in (0x49ec5c,0x49dedf)
        finally:o.cpu.hook_del(hook)
        lines.append(' '.join(map(str,[owner,hosted,mask,expired,group,flag,tech,requested[0] if requested else -1])))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routine':'0x49ebb5..0x49ec5c','cases':len(lines),'scope':__doc__.strip()}
    if a.gold:a.gold.write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:a.vectors.write_text('# owner hosted knowledge_mask expired group group_flag advance recipient\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
