#!/usr/bin/env python3
"""Assemble an isolated feasibility package, never original art or font files."""
import argparse
from pathlib import Path
import shutil

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[1]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--sdl-runtime', type=Path)
    parser.add_argument('--unwind-runtime', type=Path)
    parser.add_argument('--rust-docs', type=Path, required=True, help='toolchain share/doc/rust containing COPYRIGHT-library.html and licenses')
    args = parser.parse_args()
    windows = args.binary.suffix.lower() == '.exe'
    if windows and (not args.sdl_runtime or not args.unwind_runtime): parser.error('Windows requires --sdl-runtime and --unwind-runtime')
    if args.out.exists(): parser.error('--out must not exist')
    files = [args.binary, args.rust_docs / 'COPYRIGHT-library.html']
    if windows:
        if args.sdl_runtime.name != 'SDL2.dll' or args.unwind_runtime.name != 'libunwind.dll': parser.error('runtime files must be SDL2.dll and libunwind.dll')
        files += [args.sdl_runtime, args.unwind_runtime]
    if any(not f.is_file() for f in files) or not (args.rust_docs / 'licenses').is_dir(): parser.error('required executable, runtime or Rust notice missing')
    args.out.mkdir(parents=True)
    binary = args.out / ('tot-desktop-spike.exe' if windows else 'tot-desktop-spike')
    shutil.copy2(args.binary, binary)
    if windows:
        shutil.copy2(args.sdl_runtime, args.out / 'SDL2.dll')
        shutil.copy2(args.unwind_runtime, args.out / 'libunwind.dll')
    shutil.copytree(ROOT / 'notices', args.out / 'notices')
    shutil.copytree(REPO / 'docs/third-party', args.out / 'notices/shared-decoder')
    shutil.copy2(REPO / 'LICENSE', args.out / 'LICENSE')
    shutil.copy2(args.rust_docs / 'COPYRIGHT-library.html', args.out / 'notices/Rust-COPYRIGHT-library.html')
    shutil.copytree(args.rust_docs / 'licenses', args.out / 'notices/rust')
    (args.out / 'README.txt').write_text('''ToTEngine isolated desktop feasibility spike (#126), not a playable client.
Run tot-desktop-spike (Windows: tot-desktop-spike.exe).
No development tools, browser or HTTP server are needed to launch it.
Linux needs the system SDL2 shared library. Windows DLLs are included.
A system DejaVu Sans (Linux) or Arial (Windows) font is read locally;
set TOTENGINE_FONT to another owned TTF if the default is absent.
CIVTOT_HOME optionally selects your original ToT installation. Without it,
the window draws our synthetic fixtures. No game assets or fonts are bundled.
N opens the city-name modal. Tab/Shift-Tab changes focus, Enter accepts,
Escape cancels the modal or closes the main window. The OS close button works.
This spike does not alter the installation and cannot advance a game.
Retain LICENSE and the notices directory when redistributing the program.
''')
    print(args.out)

if __name__ == '__main__': main()
