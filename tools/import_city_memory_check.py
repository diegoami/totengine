#!/usr/bin/env python3
"""Execute the original post-city-processing remembered-size refresh.

0x470bd1..0x470c4e executes original instructions without replacements.
Prepared owner/slot locals represent a surviving city after processing; this
does not run growth/production, ownership changes or a full civilization phase.
Only city remembered sizes may change; discovery masks and other bytes stay.
"""
import argparse
import hashlib
import itertools
import json
import os
from pathlib import Path
import struct

from import_knowledge_check import OriginalCity, CITY, FRAME, frame, fragment


def refresh(o,owner,allies,apollo,size,slot,known,old):
    cities=[]
    for i in range(3):
        c=bytearray((17*j+29*i)%256 for j in range(92))
        struct.pack_into('<hhH',c,0,2*i,2,0)
        c[10],c[11],c[14]=owner,size,known
        c[15:23]=bytes(old)
        struct.pack_into('<I',c,86,i+1)
        cities.append(c)
    before=b''.join(cities)
    o.cpu.mem_write(CITY,before)
    for a in range(8):
        for b in range(8):
            value=0x104 | (8 if (a==owner and allies & (1<<b)) else 0)
            # A different reverse relation catches accidental transposition.
            if b==owner and a!=owner:
                value=0x104 | (8 if not allies & (1<<a) else 0)
            o.write(0x5F68A0+3348*a+40+4*b,value,'<I')
    o.write(0x607768,apollo,'<h')
    frame(o)
    o.write(FRAME+8,owner,'<I')
    o.write(FRAME-0x24,slot,'<I')
    fragment(o,0x470BD1,0x470C4E)
    after=bytes(o.cpu.mem_read(CITY,92*3))
    expected=bytearray(before)
    for viewer in range(1,8):
        if allies & (1<<viewer) or apollo!=-1:
            expected[92*slot+15+viewer]=size
    assert after==expected
    return list(after[92*slot+14:92*slot+23])


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--out',type=Path,required=True)
    p.add_argument('--vectors',type=Path,required=True)
    args=p.parse_args()
    binary=(Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes()
    o=OriginalCity(binary)
    # Slot 25's saved word is the raw predicate, not an active-wonder helper.
    assert 0x607736+2*25==0x607768
    rows=['# owner allies_mask apollo_word current_size slot known_mask old_sizes[8] | known_mask new_sizes[8]']
    for j,(owner,allies,apollo,size) in enumerate(itertools.product(range(8),(0,1,2,4,8,16,32,64,128,85,170,254,255),(-2,-1,0,7),(1,7,127))):
        slot=j%3
        known=(37*j)%256
        old=[(j+17+13*i)%256 for i in range(8)]
        inputs=[owner,allies,apollo,size,slot,known,*old]
        output=refresh(o,owner,allies,apollo,size,slot,known,old)
        rows.append(' '.join(map(str,inputs))+' | '+' '.join(map(str,output)))
    vectors='\n'.join(rows)+'\n'
    args.vectors.write_text(vectors)
    report={'schema':1,'binary_sha256':hashlib.sha256(binary).hexdigest(),
            'fragment':'0x470bd1..0x470c4e','method':__doc__.strip(),
            'cases':len(rows)-1,'vectors_sha256':hashlib.sha256(vectors.encode()).hexdigest(),
            'limits':'Surviving processed surface cities, owners 0..7 and current sizes 1..127. Memory refresh only; no discovery, phase execution, death/capture timing, live original UI or faithful runtime activation.'}
    args.out.write_text(json.dumps(report,indent=2)+'\n')
    print(f'{len(rows)-1} native post-city memory refreshes passed')


if __name__=='__main__':
    main()
