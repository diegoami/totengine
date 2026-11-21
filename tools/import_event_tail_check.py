#!/usr/bin/env python3
"""Trace event-section reads in the unpatched loader on the pinned Original save.

Only numeric extents and hashes are retained. The probe substitutes bounded
reads and allocations. It executes the native post-load flag write and checks
the round-start dispatch boundary without executing any event actions.
"""
import argparse
import hashlib
import json
import os
import struct
from pathlib import Path

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from import_phase_check import FRAME, frame, read, run
from totctl import walk_save_bytes
from yields_check import OriginalCity

ENTRY = 0x461D7B
CALLBACK = 0x4622DB
CALLBACK_ROUTINE = range(0x4AC0F5, 0x4AC105)
SKIP = 0x4622EA
HEAD = 0x5F53B0 + 0x698
ROUND_FLAG = 0x5D0A0C
RECORD_SIZE = 0x114
FIRST_RECORD = 0x700000
HELPERS = {
    0x5B3810: 'bounded file read',
    0x5B4110: 'marker comparison',
    0x4D16CE: 'reset allocation state',
    0x4D170C: 'allocate and link record',
    0x45F346: 'bounded NUL-terminated string read with non-null witness pointer',
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def trace(binary, data):
    original = OriginalCity(binary)
    frame(original)
    cursor = 0
    blocks = []
    strings = []
    records = []
    stop = None
    before_callback = None
    original.write(ROUND_FLAG, 0, '<I')
    original.write(FRAME - 0x1990, 1, '<I')

    def watch(cpu, addr, size, _):
        nonlocal cursor, stop, before_callback
        if addr == CALLBACK:
            before_callback = read(original, ROUND_FLAG)
        if ENTRY <= addr < SKIP or addr in CALLBACK_ROUTINE:
            return
        if addr not in HELPERS:
            stop = addr
            cpu.emu_stop()
            return
        esp = cpu.reg_read(UC_X86_REG_ESP)
        ret = read(original, esp, '<I')
        pop = 0
        if addr == 0x5B3810:
            dst, width, count, handle = struct.unpack('<4I', cpu.mem_read(esp + 4, 16))
            assert handle == 1 and count == 1 and width in (4, RECORD_SIZE)
            payload = data[cursor:cursor + width]
            blocks.append({'offset': cursor, 'requested': width, 'available': len(payload)})
            if payload:
                cpu.mem_write(dst, payload)
            cursor += len(payload)
            value = int(len(payload) == width)
        elif addr == 0x5B4110:
            left, right, length = struct.unpack('<3I', cpu.mem_read(esp + 4, 12))
            assert length == 4
            value = int(cpu.mem_read(left, length) != cpu.mem_read(right, length))
        elif addr == 0x4D16CE:
            original.write(HEAD, 0, '<I')
            pop = 4
            value = 0
        elif addr == 0x4D170C:
            assert len(records) < 64
            value = FIRST_RECORD + len(records) * 0x200
            if records:
                original.write(records[-1] + RECORD_SIZE, value, '<I')
            else:
                original.write(HEAD, value, '<I')
            records.append(value)
        else:
            assert addr == 0x45F346 and read(original, esp + 4, '<I') == 1
            end = data.find(b'\0', cursor, cursor + 255)
            if end < 0:
                value = 0
            else:
                strings.append({'offset': cursor, 'bytes': end + 1 - cursor})
                cursor = end + 1
                # The outer reader only tests the returned pointer for null.
                value = 0x710000 + len(strings) * 0x100
        cpu.reg_write(UC_X86_REG_EAX, value)
        cpu.reg_write(UC_X86_REG_ESP, esp + 4 + pop)
        cpu.reg_write(UC_X86_REG_EIP, ret)

    hook = original.cpu.hook_add(UC_HOOK_CODE, watch)
    try:
        original.cpu.emu_start(ENTRY, 0, count=50000)
    finally:
        original.cpu.hook_del(hook)
    assert stop is not None
    result = {
        'stop': f'{stop:08x}',
        'blocks': blocks,
        'strings': strings,
        'consumed': cursor,
        'remaining': len(data) - cursor,
        'round_flag_before_callback': before_callback,
        'round_flag_at_stop': read(original, ROUND_FLAG),
    }
    if before_callback is not None:
        result['round_start_gate'] = []
        for resumed_civ, restart in ((4, 0), (4, 1), (0, 0)):
            original.write(0x5D1570, resumed_civ, '<B')
            original.write(0x5D4FA0, restart, '<I')
            boundary = run(original, 0x4744ED, {0x474516, 0x474574})
            result['round_start_gate'].append({
                'resume_civ': resumed_civ, 'restart': restart,
                'dispatches': boundary == 0x474516,
            })
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    pin = json.loads((root / 'docs/evidence/import-knowledge.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(binary) == pin['binary_sha256']
    path = pin['source']['path']
    save = (home / path).read_bytes()
    assert digest(save) == pin['source']['sha256']
    assert struct.unpack_from('<H', save, 10)[0] == 0x32 and save[678] == 0
    assert struct.unpack_from('<H', save, 660)[0] & 0xc0 == 0
    cities = walk_save_bytes(save)['cities']
    city_end = cities['offset'] + cities['size']
    # Single-player plain-save reads before 0x461d7b: existing 1513-byte
    # post-city prefix, followed by the 0x154-byte read at 0x461bef.
    prefix = 1513 + 0x154
    section = save[city_end + prefix:]
    original = OriginalCity(binary)
    marker = bytes(original.cpu.mem_read(0x5D4758, 4))
    assert section.startswith(marker) and marker == b'EVNT'
    native = trace(binary, section)
    assert native['stop'] == f'{SKIP:08x}'
    assert native['round_flag_before_callback'] == 0
    assert native['round_flag_at_stop'] == 1
    assert [row['dispatches'] for row in native['round_start_gate']] == [False, True, True]
    records = struct.unpack_from('<I', section, 4)[0]
    assert records == 2
    expected = [4, 4] + [RECORD_SIZE] * records
    assert [b['requested'] for b in native['blocks']] == expected
    assert [b['available'] for b in native['blocks']] == expected
    assert native['consumed'] == sum(expected) + sum(s['bytes'] for s in native['strings'])
    assert native['remaining'] == 0
    bypass = trace(binary, b'NONE' + section[4:])
    assert bypass['stop'] == f'{SKIP:08x}' and bypass['consumed'] == 4
    assert bypass['round_flag_before_callback'] is None and bypass['round_flag_at_stop'] == 0
    cases = {'marker_mismatch': bypass, 'truncated_count': trace(binary, section[:6]),
             'truncated_record': trace(binary, section[:8 + RECORD_SIZE - 1]),
             'truncated_string': trace(binary, section[:native['consumed'] - 1])}
    for case, error in [('truncated_count', 0x463048), ('truncated_record', 0x463043),
                        ('truncated_string', 0x462FF8)]:
        assert cases[case]['stop'] == f'{error:08x}'
    report = {
        'schema': 1,
        'binary_sha256': digest(binary),
        'source': {'path': path, 'sha256': digest(save)},
        'post_city_bytes': len(save) - city_end,
        'event_offset_after_cities': prefix,
        'record_count': records,
        'native': native,
        'controlled_cases': cases,
        'substituted_helpers': {f'{addr:08x}': name for addr, name in HELPERS.items()},
        'scope': 'Native outer loader framing, flag write and round-start dispatch. The pre-event prefix, event record meanings and pending choices require separate evidence.',
    }
    args.out.write_text(json.dumps(report, indent=2) + '\n')
    print(f"EVNT at city end + {prefix}, {records} records, "
          f"{native['consumed']} bytes consumed, {native['remaining']} bytes remain")


if __name__ == '__main__':
    main()
