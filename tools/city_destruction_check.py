#!/usr/bin/env python3
"""Execute native city destruction on controlled saved inputs.

Derived identity/home/route/wonder changes only. No game assets emitted.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
from yields_check import OriginalCity


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold');p.add_argument('--vectors');p.add_argument('--map-vectors');a=p.parse_args()
    from unicorn import UC_HOOK_CODE
    home=Path(os.environ['CIVTOT_HOME']);binary=(home/'civ2.bak').read_bytes();lines=[];tile_changes=[]
    offsets=None
    for humans in (0,16,255):
        for target in range(10):
            o=OriginalCity(binary);data,walk,rules,cities=o.load_saved(home,'Original/Ph_Auto2.SAV');o.cpu.mem_map(0,4096)
            o.write(0x6762f8,0,'<i');o.write(0x659254,1,'<i');o.write(0x60765b,humans,'<B')
            offsets=list(zip(struct.unpack('<45b',o.cpu.mem_read(0x5d1620,45)),struct.unpack('<45b',o.cpu.mem_read(0x5d1650,45))))
            for j in range(walk['units']['count']):o.write(0x607c58+40*j+18,target,'<B')
            # Type nine's special home-clearing branch, without changing stack links.
            o.write(0x607c58+8,9,'<B')
            for j,c in enumerate(cities):
                base=0x5fd150+92*j;o.write(base+60,3,'<b')
                for k in range(3):o.write(base+67+k,k-1,'<b');o.write(base+70+2*k,target if k!=1 else (target+1)%10,'<h')
            for k in range(28):o.write(0x607736+2*k,target if k%3==0 else -1,'<h')
            boundary={0x467c89:0,0x467d19:0,0x4326d7:0,0x4edddc:4}
            def hook(cpu,address,length,context):
                if address in boundary:
                    sp=cpu.reg_read(o.sp);ret=struct.unpack('<I',cpu.mem_read(sp,4))[0]
                    cpu.reg_write(o.sp,sp+4+boundary[address]);cpu.reg_write(o.ip,ret);cpu.reg_write(o.ax,0)
            o.cpu.hook_add(UC_HOOK_CODE,hook)
            before=bytes(o.cpu.mem_read(0x700000,6*walk['map_header']['values'][2]))
            try:o.call(0x43456d,target,0)
            except Exception:
                from unicorn.x86_const import UC_X86_REG_EBP
                f=o.cpu.reg_read(UC_X86_REG_EBP);chain=[]
                for _ in range(15):
                    if not 0x100000<=f<0x110000:break
                    f,r=struct.unpack('<II',o.cpu.mem_read(f,8));chain.append(hex(r))
                print('failure',humans,target,chain,flush=True);raise
            counts=[struct.unpack('<h',o.cpu.mem_read(0x5f6910+3348*c,2))[0] for c in range(8)]
            units=[]
            for j in range(walk['units']['count']):
                u=bytes(o.cpu.mem_read(0x607c58+40*j,40));units += [int(struct.unpack_from('<I',u,30)[0]!=0),u[18]]
            result=[]
            for j in range(10):
                c=bytes(o.cpu.mem_read(0x5fd150+92*j,92))
                result += [struct.unpack_from('<I',c,86)[0],struct.unpack_from('<I',c,6)[0],c[60],*struct.unpack_from('<3b',c,67),*struct.unpack_from('<3h',c,70)]
            wonders=struct.unpack('<28h',o.cpu.mem_read(0x607736,56))
            lines.append(' '.join(map(str,[humans,target,*counts,*units,*result,*wonders])))
            after=bytes(o.cpu.mem_read(0x700000,len(before)))
            tile_changes.append({'humans':humans,'city':target,'changes':[[n//6,n%6,x,y] for n,(x,y) in enumerate(zip(before,after)) if x!=y]})
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routine':'0x43456d','cases':len(lines),
            'radius45':offsets,'tile_change_count':sum(len(c['changes']) for c in tile_changes),
            'scope':'Surface city removal with all saved units homed to target, first unit changed to type 9, synthetic incoming routes and wonder table; replaces map redraw/city UI callbacks, executes unit removal and route helpers; excludes caller civilization elimination and non-surface refresh.'}
    if a.gold:Path(a.gold).write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:Path(a.vectors).write_text('# humans city counts*8 [alive home]*units [id flags routes commodities*3 partners*3]*cities wonders*28\n'+'\n'.join(lines)+'\n')
    if a.map_vectors:
        Path(a.map_vectors).write_text('# humans city tile_index byte_offset before after\n'+''.join(
            ' '.join(map(str,[case['humans'],case['city'],*change]))+'\n'
            for case in tile_changes for change in case['changes']))
    print(json.dumps({k:v for k,v in report.items() if k not in ('tile_changes','radius45')}))


if __name__=='__main__':main()
