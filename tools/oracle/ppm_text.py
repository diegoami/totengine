#!/usr/bin/env python3
"""Render a PPM region as ASCII art (for reading UI text without vision)."""
import sys

def read_ppm(path):
    d = open(path, 'rb').read()
    parts = d.split(b'\n', 3)
    w, h = map(int, parts[1].split())
    return w, h, parts[3]

def main():
    path, x0, y0, x1, y1 = sys.argv[1], *map(int, sys.argv[2:6])
    thresh = int(sys.argv[6]) if len(sys.argv) > 6 else 120
    w, h, px = read_ppm(path)
    x0, y0 = max(0, x0), max(0, y0)
    x1, y1 = min(w, x1), min(h, y1)
    for y in range(y0, y1):
        line = ''
        for x in range(x0, x1):
            o = (y * w + x) * 3
            r, g, b = px[o], px[o+1], px[o+2]
            lum = (r + g + b) / 3
            line += '#' if lum > thresh else ('+' if lum > thresh // 2 else ' ')
        print(line)

if __name__ == '__main__':
    main()
