#!/usr/bin/env python3
"""Gate wizard Returns on visually verified dialog pixels.

Usage: dialog_guard.py TITLE DIRECTORY (outside Git).
Unknown dialogs produce pending.png/pending.json and wait for an approve
file. Inspect the image before creating it. Radio lists compare the selected
button position in a fixed title/geometry; other dialogs compare all pixels.
Changed signatures pause again. This is not OCR of arbitrary dialog labels.
"""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent


def main(title, directory):
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    listing = subprocess.check_output(["xwininfo", "-tree", "-root"], text=True)
    pattern = re.compile(r'"' + re.escape(title) + r'[^"\n]*".*?\s(\d+)x(\d+)\+(-?\d+)\+(-?\d+)')
    matches = [pattern.search(line) for line in listing.splitlines()]
    matches = [match for match in matches if match]
    if len(matches) != 1:
        sys.exit(f"expected one visible dialog: {title}")
    width, height, x, y = map(int, matches[0].groups())
    shot = directory / "screen.ppm"
    subprocess.run([str(HERE / "xtool"), "shot", str(shot)], check=True, timeout=15)
    magic, dims, maximum, pixels = shot.read_bytes().split(b"\n", 3)
    screen_width, screen_height = map(int, dims.split())
    if (magic != b"P6" or maximum != b"255" or x < 0 or y < 0
            or x + width > screen_width or y + height > screen_height
            or len(pixels) != screen_width * screen_height * 3):
        sys.exit("unexpected screenshot geometry")
    crop = b"".join(pixels[((y + row) * screen_width + x) * 3:
                           ((y + row) * screen_width + x + width) * 3] for row in range(height))
    # Wizard stone backgrounds vary between launches. For radio lists the
    # stable semantic feature is the red selected-button mask, with geometry
    # and title fixing the list and row positions. Other dialogs retain the
    # stricter full-pixel check and can require a new visual inspection.
    radio_titles = {"Select Size of World", "Select Difficulty Level", "Select Level Of Competition",
                    "Select Level Of Barbarian Activity", "Select Game Rules", "Select Your Tribe", "Select Gender"}
    if title in radio_titles:
        red = {(i % width, i // width) for i, (r, g, b) in enumerate(zip(crop[::3], crop[1::3], crop[2::3]))
               if r > 200 and g < 110 and b < 110}
        centers = []
        while red:
            stack, component = [red.pop()], []
            while stack:
                a, b = stack.pop()
                component.append((a, b))
                for point in ((a + 1, b), (a - 1, b), (a, b + 1), (a, b - 1)):
                    if point in red:
                        red.remove(point)
                        stack.append(point)
            if len(component) >= 12:
                # Rows are 33 pixels apart and tribe columns 200 apart.
                # Quantization tolerates tiny button-rendering shifts without
                # merging any two choices. Ignore isolated stone-texture flecks.
                centers.append(tuple(round(sum(p[axis] for p in component) / len(component) / 16)
                                     for axis in (0, 1)))
        if len(centers) != 1:
            sys.exit(f"expected one selected radio button: {title}")
        digest = "radio-center:" + str(centers[0])
    else:
        digest = "pixels:" + hashlib.sha256(crop).hexdigest()
    templates = directory / "approved.json"
    approved = json.loads(templates.read_text()) if templates.exists() else {}
    key = f"{title}:{width}x{height}"
    if approved.get(key) == digest:
        print(f"dialog verified: {title}", flush=True)
        return
    approve = directory / "approve"
    approve.unlink(missing_ok=True)
    subprocess.run([sys.executable, str(HERE / "ppm2png.py"), str(shot), str(directory / "pending.png")],
                   check=True, timeout=15)
    pending = directory / "pending.json"
    pending.write_text(json.dumps(dict(title=title, width=width, height=height, sha256=digest)))
    print(f"INSPECT {directory / 'pending.png'} ({title}); then create {approve}", flush=True)
    deadline = time.monotonic() + 180
    while not approve.exists():
        if time.monotonic() > deadline:
            sys.exit(f"dialog inspection timed out: {title}")
        time.sleep(0.5)
    approve.unlink()
    approved[key] = digest
    templates.write_text(json.dumps(approved, indent=2) + "\n")
    pending.unlink()


if __name__ == "__main__":
    main(*sys.argv[1:])
