#!/usr/bin/env python3
"""Original Colossus/King Richard tile yields before and after global expiry.

Existing standard city/radius and Rules, invented wonder hosts and foreign
knowledge of the actual obsoleting advances. All original callees execute.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
from yields_check import OriginalCity


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold',type=Path);p.add_argument('--vectors',type=Path);a=p.parse_args()
    home=Path(os.environ['CIVTOT_HOME']);binary=(home/'civ2.bak').read_bytes();lines=[];techs=[]
    for colossus in range(2):
        for richard in range(2):
            for expired in range(2):
                o=OriginalCity(binary);_,_,rules,cities=o.load_saved(home,'Original/Ph_Auto2.SAV')
                ids={s:i for i,s in enumerate(rules['CIVILIZE']['comments'])};techs=[ids[rules['ENDWONDER']['rows'][w][0]] for w in (2,8)]
                o.cpu.mem_write(0x607736,b'\xff\xff'*28)
                for w,active in ((2,colossus),(8,richard)):o.write(0x607736+2*w,0 if active else -1,'<h')
                foreign=cities[0][10]%7+1
                for tech in techs:
                    o.write(0x6076d2+tech,(1<<foreign) if expired else 0,'<B')
                    for civ in range(1,8):
                        address=0x5f68a0+3348*civ+96+tech//8
                        value=bytes(o.cpu.mem_read(address,1))[0]&~(1<<(tech%8))
                        if expired and civ==foreign:value|=1<<(tech%8)
                        o.write(address,value,'<B')
                for k in range(21):
                    values=[o.call(0x4c22a7,0,k,kind) for kind in range(3)]
                    lines.append(' '.join(map(str,[colossus,richard,expired,k,*values])))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routine':'0x4c22a7','cases':len(lines),'original_x86_calls':3*len(lines),'obsolete_tech_ids':techs,'scope':__doc__.strip()}
    if a.gold:a.gold.write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:a.vectors.write_text('# colossus richard expired radius food shields trade\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
