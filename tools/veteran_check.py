#!/usr/bin/env python3
"""Original production veteran eligibility, including one-shot exemption.

Execute 0x4c6728..0x4c68aa, with original improvement and active-wonder
helpers, no replacements. Numeric synthetic inputs only.
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
    binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes();o=OriginalCost(binary);frame=0x108100;rng=random.Random(887);lines=[]
    o.cpu.mem_write(0x5f5c18,b'\xff'*28)
    for _ in range(512):
        domain=rng.randrange(3);role=rng.randrange(8);flags=rng.choice((0,4096));option=rng.randrange(2)
        barracks=rng.randrange(2);port=rng.randrange(2);airport=rng.randrange(2);sun=rng.randrange(2);lighthouse=rng.randrange(2)
        city=bytearray(92);city[10]=1;struct.pack_into('<I',city,86,1)
        for building,present in ((2,barracks),(34,port),(32,airport)):
            if present:city[54+building//8]|=1<<(building%8)
        o.cpu.mem_write(0x5fd150,bytes(city));o.cpu.mem_write(0x607c58,bytes(40));o.cpu.mem_write(0x607736,b'\xff\xff'*28)
        for address,value,fmt in ((0x5f4709,domain,'<B'),(0x5f4712,role,'<B'),(0x5f4704,flags,'<H'),(0x607638,16*option,'<H'),
            (0x607736+14,0 if sun else -1,'<h'),(0x607736+6,0 if lighthouse else -1,'<h'),(frame+8,0,'<I'),(frame-0x60,0,'<I'),(frame-0x80,1,'<I'),(frame-0x7c,0,'<I')):o.write(address,value,fmt)
        o.cpu.reg_write(UC_X86_REG_EBP,frame);o.cpu.reg_write(o.sp,frame-512)
        o.cpu.emu_start(0x4c6728,0x4c68aa,count=10000);assert o.cpu.reg_read(o.ip)==0x4c68aa
        result=int(bool(struct.unpack('<H',o.cpu.mem_read(0x607c5e,2))[0]&8192))
        lines.append(' '.join(map(str,[domain,role,flags,option,barracks,port,airport,sun,lighthouse,result])))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routine':'0x4c6728..0x4c68aa','cases':len(lines),'replacement_callees':[],'scope':__doc__.strip()}
    if a.gold:a.gold.write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:a.vectors.write_text('# domain role flags option barracks port airport sun_tzu lighthouse veteran\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
