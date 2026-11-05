#!/usr/bin/env python3
"""Execute original city-radius availability on reference saves, no stubs.

Original 0x4c109f and the radius-overlap part of 0x4c13fe. No original
records/maps/names are emitted; vectors contain derived per-radius flags.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
from connections_check import OriginalConnection,saved_model
from totctl import walk_save,parse_rules


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold');p.add_argument('--vectors');a=p.parse_args()
    home=Path(os.environ['CIVTOT_HOME']);binary=(home/'civ2.bak').read_bytes();lines=[]
    for name in ('Original/Ph_Auto2.SAV','FallRome/ROMFALL.scn'):
        path=home/name;rules_path=home/Path(name).parent/'Rules.txt'
        model,cities=saved_model(path,rules_path);data=path.read_bytes();walk=walk_save(str(path));rules=parse_rules(str(rules_path))
        o=OriginalConnection(binary);o.load(model)
        o.cpu.mem_write(0x5fd150,b''.join(cities));o.cpu.mem_write(0x5f68a0,data[2944:29728]);o.cpu.mem_write(0x607638,data[652:1250])
        start=walk['units']['region'][0];count=walk['units']['count'];o.cpu.mem_write(0x607c58,data[start:start+40*count])
        o.write(0x5d75a8,0,'<I')
        for i,row in enumerate(rules['UNITS']['rows'][:80]):
            if len(row)>11:
                o.write(0x5f4709+40*i,int(row[2]),'<b');o.write(0x5f470c+40*i,int(row[5].rstrip('aA')),'<b')
        for i in range(len(cities)):
            for address,stop in ((0x4c109f,0x100000),(0x4c13fe,0x4c1710)):
                o.cpu.mem_write(0x108000,struct.pack('<II',0x100000,i));o.cpu.reg_write(o.sp,0x108000)
                o.cpu.emu_start(address,stop,count=1000000)
                assert o.cpu.reg_read(o.ip)==stop,hex(o.cpu.reg_read(o.ip))
            blocked=list(o.cpu.mem_read(0x6537b0,20))
            lines.append(name+' '+str(i)+' '+' '.join(map(str,blocked)))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routines':['0x4c109f','0x4c13fe..0x4c1710'],
            'cases':len(lines),'scope':'Saved-input city worker radius flags, original map, visibility, stacks and city-overlap helpers; no replacements; no full city-turn claim.'}
    if a.gold:Path(a.gold).write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:Path(a.vectors).write_text('# save city_index availability_flags*20\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
