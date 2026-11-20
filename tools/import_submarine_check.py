#!/usr/bin/env python3
"""Execute native submarine/detector and mixed-stack observation contexts.

The original 0x41f8a7..0x4205a7 loops and callees execute unchanged, with
native redraw disabled. Conversations stop at 0x526aca entry; none are
executed or replaced. This checks knowledge/order writes and call boundaries,
not combat, transport, full movement, diplomatic outcomes or runtime entry.
"""
import argparse
from collections import Counter
import hashlib
import itertools
import json
import os
from pathlib import Path
import struct

from import_contact_check import run
from import_knowledge_check import OriginalCity, FRAME, UNIT, CITY


def observation(o,values):
    base = values[:13]
    extended,submarines,detectors,stack_mode,tail_order,mixed_domain = values[13:]
    def configure(o,tiles,units,cities):
        o.cpu.mem_write(0x5F4700,bytes(160))
        for slot,u in enumerate(units):
            if not struct.unpack_from('<I',u,30)[0]:
                continue
            u[8],u[11] = slot,1
            if slot % 2:
                u[17] = tail_order
            flags = (1 if extended & (1<<slot) else 0) | (8 if submarines & (1<<slot) else 0) | (0x4000 if detectors & (1<<slot) else 0)
            domain = base[0 if slot<2 else 1]
            if slot % 2 and mixed_domain & (1<<(slot//2)):
                domain = 0
            for off,value,fmt in [(4,flags,'<I'),(9,domain,'<B'),(12,1,'<B'),(18,base[6],'<B')]:
                o.write(0x5F4700+40*slot+off,value,fmt)
        if stack_mode == 1:
            units[3][:] = bytes(40)
            struct.pack_into('<h',units[2],28,-1)
        for first in ((2,) if stack_mode==2 else (0,2) if stack_mode==3 else ()):
            struct.pack_into('<hh',units[first],26,first+1,-1)
            struct.pack_into('<hh',units[first+1],26,-1,first)
        for slot,u in enumerate(units):
            o.cpu.mem_write(UNIT+40*slot,bytes(u))
        for slot,c in enumerate(cities):
            if struct.unpack_from('<I',c,86)[0]:
                c[14] = 1
                o.cpu.mem_write(CITY+92*slot,bytes(c))
        # Mode 2 counts members, independent of flags, including from a tail.
        assert o.call(0x56B64B,2,2) == (1 if stack_mode==1 else 2)
        o.write(FRAME-0x3C,int(bool(extended & 1)),'<I')
        o.write(FRAME-0x20,8 if submarines & 1 else 0,'<I')
    return run(o,*base,configure=configure)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--out',type=Path,required=True)
    p.add_argument('--vectors',type=Path,required=True)
    args=p.parse_args()
    binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes()
    o=OriginalCity(binary)
    rows=['# X/F: first 13 inputs as import-contact, then extended_mask submarine_mask detector_mask stack_mode tail_order mixed_domain_mask | U/C same output format',
          '# stack_mode: 0 paired, 1 single foreign, 2 reverse foreign, 3 reverse both; bit masks use unit slots 0..3; mixed-domain mask makes the corresponding tail land-domain']
    counts=Counter()
    witnesses=Counter()
    for j,(subs,det,ext,mode,city) in enumerate(itertools.product((0,1,4,5,8,12),(0,1,4,8,5),(0,5,15),range(4),range(4))):
        base=[j%4,(j//4)%4,(1,8,21)[(j//16)%3],city,(j//3)%4,
              12,7 if (j//7)%2 else 0,(0,16,32,48)[(j//5)%4],
              5 if (j//11)%2 else 13,(j//13)%2,0,0,(j//17)%2]
        values=base+[ext,subs,det,mode,12 if j%2 else 3,(j//19)%4]
        kind,out=observation(o,values)
        head=3 if mode in (2,3) else 2
        if kind=='U' and base[2]==1 and not city & 2 and not subs & 1 and subs & (1<<head):
            if mode==1 and not det & 1:
                assert not out[2] & 16
                witnesses['single_submarine_hidden_without_detector']+=1
            elif mode!=1 and not det & 1:
                assert out[2] & 16 and out[3] & 16
                witnesses['paired_submarine_seen_without_detector']+=1
            elif mode==1 and det & 1:
                assert out[2] & 16
                witnesses['single_submarine_seen_by_detector']+=1
        rows.append('X '+' '.join(map(str,values))+' | '+kind+' '+' '.join(map(str,out)))
        counts['completed' if kind=='U' else 'contact_boundary']+=1
    # Explicitly pin both review notes: extended=0 and goto-ordered tails.
    for k,ext,ocean in itertools.product((1,8,21),(0,15),(0,3)):
        base=[0,0,k,0,ocean,12,0,16,5,0,0,0,0]
        values=base+[ext,0,0,0,12,0]
        kind,out=observation(o,values)
        assert kind=='U'
        if k==1 and ocean==0:
            assert out[4:8]==[255,12,255,12]
        rows.append('F '+' '.join(map(str,values))+' | '+kind+' '+' '.join(map(str,out)))
        counts['completed']+=1
    vectors='\n'.join(rows)+'\n'
    assert len(witnesses)==3 and all(witnesses.values()),witnesses
    args.vectors.write_text(vectors)
    report={'schema':1,'binary_sha256':hashlib.sha256(binary).hexdigest(),
            'method':__doc__.strip(),'cases':dict(counts),
            'numeric_witnesses':dict(witnesses),
            'vectors_sha256':hashlib.sha256(vectors.encode()).hexdigest(),
            'limits':'Controlled classic single-player surface snapshots and coherent mixed stacks. Submarine/detector observation only; no combat/transport, full movement, conversation outcomes, AI sighting bookkeeping, startup or faithful admission.'}
    args.out.write_text(json.dumps(report,indent=2)+'\n')
    print(dict(counts))


if __name__=='__main__':
    main()
