#!/usr/bin/env python3
"""Full-memory dump/diff for the oracle game process (memspool client).

Usage:
  ramdump.py dump <out.bin>       - dump all rw anonymous regions (with an index)
  ramdump.py diff <a.bin> <b.bin> - diff two dumps, print clustered changes
"""
import json
import os
import socket
import sys

SOCK = os.path.expanduser("~/review-tmp/memspool.sock")


def rpc(req):
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect(SOCK)
    s.sendall((json.dumps(req) + "\n").encode())
    with s.makefile("r") as fh:
        try:
            return json.loads(fh.readline())
        except Exception:
            return {}


def regions():
    out = []
    for r in rpc({"cmd": "maps"}).get("maps", []):
        parts = r.split(" ")
        lo, hi = (int(x, 16) for x in parts[0].split("-"))
        perm = parts[1]
        path = " ".join(parts[2:])
        if "r" not in perm or "w" not in perm:
            continue
        if path.strip() and "memfd" not in path:  # skip file-backed (mmap'd files)
            continue
        if hi - lo > 0x8000000:  # skip >128MB arenas
            continue
        out.append((lo, hi))
    return out


def dump(path):
    idx = []
    with open(path, "wb") as f:
        for lo, hi in regions():
            a = lo
            while a < hi:
                n = min(0x40000, hi - a)
                h = rpc({"cmd": "read", "addr": hex(a), "len": n}).get("hex")
                blob = bytes.fromhex(h) if h else None
                if blob:
                    idx.append((a, len(blob), f.tell()))
                    f.write(blob)
                a += n
    with open(path + ".idx", "w") as f:
        json.dump(idx, f)
    print(f"{path}: {len(idx)} segments, {os.path.getsize(path)} bytes")


def diff(a_path, b_path):
    a = open(a_path, "rb").read()
    b = open(b_path, "rb").read()
    idx = json.load(open(a_path + ".idx"))
    n = min(len(a), len(b))
    diffs = [i for i in range(n) if a[i] != b[i]]
    print(f"{len(diffs)} bytes differ")
    regions = []
    for i in diffs:
        if regions and i - regions[-1][1] <= 24:
            regions[-1][1] = i
        else:
            regions.append([i, i])
    for r in regions[:200]:
        addr = next((lo + (r[0] - fo) for lo, ln, fo in idx if fo <= r[0] < fo + ln), None)
        seg = [(a[j], b[j]) for j in range(r[0], min(r[1] + 1, r[0] + 10))]
        print(f"  @~{hex(addr) if addr else '?'} +{r[1]-r[0]+1}B:",
              " ".join(f"{x:02x}>{y:02x}" for x, y in seg))


if __name__ == "__main__":
    if sys.argv[1] == "dump":
        dump(sys.argv[2])
    elif sys.argv[1] == "diff":
        diff(sys.argv[2], sys.argv[3])
