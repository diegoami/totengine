#!/usr/bin/env python3
"""Trace append and reset of the plain-save history block before EVNT.

The native append fragment runs with a synthetic label and substitutes two
name helpers. The native reset runs without substitutions. The report keeps
counts and offsets and does not retain names or raw record bytes.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from import_phase_check import FRAME, frame, read, run
from totctl import walk_save_bytes
from yields_check import OriginalCity

HISTORY = 0x606B38
SIZE = 340
CAPACITY = 12
APPEND = 0x48BE1F
APPEND_STOP = 0x48BEA3
RESET = 0x487E4A
SYNTHETIC_LABEL = b'Fixture civ\0'
LABEL_ADDR = 0x710000


def append_case(binary, count):
    original = OriginalCity(binary)
    frame(original)
    original.cpu.mem_write(HISTORY, bytes([0xA5]) * SIZE)
    original.write(HISTORY, count, '<h')
    original.write(FRAME + 8, 4, '<I')
    original.write(FRAME + 12, 2, '<I')
    original.write(0x607648, 43, '<h')
    original.write(0x5F7390 + 3348 * 4, 7, '<B')
    original.cpu.mem_write(LABEL_ADDR, SYNTHETIC_LABEL)
    before = bytes(original.cpu.mem_read(HISTORY, SIZE))
    calls = []

    def replace(cpu, addr, _size, _data):
        if addr not in (0x47A074, 0x5B1220):
            return
        esp = cpu.reg_read(UC_X86_REG_ESP)
        ret = read(original, esp, '<I')
        calls.append(f'{addr:08x}')
        if addr == 0x47A074:
            assert read(original, esp + 4) == 4
            result = LABEL_ADDR
        else:
            dst = read(original, esp + 4, '<I')
            src = read(original, esp + 8, '<I')
            assert src == LABEL_ADDR and dst == HISTORY + 50 + count * 24
            cpu.mem_write(dst, SYNTHETIC_LABEL)
            result = dst
        cpu.reg_write(UC_X86_REG_EAX, result)
        cpu.reg_write(UC_X86_REG_ESP, esp + 4)
        cpu.reg_write(UC_X86_REG_EIP, ret)

    hook = original.cpu.hook_add(UC_HOOK_CODE, replace)
    try:
        run(original, APPEND, {APPEND_STOP}, calls=(0x47A074, 0x5B1220))
    finally:
        original.cpu.hook_del(hook)
    after = bytes(original.cpu.mem_read(HISTORY, SIZE))
    expected = bytearray(before)
    if count < CAPACITY:
        expected[:2] = (count + 1).to_bytes(2, 'little')
        expected[2 + 2 * count:4 + 2 * count] = (43).to_bytes(2, 'little')
        expected[26 + count] = 2
        expected[38 + count] = 7
        label_start = 50 + 24 * count
        expected[label_start:label_start + len(SYNTHETIC_LABEL)] = SYNTHETIC_LABEL
        assert calls == ['0047a074', '005b1220']
    else:
        assert not calls
    assert after == expected
    return {'initial_count': count, 'final_count': read(original, HISTORY, '<h'),
            'changed_offsets': [i for i, (a, b) in enumerate(zip(before, after, strict=True)) if a != b],
            'substituted_calls': calls}


def reset_case(binary):
    original = OriginalCity(binary)
    original.cpu.mem_write(HISTORY, bytes([0xA5]) * SIZE)
    original.call(RESET)
    after = bytes(original.cpu.mem_read(HISTORY, SIZE))
    cleared = [i for i, byte in enumerate(after) if byte == 0]
    assert cleared == list(range(38)) + [50 + i * 24 for i in range(CAPACITY)]
    assert all(byte == 0 or byte == 0xA5 for byte in after)
    return {'entry': f'{RESET:08x}', 'cleared_offsets': cleared}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    evidence = json.loads((root / 'docs/evidence/import-event-tail.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    save = (home / evidence['source']['path']).read_bytes()
    assert hashlib.sha256(binary).hexdigest() == evidence['binary_sha256']
    assert hashlib.sha256(save).hexdigest() == evidence['source']['sha256']
    cities = walk_save_bytes(save)['cities']
    city_end = cities['offset'] + cities['size']
    last_read = evidence['prefix_trace']['reads']['00461bef']
    assert last_read['calls'] == 1 and last_read['width'] == last_read['bytes_read'] == SIZE
    end = evidence['event_offset_after_cities']
    offset = end - SIZE
    history = save[city_end + offset:city_end + end]
    assert len(history) == SIZE and int.from_bytes(history[:2], 'little', signed=True) == 0
    assert save[city_end + end:city_end + end + 4] == b'EVNT'
    report = {
        'schema': 1,
        'binary_sha256': evidence['binary_sha256'],
        'source': evidence['source'],
        'source_witness': {'offset_after_cities': offset, 'bytes': SIZE, 'used_entries': 0},
        'layout': {
            'count_offset': 0, 'capacity': CAPACITY, 'turn_words_offset': 2,
            'first_byte_array_offset': 26, 'second_byte_array_offset': 38,
            'label_array_offset': 50, 'label_stride': 24,
        },
        'append': {'entry': f'{APPEND:08x}', 'stop': f'{APPEND_STOP:08x}',
                   'cases': [append_case(binary, count) for count in (0, 11, 12)]},
        'reset': reset_case(binary),
        'scope': 'Append, reset and pinned count. Entry payload meanings beyond the turn and label fields and the unused ending bytes remain open.',
    }
    args.out.write_text(json.dumps(report, indent=2) + '\n')
    print(f'History block: {SIZE} bytes at city end + {offset}, zero entries in the pinned save')


if __name__ == '__main__':
    main()
