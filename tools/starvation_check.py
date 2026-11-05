#!/usr/bin/env python3
"""Execute original starvation decisions, route removal and population decrement.

Stops before city destruction. Unit removal is recorded at its call boundary;
presentation setup is skipped. This is not a full city-turn comparison.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct
from research_check import OriginalCost


class OriginalStarvation(OriginalCost):
    def run(self, values):
        from unicorn import UC_HOOK_CODE
        from unicorn.x86_const import UC_X86_REG_EBP
        size, humans, known, routes, outgoing, partner, *rest = values
        commodities, partners, units = rest[:3], rest[3:6], rest[6:]
        frame = 0x108100
        self.write(frame+8, 0, '<I'); self.write(frame-0x34, 1, '<I')
        city = bytearray(92); city[10] = 1; city[11] = size; city[14] = known; city[60] = routes
        struct.pack_into('<h', city, 28, -1)
        for k in range(3):
            struct.pack_into('<b', city, 67+k, commodities[k]); struct.pack_into('<h', city, 70+2*k, partners[k])
        self.cpu.mem_write(0x5fd150, bytes(city))
        self.write(0x60765b, humans, '<B'); self.write(0x607666, 4, '<h')
        self.write(0x653830, outgoing, '<i'); self.write(0x6537f0, partner, '<i')
        for k in range(4):
            alive, role, home = units[3*k:3*k+3]
            unit = bytearray(40); unit[8] = k; unit[18] = home; struct.pack_into('<I', unit, 30, alive)
            self.cpu.mem_write(0x607c58+40*k, bytes(unit)); self.write(0x5f4712+40*k, role, '<B')
        killed = []
        stops = {0x4c5c60, 0x4c5c97}
        def hook(cpu, address, length, data):
            if address == 0x4c5a43: cpu.reg_write(self.ip, 0x4c5ae3)
            elif address == 0x56c376:
                sp = cpu.reg_read(self.sp); ret, index, reason = struct.unpack('<3I', cpu.mem_read(sp,12))
                assert reason == 1; killed.append(index)
                cpu.reg_write(self.sp, sp+4); cpu.reg_write(self.ip, ret)
            elif address in stops: cpu.emu_stop()
        h = self.cpu.hook_add(UC_HOOK_CODE, hook)
        try:
            self.cpu.reg_write(UC_X86_REG_EBP, frame); self.cpu.reg_write(self.sp, frame-256)
            self.cpu.emu_start(0x4c59b9, 0x100000, count=10000)
            assert self.cpu.reg_read(self.ip) in stops, hex(self.cpu.reg_read(self.ip))
            destroyed = self.cpu.reg_read(self.ip) == 0x4c5c60
        finally: self.cpu.hook_del(h)
        c = bytes(self.cpu.mem_read(0x5fd150,92))
        return [c[11], struct.unpack_from('<h',c,28)[0], c[60], struct.unpack_from('<I',c,6)[0],
                killed[0] if killed else -1, int(destroyed),
                *struct.unpack_from('<3b',c,67), *struct.unpack_from('<3h',c,70)]


def main():
    p=argparse.ArgumentParser(description=__doc__); p.add_argument('--gold'); p.add_argument('--vectors'); a=p.parse_args()
    binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes(); o=OriginalStarvation(binary)
    rng=random.Random(1314); lines=[]; kills=deaths=0
    for _ in range(1024):
        values=[rng.randrange(1,16),rng.randrange(256),rng.randrange(256),rng.randrange(4),rng.randrange(4),rng.randrange(-1,4)]
        values += [rng.choice((-1,-1,0,1)) for _ in range(3)] + [rng.randrange(4) for _ in range(3)]
        values += [v for _ in range(4) for v in (rng.randrange(2),rng.choice((1,5)),rng.choice((0,0,255)))]
        result=o.run(values); kills+=result[4]>=0; deaths+=result[5]
        lines.append(' '.join(map(str,values))+' | '+' '.join(map(str,result)))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routine':'0x4c59b9..0x4c5c97',
            'cases':len(lines),'unit_removal_requests':kills,'city_destruction_requests':deaths,
            'scope':'Original negative-box decisions, actual route compaction and population decrement; skip presentation setup, record unit-removal boundary, stop before city destruction; excludes total recomputation/collection/full turns.'}
    if a.gold:Path(a.gold).write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:Path(a.vectors).write_text('# size humans known routes outgoing partner commodities*3 partners*3 [alive role home]*4 | size box routes flags killed destroyed commodities*3 partners*3\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
