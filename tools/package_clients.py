#!/usr/bin/env python3
"""Package clients and notices, optionally with separately licensed Isotrident art."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
from prepare_isotrident import copy_pack

REPO = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--windows', action='store_true')
    parser.add_argument('--sdl-runtime', type=Path)
    parser.add_argument('--unwind-runtime', type=Path)
    parser.add_argument('--rust-docs', type=Path, required=True)
    parser.add_argument('--isotrident-pack', type=Path,
                        help='verified pack made by prepare_isotrident.py; never a ToT install')
    args = parser.parse_args()
    suffix = '.exe' if args.windows else ''
    files = [args.bin_dir / (name + suffix) for name in ('totplay', 'totdesktop')]
    if args.windows:
        if not args.sdl_runtime or not args.unwind_runtime:
            parser.error('Windows requires --sdl-runtime and --unwind-runtime')
        if args.sdl_runtime.name != 'SDL2.dll' or args.unwind_runtime.name != 'libunwind.dll':
            parser.error('runtime names must be SDL2.dll and libunwind.dll')
        files += [args.sdl_runtime, args.unwind_runtime]
    copyright_file = args.rust_docs / 'COPYRIGHT-library.html'
    if not all(p.is_file() for p in files + [copyright_file]):
        parser.error('an executable, runtime or Rust copyright file is missing')
    if not (args.rust_docs / 'licenses').is_dir():
        parser.error('Rust licenses directory missing')
    if args.out.exists():
        parser.error('--out must not exist')
    args.out.mkdir(parents=True)
    for file in files:
        shutil.copy2(file, args.out / file.name)
    shutil.copy2(REPO / 'LICENSE', args.out / 'LICENSE')
    shutil.copytree(REPO / 'docs/third-party', args.out / 'notices')
    shutil.copy2(copyright_file, args.out / 'notices/Rust-COPYRIGHT-library.html')
    shutil.copytree(args.rust_docs / 'licenses', args.out / 'notices/rust')
    shutil.copy2(REPO / 'docs/playable-clients.md', args.out / 'README.md')
    if args.isotrident_pack:
        copy_pack(args.isotrident_pack, args.out / 'isotrident')
    manifest = {
        str(p.relative_to(args.out)): hashlib.sha256(p.read_bytes()).hexdigest()
        for p in sorted(args.out.rglob('*')) if p.is_file()
    }
    (args.out / 'SHA256.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(args.out)


if __name__ == '__main__':
    main()
