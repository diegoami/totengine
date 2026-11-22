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
PREFIX_ENTRY = 0x461748
PREFIX_HELPERS = {
    0x45ED54: 'skip post-read refresh helper',
    0x46D7ED: 'skip presentation helper',
    0x46D730: 'skip calendar helper',
    0x45F0F8: 'select the single-player continuation',
    0x47DD64: 'skip initialization helper',
}
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


def trace_prefix(binary, save, tail):
    original = OriginalCity(binary)
    frame(original)
    original.cpu.mem_write(0x607638, save[652:1250])
    original.write(FRAME - 0x1990, 1, '<I')
    original.write(FRAME - 0xFA8, 0x32, '<I')
    original.write(FRAME - 0xE9C, 0, '<I')
    original.write(FRAME - 0xD8C, 0, '<I')
    cursor = 0
    reads = {}
    stop = None
    skipped = []

    def watch(cpu, addr, _size, _data):
        nonlocal cursor, stop
        if PREFIX_ENTRY <= addr < ENTRY:
            return
        if addr != 0x5B3810 and addr not in PREFIX_HELPERS:
            stop = addr
            cpu.emu_stop()
            return
        esp = cpu.reg_read(UC_X86_REG_ESP)
        ret = read(original, esp, '<I')
        value = 0
        if addr == 0x5B3810:
            dst, width, count, handle = struct.unpack('<4I', cpu.mem_read(esp + 4, 16))
            assert handle == 1 and count == 1
            payload = tail[cursor:cursor + width]
            site = f'{ret - 5:08x}'
            group = reads.setdefault(site, {'width': width, 'calls': 0, 'bytes_read': 0})
            assert group['width'] == width
            group['calls'] += 1
            group['bytes_read'] += len(payload)
            if payload:
                cpu.mem_write(dst, payload)
            cursor += len(payload)
            value = int(len(payload) == width)
        else:
            skipped.append(f'{addr:08x}')
        cpu.reg_write(UC_X86_REG_EAX, value)
        cpu.reg_write(UC_X86_REG_ESP, esp + 4)
        cpu.reg_write(UC_X86_REG_EIP, ret)

    hook = original.cpu.hook_add(UC_HOOK_CODE, watch)
    try:
        original.cpu.emu_start(PREFIX_ENTRY, 0, count=50000)
    finally:
        original.cpu.hook_del(hook)
    assert stop is not None
    return {'stop': f'{stop:08x}', 'consumed': cursor, 'reads': reads, 'skipped': skipped}


def trace(binary, data, *, include_record_words=False, after_load=None,
          original=None, record_base=FIRST_RECORD):
    if original is None:
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
            value = record_base + len(records) * 0x200
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
        if include_record_words:
            result['action_and_modifier_words'] = [
                [read(original, record + offset, '<I') for offset in (4, 8)]
                for record in records
            ]
        if after_load is not None:
            result['after_load'] = after_load(original, records)
        # Map each witness pointer to the slot where native code stored it.
        # The witness addresses are private to this trace.
        for ordinal, entry in enumerate(strings, start=1):
            pointer = 0x710000 + ordinal * 0x100
            matches = [
                (record_index, offset)
                for record_index, record in enumerate(records)
                for offset in range(0, RECORD_SIZE, 4)
                if read(original, record + offset, '<I') == pointer
            ]
            assert len(matches) == 1
            entry['record'], entry['slot'] = matches[0]
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


