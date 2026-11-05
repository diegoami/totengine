#!/usr/bin/env python3
"""Execute human forced-anarchy dispatch on controlled standard-save inputs.

Original 0x52588e -> 0x52522f, including city total refresh; no replaced
callees. Does not cover player-selected revolutions or later government choice.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
from yields_check import OriginalCity
from city_totals_check import fields


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold');p.add_argument('--vectors');a=p.parse_args()
    home=Path(os.environ['CIVTOT_HOME']);binary=(home/'civ2.bak').read_bytes();lines=[]
    for civ in range(1,8):
        for government in range(7):
            o=OriginalCity(binary);data,walk,rules,cities=o.load_saved(home,'Original/Ph_Auto2.SAV')
            for i in range(len(cities)):o.write(0x5fd150+92*i+59,8,'<b')
            o.write(0x60765b,1<<civ,'<B');o.write(0x5f68bd+3348*civ,government,'<B')
            for other in range(8):o.write(0x5f68c8+3348*other+4*civ,0x135+32*other,'<I')
            o.call(0x52588e,civ,0)
            result=[o.cpu.mem_read(0x5f68bd+3348*civ,1)[0]]
            result += [struct.unpack('<I',o.cpu.mem_read(0x5f68c8+3348*other+4*civ,4))[0] for other in range(8)]
            for i in range(len(cities)):
                record=bytes(o.cpu.mem_read(0x5fd150+92*i,92));result+=fields(record)+[struct.unpack_from('<b',record,59)[0]]
            lines.append(' '.join(map(str,[civ,government,*result])))
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routine':'0x52588e -> 0x52522f','cases':len(lines),'queue_control':'all cities unit 8',
            'scope':__doc__.strip()}
    if a.gold:Path(a.gold).write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:Path(a.vectors).write_text('# civ previous_government | government incoming_treaties*8 [worked specialists flags food_box route_trade science taxes trade food shields happy unhappy build_item]*10\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
