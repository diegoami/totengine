#!/usr/bin/env python3
"""Execute full original defence strength, including walls and air-in-city.

Invented unit/tile/city inputs on a loaded surface map. Original city lookup,
building and wonder callees execute; no replacements. Numeric vectors only.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct
from yields_check import OriginalCity


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold',type=Path);p.add_argument('--vectors',type=Path);a=p.parse_args()
    home=Path(os.environ['CIVTOT_HOME']);binary=(home/'civ2.bak').read_bytes();o=OriginalCity(binary)
    o.load_saved(home,'Original/Ph_Auto2.SAV')
    o.cpu.mem_write(0x607736,b'\xff\xff'*28);o.write(0x607668,1,'<h');o.write(0x607666,2,'<h')
    width=struct.unpack('<h',o.cpu.mem_read(0x6705a0,2))[0]
    tile=struct.unpack('<I',o.cpu.mem_read(0x5deaa8,4))[0]+6*(20*(width//2)+10)
    rng=random.Random(15);lines=[]
    for i in range(1536):
        domain=rng.randrange(3);defense=rng.randrange(1,16);veteran=rng.randrange(2);terrain=rng.choice((2,3,4,6))
        river=rng.randrange(2);fortified=rng.randrange(2);fortress=rng.randrange(2);city=rng.randrange(2);walls=rng.randrange(2)
        att_domain=rng.randrange(-1,3);att_flags=rng.choice((0,16,64,4096,16|4096,16|64));def_flags=rng.choice((0,16))
        for k,dom,flags in ((0,max(0,att_domain),att_flags),(1,domain,def_flags)):
            unit=bytearray(40);struct.pack_into('<hhhH',unit,0,20,20 if k else 18,0,8192 if k and veteran else 0)
            unit[8]=k;unit[9]=2 if k else 1;unit[17]=2 if k and fortified else 255;struct.pack_into('<I',unit,30,k+1)
            o.cpu.mem_write(0x607c58+40*k,bytes(unit));o.write(0x5f4700+40*k+9,dom,'<B');o.write(0x5f4700+40*k+4,flags,'<H')
        o.write(0x5f4700+40+13,defense,'<B');o.write(0x5d11ec+24+5,terrain,'<B')
        record=bytearray(92);struct.pack_into('<hh',record,0,20,20);record[10]=2;record[11]=3
        struct.pack_into('<I',record,86,1 if city else 0);record[55]=1 if walls else 0
        o.cpu.mem_write(0x5fd150,bytes(record));o.cpu.mem_write(tile,bytes((1|128*river,2 if city else 64*fortress,0,1,255,32)))
        result=o.call(0x53f67a,1,1,0 if att_domain>=0 else 0xffffffff)
        lines.append(' '.join(map(str,[domain,defense,veteran,terrain,river,fortified,int(fortress and not city),city,walls,att_domain,att_flags,def_flags,result])))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routine':'0x53f67a','cases':len(lines),'replacement_callees':[],
        'scope':__doc__.strip()}
    if a.gold:a.gold.write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:a.vectors.write_text('# domain defense veteran terrain river fortified fortress city walls att_domain att_flags def_flags result\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
