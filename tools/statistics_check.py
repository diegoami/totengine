#!/usr/bin/env python3
"""Execute original own-city population and landmass-count accumulation.

0x507794..0x507908, with original landmass helper. Inputs are already
admitted live/valid/owned cities; filtering and phase placement are separately
read from the enclosing original pass. No map/city records are emitted.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct
from research_check import OriginalCost


class OriginalStatistics(OriginalCost):
    def run(self, cities):
        from unicorn.x86_const import UC_X86_REG_EBP
        frame=0x108100
        self.cpu.reg_write(UC_X86_REG_EBP,frame)
        self.write(frame+8,1,'<I')
        for a,v,f in ((0x6705a0,64,'<h'),(0x6705a2,48,'<h'),(0x6762f8,0,'<I'),
                      (0x5deaa8,0x700000,'<I'),(0x5f6910+3348,0,'<h'),(0x5f6914+3348,0,'<h')):
            self.write(a,v,f)
        self.cpu.mem_write(0x5f6e70+3348,bytes(256))
        self.cpu.mem_write(0x5f6f70+3348,bytes(256))
        for i,(size,landmass,water) in enumerate(cities):
            city=bytearray(92);city[11]=size;city[59]=255
            self.cpu.mem_write(0x5fd150,bytes(city))
            self.cpu.mem_write(0x700000,bytes((10 if water else 1,0,0,landmass,0,240)))
            for offset,value in ((-0x94c,0),(-0x858,0),(-0x870,0),(-0x878,0)):
                self.write(frame+offset,value,'<I')
            self.cpu.reg_write(self.sp,frame-0x1000)
            self.cpu.emu_start(0x507794,0x507908,count=1000)
            assert self.cpu.reg_read(self.ip)==0x507908
        count=struct.unpack('<h',self.cpu.mem_read(0x5f6910+3348,2))[0]
        population=struct.unpack('<h',self.cpu.mem_read(0x5f6914+3348,2))[0]
        landmasses=list(self.cpu.mem_read(0x5f6e70+3348,256))
        return count,population,landmasses


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold');p.add_argument('--vectors');a=p.parse_args()
    binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes();oracle=OriginalStatistics(binary)
    rng=random.Random(79);lines=[]
    for i in range(128):
        cities=[(rng.randrange(1,128),rng.choice((0,1,2,254,255)),rng.randrange(8)==0) for _ in range(rng.randrange(1,40))]
        if i==0:cities=[(127,1,False)]*260
        count,pop,land=oracle.run(cities)
        lines.append(' '.join(','.join(map(str,(s,l,int(w)))) for s,l,w in cities)+' | '+str(count)+' '+str(pop)+' | '+' '.join(f'{i},{v}' for i,v in enumerate(land) if v))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'fragment':'0x507794..0x507908','cases':128,
            'scope':'Admitted own-city population and surface landmass city counts, including byte wrapping and ocean exclusion.'}
    if a.gold:Path(a.gold).write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:Path(a.vectors).write_text('# size,landmass,water ... | count population | landmass,count ...\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
