#!/usr/bin/env python3
"""memspool.py — spawn the game as our own descendant and read its memory.

Why: yama ptrace_scope=1 means a process may only be traced/read by an
ancestor. The oracle game used to be started by throwaway shells, so
process_vm_readv failed with EPERM. Running the game as a child of a
persistent spawner makes every wine process our descendant and memory
readable — a Cheat-Engine-grade RE tool with zero installs.

Usage:
  memspool.py serve              # daemon: spawns the game, serves commands
  memspool.py pids               # civ2.exe descendants
  memspool.py maps               # readable/writable mappings
  memspool.py read <hexaddr> <len>
  memspool.py scan <hexpattern>  # search writable anonymous memory
  memspool.py snapshot <dir>     # dump all rw anon regions + manifest.json
  memspool.py diffsnap <dirA> <dirB> [maxrows]
  memspool.py restart            # kill the game + start a fresh child

Protocol: JSON lines over a unix socket (~/review-tmp/memspool.sock).
"""
import ctypes
import json
import os
import signal
import socket
import subprocess
import sys
import time

SOCK = os.path.expanduser("~/review-tmp/memspool.sock")
GAME_EXE = "civ2.exe"


# ---------------------------------------------------------------- memory

class IOV(ctypes.Structure):
    _fields_ = [("base", ctypes.c_void_p), ("len", ctypes.c_size_t)]


def _libc():
    libc = ctypes.CDLL(None, use_errno=True)
    libc.process_vm_readv.restype = ctypes.c_ssize_t
    libc.process_vm_readv.argtypes = [ctypes.c_int, ctypes.POINTER(IOV),
                                      ctypes.c_ulong, ctypes.POINTER(IOV),
                                      ctypes.c_ulong, ctypes.c_ulong]
    return libc


def read_mem(pid: int, addr: int, length: int) -> bytes:
    libc = _libc()
    out = bytearray()
    while length > 0:
        n = min(length, 1 << 20)
        buf = ctypes.create_string_buffer(n)
        local = IOV(ctypes.cast(buf, ctypes.c_void_p), n)
        remote = IOV(ctypes.c_void_p(addr + len(out)), n)
        got = libc.process_vm_readv(pid, ctypes.byref(local), 1,
                                    ctypes.byref(remote), 1, 0)
        if got <= 0:
            err = ctypes.get_errno()
            raise OSError(err, os.strerror(err), hex(addr + len(out)))
        out += buf.raw[:got]
        length -= got
    return bytes(out)


def write_mem(pid: int, addr: int, data: bytes) -> int:
    """process_vm_writev — for patch-and-prove experiments (seed hunting)."""
    libc = ctypes.CDLL(None, use_errno=True)
    libc.process_vm_writev.restype = ctypes.c_ssize_t
    libc.process_vm_writev.argtypes = [ctypes.c_int, ctypes.POINTER(IOV),
                                       ctypes.c_ulong, ctypes.POINTER(IOV),
                                       ctypes.c_ulong, ctypes.c_ulong]
    buf = ctypes.create_string_buffer(data, len(data))
    local = IOV(ctypes.cast(buf, ctypes.c_void_p), len(data))
    remote = IOV(ctypes.c_void_p(addr), len(data))
    got = libc.process_vm_writev(pid, ctypes.byref(local), 1,
                                 ctypes.byref(remote), 1, 0)
    if got <= 0:
        err = ctypes.get_errno()
        # Read-only pages (live .text patches): /proc/<pid>/mem writes
        # through page protection for a tracer-permitted ancestor.
        try:
            with open(f"/proc/{pid}/mem", "r+b", buffering=0) as f:
                f.seek(addr)
                return f.write(data)
        except OSError:
            raise OSError(err, os.strerror(err), hex(addr))
    return got


def descendants(pid: int = None) -> list:
    me = pid or os.getpid()
    children = {}
    for entry in os.listdir("/proc"):
        if not entry.isdigit():
            continue
        try:
            with open(f"/proc/{entry}/stat") as fh:
                parts = fh.read().rsplit(")", 1)[1].split()
            ppid = int(parts[1])
        except (OSError, IndexError, ValueError):
            continue
        children.setdefault(ppid, []).append(int(entry))
    out, stack = [], [me]
    while stack:
        p = stack.pop()
        for c in children.get(p, []):
            out.append(c)
            stack.append(c)
    return out


def game_pids() -> list:
    out = []
    for pid in descendants():
        try:
            with open(f"/proc/{pid}/comm") as fh:
                if GAME_EXE in fh.read():
                    out.append(pid)
        except OSError:
            continue
    return out


def maps(pid: int, perms: str = "rw") -> list:
    out = []
    with open(f"/proc/{pid}/maps") as fh:
        for line in fh:
            m = line.split()
            rng, perm = m[0], m[1]
            path = m[5] if len(m) > 5 else ""
            if perm[:len(perms)] != perms:
                continue
            lo, hi = (int(x, 16) for x in rng.split("-"))
            out.append((lo, hi, perm, path))
    return out


def scan(pid: int, pattern: bytes, anon_only: bool = True) -> list:
    hits = []
    for lo, hi, perm, path in maps(pid, "rw"):
        if anon_only and path:
            continue
        try:
            blob = read_mem(pid, lo, hi - lo)
        except OSError:
            continue
        off = blob.find(pattern)
        while off != -1:
            hits.append({"addr": lo + off,
                         "ctx": blob[max(0, off - 8):off + 24].hex()})
            off = blob.find(pattern, off + 1)
    return hits


# ---------------------------------------------------------------- snapshots

