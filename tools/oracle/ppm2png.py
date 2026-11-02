#!/usr/bin/env python3
"""ppm2png.py — convert xtool's PPM screenshots to PNG (stdlib only) so
vision-capable models can read them (the read tool presents PNG images).

  ppm2png.py <in.ppm> <out.png>
"""
import re
import struct
import sys
import zlib


def convert(src: str, dst: str) -> None:
    data = open(src, "rb").read()
    m = re.match(rb"P6\s+(\d+)\s+(\d+)\s+(\d+)\s", data)
    if not m:
        raise SystemExit("not a binary PPM")
    w, h = int(m.group(1)), int(m.group(2))
    raw = data[m.end():]

    def chunk(t: bytes, d: bytes) -> bytes:
        return (struct.pack(">I", len(d)) + t + d
                + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF))

    scan = b"".join(b"\x00" + raw[3 * w * y: 3 * w * (y + 1)]
                    for y in range(h))
    png = (b"\x89PNG\r\n\x1a\n"
           + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
           + chunk(b"IDAT", zlib.compress(scan, 6))
           + chunk(b"IEND", b""))
    open(dst, "wb").write(png)
    print(f"{dst}: {w}x{h}")


if __name__ == "__main__":
    convert(sys.argv[1], sys.argv[2])