def compare_victory_strings(section, strings, events):
    """Compare the known stock block directly without retaining its text."""
    lines = events.splitlines()
    commands = [line.strip().upper() for line in lines]
    assert commands.count(b'@IF') == 1
    assert commands.count(b'TEXT') == commands.count(b'ENDTEXT') == 1
    first, last = commands.index(b'TEXT'), commands.index(b'ENDTEXT')
    assert first < last
    text_lines = [line.rstrip(b' \t') for line in lines[first + 1:last]]
    receivers = [line.split(b'=', 1)[1].strip() for line in lines
                 if line.strip().lower().startswith(b'receiver=')]
    assert len(receivers) == 1
    assert commands.count(b'PLAYAVIFILE') == 1
    video = lines[commands.index(b'PLAYAVIFILE') + 1].strip()

    expected = {(0, 0x18): receivers[0]}
    for record in (0, 1):
        for index, line in enumerate(text_lines):
            expected[(record, 0x34 + index * 4)] = line
        expected[(record, 0x84)] = video
    actual = {
        (entry['record'], entry['slot']):
            section[entry['offset']:entry['offset'] + entry['bytes'] - 1]
        for entry in strings
    }
    assert len(actual) == len(strings)
    assert actual.keys() == expected.keys(), 'saved event string slots differ'
    for slot, value in expected.items():
        assert actual[slot] == value, f'saved event string differs at slot {slot}'
    return {
        'receiver_strings_matched': 1,
        'text_lines_matched_per_record': [len(text_lines), len(text_lines)],
        'video_strings_matched': 2,
        'total_strings_matched': len(actual),
        'text_line_normalization': 'strip trailing ASCII spaces and tabs from Events.txt lines',
    }


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
    tail = save[city_end:]
    prefix_trace = trace_prefix(binary, save, tail)
    assert prefix_trace['stop'] == f'{ENTRY:08x}'
    prefix = prefix_trace['consumed']
    assert prefix == 1513 + 0x154
    assert prefix_trace['reads']['00461bef'] == {'width': 340, 'calls': 1, 'bytes_read': 340}
    truncated_prefix = trace_prefix(binary, save, tail[:prefix - 1])
    assert truncated_prefix['stop'] == '00463052'
    section = tail[prefix:]
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
    # These fields have independent decoder evidence in the victory probe.
    # Hash the installed event text and compare payloads without emitting them.
    victory = json.loads((root / 'docs/evidence/original-victory.json').read_text())
    events = (home / 'Original/Events.txt').read_bytes()
    events_sha = digest(events)
    assert events_sha == victory['events_sha256']
    first, second = [8 + i * RECORD_SIZE for i in range(records)]
    fields = {
        'first_header_words': list(struct.unpack_from('<III', section, first)),
        'second_header_words': list(struct.unpack_from('<III', section, second)),
        'first_receiver': struct.unpack_from('<i', section, first + 0x20)[0],
        'first_futuretech': section[first + 0x30],
        'second_denominator': struct.unpack_from('<h', section, second + 0x2C)[0],
    }
    assert fields == {
        'first_header_words': [0x100, 0x5001, 0x10000108],
        'second_header_words': [0x40, 0x5001, 0x40000008],
        'first_receiver': -2, 'first_futuretech': 5, 'second_denominator': 3,
    }
    bypass = trace(binary, b'NONE' + section[4:])
    assert bypass['stop'] == f'{SKIP:08x}' and bypass['consumed'] == 4
    assert bypass['round_flag_before_callback'] is None and bypass['round_flag_at_stop'] == 0
    cases = {'marker_mismatch': bypass, 'truncated_count': trace(binary, section[:6]),
             'truncated_record': trace(binary, section[:8 + RECORD_SIZE - 1]),
             'truncated_string': trace(binary, section[:native['consumed'] - 1])}
    for case, error in [('truncated_count', 0x463048), ('truncated_record', 0x463043),
                        ('truncated_string', 0x462FF8)]:
        assert cases[case]['stop'] == f'{error:08x}'
    matched = compare_victory_strings(section, native['strings'], events)
    altered = bytearray(section)
    text_start = next(s['offset'] for s in native['strings']
                      if (s['record'], s['slot']) == (0, 0x34))
    altered[text_start] = ord('x') if altered[text_start] != ord('x') else ord('y')
    try:
        compare_victory_strings(altered, native['strings'], events)
    except AssertionError as error:
        assert str(error) == 'saved event string differs at slot (0, 52)'
    else:
        raise AssertionError('same-length TEXT change was accepted')
    matched['rejects_same_length_text_change'] = True
    report = {
        'schema': 3,
        'binary_sha256': digest(binary),
        'source': {'path': path, 'sha256': digest(save)},
        'post_city_bytes': len(save) - city_end,
        'event_offset_after_cities': prefix,
        'prefix_trace': prefix_trace,
        'truncated_prefix': truncated_prefix,
        'prefix_substituted_helpers': {f'{addr:08x}': name for addr, name in PREFIX_HELPERS.items()},
        'record_count': records,
        'events_txt_sha256': events_sha,
        'victory_fields': fields,
        'victory_string_comparison': matched,
        'native': native,
        'controlled_cases': cases,
        'substituted_helpers': {f'{addr:08x}': name for addr, name in HELPERS.items()},
        'scope': 'Native reader extents, flag write, round-start dispatch and stock victory string comparison. Remaining prefix and event fields and pending choices require separate evidence.',
    }
    args.out.write_text(json.dumps(report, indent=2) + '\n')
    print(f"EVNT at city end + {prefix}, {records} records, "
          f"{native['consumed']} bytes consumed, {native['remaining']} bytes remain")


if __name__ == '__main__':
    main()
