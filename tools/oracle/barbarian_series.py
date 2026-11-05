#!/usr/bin/env python3
"""Sustained live original-game arrival phases, with a continuous mechanics CRT.

Load Original/Ph_Auto2.SAV first. Pin level 3, difficulty 4, all-human mechanics
player mask (barbarians excluded) and treasury 32000 to exercise land and naval arrivals. Cities/units persist
between phases; only arrivals run. UI/network presentation callees are suppressed;
allocation and order-setting execute in the game. The original quiet-context
word 0x6031b8=1 suppresses arrival alert dialogs after creation. The previous phase's final CRT
is restored before the next phase so intervening screen repaint draws cannot
change the mechanics stream. No full AI-turn/pollution RNG parity is claimed.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import struct
import subprocess
import unitram as u
from city_series import CAVE,HERE,wi

CRT=0x5e2590
BUF=0x607c60


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--helper',type=Path,required=True);p.add_argument('--turns',type=int,default=64)
    p.add_argument('--seed',type=lambda s:int(s,0),default=1);p.add_argument('--gold',type=Path);p.add_argument('--vectors',type=Path)
    a=p.parse_args();home=Path(os.environ['CIVTOT_HOME'])
    windows=wi('wins');window=next(re.match(r'hwnd=(\w+)',line).group(1) for line in windows.splitlines()
        if 'vis=1' in line and 'Civilization II: Test of Time' in line)
    assert u.u16(0x607648)==43,'reload reference first'
    units,_=u.bases();assert units!=0x607c58;state=a.seed
    u.write(0x60765b,b'\xfe');u.write(0x607658,b'\x04');u.write(0x607659,b'\x03')
    notification_before=u.read(0x6031b8,2)
    u.write(0x6031b8,b'\x01\x00') # original isolated/quiet notification context
    for civ in range(1,8):u.write(0x5f68aa+3348*civ,struct.pack('<i',32000))
    rows=[];total=0
    for turn in range(128,128+a.turns):
        before=u.read(units,40*u.u16(0x607666))
        code=b'\x60\xc7\x05'+struct.pack('<II',CRT,state)
        code+=b'\x66\xc7\x05'+struct.pack('<IH',0x607648,turn)
        code+=b'\xe8'+struct.pack('<i',0x46e6cd-(CAVE+len(code)+5))
        code+=b'\xa1'+struct.pack('<I',CRT)+b'\xa3'+struct.pack('<I',BUF)
        code+=b'\x61\xb8'+struct.pack('<I',0x8383838)+b'\xc2\x04\x00'
        # Network/animation/text/redraw boundaries; unit creation and orders run.
        patches={CAVE:code,0x569c88:b'\x31\xc0\xc3',0x40cef0:b'\x31\xc0\xc3',
            0x467d19:b'\x31\xc0\xc3'}
        backups={addr:u.read(addr,len(v)) for addr,v in patches.items()};buffer_before=u.read(BUF,4)
        completed=False
        try:
            for addr,value in patches.items():u.write(addr,value)
            result=subprocess.run(['timeout','40','wine',str(a.helper),window,hex(CAVE)],capture_output=True,text=True,check=True)
            assert result.stdout.strip()==str(0x8383838),result.stdout+result.stderr
            after_state=u.u32(BUF);after=u.read(units,40*u.u16(0x607666));completed=True
        except subprocess.CalledProcessError as failure:
            print(failure.stdout,failure.stderr,flush=True)
            raise
        finally:
            if not completed:subprocess.run([str(HERE/'stop_game.sh')],check=False)
            else:
                for addr,value in backups.items():u.write(addr,value)
                u.write(BUF,buffer_before)
        new=[]
        for i in range(len(after)//40):
            r=after[40*i:40*(i+1)];uid=struct.unpack_from('<I',r,30)[0]
            old=struct.unpack_from('<I',before,40*i+30)[0] if 40*i+40<=len(before) else 0
            if uid and uid!=old:
                assert r[9]==0
                x,y=struct.unpack_from('<hh',r);new.append([i,r[8],x,y,r[17],int(bool(struct.unpack_from('<H',r,6)[0]&8192))])
        total+=len(new);rows.append(f'{turn} {state} {after_state} | '+' '.join(','.join(map(str,r)) for r in new));state=after_state
        print('live arrival turn',turn,'new units',len(new),flush=True)
    u.write(0x6031b8,notification_before)
    report={'seed':a.seed,'turn_start':128,'turns':a.turns,'new_units':total,'final_crt':state,
        'reference_save_sha256':hashlib.sha256((home/'Original/Ph_Auto2.SAV').read_bytes()).hexdigest(),
        'runtime_executable_sha256':hashlib.sha256((home/'civ2.exe').read_bytes()).hexdigest(),
        'notification_context_0x6031b8':1,'call_thread':'GUI','boundaries':['0x569c88','0x40cef0','0x467d19'],'scope':__doc__.strip()}
    if a.gold:a.gold.write_text(json.dumps(report,indent=2)+'\n')
    if a.vectors:a.vectors.write_text('# turn starting_CRT final_CRT | slot,type,x,y,orders,veteran ...\n'+'\n'.join(rows)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
