#!/usr/bin/env python3
"""Measure the native contact prefix for the prepared turn-44 opponent pair.

Load the pinned snapshot and stock Rules, then set the observed engine
pair, treaty words, turn, and single-player viewer. The scenario gate
is clear. Native callees run until return or the first advance grant.
The full control supplies a minimal exception-chain page and handles
SetRect as a memory write. Full controls include the CRT state from
the checked engine checkpoint immediately before the encounter.
"""
import argparse
import json
import os
from pathlib import Path
import struct

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EBP, UC_X86_REG_EBX

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
CRT = 0x5E2590
ENGINE_CONTACT_CRT = 0xD766CE46


def encounter(home, binary, path, rules, turn, known, *, complete=False, crt_seed=None):
    original, viewer, _, regions, _ = prepare(home, binary, path, exhaust=False)
    frame(original)
    original.write(FRAME - 0x1990, 1, '<I')
    original.write(FRAME - 0xE9C, 0, '<I')
    rules_refresh(original, rules)
    assert viewer == 4
    if complete:
        original.cpu.mem_map(0, 4096)
        original.write(0, 0xffffffff, '<I')
    original.write(0x5D0A0C, 0, '<I')
    original.write(0x607652, 0, '<B')
    original.write(0x60765B, 1 << viewer, '<B')
    original.write(0x607648, turn, '<h')
    original.write(0x67A6E0, viewer, '<I')
    if crt_seed is not None:
        original.write(CRT, crt_seed, '<I')
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
    grants = []
    host_calls = []
    tech_masks_before = bytes(original.cpu.mem_read(0x6076D2, 100))
    crt_before = read(original, CRT, '<I')
    crt_draws = []
    masked_techs = [list(before[0][seat * STRIDE + 96:seat * STRIDE + 109]) for seat in PAIR]
    scores = []
    base_value = None

    def watch(cpu, at, size, _):
        nonlocal trade_calls, base_value
        if at == 0x5B11D0:
            crt_draws.append(read(original, CRT, '<I'))
        if at in (0x5265A5, 0x526624):
            base_value = cpu.reg_read(UC_X86_REG_EAX)
        if at in (0x5265B9, 0x526638):
            ebp = cpu.reg_read(UC_X86_REG_EBP)
            recipient = PAIR[int(at == 0x526638)]
            scores.append([recipient, read(original, ebp - 0x18),
                           base_value, cpu.reg_read(UC_X86_REG_EBX)])
        if at == TRADE:
            trade_calls += 1
        if at == GRANT:
            sp = cpu.reg_read(original.sp)
            grants.append(list(struct.unpack('<5i', cpu.mem_read(sp + 4, 20))))
        if at == RETURN or (at == GRANT and not complete):
            cpu.emu_stop()
            return
        instruction = bytes(cpu.mem_read(at, size))
        if instruction.startswith(b'\xff\x15'):
            assert complete and int.from_bytes(instruction[2:6], 'little') == 0x691A84, hex(at)
            sp = cpu.reg_read(original.sp)
            cpu.mem_write(read(original, sp), bytes(cpu.mem_read(sp + 4, 16)))
            cpu.reg_write(UC_X86_REG_EAX, 1)
            cpu.reg_write(original.sp, sp + 20)
            cpu.reg_write(original.ip, at + size)
            host_calls.append('SetRect')

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
    treaty = 0x4405 if complete else (0x4401 if not known else 0x401)
    for offset in offsets:
        struct.pack_into('<I', expected, offset, treaty)
    result = {
        'turn': turn, 'prior_treaty_words': [known, known], 'treaty_words': [treaty, treaty],
        'trade_calls': trade_calls, 'first_grant_arguments': grants[0] if grants else None,
        'stopped_at': f'{stop:08x}', 'explored_tiles': sum(bool(mask & (1 << viewer))
                                                         for mask in after[-2][4::6]),
        'unit_city_map_and_viewer_memory_unchanged': True,
    }
    if complete:
        assert stop == RETURN
        regions_changed = [(i // STRIDE, i % STRIDE, a, b)
                           for i, (a, b) in enumerate(zip(before[0], after[0], strict=True)) if a != b]
        assert {seat for seat, _, _, _ in regions_changed} == set(PAIR)
        assert [read(original, CIV + offset) for offset in offsets] == [treaty, treaty]
        assert len(grants) == 2
        tech_masks_after = bytes(original.cpu.mem_read(0x6076D2, 100))
        result.update(
            grant_arguments=grants, civ_record_byte_changes=regions_changed,
            tech_owner_mask_changes=[(i, a, b) for i, (a, b) in
                                     enumerate(zip(tech_masks_before, tech_masks_after, strict=True))
                                     if a != b],
            prepared_technology_masks=masked_techs, candidate_scores=scores,
            crt_before=crt_before, crt_after=read(original, CRT, '<I'), crt_draws=len(crt_draws),
            host_calls={'SetRect': len(host_calls)},
        )
    else:
        assert after[0] == expected
    return result


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
    complete = encounter(*context, 44, 0, complete=True)
    seeded = encounter(*context, 44, 0, complete=True, crt_seed=ENGINE_CONTACT_CRT)
    assert complete['grant_arguments'] == [[5, 39, 6, 0, 0], [6, 46, 5, 0, 0]]
    report = {'schema': 1, **{key: reference[key] for key in ('source', 'rules', 'binary_sha256')},
              'pair': PAIR, 'contact': CONTACT, 'cases': cases, 'complete': complete,
              'engine_seed_control': seeded, 'scope': __doc__.strip()}
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
