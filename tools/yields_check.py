#!/usr/bin/env python3
"""Execute original per-radius yields on reference saves, without replacements.

Original 0x4c22a7 and all its helpers. Derived numeric outputs only; collection
side effects are disabled for this isolated check (0x5d75a4 = 0).
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
from connections_check import OriginalConnection,saved_model
from totctl import walk_save,parse_rules


class OriginalCity(OriginalConnection):
    def load_saved(self,home,name):
        path=home/name;rules_path=home/Path(name).parent/'Rules.txt'
        model,cities=saved_model(path,rules_path);data=path.read_bytes();walk=walk_save(str(path));rules=parse_rules(str(rules_path))
        self.load(model);self.cpu.mem_write(0x5fd150,b''.join(cities));self.cpu.mem_write(0x5f68a0,data[2944:29728])
        self.cpu.mem_write(0x607638,data[652:1250]);self.write(0x6705a8,walk['map_header']['values'][4],'<h')
        start=walk['units']['region'][0];count=walk['units']['count'];self.cpu.mem_write(0x607c58,data[start:start+40*count])
        for address in (0x5d75a8,0x5d75ac,0x5d75a4,0x5de648):self.write(address,0,'<I')
        self.write(0x607652,0,'<B')
        for i,row in enumerate(rules['UNITS']['rows'][:80]):
            if len(row)>11:
                for off,field in ((9,2),(12,5),(13,6),(18,11)):
                    self.write(0x5f4700+40*i+off,int(row[field].rstrip('aAdD')) & 255,'<B')
        for i,row in enumerate(rules['COSMIC']['rows']):
            if row and row[0].lstrip('-').isdigit():self.write(0x5f5eb8+i,int(row[0]) & 255,'<B')
        ids={s:i for i,s in enumerate(rules['CIVILIZE']['comments'])}
        for i,row in enumerate(rules['ENDWONDER']['rows'][:28]):self.write(0x5f5c18+i,ids.get(row[0],-1),'<b')
        for i,row in enumerate(rules['TERRAIN']['rows'][:33]):
            if len(row)<6:continue
            for off,field in ((4,1),(5,2),(6,3),(7,4),(8,5),(12,7),(13,11)):
                if field<len(row) and row[field].lstrip('-').isdigit():self.write(0x5d11ec+24*i+off,int(row[field]) & 255,'<B')
        surface=model.width*model.height//2
        cache_start=(0x700000+len(model.tiles)+4095)&~4095;stride=(surface+4095)&~4095
        assert cache_start+7*stride<0x800000
        raw=data[walk['sections_derived']['map_data']['offset']:][:7*surface]
        for civ in range(1,8):
            address=cache_start+(civ-1)*stride;self.write(0x5dead0+4*civ,address,'<I')
            self.cpu.mem_write(address,raw[(civ-1)*surface:civ*surface])
        return data,walk,rules,cities

    def call(self,address,*args):
        self.cpu.mem_write(0x108000,struct.pack('<'+'I'*(len(args)+1),0x100000,*args));self.cpu.reg_write(self.sp,0x108000)
        self.cpu.emu_start(address,0x100000,count=20000000)
        assert self.cpu.reg_read(self.ip)==0x100000,hex(self.cpu.reg_read(self.ip))
        result=self.cpu.reg_read(self.ax)
        return result if result<0x80000000 else result-0x100000000


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--gold');p.add_argument('--vectors');a=p.parse_args()
    home=Path(os.environ['CIVTOT_HOME']);binary=(home/'civ2.bak').read_bytes();lines=[];saved=[]
    for name in ('Original/Ph_Auto2.SAV','FallRome/ROMFALL.scn'):
        o=OriginalCity(binary);data,walk,rules,cities=o.load_saved(home,name)
        sums=[]
        for i,city in enumerate(cities):
            total=[0,0,0];worked=struct.unpack_from('<I',city,50)[0]
            for k in range(21):
                value=[o.call(0x4c22a7,i,k,kind) for kind in range(3)]
                lines.append(name+' '+str(i)+' '+str(k)+' '+' '.join(map(str,value)))
                if worked&(1<<k):total=[x+y for x,y in zip(total,value)]
            sums.append(total)
        saved.append({'save':name,'cities':len(cities),'stored_food_exact':sum(s[0]==c[82] for s,c in zip(sums,cities))})
    report={'binary_sha256':hashlib.sha256(binary).hexdigest(),'routine':'0x4c22a7','cases':len(lines),
            'saved':saved,'scope':'Native per-radius food/shields/trade from saved world and Rules inputs, no replacement callees; not a claim about complete city totals/phase or saved-cache freshness.'}
    if a.gold:Path(a.gold).write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:Path(a.vectors).write_text('# save city_index radius_index food shields trade\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
