"""corrhook.py on|off|dump — log calls of the corruption function 0x4c35a2.

Each call records (city, amount, waste flag, d = [0x653808], the capital
distance) into a buffer, as randhook.py does for rand(). It found the
'· 3' a disassembly filter had hidden (evidence §T1-4 city totals). The
cave is the dead startup function 0x41562b; do not run it together with
randhook.py.
"""
import sys, struct, time
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import unitram as u
FN, CAVE, IDX, BUF, CAP = 0x4c35a2, 0x41562b, 0x607c60, 0x607c70, 2048
PRO = bytes.fromhex('558bec83ec28')
p32 = lambda v: struct.pack('<I', v)
def jmp(src, dst): return b'\xe9' + struct.pack('<i', dst - (src + 5))
def cave():
    rec = b'\x8b\xc8' + b'\xc1\xe1\x04' + b'\x52'            # mov ecx,eax; shl ecx,4; push edx
    for k, src in enumerate((b'\x8b\x54\x24\x10', b'\x8b\x54\x24\x14', b'\x8b\x54\x24\x18', b'\x8b\x15' + p32(0x653808))):
        rec += src + b'\x89\x91' + p32(BUF + 4 * k)               # mov edx,..; mov [ecx+BUF+4k],edx
    rec += b'\x5a' + b'\xff\x05' + p32(IDX)                    # pop edx; inc [IDX]
    c = b'\x50\x51' + b'\xa1' + p32(IDX) + b'\x3d' + p32(CAP) + b'\x73' + bytes([len(rec)]) + rec
    c += b'\x59\x58' + PRO
    return c + jmp(CAVE + len(c), FN + 6)
cmd = sys.argv[1]
if cmd == 'on':
    assert u.read(FN, 6) == PRO, u.read(FN, 6).hex()
    c = cave(); u.write(IDX, b'\0' * 16); u.write(CAVE, c)
    u.write(FN, jmp(FN, CAVE) + b'\x90'); print('hook on', len(c))
elif cmd == 'off':
    u.write(FN, PRO); print('hook off')
elif cmd == 'dump':
    n = min(u.u32(IDX), CAP); raw = u.read(BUF, 16 * n)
    for i in range(n): print(*struct.unpack_from('<iiii', raw, 16 * i))
