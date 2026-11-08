#!/usr/bin/env python3
"""Prepare a separately licensed Isotrident artwork pack from Freeciv 3.2.7.

Copies artwork and its editable sprite definitions, never Freeciv game code or
rules. Output must not exist. The manifest is independently generated from the
documented grid coordinates, preserving upstream sprite tags and artist credits.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil

SPECS = ('isotrident/terrain1', 'isotrident/terrain2', 'isotrident/cities',
         'trident/units', 'misc/small', 'misc/specialists')


def copy_pack(pack, out):
    """Copy only the expected artwork/notice files after checking all hashes."""
    expected = {'COPYING', 'FREECIV-README', 'isotrident.tilespec', 'atlas.json'}
    expected.update(s + ext for s in SPECS for ext in ('.png', '.spec'))
    manifest = json.loads((pack / 'SHA256.json').read_text())
    if set(manifest) != expected:
        raise ValueError('unexpected or missing Isotrident pack files')
    root = pack.resolve()
    for name in sorted(expected):
        path = pack / name
        if not path.resolve().is_relative_to(root) or not path.is_file():
            raise ValueError(f'unsafe Isotrident pack path: {name}')
        if hashlib.sha256(path.read_bytes()).hexdigest() != manifest[name]:
            raise ValueError(f'Isotrident hash mismatch: {name}')
    out.mkdir(parents=True, exist_ok=False)
    for name in sorted(expected | {'SHA256.json'}):
        dest = out / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(pack / name, dest)


def sprites(text, sheet):
    result, grid = {}, {}
    in_tiles = False
    for raw in text.splitlines():
        line = raw.split(';', 1)[0].strip()
        if line.startswith('[grid_'):
            grid, in_tiles = {}, False
        elif re.fullmatch(r'\[.*\]', line):
            grid, in_tiles = {}, False
        elif line.startswith('tiles = {'):
            in_tiles = True
        elif line == '}':
            in_tiles = False
        elif in_tiles:
            match = re.match(r'(\d+)\s*,\s*(\d+)\s*,(.*)', line)
            if match:
                row, col = map(int, match.group(1, 2))
                w, h = grid['dx'], grid['dy']
                border = grid.get('pixel_border', 0)
                rect = [grid.get('x_top_left', 0) + col * (w + border),
                        grid.get('y_top_left', 0) + row * (h + border), w, h]
                for tag in re.findall(r'"([^"]+)"', match[3]):
                    if tag in result:
                        raise ValueError(f'duplicate sprite: {tag}')
                    result[tag] = dict(sheet=sheet + '.png', rect=rect)
        else:
            match = re.fullmatch(r'(\w+)\s*=\s*(\d+)', line)
            if match:
                grid[match[1]] = int(match[2])
    if not result:
        raise ValueError(f'no sprites: {sheet}')
    return result


def prepare(source, out):
    if out.exists():
        raise ValueError('output must not exist')
    data = source / 'data'
    version = (source / 'fc_version').read_text()
    for key, value in [('MAJOR_VERSION', '3'), ('MINOR_VERSION', '2'), ('PATCH_VERSION', '7')]:
        if not re.search(rf'^{key}="{value}"$', version, re.MULTILINE):
            raise ValueError('this adapter is pinned to Freeciv 3.2.7')
    files = [source / 'COPYING', source / 'doc/README', data / 'isotrident.tilespec']
    files += [data / (s + ext) for s in SPECS for ext in ('.png', '.spec')]
    if not all(p.is_file() for p in files):
        raise ValueError('expected the complete Freeciv 3.2.7 source directory')
    tags = {}
    for spec in SPECS:
        tags.update(sprites((data / (spec + '.spec')).read_text(), spec))
    out.mkdir(parents=True)
    shutil.copy2(source / 'COPYING', out / 'COPYING')
    shutil.copy2(source / 'doc/README', out / 'FREECIV-README')
    for file in files[2:]:
        dest = out / file.relative_to(data)
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(file, dest)
    manifest = dict(version=1, upstream='Freeciv 3.2.7', license='GPL-2.0-or-later',
                    source='https://files.freeciv.org/stable/freeciv-3.2.7.tar.xz',
                    sprites=tags)
    (out / 'atlas.json').write_text(json.dumps(manifest, indent=2, sort_keys=True) + '\n')
    hashes = {str(p.relative_to(out)): hashlib.sha256(p.read_bytes()).hexdigest()
              for p in sorted(out.rglob('*')) if p.is_file()}
    (out / 'SHA256.json').write_text(json.dumps(hashes, indent=2) + '\n')
    return len(tags)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    print(f'{prepare(args.source, args.out)} sprite definitions: {args.out}')
