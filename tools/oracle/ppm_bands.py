#!/usr/bin/env python3
"""Find clickable bright bands (menu buttons) in a PPM screenshot."""
import sys

def read_ppm(path):
    d = open(path, 'rb').read()
    assert d[:2] == b'P6', 'not P6'
    parts = d.split(b'\n', 3)
    w, h = map(int, parts[1].split())
    return w, h, parts[3]

def bands(path, thresh=150):
    w, h, px = read_ppm(path)
    rows = []
    for y in range(0, h, 2):
        bright = 0
        xs = []
        for x in range(0, w, 2):
            o = (y * w + x) * 3
            r, g, b = px[o], px[o+1], px[o+2]
            if (r + g + b) / 3 > thresh:
                bright += 1
                xs.append(x)
        if bright > 12:
            rows.append((y, bright, min(xs), max(xs)))
    # merge adjacent rows into bands
    out = []
    for y, n, x0, x1 in rows:
        if out and y - out[-1][1] <= 4:
            y0, _, a0, a1 = out[-1]
            out[-1] = (y0, y, min(a0, x0), max(a1, x1))
        else:
            out.append((y, y, x0, x1))
    for y0, y1, x0, x1 in out:
        print(f"band y {y0}-{y1}  x {x0}-{x1}  center ({(x0+x1)//2},{(y0+y1)//2})")

if __name__ == '__main__':
    bands(sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 150)
