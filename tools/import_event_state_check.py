#!/usr/bin/env python3
"""Measure event state words carried by the native save writer and reader.

The writer runs its event-only path over two synthetic records with no string
slots. File I/O is redirected to a bounded in-memory buffer. The existing
event reader trace reads that buffer using its documented substitutes.
No event actions execute and no original game data or saves are emitted.
"""
import argparse
import itertools
import json
import os
import struct
from pathlib import Path

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from import_event_tail_check import FRAME, HEAD, RECORD_SIZE, ROUND_FLAG, digest, frame, read, trace
from totctl import walk_save_bytes
from yields_check import OriginalCity

WRITE_ENTRY = 0x4605DF
WRITE_STOP = 0x460ADA
RECORDS = (0x700000, 0x700200)
SPENT = 0x20000000
ACTIVE = (0x20000000, 0x80000000)


def write_events(binary, words, *, enabled):
    original = OriginalCity(binary)
    frame(original)
    original.write(FRAME - 0x118, 1, '<I')
    original.write(FRAME + 0x0C, 0, '<I')
    original.write(ROUND_FLAG, int(enabled), '<I')
    original.write(HEAD, RECORDS[0], '<I')
    for record, (action, modifier) in zip(RECORDS, words, strict=True):
        original.cpu.mem_write(record, bytes(RECORD_SIZE + 8))
        original.write(record + 4, action, '<I')
        original.write(record + 8, modifier, '<I')
    original.write(RECORDS[0] + RECORD_SIZE, RECORDS[1], '<I')
    original.write(RECORDS[1] + RECORD_SIZE + 4, RECORDS[0], '<I')
    output = bytearray()
    sizes = []

    def capture(cpu, address, _size, _data):
        if WRITE_ENTRY <= address < WRITE_STOP:
            return
        assert address in (0x5B4850, 0x5B4250), f'unexpected writer call {address:#x}'
        esp = cpu.reg_read(UC_X86_REG_ESP)
        ret = read(original, esp, '<I')
        if address == 0x5B4850:
            src, handle = struct.unpack('<2I', cpu.mem_read(esp + 4, 8))
            assert handle == 1
            payload = bytes(cpu.mem_read(src, 5))
            assert payload == b'EVNT\0'
            payload = payload[:-1]
            value = 0  # fputs success
        else:
            src, width, count, handle = struct.unpack('<4I', cpu.mem_read(esp + 4, 16))
            assert count == handle == 1 and width in (4, RECORD_SIZE)
            assert len(sizes) < 4
            payload = bytes(cpu.mem_read(src, width))
            value = 1  # fwrite one item
        sizes.append(len(payload))
        output.extend(payload)
        cpu.reg_write(UC_X86_REG_EAX, value)
        cpu.reg_write(UC_X86_REG_ESP, esp + 4)
        cpu.reg_write(UC_X86_REG_EIP, ret)

    hook = original.cpu.hook_add(UC_HOOK_CODE, capture)
    try:
        original.cpu.emu_start(WRITE_ENTRY, WRITE_STOP, count=10000)
    finally:
        original.cpu.hook_del(hook)
    assert original.cpu.reg_read(original.ip) == WRITE_STOP
    assert sizes == ([4, 4, RECORD_SIZE, RECORD_SIZE] if enabled else [])
    return bytes(output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    evidence = json.loads((root / 'docs/evidence/import-event-tail.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(binary) == evidence['binary_sha256']
    save = (home / evidence['source']['path']).read_bytes()
    assert digest(save) == evidence['source']['sha256']
    cities = walk_save_bytes(save)['cities']
    event_offset = cities['offset'] + cities['size'] + evidence['event_offset_after_cities']
    pinned = trace(binary, save[event_offset:], include_record_words=True)
    assert pinned['round_flag_at_stop'] == 1 and pinned['remaining'] == 0
    pinned_words = pinned['action_and_modifier_words']
    assert pinned_words == [[0x5001, 0x10000108], [0x5001, 0x40000008]]
    assert all(not (action & (SPENT | 0x40) or modifier & active)
               for (action, modifier), active in zip(pinned_words, ACTIVE, strict=True))

    cases = []
    for first_spent, second_spent, first_active, second_active in itertools.product(
        (False, True), repeat=4
    ):
        words = [
            [0x40 | (SPENT if first_spent else 0), 0x10000000 | (ACTIVE[0] if first_active else 0)],
            [0x40 | (SPENT if second_spent else 0), 0x40000000 | (ACTIVE[1] if second_active else 0)],
        ]
        wire = write_events(binary, words, enabled=True)
        assert len(wire) == 8 + 2 * RECORD_SIZE
        assert wire.startswith(b'EVNT') and struct.unpack_from('<I', wire, 4)[0] == 2
        written = [list(struct.unpack_from('<II', wire, 12 + i * RECORD_SIZE))
                   for i in range(2)]
        loaded = trace(binary, wire, include_record_words=True)
        assert loaded['round_flag_at_stop'] == 1 and loaded['remaining'] == 0
        assert written == words == loaded['action_and_modifier_words']
        cases.append({'input': words, 'written': written, 'read': loaded['action_and_modifier_words']})
    disabled = write_events(binary, cases[-1]['input'], enabled=False)
    assert not disabled
    report = {
        'schema': 1,
        'binary_sha256': evidence['binary_sha256'],
        'source': evidence['source'],
        'writer': {'entry': '004605df', 'stop': '00460ada',
                   'write_sizes': [4, 4, RECORD_SIZE, RECORD_SIZE],
                   'substituted_helpers': ['005b4850', '005b4250'],
                   'bytes_with_round_flag_clear': len(disabled)},
        'pinned_action_and_modifier_words': pinned_words,
        'cases': cases,
        'scope': 'Single-player event section writer and reader boundaries. No action dispatch, later loader passes, or engine import compatibility are established.',
    }
    args.out.write_text(json.dumps(report, indent=2) + '\n')
    print(f'{len(cases)} native event-state write/read comparisons match')


if __name__ == '__main__':
    main()
