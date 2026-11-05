#!/usr/bin/env python3
"""Execute original Leonardo upgrades; suppress redraw only, AI/no notification.

0x49d4f9 runs original wonder ownership, expiry, tech tests, candidate scan
and type/veteran mutation. Deterministic synthetic Rules tables are inputs.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct
from research_check import OriginalCost


class OriginalLeonardo(OriginalCost):
    def run(self, case):
        from unicorn import UC_HOOK_CODE
        enabled,known,t,owner,alive,attrs,damage,spent,rows=case
        for a,v,f in ((0x607652,0,'<B'),(0x60765b,0,'<B'),(0x607666,1,'<h'),
                      (0x607736+28,0 if enabled else -1,'<h'),(0x5f5c18+14,-1,'<b'),
                      (0x6762f8,0,'<I')):self.write(a,v,f)
        c=bytearray(92);c[10]=1;struct.pack_into('<I',c,86,1);self.cpu.mem_write(0x5fd150,bytes(c))
        techs=bytearray(13)
        for i,tech in enumerate((0,1,2,3,4,5,35)):
            if known & (1<<i):techs[tech//8]|=1<<(tech%8)
        self.cpu.mem_write(0x5f6900+3348,bytes(techs))
        units=bytearray(80*40)
        for i in range(80):
            struct.pack_into('<b',units,40*i+8,-1);struct.pack_into('<b',units,40*i+19,-2)
        for i,row in enumerate(rows):
            until,role,defense,cost,prereq,mask=row
            for off,value in ((8,until),(18,role),(13,defense),(17,cost),(19,prereq),(20,mask)):
                units[40*i+off]=value & 255
        self.cpu.mem_write(0x5f4700,bytes(units))
        u=bytearray(40);u[8:13]=bytes((t,owner,spent,0,damage));struct.pack_into('<H',u,6,attrs)
        struct.pack_into('<I',u,30,int(alive));self.cpu.mem_write(0x607c58,bytes(u))
        def ui(cpu,address,size,data):
            if address==0x467d19:
                sp=cpu.reg_read(self.sp);ret=struct.unpack('<I',cpu.mem_read(sp,4))[0]
                cpu.reg_write(self.sp,sp+4);cpu.reg_write(self.ip,ret)
        hook=self.cpu.hook_add(UC_HOOK_CODE,ui)
        try:
            self.cpu.mem_write(0x108000,struct.pack('<II',0x100000,1));self.cpu.reg_write(self.sp,0x108000)
            self.cpu.emu_start(0x49d4f9,0x100000,count=100000)
            assert self.cpu.reg_read(self.ip)==0x100000
        finally:self.cpu.hook_del(hook)
        out=bytes(self.cpu.mem_read(0x607c58,40))
        return out[8],struct.unpack_from('<H',out,6)[0],out[12],out[10]


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold');p.add_argument('--vectors');a=p.parse_args()
    binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes();o=OriginalLeonardo(binary);rng=random.Random(88);lines=[];changed=0
    for i in range(256):
        rows=[(rng.choice((-1,0,1,2,3,4,5,35)),rng.randrange(8),rng.randrange(1,10),rng.randrange(1,12),rng.choice((-2,-1,0,1,2,3,4,5,35)),rng.randrange(256)) for _ in range(16)]
        values=(int(i%8!=0),rng.randrange(128),rng.randrange(16),1 if i%9 else 2,int(i%11!=0),8192 if i%2 else 0,rng.randrange(30),rng.randrange(7))
        if i%2:
            values=list(values);values[1]|=1;values[2]%=14
            role=rng.randrange(4)
            rows[values[2]]=(0,role,10,2,-2,255)
            rows[14]=(-1,role,11,8,0,255)
            rows[15]=(-1,role,12,6,0,255)
        case=(*values,rows);result=o.run(case);changed+=result[0]!=values[2]
        lines.append(' '.join(map(str,values))+' '+' '.join(','.join(map(str,row)) for row in rows)+' | '+' '.join(map(str,result)))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routine':'0x49d4f9','cases':len(lines),'upgraded':changed,
            'scope':'Original ownership/expiry and upgrade dispatch; 16 synthetic unit types, remaining 64 unavailable; AI notifications absent and redraw suppressed.'}
    if a.gold:Path(a.gold).write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:Path(a.vectors).write_text('# enabled knownmask type owner alive attrs damage spent [until,role,defense,cost,prereq,civmask]*16 | type attrs damage spent\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
