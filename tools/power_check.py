#!/usr/bin/env python3
"""Execute original power score/ranking pass up to its diplomacy boundary.

No replacement callees: 0x46de84..0x46e354 including city objective values.
Only derived scores, ranks, references and synthetic inputs are emitted.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct
from research_check import OriginalCost
from totctl import walk_save


class OriginalPower(OriginalCost):
    def run(self, alive, human, flags, scenario_flags, inputs, city_flags, wonders, saved=None):
        from unicorn import UC_HOOK_CODE
        from unicorn.x86_const import UC_X86_REG_EBP
        for a,v,f in ((0x60765a,alive,'<B'),(0x60765b,human,'<B'),
                      (0x607640,flags,'<H'),(0x5f5e50,scenario_flags,'<H'),
                      (0x607648,200,'<h'),(0x607668,7,'<h'),(0x6762f8,0,'<I')):
            self.write(a,v,f)
        self.cpu.mem_write(0x607772,bytes(8))
        self.cpu.mem_write(0x5f4700,bytes(80*40))
        for c,(tech,pop,money) in enumerate(inputs,1):
            b=0x5f68a0+3348*c
            self.write(b+24,tech,'<B');self.write(b+116,pop,'<h');self.write(b+10,money,'<i')
            self.cpu.mem_write(b+224,bytes(80))
            record=bytearray(92)
            record[10]=c
            struct.pack_into('<I',record,6,city_flags[c-1])
            struct.pack_into('<I',record,86,c)
            self.cpu.mem_write(0x5fd150+92*(c-1),bytes(record))
        for i,city in enumerate(wonders): self.write(0x607736+2*i,city,'<h')
        if saved is not None:
            data=saved.read_bytes();walk=walk_save(str(saved))
            self.cpu.mem_write(0x607638,data[652:1250])
            self.cpu.mem_write(0x5f68a0,data[2944:29728])
            start=walk['cities']['offset'];end=start+92*walk['cities']['count']
            self.cpu.mem_write(0x5fd150,data[start:end])
            if data[660] & 128:
                leaders=1 if data[678]==0 else data[687].bit_count()
                parameters=end+1453+60*leaders
                self.cpu.mem_write(0x5f5e50,data[parameters:parameters+100])
        scores=[]
        def capture(cpu,address,size,data):
            if address==0x46e1a1:
                bp=cpu.reg_read(UC_X86_REG_EBP)
                scores.extend(struct.unpack('<7i',cpu.mem_read(bp-0x40,28)))
        hook=self.cpu.hook_add(UC_HOOK_CODE,capture)
        try:
            self.cpu.mem_write(0x108000,struct.pack('<I',0x100000))
            self.cpu.reg_write(self.sp,0x108000)
            self.cpu.emu_start(0x46de84,0x46e354,count=100000)
            assert self.cpu.reg_read(self.ip)==0x46e354
        finally: self.cpu.hook_del(hook)
        ranks=list(self.cpu.mem_read(0x607772,8));order=list(self.cpu.mem_read(0x60777a,8))
        refs=list(self.cpu.mem_read(0x607770,2))
        return scores,ranks,order,refs


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold');p.add_argument('--vectors');p.add_argument('--saved-vectors')
    a=p.parse_args();binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes()
    oracle=OriginalPower(binary);rng=random.Random(89);lines=[]
    for i in range(256):
        alive=rng.randrange(256);human=rng.randrange(256);flags=rng.choice((0,128));sf=rng.choice((0,2,6))
        inputs=[(rng.randrange(100),rng.randrange(300),rng.randrange(32001)) for _ in range(7)]
        cf=[rng.choice((0,0x04000000)) for _ in range(7)]
        wonders=[rng.randrange(-2,7) for _ in range(28)]
        scores,ranks,order,refs=oracle.run(alive,human,flags,sf,inputs,cf,wonders)
        values=[alive,human,flags,sf]+[v for row in inputs for v in row]+cf+wonders
        lines.append(' '.join(map(str,values))+' | '+' '.join(map(str,scores+ranks+order+refs)))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routine':'0x46de84..0x46e354',
            'cases':len(lines),'scope':'Native power score, objective-city value, rank sorting, order and human rank references; diplomacy tail excluded.'}
    if a.gold:Path(a.gold).write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:Path(a.vectors).write_text('# alive human flags scenario_flags [tech population treasury]*7 city_flags*7 wonders*28 | scores*7 ranks*8 order*8 references*2\n'+'\n'.join(lines)+'\n')
    if a.saved_vectors:
        saved=[]
        for name in ('Original/Ph_Auto2.SAV','FallRome/ROMFALL.scn'):
            path=Path(os.environ['CIVTOT_HOME'])/name
            scores,ranks,order,refs=oracle.run(0,0,0,0,[(0,0,0)]*7,[0]*7,[-1]*28,path)
            saved.append(name+' '+' '.join(map(str,scores+ranks+order+refs)))
        Path(a.saved_vectors).write_text('# save scores*7 ranks*8 order*8 references*2\n'+'\n'.join(saved)+'\n')
        report['saved_inputs']=len(saved)
        if a.gold:Path(a.gold).write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
