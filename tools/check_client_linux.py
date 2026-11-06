#!/usr/bin/env python3
"""Play the standalone client through real XTest input on a private Xvfb.

No original artwork is copied. CIVTOT_HOME optionally enables installed art.
Needs libX11, libXtst, Xvfb and the already built totdesktop executable.
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
                class Attributes(c.Structure):
                    _fields_=[("x",c.c_int),("y",c.c_int),("width",c.c_int),("height",c.c_int),("border_width",c.c_int),("depth",c.c_int),("visual",ptr),("root",ul),("klass",c.c_int),("bit_gravity",c.c_int),("win_gravity",c.c_int),("backing_store",c.c_int),("backing_planes",ul),("backing_pixel",ul),("save_under",c.c_int),("colormap",ul),("map_installed",c.c_int),("map_state",c.c_int),("all_event_masks",c.c_long),("your_event_mask",c.c_long),("do_not_propagate_mask",c.c_long),("override_redirect",c.c_int),("screen",ptr)]
                get_attributes=bind(x,"XGetWindowAttributes",c.c_int,ptr,ul,c.POINTER(Attributes))
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
                env = dict(os.environ, DISPLAY=address, SDL_VIDEODRIVER="x11")
                work=Path(tempfile.mkdtemp(prefix="tot-client-linux-"))
                env["TOTENGINE_SAVES"]=str(work/"saves")
                env["TOTENGINE_CLIENT_TRACE"]=str(work/"trace.jsonl")
                app = subprocess.Popen([os.environ.get("TOTENGINE_CLIENT_BINARY", str(ROOT.parent / "target/debug/totdesktop"))], env=env)
                (work/"owned-processes.json").write_text(__import__("json").dumps({"app_pid":app.pid,"display_pid":server.pid,"authority":str(auth)}))
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
                    if app.poll() is not None: raise RuntimeError("client exited before opening a window")
                    time.sleep(.05)
                if not window: raise RuntimeError("our SDL window did not open")
                for _ in range(200):
                    attributes=Attributes()
                    if get_attributes(display,window,c.byref(attributes)) and attributes.map_state==2:break
                    if app.poll() is not None:raise RuntimeError("client exited before its window mapped")
                    time.sleep(.05)
                else:raise RuntimeError("our client window never became viewable")
                focus(display, window, 2, 0); sync(display, 0); time.sleep(.3)
                import json
                def scene():
                    try: return json.loads((work/"trace.painted.json").read_text())
                    except (OSError,ValueError): return None
                def wait(predicate):
                    for _ in range(500):
                        value=scene()
                        if value and predicate(value): return value
                        if app.poll() is not None: raise RuntimeError("client exited during route")
                        time.sleep(.03)
                    raise RuntimeError("UI wait timed out: "+str(scene()))
                rawkey=key
                def key(name,down=True,up=True):
                    before=scene(); old=before["revision"] if before else -1
                    closing=before and any(d.get("text")=="Close client? Save first if you want to resume." for d in before["draws"]) and name=="Return"
                    rawkey(name,down,up)
                    if down and up and not closing: wait(lambda s:s["revision"]>old)
                def has(s,label):return any(d.get("text")==label for d in s["draws"])
                def button(label):
                    for _ in range(30):
                        s=scene(); t=next(d for d in s["draws"] if d.get("text")==label)
                        if any(d.get("kind")=="rect" and d["x"]==t["x"]-7 and d["y"]==t["y"]-7 and d["color"]=="#6b7057" for d in s["draws"]):key("Return");return
                        key("Tab")
                    raise RuntimeError("button could not focus: "+label)
                def research():
                    if has(scene(),"Choose research explicitly"):
                        key("Down");key("Return");wait(lambda s:s["status"]=="research-set")
                def save_dialog(name):
                    write(name);wait(lambda s:s.get("entry") and s["entry"]["value"]==name);key("Return")
                wait(lambda s:s["phase"]=="Setup")
                button("Start Original surface game")
                wait(lambda s:has(s,"Choose research explicitly"));button("Save pending choice");save_dialog("research");research()
                key("f");wait(lambda s:has(s,"Name your city"));button("Save pending choice");save_dialog("founding")
                wait(lambda s:has(s,"Name your city"));write("athenx");key("BackSpace");key("a");key("Return");wait(lambda s:s["status"]=="founded 0")
                button("Choose production")
                for _ in range(80):
                    key("Down")
                    if any(d.get("text","").startswith("●") and "Warriors" in d["text"] for d in scene()["draws"]):break
                key("Return");wait(lambda s:s["status"]=="prod-set");key("s");save_dialog("normal")
                for _ in range(10):
                    research();s=scene();turn=int(next(d["text"] for d in s["draws"] if d.get("text","").startswith("Turn ")).split()[1])
                    key("e")
                    if has(scene(),"End your turn with movable units?"):key("Return")
                    wait(lambda s:s["phase"]=="Human" and any(d.get("text","").startswith("Turn "+str(turn+1)+" ") for d in s["draws"]))
                research();key("s");save_dialog("tenrounds");wait(lambda s:s["status"]=="Engine checkpoint saved: tenrounds")
                def load(name):
                    key("l");save_dialog(name)
                load("research");research();key("f");button("Save pending choice");save_dialog("researchnext")
                key("Escape");load("founding");write("athena");key("Return")
                button("Choose production")
                for _ in range(80):
                    key("Down")
                    if any(d.get("text","").startswith("●") and "Warriors" in d["text"] for d in scene()["draws"]):break
                key("Return");key("s");save_dialog("foundingnext")
                load("normal");key("e")
                if has(scene(),"End your turn with movable units?"):key("Return")
                wait(lambda s:s["phase"]=="Human" and any(d.get("text","").startswith("Turn 2 ") for d in s["draws"]))
                research();key("s");save_dialog("normalnext")
                citizens=os.environ.get("TOTENGINE_CITIZENS_FIXTURES")
                if citizens:
                    import shutil
                    fixtures_root=Path(citizens)
                    shutil.copyfile(fixtures_root/"mature.cth",work/"saves/mature.cth")
                    meta=json.loads((fixtures_root/"route.json").read_text())
                    def compare(name,expected):
                        key("s");save_dialog(name)
                        assert (work/"saves"/(name+".cth")).read_bytes()==(fixtures_root/(expected+".cth")).read_bytes(),name
                    load("mature");key("o")
                    assert any("population 5" in d.get("text","") for d in scene()["draws"])
                    for _ in range(21):
                        if has(scene(),"Tile "+str(meta["worker_tile"])):break
                        key("Right")
                    key("space");compare("cityentertainer","after-entertainer")
                    button("1: Entertainer");compare("citytax","after-tax")
                    button("1: Tax collector");compare("cityscience","after-science")
                    button("1: Scientist");compare("citycycle","after-cycle")
                    button("Choose production");key("Down");key("Escape")
                    button("Auto workers (centre tile)");load("citycycle")
                    for _ in range(21):
                        if has(scene(),"Tile "+str(meta["worker_tile"])):break
                        key("Right")
                    key("space");compare("cityresume","after-resume")
                    key("Escape");key("o")
                    navigation=os.environ.get("TOTENGINE_NAVIGATION_FIXTURE")
                    if navigation:
                        shutil.copyfile(navigation,work/"saves/navigation.cth");load("navigation")
                        first=next(d["text"] for d in scene()["draws"] if " — population " in d.get("text",""))
                        key("Next")
                        assert next(d["text"] for d in scene()["draws"] if " — population " in d.get("text",""))!=first
                        key("Prior");key("s");save_dialog("citynavigation")
                        assert (work/"saves/citynavigation.cth").read_bytes()==Path(navigation).read_bytes()
                    key("Escape")
                fixtures=os.environ.get("TOTENGINE_ENCOUNTER_FIXTURES")
                if fixtures:
                    import shutil
                    for name,actor in [("battle",1),("capture",7)]:
                        shutil.copyfile(Path(fixtures)/("before-"+name+".cth"),work/"saves"/(name+".cth"))
                        load(name)
                        for _ in range(80):
                            if any(d.get("text","").endswith(" #"+str(actor)) for d in scene()["draws"]):break
                            key("w")
                        else:raise RuntimeError("combat actor absent")
                        if has(scene(),"Viewing [M]"):key("m")
                        key("8")
                        wait(lambda s:any(d.get("text","").startswith("Confirm ") and d["text"].endswith("?") for d in s["draws"]))
                        key("Escape");key("8");key("Return")
                        key("s");save_dialog(name+"after")
                        assert (work/"saves"/(name+"after.cth")).read_bytes()==(Path(fixtures)/("after-"+name+".cth")).read_bytes(),name
                resize(display,window,1440,1000);sync(display,0);time.sleep(.3)
                focus(display,root,2,0);sync(display,0);time.sleep(.2);focus(display,window,2,0);sync(display,0);time.sleep(.2)
                button("Close client");key("Return")
                if app.wait(timeout=15)!=0:raise RuntimeError("client did not close cleanly")
                print(work)
            finally:
                if app and app.poll() is None:
                    app.terminate(); app.wait(timeout=10)
                if display: bind(x, "XCloseDisplay", c.c_int, c.c_void_p)(display)
                server.terminate(); server.wait(timeout=10)

if __name__ == "__main__": run()
