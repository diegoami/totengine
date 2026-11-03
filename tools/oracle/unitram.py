#!/usr/bin/env python3
"""unitram.py — read/drive units in the live game through RAM (no clicking).

    unitram.py bases                     # live unit-array / unit-type bases
    unitram.py unit <idx>                # decode one unit record
    unitram.py near <x> <y> [r]          # units within r tiles (default 1)
    unitram.py state                     # cursor, active unit, both RNG slots
    unitram.py ready <idx>               # make a unit selectable this turn
    unitram.py select <idx> [maxW]       # ready + press W until it is active
    unitram.py frame <ax> <ay> <dx> <dy> [secs]
                                         # poll the live combat frame (run it
                                         # while an attack animates)

This replaces map navigation for oracle experiments (tools/oracle/README.md
§Navigation): instead of clicking a tile, pick the unit by index, `select`
it, and press a numpad direction with winject. Evidence: docs/evidence/
README.md §T1-6 combat.

Indices are the GAME's unit numbers (the active-unit field holds them),
i.e. record n sits at units_base + 40*n.

Live addresses (ToTPP build in the oracle prefix):
  units base   imm32 of `movsx eax,[eax*8+units+9]`  at VA 0x5414da+4, minus 9
  types base   imm32 of `movsx eax,[eax*8+types+0xc]` at VA 0x53f61f+4, minus 0xc
               (ToTPP relocates both tables to the heap and patches .text,
               so the stock-exe disassembly shows different addresses)
  cursor       i32 y @0x5f46f8, i32 x @0x5f46fc
  active unit  u16 @0x60764e (inside the RAM copy of the save's game block;
               the game WRITES it on every selection change — writing it
               yourself does not select a unit, use `select`)
  NR LCG       @0x5dd53c (rand_int);  MSVC CRT rand() state @0x5e2590 (combat)

Unit record (40 bytes, true framing — x at +0):
  +0 x i16, +2 y i16, +4 z/map i16, +6 attr u16 (0x2000 veteran; 0x0100 set
  on fortified/sentried units, clear on units in the W rotation),
  +8 type, +9 owner, +10 moves spent (in units of the byte @0x5f5eb8, which
  is 15 in FallRome), +11 visibility mask, +12 damage (HP lost; HP = h*10),
  +13 last move dir (0xff none), +17 orders (0xff none, 1 fortifying,
  2 fortified), +18 home city, +30 id. Dead units: x/y/z = -200 (attacker
  killed) or -100 (defender killed), id 0.
"""
import json
import os
import socket
import struct
import subprocess
import sys
import time

SOCK = os.path.expanduser("~/review-tmp/memspool.sock")
HERE = os.path.dirname(os.path.abspath(__file__))
REC = 40


def rpc(req, timeout=30.0):
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.settimeout(timeout)
    s.connect(SOCK)
    s.sendall((json.dumps(req) + "\n").encode())
    buf = b""
    while not buf.endswith(b"\n"):
        chunk = s.recv(1 << 20)
        if not chunk:
            break
        buf += chunk
    s.close()
    r = json.loads(buf.decode() or "{}")
    if "error" in r:
        raise SystemExit(f"memspool: {r['error']}")
    return r


def read(addr, n):
    return bytes.fromhex(rpc({"cmd": "read", "addr": hex(addr), "len": n})["hex"])


def write(addr, data):
    return rpc({"cmd": "write", "addr": hex(addr), "hex": data.hex()})


def u16(addr):
    return struct.unpack("<H", read(addr, 2))[0]


def u32(addr):
    return struct.unpack("<I", read(addr, 4))[0]


def bases():
    units = u32(0x5414DA + 4) - 0x09
    types = u32(0x53F61F + 4) - 0x0C
    return units, types


def decode(idx, units, types):
    r = read(units + idx * REC, REC)
    x, y, z, attr = struct.unpack_from("<hhhH", r, 0)
    t = read(types + r[8] * REC, REC)
    return {
        "idx": idx, "x": x, "y": y, "z": z, "attr": hex(attr),
        "veteran": bool(attr & 0x2000), "type": r[8], "owner": r[9],
        "moves_spent": r[10], "damage": r[12], "last_dir": r[13],
        "orders": r[17], "home": r[18],
        "id": struct.unpack_from("<H", r, 30)[0],
        "type_att": t[12], "type_def": t[13], "type_hp": t[14],
        "type_fp": t[15], "type_move": t[10], "raw": r.hex(" "),
    }


