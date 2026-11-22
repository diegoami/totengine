#!/usr/bin/env python3
"""Measure the native peace routine up to its first post-treaty callback."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import struct

from import_phase_check import CIV, FRAME, STRIDE, frame, read, run
from yields_check import OriginalCity

ENTRY = 0x44A949
STOP = 0x44A992
CALLS = (0x4545D2, 0x454500, 0x4546AE, 0x566A30, 0x4546DC)


def encounter_gate(original, *, a, b, turn, humans, viewer, treaty, last_contact):
    """Single-player contact entry with the scenario event gate clear."""
    original.write(0x5d0a0c, 0, '<I')
    original.write(0x607652, 0, '<B')
    original.write(0x60765b, humans, '<B')
    original.write(0x607648, turn, '<h')
    original.write(0x67a6e0, viewer, '<I')
    original.write(CIV + a * STRIDE + 40 + 4 * b, treaty, '<I')
    original.write(0x5f7380 + a * STRIDE + 2 * b, last_contact, '<h')
    original.cpu.mem_write(0x108000, struct.pack('<6I', 0x100000, a, b, 10, 12, 0))
    original.cpu.reg_write(original.sp, 0x108000)
    before = original.cpu.mem_read(CIV, STRIDE * 8)
    stop = run(original, 0x526ACA, {0x526EA2, 0x526EAA, 0x52877C})
    expected = 'return' if not a or not b or turn == 0 else 'other_seat'
    if expected != 'return' and humans & (1 << a) and viewer == a:
        if not treaty & 1 or treaty & 0x10000 or last_contact < 0 or turn - last_contact >= 16:
            expected = 'conversation'
    assert {0x526EA2: 'conversation', 0x526EAA: 'other_seat', 0x52877C: 'return'}[stop] == expected
    assert original.cpu.mem_read(CIV, STRIDE * 8) == before
    return {'pair': [a, b], 'turn': turn, 'human_mask': humans, 'viewer': viewer,
            'outgoing_treaty': treaty, 'last_contact': last_contact,
            'stop': f'{stop:08x}', 'boundary': expected}


def offer_gate(original, *, treaty, prior, forced, suppressed, threshold_base,
               civ_byte, pair_byte):
    """The last local eligibility checks before the separate offer helper."""
    frame(original)
    a, b = 1, 2
    original.write(FRAME + 8, a, '<I')
    original.write(FRAME + 12, b, '<I')
    original.write(FRAME - 0x20, prior, '<I')
    original.write(FRAME - 0x2c, 0, '<I')
    original.write(0x5f4684, forced, '<I')
    original.write(0x5f4648, threshold_base, '<I')
    original.write(0x5f4678, suppressed, '<I')
    original.write(0x5f4650, 0, '<I')
    original.write(CIV + a * STRIDE + 38, civ_byte, '<B')
    original.write(CIV + b * STRIDE + 80 + a, pair_byte, '<b')
    original.write(CIV + b * STRIDE + 40 + 4 * a, treaty, '<I')
    before = original.cpu.mem_read(CIV, STRIDE * 8)
    stop = run(original, 0x452CC1, {0x452D97, 0x4530ED})
    expected = forced or (prior and not treaty & 2) or (
        not suppressed and civ_byte - pair_byte < threshold_base + 4
    )
    assert (stop == 0x452D97) == bool(expected)
    assert original.cpu.mem_read(CIV, STRIDE * 8) == before
    return {'treaty': treaty, 'prior_local': prior, 'forced': forced, 'suppressed': suppressed,
            'threshold_base': threshold_base, 'unsigned_civ_byte_38': civ_byte,
            'signed_pair_byte_80': pair_byte,
            'reaches_offer_helper': bool(expected)}


def resolve_gate(original, status):
    """Start with a prepared final status after all earlier negotiation."""
    frame(original)
    original.write(FRAME + 8, 1, '<I')
    original.write(FRAME + 12, 2, '<I')
    original.write(FRAME - 0x6c, status, '<i')
    before = original.cpu.mem_read(CIV, STRIDE * 8)
    stop = run(original, 0x4530A0, {0x4530B9, 0x4530D6})
    assert stop == (0x4530D6 if status else 0x4530B9)
    assert read(original, FRAME - 0x44) == 1
    assert original.cpu.mem_read(CIV, STRIDE * 8) == before
    return {'final_status': status, 'stop': f'{stop:08x}',
            'boundary': 'refusal_notice' if status else 'apply_peace'}


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
    gates = [
        offer_gate(original, treaty=treaty, prior=prior, forced=forced, suppressed=suppressed,
                   threshold_base=5, civ_byte=civ_byte, pair_byte=pair_byte)
        for treaty, prior, forced, suppressed, civ_byte, pair_byte in (
            (1, 0, 0, 0, 10, 2), (1, 0, 0, 0, 11, 2), (1, 0, 0, 0, 12, 2),
            (1, 0, 0, 0, 7, -2),
            (1, 0, 0, 1, 10, 2), (1, 0, 1, 1, 12, 2),
            (1, 1, 0, 1, 12, 2), (3, 1, 0, 1, 12, 2),
        )
    ]
    encounters = [
        encounter_gate(original, a=a, b=2, turn=turn, humans=humans, viewer=viewer,
                       treaty=treaty, last_contact=last_contact)
        for a, turn, humans, viewer, treaty, last_contact in (
            (0, 20, 2, 1, 0, 20), (1, 0, 2, 1, 0, 0),
            (1, 20, 0, 1, 0, 20), (1, 20, 2, 2, 0, 20), (1, 20, 2, 1, 0, 20),
            (1, 20, 2, 1, 1, 5), (1, 20, 2, 1, 1, 4),
            (1, 20, 2, 1, 1, -1), (1, 20, 2, 1, 0x10001, 20),
        )
    ]
    report = {'schema': 2, 'binary_sha256': hashlib.sha256(binary).hexdigest(),
              'entry': f'{ENTRY:08x}', 'stop': f'{STOP:08x}', 'cases': cases,
              'offer_gate': {'entry': '00452cc1', 'cases': gates},
              'encounter_gate': {'entry': '00526aca', 'cases': encounters},
              'resolution_gate': {'entry': '004530a0',
                                  'cases': [resolve_gate(original, value) for value in (0, 1, -1)]},
              'scope': 'Native treaty/attitude prefix and contact, offer and response gates from '
                       'controlled inputs. No substituted callees. Stops precede offer helper, '
                       'refusal presentation and post-treaty callbacks.'}
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
