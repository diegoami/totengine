#!/usr/bin/env python3
"""Execute a controlled 50-turn native city phase, with presentation boundaries.

Standard reference terrain/cities; all human-mask, Chieftain, size-one start,
Capitalization, 100% tax, and unhomed units isolate city phase sequencing.
This is original-x86 execution, not Wine/autosave parity or an AI campaign.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
from yields_check import OriginalCity


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold');p.add_argument('--vectors');p.add_argument('--round-order',action='store_true');a=p.parse_args()
    from unicorn import UC_HOOK_CODE
    home=Path(os.environ['CIVTOT_HOME']);binary=(home/'civ2.bak').read_bytes()
    o=OriginalCity(binary);data,walk,rules,cities=o.load_saved(home,'Original/Ph_Auto2.SAV')
    o.cpu.mem_map(0,4096) # native production's FS:[0] exception-registration chain
    o.write(0x60765b,255,'<B');o.write(0x607658,0,'<B');o.write(0x5d75b0,0,'<I')
    for civ in range(8):
        o.write(0x5f68a0+3348*civ+27,0,'<B');o.write(0x5f68a0+3348*civ+28,10,'<B')
    for i,c in enumerate(cities):
        base=0x5fd150+92*i
        for offset,value,fmt in ((24,0,'<I'),(59,-39,'<b'),(6,0,'<I'),(11,1,'<B'),(28,0,'<h'),(50,1<<20,'<I')):
            o.write(base+offset,value,fmt)
        o.cpu.mem_write(base+54,bytes([c[54]&2,0,0,0,0]))
    for j in range(walk['units']['count']):o.write(0x607c58+40*j+18,255,'<B')
    ids={s:i for i,s in enumerate(rules['CIVILIZE']['comments'])}
    for i,row in enumerate(rules['CIVILIZE']['rows'][:100]):
        first=row[3];o.write(0x5d0a25+20*i,int(first!='no'),'<B')
        o.write(0x5d0a2a+20*i,-1 if first=='nil' else -2 if first=='no' else ids[first],'<b')
    for i,row in enumerate(rules['IMPROVE']['rows'][:40]):
        for offset,field in ((4,1),(5,2)):o.write(0x5f6680+8*i+offset,int(row[field])&255,'<B')
    # UI query, notification dialogs and map redraw only. Gameplay callees run.
    boundary={0x411c50:-1,0x4c52ec:0,0x4cbc30:0,0x467c89:0}
    used=set()
    def hook(cpu,address,length,context):
        if address in boundary:
            used.add(address);sp=cpu.reg_read(o.sp);ret=struct.unpack('<I',cpu.mem_read(sp,4))[0]
            cpu.reg_write(o.sp,sp+4);cpu.reg_write(o.ip,ret);cpu.reg_write(o.ax,boundary[address]&0xffffffff)
    o.cpu.hook_add(UC_HOOK_CODE,hook)
    lines=[]
    for turn in range(43,93):
        o.write(0x607648,turn,'<h')
        order=sorted(range(len(cities)),key=lambda i:(cities[i][10],-i)) if a.round_order else range(len(cities))
        for i in order:
            c=cities[i]
            o.write(0x67a6e0,c[10],'<i');o.write(0x6762f8,0,'<i')
            result=o.call(0x4cb01d,i);assert result!=-999,(turn,i,'city destroyed')
            record=bytes(o.cpu.mem_read(0x5fd150+92*i,92))
            values=[record[11],*struct.unpack_from('<2h',record,28),struct.unpack_from('<I',record,50)[0],
                    struct.unpack_from('<I',record,24)[0],struct.unpack_from('<I',record,6)[0],
                    struct.unpack_from('<h',record,32)[0],*struct.unpack_from('<3H',record,76),*record[82:86],
                    struct.unpack('<i',o.cpu.mem_read(0x5f68aa+3348*c[10],4))[0]]
            lines.append(' '.join(map(str,[turn,i,*values])))
        if (turn-42)%10==0:print('native city turns',turn-42,flush=True)
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routine':'0x4cb01d',
            'turns':50,'cities':len(cities),'cases':len(lines),'presentation_boundaries':[hex(x) for x in sorted(used)],
            'scope':__doc__.strip()}
    if a.round_order:report['city_order']='civ 0..7, descending city slots per civ'
    if a.gold:Path(a.gold).write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:Path(a.vectors).write_text('# turn city size food_box shield_box worked specialists flags route_trade science taxes trade food shields happy unhappy treasury\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
