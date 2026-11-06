#!/usr/bin/env python3
"""Run our isolated SDL window with real XTest input on a private Xvfb.

No original artwork is copied. CIVTOT_HOME optionally enables installed art.
Needs libX11, libXtst, Xvfb and the already built desktop spike executable.
"""
import ctypes as c
import os
from pathlib import Path
import secrets
import select
import struct
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parent

def bind(lib, name, result, *args):
    fn = getattr(lib, name)
    fn.restype, fn.argtypes = result, args
    return fn

def run():
    with tempfile.TemporaryDirectory(prefix="tot-desktop-x11-") as folder:
        scratch = Path(folder)
        cookie = secrets.token_bytes(16)
        def field(data): return struct.pack(">H", len(data)) + data
        auth = scratch / "Xauthority"
        auth.write_bytes(struct.pack(">H", 65535) + field(b"") + field(b"") + field(b"MIT-MAGIC-COOKIE-1") + field(cookie))
        with (scratch / "xvfb.log").open("w") as log:
            server = subprocess.Popen(["Xvfb", "-displayfd", "1", "-screen", "0", "1400x1000x24", "-nolisten", "unix", "-listen", "tcp", "-auth", str(auth)], stdout=subprocess.PIPE, stderr=log, text=True)
            app = None
            display = None
            try:
                if not select.select([server.stdout], [], [], 10)[0]: raise RuntimeError("Xvfb startup timeout")
                number = server.stdout.readline().strip()
                if not number: raise RuntimeError((scratch / "xvfb.log").read_text()[-2000:])
                os.environ["XAUTHORITY"] = str(auth)
                address = "127.0.0.1:" + number
                x = c.CDLL("libX11.so.6")
                xt = c.CDLL("libXtst.so.6")
                ptr, ul, ui = c.c_void_p, c.c_ulong, c.c_uint
                open_display = bind(x, "XOpenDisplay", ptr, c.c_char_p)
                display = open_display(address.encode())
                if not display: raise RuntimeError("cannot open our Xvfb display")
                root = bind(x, "XDefaultRootWindow", ul, ptr)(display)
                query = bind(x, "XQueryTree", c.c_int, ptr, ul, c.POINTER(ul), c.POINTER(ul), c.POINTER(c.POINTER(ul)), c.POINTER(ui))
                fetch = bind(x, "XFetchName", c.c_int, ptr, ul, c.POINTER(c.c_char_p))
                free = bind(x, "XFree", c.c_int, ptr)
                sync = bind(x, "XSync", c.c_int, ptr, c.c_int)
                focus = bind(x, "XSetInputFocus", c.c_int, ptr, ul, c.c_int, ul)
                resize = bind(x, "XResizeWindow", c.c_int, ptr, ul, ui, ui)
                keysym = bind(x, "XStringToKeysym", ul, c.c_char_p)
                keycode = bind(x, "XKeysymToKeycode", c.c_ubyte, ptr, ul)
                fake = bind(xt, "XTestFakeKeyEvent", c.c_int, ptr, ui, c.c_int, ul)
                def key(name, down=True, up=True):
                    code = keycode(display, keysym(name.encode()))
                    if not code: raise RuntimeError("unmapped test key: " + name)
                    if down: fake(display, code, 1, 0)
                    if up: fake(display, code, 0, 0)
                    sync(display, 0)
                    time.sleep(.06)
                def write(text):
                    for char in text: key(char)
                report = Path(os.environ.get("TOTENGINE_SPIKE_REPORT", "/tmp/tot-desktop-linux-input.json")).resolve()
                env = dict(os.environ, DISPLAY=address, SDL_VIDEODRIVER="x11")
                app = subprocess.Popen([os.environ.get("TOTENGINE_SPIKE_BINARY", str(ROOT / "target/debug/tot-desktop-spike")), "--external-test", str(report)], env=env)
                window = None
                for _ in range(100):
                    owner, parent, children, count = ul(), ul(), c.POINTER(ul)(), ui()
                    query(display, root, c.byref(owner), c.byref(parent), c.byref(children), c.byref(count))
                    try:
                        for i in range(count.value):
                            name = c.c_char_p()
                            if fetch(display, children[i], c.byref(name)):
                                try:
                                    if name.value and b"ToTEngine" in name.value: window = children[i]
                                finally: free(name)
                    finally:
                        if children: free(children)
                    if window: break
                    if app.poll() is not None: raise RuntimeError("spike exited before opening a window")
                    time.sleep(.05)
                if not window: raise RuntimeError("our SDL window did not open")
                focus(display, window, 2, 0); sync(display, 0); time.sleep(.3)
                key("n"); time.sleep(.2)
                write("athenx"); key("BackSpace"); key("a")
                key("Tab"); key("Shift_L", up=False); key("Tab"); key("Shift_L", down=False); key("Tab"); key("Return")
                key("n"); time.sleep(.2); write("discard"); key("Escape")
                focus(display, root, 2, 0); sync(display, 0); time.sleep(.2)
                focus(display, window, 2, 0); sync(display, 0); time.sleep(.2)
                resize(display, window, 1200, 800); sync(display, 0); time.sleep(.3)
                key("Escape")
                if app.wait(timeout=15) != 0: raise RuntimeError("desktop external input failed")
                print(report.read_text())
            finally:
                if app and app.poll() is None:
                    app.terminate(); app.wait(timeout=10)
                if display: bind(x, "XCloseDisplay", c.c_int, c.c_void_p)(display)
                server.terminate(); server.wait(timeout=10)

if __name__ == "__main__": run()
