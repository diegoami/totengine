#!/usr/bin/env python3
"""Execute original city totals with fixed and automatically assigned workers.

Native map/radius, capital, support, yields, routes, finance and mood helpers.
No calls replaced. No food/shield collection, production or full-turn claim.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
from yields_check import OriginalCity


def fields(record):
    return [struct.unpack_from('<I',record,o)[0] for o in (50,24,6)]+[struct.unpack_from('<h',record,o)[0] for o in (28,32)]+[struct.unpack_from('<H',record,o)[0] for o in (76,78,80)]+list(record[82:86])


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold');p.add_argument('--vectors');a=p.parse_args()
    home=Path(os.environ['CIVTOT_HOME']);binary=(home/'civ2.bak').read_bytes();lines=[];reports=[]
    for name in ('Original/Ph_Auto2.SAV','FallRome/ROMFALL.scn'):
        for mode in ('fixed','auto'):
            o=OriginalCity(binary);data,walk,rules,cities=o.load_saved(home,name);changed=0
            for i,record in enumerate(cities):
                for address in (0x4c109f,0x4c13fe,0x4c1bdf):o.call(address,i)
                if mode=='auto':o.call(0x4c521c,i)
                else:
                    worked=struct.unpack_from('<I',record,50)[0]
                    o.cpu.mem_write(0x653848,bytes(12));o.write(0x6537cc,worked>>26,'<i')
                    for k in range(21):
                        if worked&(1<<k):o.call(0x4c2b1f,i,k,1)
                    for address in (0x4c506e,0x4c34b6,0x4c3919,0x4c45ff):o.call(address,i)
                result=fields(bytes(o.cpu.mem_read(0x5fd150+92*i,92)))
                changed+=result!=fields(record)
                support=struct.unpack('<i',o.cpu.mem_read(0x6537e8,4))[0]
                settlers=struct.unpack('<i',o.cpu.mem_read(0x653858,4))[0]
                lines.append(name+' '+mode+' '+str(i)+' '+' '.join(map(str,[support,settlers]+result)))
            reports.append({'save':name,'mode':mode,'cities':len(cities),'changed_from_saved':changed})
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routines':['0x4c109f','0x4c13fe','0x4c1bdf','0x4c521c'],
            'cases':len(lines),'saved':reports,
            'scope':'Original sequential city-total recomputation, fixed/auto worker modes, no replacement callees; excludes growth, box collection, production, treasury and turn events.'}
    if a.gold:Path(a.gold).write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:Path(a.vectors).write_text('# save mode city_index support settlers worked specialists flags food_box route_trade science taxes trade food shields happy unhappy\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
