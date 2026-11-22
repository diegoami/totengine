#!/usr/bin/env python3
"""Measure the native peace routine up to its first post-treaty callback."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import struct

from import_phase_check import CIV, STRIDE, run
from yields_check import OriginalCity

ENTRY = 0x44A949
STOP = 0x44A992
CALLS = (0x4545D2, 0x454500, 0x4546AE, 0x566A30, 0x4546DC)


def witness(original, a, b, initial_words, attitudes):
    treaty = (a * STRIDE + 40 + 4 * b, b * STRIDE + 40 + 4 * a)
    attitude = (a * STRIDE + 72 + b, b * STRIDE + 72 + a)
    for offset, value in zip(treaty, initial_words, strict=True):
        original.write(CIV + offset, value, '<I')
    for offset, value in zip(attitude, attitudes, strict=True):
        original.write(CIV + offset, value, '<B')
    expected = bytearray(original.cpu.mem_read(CIV, STRIDE * 8))
    original.cpu.mem_write(0x108000, struct.pack('<III', 0x100000, a, b))
    original.cpu.reg_write(original.sp, 0x108000)
    run(original, ENTRY, {STOP}, calls=CALLS)
    words = [(word & ~0x3a60) | 0x4004 for word in initial_words]
    final_attitudes = [attitudes[0], min(50, attitudes[1])]
    for offset, value in zip(treaty, words, strict=True):
        struct.pack_into('<I', expected, offset, value)
    for offset, value in zip(attitude, final_attitudes, strict=True):
        expected[offset] = value
    assert original.cpu.mem_read(CIV, STRIDE * 8) == expected
    return {'pair': [a, b], 'initial_words': initial_words, 'treaty_words': words,
            'initial_attitudes': attitudes, 'attitudes': final_attitudes}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    binary = (Path(os.environ['CIVTOT_HOME']) / 'civ2.bak').read_bytes()
    original = OriginalCity(binary)
    cases = [
        witness(original, a, b, words, attitudes)
        for a, b in ((1, 2), (4, 7))
        for words in ((1, 1), (0x202261, 0x203201))
        for attitudes in ((20, 99), (99, 20), (60, 255))
    ]
    report = {'schema': 1, 'binary_sha256': hashlib.sha256(binary).hexdigest(),
              'entry': f'{ENTRY:08x}', 'stop': f'{STOP:08x}', 'cases': cases,
              'scope': 'Controlled inputs and native treaty/attitude prefix with no substituted '
                       'callees. Stops before post-treaty callbacks. No offer or refusal is executed.'}
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
