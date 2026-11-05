#!/usr/bin/env python3
"""Full original city totals with invented positive routes and final corruption.

Standard surface terrain/cities and fixed workers; vary all governments and
human masks, set rates to 4 science/5 tax and cached partner trade to 64+16*i.
Three positive route slots per city, sequential cache updates. No replaced
callees. Numeric derived fields only, never original city or map records.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
from yields_check import OriginalCity
from city_totals_check import fields


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold',type=Path);p.add_argument('--vectors',type=Path);a=p.parse_args()
    home=Path(os.environ['CIVTOT_HOME']);binary=(home/'civ2.bak').read_bytes();lines=[];positive=0;corrupted=0
    for gov in range(7):
        for humans in (0,254):
            o=OriginalCity(binary);_,_,_,cities=o.load_saved(home,'Original/Ph_Auto2.SAV')
            o.write(0x60765b,humans,'<B')
            for civ in range(8):
                o.write(0x5f68a0+3348*civ+27,4,'<B');o.write(0x5f68a0+3348*civ+28,5,'<B');o.write(0x5f68a0+3348*civ+29,gov,'<B')
            for i in range(len(cities)):
                base=0x5fd150+92*i;o.write(base+32,64+16*i,'<h');o.write(base+60,3,'<b')
                o.cpu.mem_write(base+67,b'\0\x01\x02');o.cpu.mem_write(base+70,struct.pack('<3h',*((i+k)%len(cities) for k in (1,2,3))))
            for i,record in enumerate(cities):
                for address in (0x4c109f,0x4c13fe,0x4c1bdf):o.call(address,i)
                worked=struct.unpack_from('<I',record,50)[0]
                o.cpu.mem_write(0x653848,bytes(12));o.write(0x6537cc,worked>>26,'<i')
                for k in range(21):
                    if worked&(1<<k):o.call(0x4c2b1f,i,k,1)
                for address in (0x4c506e,0x4c34b6,0x4c3919,0x4c45ff):o.call(address,i)
                result=fields(bytes(o.cpu.mem_read(0x5fd150+92*i,92)))
                support=struct.unpack('<i',o.cpu.mem_read(0x6537e8,4))[0];settlers=struct.unpack('<i',o.cpu.mem_read(0x653858,4))[0]
                routes=sum(struct.unpack('<3i',o.cpu.mem_read(0x653810,12)));corruption=struct.unpack('<i',o.cpu.mem_read(0x653800,4))[0]
                positive+=routes>0;corrupted+=routes>0 and corruption>0
                lines.append(' '.join(map(str,[gov,humans,i,support,settlers,*result,routes,corruption])))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routine':'0x4c45ff and original prerequisites/callees','cases':len(lines),'positive_routes':positive,'positive_routes_with_final_corruption':corrupted,'replacement_callees':[],'scope':__doc__.strip()}
    if a.gold:a.gold.write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:a.vectors.write_text('# gov humans city support settlers worked specialists flags food_box route_trade science taxes trade food shields happy unhappy route_arrows final_corruption\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
