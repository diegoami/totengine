#!/usr/bin/env python3
"""Execute original city-slot reuse/capacity and full-width identity writes.

Stops before city initialization or capacity notification. Numeric derived
vectors only; no executable or original asset bytes are emitted.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
from research_check import OriginalCost


class OriginalCityAllocation(OriginalCost):
    def slot(self,count,human,free):
        from unicorn import UC_HOOK_CODE
        from unicorn.x86_const import UC_X86_REG_EBP
        frame=0x108100;records=bytearray(92*count)
        for i in range(count):struct.pack_into('<I',records,92*i+86,0 if i==free else 0x10000+i)
        self.cpu.mem_write(0x5fd150,bytes(records));self.write(0x607668,count,'<h')
        self.write(0x60765b,2 if human else 0,'<B');self.write(frame+16,1,'<I')
        stops={0x4329de,0x4329c3,0x4329b6}
        def hook(cpu,address,length,data):
            if address in stops:cpu.emu_stop()
        h=self.cpu.hook_add(UC_HOOK_CODE,hook)
        try:
            self.cpu.reg_write(UC_X86_REG_EBP,frame);self.cpu.reg_write(self.sp,frame-256)
            self.cpu.emu_start(0x432927,0x100000,count=10000)
            assert self.cpu.reg_read(self.ip) in stops
            slot=-1 if self.cpu.reg_read(self.ip)!=0x4329de else struct.unpack('<i',self.cpu.mem_read(frame-36,4))[0]
        finally:self.cpu.hook_del(h)
        return slot,struct.unpack('<h',self.cpu.mem_read(0x607668,2))[0]


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold');p.add_argument('--vectors');a=p.parse_args()
    binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes();o=OriginalCityAllocation(binary);lines=[]
    for count in (0,1,3,254,255,256,258):
        for human in (0,1):
            for free in sorted(set((-1,0 if count else -1,count//2 if count else -1,count-1))):
                try:result=o.slot(count,human,free)
                except Exception:
                    print('failure',count,human,free,hex(o.cpu.reg_read(o.ip)));raise
                lines.append('A '+' '.join(map(str,[count,human,free,*result])))
    from unicorn.x86_const import UC_X86_REG_EBP
    identities=[];frame=0x108100
    for counter in (1,65535,65536,0xfffffffe):
        o.write(frame-36,0,'<I');o.write(0x5d1504,counter,'<I')
        o.cpu.reg_write(UC_X86_REG_EBP,frame);o.cpu.reg_write(o.sp,frame-256)
        o.cpu.emu_start(0x432ad7,0x432af6,count=30)
        identities.append([counter,struct.unpack('<I',o.cpu.mem_read(0x5fd1a6,4))[0],struct.unpack('<I',o.cpu.mem_read(0x5d1504,4))[0]])
    allocation_cases=len(lines)
    for count in (1,2,3,254,255,258):
        for target in sorted(set((0,count//2,count-1))):
            o.write(0x607668,count,'<h');o.write(frame+8,target,'<I')
            o.cpu.reg_write(UC_X86_REG_EBP,frame);o.cpu.reg_write(o.sp,frame-256)
            o.cpu.emu_start(0x4348d7,0x4348ef,count=30)
            result=struct.unpack('<h',o.cpu.mem_read(0x607668,2))[0]
            lines.append('D '+' '.join(map(str,[count,target,result])))
    report={'binary_sha256' :hashlib.sha256(binary).hexdigest(),'routines':['0x432927..0x4329de','0x432ad7..0x432af6','0x4348d7..0x4348ef'],
            'allocation_cases':allocation_cases,'deletion_cases':len(lines)-allocation_cases,'identity_writes':identities,
            'scope':'Original first-dead city slot, 255-slot append limit independent of human mask, counter increment and u32 identity write, last-slot deletion decrements extent once; excludes city initialization, UI, patched capacity and loader renumbering.'}
    lines.extend('I '+' '.join(map(str,row)) for row in identities)
    if a.gold:Path(a.gold).write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:Path(a.vectors).write_text('# A count human dead_slot allocated_slot new_count; D count deleted_slot new_count; I counter written_id next_counter\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
