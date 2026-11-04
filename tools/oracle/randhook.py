"""randhook.py — log the call sites of the CRT rand() in the live game.

    python3 randhook.py <seconds> <out.txt> [--return]

Hooks `rand` (VA 0x5b11d0) with a 5-byte jmp into a code cave, sleeps,
dumps, and unhooks. The cave records, per draw, (return address, caller's
return address) into a linear buffer that stops at CAP draws. `--return`
presses Return (xtool) right after hooking, so the first draws are the
ones that dialog's answer triggers. Two runs diffed line by line give the
first draw where they diverge (evidence §RG-2 reproducible starts).

Placement notes (each one cost a run):
- The cave is the body of 0x41562b. That function (the startup
  `srand(GetTickCount())` + NR seeding) runs once from the init path
  `0x41a6f9` and is dead afterwards, and `.text` has no padding cave.
- The buffer lives in the zero region after the settings block, but
  ABOVE 0x607c50: the civ-placement setup (0x48c61d) zeroes
  0x6077a0..0x607c50, which silently reset an earlier buffer mid-run.
- The hook pins nothing; seed the generators separately (rngslot.py).
"""
import struct
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import unitram as u  # noqa: E402

CAVE, RAND = 0x41562B, 0x5B11D0
IDX, BUF, CAP = 0x607C60, 0x607C70, 10000  # 8 B per draw, ends 0x61b4f0
RAND_PROLOGUE = bytes.fromhex("558bec535657")  # push ebp; mov ebp,esp; push ebx/esi/edi


def p32(v):
    return struct.pack("<I", v)


def jmp(src, dst):
    return b"\xe9" + struct.pack("<i", dst - (src + 5))


def cave_code():
    c = b"\x50\x51"  # push eax; push ecx
    c += b"\xa1" + p32(IDX)  # mov eax,[IDX]
    c += b"\x3d" + p32(CAP)  # cmp eax,CAP
    c += b"\x73\x1b"  # jae over the 27-byte record
    c += b"\x8b\x4c\x24\x08"  # mov ecx,[esp+8]   (rand's return address)
    c += b"\x89\x0c\xc5" + p32(BUF)  # mov [eax*8+BUF],ecx
    c += b"\x8b\x4d\x04"  # mov ecx,[ebp+4]   (the caller's own return)
    c += b"\x89\x0c\xc5" + p32(BUF + 4)  # mov [eax*8+BUF+4],ecx
    c += b"\xff\x05" + p32(IDX)  # inc dword [IDX]
    c += b"\x59\x58"  # pop ecx; pop eax
    c += RAND_PROLOGUE[:5]  # the 5 displaced bytes
    return c + jmp(CAVE + len(c) + 0, RAND + 5)


def main(argv):
    secs, out = float(argv[0]), argv[1]
    if u.read(RAND, 6) != RAND_PROLOGUE:
        sys.exit(f"rand prologue unexpected: {u.read(RAND, 6).hex()} (hook already in?)")
    code = cave_code()
    u.write(IDX, b"\0" * 16)
    u.write(CAVE, code)
    assert u.read(CAVE, len(code)) == code
    u.write(RAND, jmp(RAND, CAVE))
    try:
        if "--return" in argv:
            subprocess.run(["timeout", "8", str(HERE / "xtool"), "key", "Return"])
        time.sleep(secs)
    finally:
        u.write(RAND, RAND_PROLOGUE[:5])
    n = min(u.u32(IDX), CAP)
    raw = u.read(BUF, 8 * n)
    with open(out, "w") as f:
        for i in range(n):
            a, b = struct.unpack_from("<II", raw, 8 * i)
            f.write(f"{a:08x} {b:08x}\n")
    print(f"{n} draws{' (CAP reached)' if n == CAP else ''} -> {out}")


if __name__ == "__main__":
    main(sys.argv[1:])
