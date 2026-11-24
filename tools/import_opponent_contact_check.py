#!/usr/bin/env python3
"""Measure the native contact prefix for the prepared turn-44 opponent pair.

Load the pinned snapshot and stock Rules, then set the observed engine
pair, treaty words, turn, and single-player viewer. The scenario gate
is clear. Native callees run until return or the first advance grant.
The later grant and remaining encounter logic stay outside this probe.
"""
import argparse
import json
import os
from pathlib import Path
import struct

from unicorn import UC_HOOK_CODE

from import_event_tail_check import digest
from import_loader_exit_check import rules_refresh
from import_phase_check import CIV, FRAME, STRIDE, frame, read
from import_unit_setup_check import prepare
from totctl import parse_rules

ENTRY = 0x526ACA
RETURN = 0x100000
TRADE = 0x5263AC
GRANT = 0x49DEDF
PAIR = (5, 6)
CONTACT = (48, 28, 0)


def encounter(home, binary, path, rules, turn, known):
    original, viewer, _, regions, _ = prepare(home, binary, path, exhaust=False)
    frame(original)
    original.write(FRAME - 0x1990, 1, '<I')
    original.write(FRAME - 0xE9C, 0, '<I')
    rules_refresh(original, rules)
    assert viewer == 4
    original.write(0x5D0A0C, 0, '<I')
    original.write(0x607652, 0, '<B')
    original.write(0x60765B, 1 << viewer, '<B')
    original.write(0x607648, turn, '<h')
    original.write(0x67A6E0, viewer, '<I')
    offsets = [a * STRIDE + 40 + 4 * b for a, b in (PAIR, PAIR[::-1])]
    for offset in offsets:
        original.write(CIV + offset, known, '<I')
    width = read(original, 0x6705A0, '<h')
    height = read(original, 0x6705A2, '<h')
    area = width * height // 2
    memory = read(original, 0x5DEAD0 + 4 * viewer, '<I')
    regions.extend([(0x700000, 6 * area), (memory, area)])
    original.cpu.mem_write(0x108000, struct.pack('<6i', RETURN, *PAIR, *CONTACT))
    original.cpu.reg_write(original.sp, 0x108000)
    before = [bytes(original.cpu.mem_read(address, length)) for address, length in regions]
    trade_calls = 0
    grant = None

    def watch(cpu, at, size, _):
        nonlocal trade_calls, grant
        if at == TRADE:
            trade_calls += 1
        if at in (RETURN, GRANT):
            if at == GRANT:
                sp = cpu.reg_read(original.sp)
                grant = list(struct.unpack('<5i', cpu.mem_read(sp + 4, 20)))
            cpu.emu_stop()
        assert bytes(cpu.mem_read(at, min(size, 2))) != b'\xff\x15', f'host call at {at:x}'

    hook = original.cpu.hook_add(UC_HOOK_CODE, watch)
    try:
        original.cpu.emu_start(ENTRY, 0, count=300000)
    finally:
        original.cpu.hook_del(hook)
    stop = original.cpu.reg_read(original.ip)
    assert stop in (RETURN, GRANT), hex(stop)
    after = [bytes(original.cpu.mem_read(address, length)) for address, length in regions]
    assert after[1:] == before[1:]
    expected = bytearray(before[0])
    treaty = (0x4401 if not known else 0x401)
    for offset in offsets:
        struct.pack_into('<I', expected, offset, treaty)
    assert after[0] == expected
    return {
        'turn': turn, 'prior_treaty_words': [known, known], 'treaty_words': [treaty, treaty],
        'trade_calls': trade_calls, 'first_grant_arguments': grant,
        'stopped_at': f'{stop:08x}', 'explored_tiles': sum(bool(mask & (1 << viewer))
                                                         for mask in after[-2][4::6]),
        'unit_city_map_and_viewer_memory_unchanged': True,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    reference = json.loads((root / 'docs/evidence/import-human-city-stack-returns.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(binary) == reference['binary_sha256']
    for role in ('source', 'rules'):
        item = reference[role]
        assert digest((home / item['path']).read_bytes()) == item['sha256']
    rules = parse_rules(home / reference['rules']['path'])
    context = (home, binary, reference['source']['path'], rules)
    cases = {
        'first_contact': encounter(*context, 44, 0),
        'known_between_trade_turns': encounter(*context, 44, 1),
        'known_on_trade_turn': encounter(*context, 45, 1),
    }
    assert [case['trade_calls'] for case in cases.values()] == [1, 0, 1]
    assert cases['first_contact']['first_grant_arguments'] == [5, 39, 6, 0, 0]
    assert all(case['explored_tiles'] == 59 for case in cases.values())
    report = {'schema': 1, **{key: reference[key] for key in ('source', 'rules', 'binary_sha256')},
              'pair': PAIR, 'contact': CONTACT, 'cases': cases, 'scope': __doc__.strip()}
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
