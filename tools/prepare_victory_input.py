#!/usr/bin/env python3
"""Make a private, controlled Original-profile copy for the ENDGAME input test.

Copies only local text/BMP/GIF resources, never video/audio. The fixture changes
the human's Future Technology counter to five, sets science to zero, and uses
authored event prose with a guaranteed RANDOMTURN roll. The installed Original
event with denominator=3 is tested separately by events_install.rs.
"""
import argparse
import os
from pathlib import Path
import shutil


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True, help='New private directory outside the repository')
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    out = args.out.resolve()
    if out.is_relative_to(root):
        parser.error('original resource copies must stay outside the repository')
    home = Path(os.environ['CIVTOT_HOME']).resolve()
    if out.is_relative_to(home) or home.is_relative_to(out):
        parser.error('output must be separate from the original installation')
    out.mkdir(parents=True, exist_ok=False)
    profile = out / 'Original'
    profile.mkdir()
    for source, target in [(home, out), (home / 'Original', profile)]:
        for file in source.iterdir():
            if file.is_file() and file.suffix.lower() in ('.bmp', '.gif', '.txt'):
                shutil.copyfile(file, target / file.name)
    save = bytearray((home / 'Original/Ph_Auto2.SAV').read_bytes())
    assert save[:8] == b'CIVILIZE' and save[10:12] == b'\x32\x00'
    human = save[680]
    assert 1 <= human <= 7
    save[2944 + 3348 * human + 25] = 5
    for civ in range(8):
        save[2944 + 3348 * civ + 27] = 0
    (profile / 'Victory.sav').write_bytes(save)
    (profile / 'Events.txt').write_text(
        '@IF\nRECEIVEDTECHNOLOGY\nreceiver=Anybody\nfuturetech=5\n@AND\n'
        'RANDOMTURN\ndenominator=1\n@THEN\nTEXT\nControlled victory.\nENDTEXT\n'
        'ENDGAME\nendscreens=yes\nPLAYAVIFILE\nScene6.avi\n@ENDIF\n')
    print(out)


if __name__ == '__main__':
    main()
