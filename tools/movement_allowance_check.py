#!/usr/bin/env python3
"""Full original maximum movement: naval wonders/advance and damage rounding.

Original 0x568da2 with a three-third movement quantum, type maximum HP,
active Lighthouse/Magellan lookups and Nuclear Power knowledge. No stubs.
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
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold',type=Path);p.add_argument('--vectors',type=Path);a=p.parse_args()
    binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes();o=OriginalCost(binary);rng=random.Random(8812);lines=[]
    o.write(0x5f5eb8,3,'<B');o.cpu.mem_write(0x5f5c18,b'\xff'*28);o.write(0x5f5c18+3,0,'<b')
    city=bytearray(92);city[10]=1;struct.pack_into('<I',city,86,1);o.cpu.mem_write(0x5fd150,bytes(city))
    for _ in range(1024):
        moves=rng.randrange(13);domain=rng.randrange(3);flags=rng.choice((0,32));hp=rng.choice((10,20,40));damage=rng.randrange(36)
        persist,nuclear,lighthouse,magellan,expired=[rng.randrange(2) for _ in range(5)]
        unit=bytearray(40);unit[9]=1;unit[12]=damage;o.cpu.mem_write(0x607c58,bytes(unit));o.cpu.mem_write(0x607736,b'\xff\xff'*28)
        for address,value,fmt in ((0x5f470a,3*moves,'<B'),(0x5f4709,domain,'<B'),(0x5f4704,flags,'<H'),(0x5f470e,hp,'<B'),
            (0x607638,16*persist,'<H'),(0x607736+6,0 if lighthouse else -1,'<h'),(0x607736+24,0 if magellan else -1,'<h'),
            (0x6076d2,128 if expired else 0,'<B'),(0x6076d2+59,2 if nuclear else 0,'<B')):o.write(address,value,fmt)
        for civ in range(8):o.cpu.mem_write(0x5f68a0+3348*civ+96,bytes(13))
        o.write(0x5f68a0+3348+96+59//8,(1<<(59%8)) if nuclear else 0,'<B');o.write(0x5f68a0+3348*7+96,1 if expired else 0,'<B')
        o.cpu.mem_write(0x108000,struct.pack('<II',0x100000,0));o.cpu.reg_write(o.sp,0x108000)
        o.cpu.emu_start(0x568da2,0x100000,count=10000);assert o.cpu.reg_read(o.ip)==0x100000
        result=o.cpu.reg_read(o.ax)
        lines.append(' '.join(map(str,[moves,domain,flags,hp,damage,persist,nuclear,lighthouse,magellan,expired,result])))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routine':'0x568da2','cases':len(lines),'replacement_callees':[],'scope':__doc__.strip()}
    if a.gold:a.gold.write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:a.vectors.write_text('# moves domain flags max_hp damage persist nuclear lighthouse magellan expired allowance_thirds\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
