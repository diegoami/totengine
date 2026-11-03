#!/usr/bin/env python3
"""rngslot.py — read / write the original game's live RNG state.

    rngslot.py get [--crt]        # print the 32-bit state as 0x........
    rngslot.py set <value> [--crt]  # seed it (decimal or 0x hex) and echo

There are TWO generators. Default: the Numerical-Recipes LCG behind
`rand_int` (VA 0x5dd53c). `--crt`: the MSVC CRT `rand()` state (VA
0x5e2590, x = x*214013 + 2531011) — **combat rolls come from this one**,
not from 0x5dd53c (evidence §T1-6 combat). Seed it before an attack to
force a known roll sequence; `engine::combat::resolve` replays it.

The slot is the global at VA `0x5dd53c` inside the running `civ2.exe`
(see docs/evidence/README.md §T1-6). Writing `1` there and then reading a
value of `0x0438e694` later is the *runtime proof* that gameplay rolls flow
through `state = state*1664525 + 1013904223 (mod 2^32)`.

Caveat worth knowing before trusting a reading: the game **swaps contexts**
around per-context roll batches (save the global, load `[ctx+0x100]`,
roll, restore — VA 0x5602ea / 0x560cc2). A read taken *during* such a
swap sees the borrowed state, not the long-lived one. Read twice a few
hundred ms apart, and prefer readings taken while the game is idle.

Requires the memspool daemon (tools/oracle/memspool.py) on
~/review-tmp/memspool.sock, and the game running as its descendant.
"""
import json
import os
import socket
import struct
import sys

SOCK = os.path.expanduser("~/review-tmp/memspool.sock")
SLOT = "0x5dd53c"
CRT_SLOT = "0x5e2590"


def rpc(req, timeout=10.0):
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.settimeout(timeout)
    s.connect(SOCK)
    s.sendall((json.dumps(req) + "\n").encode())
    buf = b""
    while not buf.endswith(b"\n"):
        chunk = s.recv(65536)
        if not chunk:
            break
        buf += chunk
    s.close()
    return json.loads(buf.decode() or "{}")


def get(slot=SLOT):
    r = rpc({"cmd": "read", "addr": slot, "len": 4})
    return struct.unpack("<I", bytes.fromhex(r.get("hex", "")))[0]


def set_(value, slot=SLOT):
    r = rpc({"cmd": "write", "addr": slot, "hex": struct.pack("<I", value & 0xFFFFFFFF).hex()})
    return r.get("written")


def main(argv):
    slot = SLOT
    if "--crt" in argv:
        argv = [a for a in argv if a != "--crt"]
        slot = CRT_SLOT
    if len(argv) < 2 or argv[1] not in ("get", "set"):
        print(__doc__.strip())
        return 2
    if argv[1] == "get":
        print(hex(get(slot)))
        return 0
    if len(argv) < 3:
        print("set needs a value", file=sys.stderr)
        return 2
    set_(int(argv[2], 0), slot)
    print(hex(get(slot)))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
