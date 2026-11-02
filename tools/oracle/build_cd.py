#!/usr/bin/env python3
"""Build a fake-CD ISO9660+Joliet image of the ToT install dir (oracle T0-10).

The classic civ2.bak CD check wants a CD-ROM drive; wine auto-detects ISO
images symlinked as dosdevices drives and reports them as DRIVE_CDROM with
the ISO's volume label. Usage: build_cd.py <src-dir> <out-iso> <volume-id>
"""
import os
import sys

import pycdlib  # noqa: E402  (PYTHONPATH-injected)

src, out, volid = sys.argv[1], sys.argv[2], sys.argv[3]


def iso_name(name: str) -> str:
    """Sanitize one path component for iso9660 interchange level 3."""
    stem, dot, ext = name.rpartition('.')
    if not dot:
        stem, ext = name, ''
    stem = ''.join(c for c in stem.upper() if c.isalnum() or c in '_-') or 'X'
    ext = ''.join(c for c in ext.upper() if c.isalnum() or c in '_-')
    return f'{stem}.{ext}' if ext else stem


def iso_path(rel: str) -> str:
    parts = [iso_name(p) for p in rel.split(os.sep) if p not in ('.', '')]
    return '/' + '/'.join(parts)


def joliet_path(rel: str) -> str:
    parts = [p for p in rel.split(os.sep) if p not in ('.', '')]
    return '/' + '/'.join(parts)


iso = pycdlib.PyCdlib()
iso.new(interchange_level=3, joliet=3, vol_ident=volid)

count = 0
for root, dirs, files in os.walk(src):
    dirs.sort()
    rel = os.path.relpath(root, src)
    if rel != '.':
        try:
            iso.add_directory(iso_path(rel), joliet_path=joliet_path(rel))
        except Exception as exc:  # duplicate/sanitized collision
            print('dir skip', rel, exc)
    for f in sorted(files):
        relf = os.path.join(rel, f) if rel != '.' else f
        try:
            iso.add_file(os.path.join(root, f), iso_path(relf),
                         joliet_path=joliet_path(relf))
            count += 1
        except Exception as exc:
            print('file skip', relf, exc)

iso.write(out)
iso.close()
print(f'wrote {out}: {count} files, volume id {volid!r}')
