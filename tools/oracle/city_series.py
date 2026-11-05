#!/usr/bin/env python3
"""Controlled live city rounds, harvested through the game's save dialog.

Load Original/Ph_Auto2.SAV in the owned oracle first. Fixture matches
city_phase_check.py --round-order. Execute only city phases, not AI turns,
pollution, calendar or barbarian arrivals. Presentation queries/redraws are
replaced during each call; the game GUI thread is suspended by call_game.c.
Raw saves stay in --output (outside the repo); report/vectors are numeric.
The original install must have been backed up before running this tool.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import time
import unitram as u
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from totctl import walk_save

CAVE=0x41562b
HERE=Path(__file__).resolve().parent


def wi(*args):
    result=subprocess.run(['timeout','15','wine',str(HERE/'winject.exe'),*map(str,args)],capture_output=True,text=True,check=True)
    return result.stdout


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--helper',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    p.add_argument('--turns',type=int,default=50);p.add_argument('--gold',type=Path);p.add_argument('--vectors',type=Path)
    a=p.parse_args();home=Path(os.environ['CIVTOT_HOME']);a.output.mkdir(parents=True,exist_ok=True)
    windows=wi('wins');window=next(re.match(r'hwnd=(\w+)',line).group(1) for line in windows.splitlines()
        if 'vis=1' in line and 'Civilization II: Test of Time' in line)
    units,_=u.bases();base=u.u32(0x54afb2+3)-11
    n=u.u16(0x607668);nu=u.u16(0x607666);assert n==10 and u.u16(0x607648)==43
    assert all(not any(p.stem==str(8701+i) and p.suffix.lower()=='.sav' for p in (home/'Original').iterdir()) for i in range(a.turns)), 'save labels 8701.. are already occupied; preserve/clear prior owned artifacts first'
    records=[u.read(base+92*i,92) for i in range(n)]
    u.write(0x60765b,b'\xff');u.write(0x607658,b'\0')
    for civ in range(8):u.write(0x5f68a0+3348*civ+27,b'\0\x0a')
    for i,c in enumerate(records):
        for offset,value,fmt in ((24,0,'<I'),(59,-39,'<b'),(6,0,'<I'),(11,1,'<B'),(28,0,'<h'),(50,1<<20,'<I')):
            u.write(base+92*i+offset,struct.pack(fmt,value))
        u.write(base+92*i+54,bytes([c[54]&2,0,0,0,0]))
    for i in range(nu):u.write(units+40*i+18,b'\xff')
    order=sorted(range(n),key=lambda i:(records[i][10],-i))
    code=b'\x60'
    for i in order:
        code+=b'\xc7\x05'+struct.pack('<II',0x67a6e0,records[i][10])
        code+=b'\x6a'+bytes([i])
        code+=b'\xe8'+struct.pack('<i',0x4cb01d-(CAVE+len(code)+5))+b'\x83\xc4\x04'
    code+=b'\x61\xb8'+struct.pack('<I',0x8080808)+b'\xc2\x04\x00'
    assert len(code)<512
    patches={CAVE:code,0x411c50:b'\xb8\xff\xff\xff\xff\xc3',0x467c89:b'\x31\xc0\xc3'}
    saved=[];lines=[]
    for round_no in range(a.turns):
        turn=43+round_no;u.write(0x607648,struct.pack('<h',turn));u.write(0x6762f8,bytes(4))
        controls=(0x5d75a8,0x5d75ac,0x5d75a4,0x5de648,0x5d75b0)
        backups={addr:u.read(addr,len(value)) for addr,value in patches.items()}
        globals_before={addr:u.read(addr,4) for addr in controls}
        completed=False
        try:
            for addr in controls:u.write(addr,bytes(4))
            for addr,value in patches.items():u.write(addr,value)
            result=subprocess.run(['timeout','40','wine',str(a.helper),window,hex(CAVE)],capture_output=True,text=True,check=True)
            assert result.stdout.strip()==str(0x8080808),result.stdout+result.stderr
            completed=True
        finally:
            if not completed:
                subprocess.run([str(HERE/'stop_game.sh')],check=False)
            else:
                for addr,value in backups.items():u.write(addr,value)
                for addr,value in globals_before.items():u.write(addr,value)
        # Persist completed phase as the next turn, matching engine run_turn.
        u.write(0x607648,struct.pack('<h',turn+1))
        label=str(8701+round_no);start=time.time()
        env={**os.environ,'SCENDIR':str(home/'Original')}
        result=subprocess.run([str(HERE/'saveturn.sh'),label],env=env,capture_output=True,text=True,timeout=70,check=True)
        candidates=[p for p in (home/'Original').iterdir() if p.suffix.lower()=='.sav' and p.stat().st_mtime>=start]
        assert candidates, result.stdout+result.stderr
        newest=max(candidates,key=lambda p:p.stat().st_mtime);raw=newest.read_bytes()
        assert struct.unpack_from('<h',raw,668)[0]==turn+1,(newest,turn)
        dest=a.output/f'round{round_no+1:02}.sav';shutil.copy2(newest,dest)
        assert newest.stem==label,'refusing to remove an unrelated save'
        newest.unlink() # only the just-created numeric save; raw corpus stays outside the install
        walk=walk_save(str(dest));offset=walk['cities']['offset']
        assert walk['cities']['count']==n
        for i in range(n):
            record=raw[offset+92*i:offset+92*(i+1)];owner=record[10]
            values=[record[11],*struct.unpack_from('<2h',record,28),struct.unpack_from('<I',record,50)[0],
                struct.unpack_from('<I',record,24)[0],struct.unpack_from('<I',record,6)[0],
                struct.unpack_from('<h',record,32)[0],*struct.unpack_from('<3H',record,76),*record[82:86],
                struct.unpack_from('<i',raw,2944+3348*owner+10)[0]]
            lines.append(' '.join(map(str,[turn,i,*values])))
        saved.append({'round':round_no+1,'turn':turn+1,'sha256':hashlib.sha256(raw).hexdigest()})
        print('live saved round',round_no+1,flush=True)
    report={'turns':a.turns,'cities':n,'cases':len(lines),'saves':saved,
        'reference_save_sha256':hashlib.sha256((home/'Original/Ph_Auto2.SAV').read_bytes()).hexdigest(),
        'runtime_executable_sha256':hashlib.sha256((home/'civ2.exe').read_bytes()).hexdigest(),
        'presentation_boundaries':['0x411c50','0x467c89'],'scope':__doc__.strip()}
    if a.gold:a.gold.write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:a.vectors.write_text('# turn city size food_box shield_box worked specialists flags route_trade science taxes trade food shields happy unhappy treasury\n'+'\n'.join(lines)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