def snapshot(pid: int, dirpath: str) -> dict:
    os.makedirs(dirpath, exist_ok=True)
    manifest = {"pid": pid, "regions": []}
    for i, (lo, hi, perm, path) in enumerate(maps(pid, "rw")):
        if path:
            continue
        size = hi - lo
        if size > 64 << 20:
            continue
        try:
            blob = read_mem(pid, lo, size)
        except OSError:
            continue
        name = f"{i:04d}-{lo:012x}-{size:x}.bin"
        with open(os.path.join(dirpath, name), "wb") as fh:
            fh.write(blob)
        manifest["regions"].append({"lo": lo, "size": size, "file": name})
    with open(os.path.join(dirpath, "manifest.json"), "w") as fh:
        json.dump(manifest, fh, indent=1)
    return manifest


def diff_snaps(a: str, b: str, maxrows: int = 200) -> list:
    ma = json.load(open(os.path.join(a, "manifest.json")))
    mb = json.load(open(os.path.join(b, "manifest.json")))
    by_lo = {r["lo"]: r for r in mb["regions"]}
    rows = []
    for ra in ma["regions"]:
        rb = by_lo.get(ra["lo"])
        if not rb or rb["size"] != ra["size"]:
            continue
        ba = open(os.path.join(a, ra["file"]), "rb").read()
        bb = open(os.path.join(b, rb["file"]), "rb").read()
        i, n = 0, len(ba)
        while i < n:
            if ba[i] == bb[i]:
                i += 1
                continue
            j = i
            while j < n and ba[j] != bb[j]:
                j += 1
            rows.append({"lo": ra["lo"], "off": i,
                         "addr": ra["lo"] + i, "len": j - i,
                         "a": ba[i:min(j, i + 16)].hex(),
                         "b": bb[i:min(j, i + 16)].hex()})
            i = j
            if len(rows) >= maxrows:
                return rows
    return rows


# ---------------------------------------------------------------- serve

def serve():
    if os.path.exists(SOCK):
        os.unlink(SOCK)
    srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    srv.bind(SOCK)
    srv.listen(4)
    # keep the daemon alive independently of this tty
    signal.signal(signal.SIGHUP, signal.SIG_IGN)
    print(f"memspool: serving {SOCK} (pid {os.getpid()})", flush=True)
    while True:
        conn, _ = srv.accept()
        try:
            with conn:
                data = conn.makefile("r").readline()
                req = json.loads(data)
                conn.sendall((json.dumps(handle(req)) + "\n").encode())
        except Exception as e:  # noqa: BLE001 - daemon must survive
            try:
                conn.sendall((json.dumps({"error": str(e)}) + "\n").encode())
            except OSError:
                pass


def handle(req: dict) -> dict:
    cmd = req.get("cmd")
    if cmd == "restart":
        here = os.path.dirname(os.path.abspath(__file__))
        env = {**os.environ,
               "CIVTOT_HOME": os.environ.get("CIVTOT_HOME",
                                             "/mnt/d/games/CIVTOT"),
               "WINEPREFIX": os.environ.get("WINEPREFIX",
                                            os.path.expanduser("~/.wine_civ2tot")),
               "DISPLAY": ":99"}
        subprocess.run(["bash", os.path.join(here, "stop_game.sh")],
                       env=env, capture_output=True)
        # wine must be a DIRECT child of this daemon (no script that
        # backgrounds it — an orphaned wine gets reparented to init and
        # becomes unreadable again under yama ptrace_scope=1)
        log = open("/tmp/civ2.log", "ab")
        subprocess.Popen(["wine", "civ2.exe"],
                         cwd=env["CIVTOT_HOME"], env=env,
                         stdout=log, stderr=log)
        time.sleep(14)
        return {"ok": True, "pids": game_pids()}
    pids = game_pids()
    if not pids:
        return {"error": "no game process (use restart)"}
    pid = pids[0]
    if cmd == "pids":
        return {"pids": pids}
    if cmd == "maps":
        return {"maps": [f"{lo:012x}-{hi:012x} {p} {pa}"
                         for lo, hi, p, pa in maps(pid)]}
    if cmd == "read":
        blob = read_mem(pid, int(req["addr"], 16), int(req["len"]))
        return {"hex": blob.hex()}
    if cmd == "write":
        n = write_mem(pid, int(req["addr"], 16),
                      bytes.fromhex(req["hex"]))
        return {"ok": True, "written": n}
    if cmd == "scan":
        return {"hits": scan(pid, bytes.fromhex(req["pattern"]),
                             bool(req.get("anon", True)))}
    if cmd == "snapshot":
        m = snapshot(pid, os.path.expanduser(req["dir"]))
        return {"ok": True, "regions": len(m["regions"])}
    if cmd == "diffsnap":
        rows = diff_snaps(os.path.expanduser(req["a"]),
                          os.path.expanduser(req["b"]),
                          int(req.get("maxrows", 200)))
        return {"rows": rows, "count": len(rows)}
    return {"error": f"unknown cmd {cmd}"}


def client(argv):
    if argv[0] == "serve":
        serve()
        return 0
    req = {"cmd": argv[0]}
    for a in argv[1:]:
        k, _, v = a.partition("=")
        req[k] = v
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect(SOCK)
    s.sendall((json.dumps(req) + "\n").encode())
    with s.makefile("r") as fh:
        resp = json.loads(fh.readline())
    print(json.dumps(resp, indent=1)[:8000])
    return 0 if "error" not in resp else 1


if __name__ == "__main__":
    sys.exit(client(sys.argv[1:]))