def ready(idx, units):
    base = units + idx * REC
    attr = struct.unpack("<H", read(base + 6, 2))[0] & ~0x0100
    write(base + 6, struct.pack("<H", attr))
    write(base + 17, b"\xff")  # no orders
    write(base + 10, b"\x00")  # full movement (avoids partial-move scaling)


def winject_key(vk):
    env = {**os.environ, "DISPLAY": ":99",
           "WINEPREFIX": os.environ.get("WINEPREFIX",
                                        os.path.expanduser("~/.wine_civ2tot"))}
    subprocess.run(["timeout", "20", "wine", os.path.join(HERE, "winject.exe"),
                    "key", str(vk)], env=env, capture_output=True)


def main(argv):
    if len(argv) < 2:
        print(__doc__.strip())
        return 2
    cmd = argv[1]
    units, types = bases()
    if cmd == "bases":
        print(f"units {units:#x}  types {types:#x}")
    elif cmd == "unit":
        print(json.dumps(decode(int(argv[2]), units, types), indent=1))
    elif cmd == "near":
        cx, cy = int(argv[2]), int(argv[3])
        rad = int(argv[4]) if len(argv) > 4 else 1
        count = 3000  # scan generously; dead slots read as -200/-100
        raw = b""
        for a in range(units, units + count * REC, 0x8000):
            raw += read(a, min(0x8000, units + count * REC - a))
        for i in range(count):
            x, y = struct.unpack_from("<hh", raw, i * REC)
            if abs(x - cx) <= rad * 2 and abs(y - cy) <= rad * 2 \
                    and abs(x - cx) + abs(y - cy) <= rad * 2:
                d = decode(i, units, types)
                print(f"{i:5d} ({x},{y}) type {d['type']} owner {d['owner']} "
                      f"spent {d['moves_spent']} dmg {d['damage']} "
                      f"orders {d['orders']} vet {d['veteran']}")
    elif cmd == "state":
        print(f"cursor ({struct.unpack('<i', read(0x5F46FC, 4))[0]},"
              f"{struct.unpack('<i', read(0x5F46F8, 4))[0]})  "
              f"active {u16(0x60764E)}  "
              f"lcg {u32(0x5DD53C):#010x}  crt {u32(0x5E2590):#010x}")
    elif cmd == "ready":
        ready(int(argv[2]), units)
        print(json.dumps(decode(int(argv[2]), units, types)))
    elif cmd == "select":
        idx = int(argv[2])
        maxw = int(argv[3]) if len(argv) > 3 else 60
        ready(idx, units)
        for _ in range(maxw):
            if u16(0x60764E) == idx:
                print(f"active {idx}")
                return 0
            winject_key(0x57)  # W = wait -> next unit in the rotation
            time.sleep(0.8)
        time.sleep(1.5)  # the field is written late after the last W
        if u16(0x60764E) == idx:
            print(f"active {idx}")
            return 0
        print(f"not reached after {maxw} W presses (active {u16(0x60764E)})",
              file=sys.stderr)
        return 1
    elif cmd == "frame":
        ax, ay, dx, dy = (int(v) for v in argv[2:6])
        secs = float(argv[6]) if len(argv) > 6 else 8.0
        lo, n = 0x110000, 0x20000  # the main thread's stack window
        seen, t0 = set(), time.time()
        while time.time() - t0 < secs:
            d = read(lo, n)

            def g(e, k):
                return struct.unpack_from("<i", d, e - k)[0]
            for e in range(0x200, n - 0x20, 4):
                if (g(e, 0x9C), g(e, 0xAC), g(e, 0x78), g(e, 0x88)) == (ax, ay, dx, dy):
                    rec = {"A": g(e, 0xB0), "D": g(e, 0x60), "fpA": g(e, 0x7C),
                           "fpD": g(e, 0x30), "reroll": g(e, 0xC8),
                           "defender": g(e, 0x8), "a": g(e, 0x1E8),
                           "d": g(e, 0x1EC),
                           "args": struct.unpack_from("<iii", d, e + 8)}
                    key = tuple(sorted(rec.items()))
                    if key not in seen:
                        seen.add(key)
                        print(json.dumps({"t": round(time.time() - t0, 3),
                                          "ebp": hex(lo + e), **rec}))
    else:
        print(__doc__.strip())
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
