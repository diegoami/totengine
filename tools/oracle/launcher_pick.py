"""launcher_pick.py <item> — select an item in the launcher's game-type list.

    python3 launcher_pick.py 0     # 0 = Original Game

The second "The Test of Time" list (Original, Science Fiction, Fantasy,
Extended Original, Midgard, Begin a Scenario, Load a Saved Game, Play a
Mod) REMEMBERS the last choice, and Up/Down WRAP around its 8 items. A
fixed key count therefore cycles: the old `Up×4` landed on Original
only from Midgard, and on "Load a Saved Game" from Fantasy (the recurring
"Select Game To Load" flake). This reads the selected radio button (the
red one) from a screenshot, presses Up the needed number of times,
re-checks, and exits non-zero if the selection is not `item`. It does not
press Return.
"""
import os
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
XT = str(HERE / "xtool")
N_ITEMS = 8
# radio centre relative to the window's top-left (291x344 window)
RADIO_DX, RADIO_DY, PITCH = 32, 49, 33


def window():
    out = subprocess.run(["xwininfo", "-tree", "-root"], capture_output=True, text=True).stdout
    for line in out.splitlines():
        m = re.search(r'"The Test of Time".*\s(\d+)x(\d+)\+(-?\d+)\+(-?\d+)', line)
        if m and int(m.group(2)) > 300:  # the 8-item list, not the 5-item menu
            return line.split()[0], int(m.group(3)), int(m.group(4))
    return None


def selected(x0, y0):
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, "s.ppm")
        subprocess.run([XT, "shot", p], capture_output=True, timeout=15)
        data = open(p, "rb").read()
    _, dims, _, px = data.split(b"\n", 3)
    w = int(dims.split()[0])
    reds = []
    for k in range(N_ITEMS):
        o = ((y0 + RADIO_DY + PITCH * k) * w + x0 + RADIO_DX) * 3
        r, g, b = px[o : o + 3]
        if r > 200 and g < 110 and b < 110:
            reds.append(k)
    return reds[0] if len(reds) == 1 else None


def main(argv):
    want = int(argv[0])
    win = window()
    if not win:
        sys.exit("game-type list not open")
    wid, x0, y0 = win
    subprocess.run([XT, "focus", wid], capture_output=True, timeout=5)
    for _ in range(3):
        cur = selected(x0, y0)
        if cur is None:
            sys.exit("cannot read the selected item")
        if cur == want:
            print(f"selected item {want}")
            return
        for _ in range((cur - want) % N_ITEMS):
            subprocess.run([XT, "key", "Up"], capture_output=True, timeout=8)
            time.sleep(0.4)
        time.sleep(0.5)
    sys.exit(f"selection stuck at {selected(x0, y0)}, wanted {want}")


if __name__ == "__main__":
    main(sys.argv[1:])
