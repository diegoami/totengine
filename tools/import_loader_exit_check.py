#!/usr/bin/env python3
"""Trace two loader regions after the pinned save's successful EVNT read."""

import argparse
import json
import os
from pathlib import Path

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from import_event_tail_check import RECORD_SIZE, ROUND_FLAG, SKIP, digest, trace
from import_phase_check import CIV, FRAME, STRIDE, read, run
from import_unit_setup_check import prepare
from totctl import walk_save_bytes

BEFORE_FINAL_REFRESH = 0x462E0D
MODE_RESTORE = 0x462EC7
EPILOGUE = 0x4630E3
DISPLAY = 0x46D7ED
MULTIPLAYER_MODE = 0x607652
LIVING_MASK = 0x60765A
HUMAN_MASK = 0x60765B
RESUME_CIV = 0x5D1570


def phase(original, player):
    research = CIV + STRIDE * player
    return dict(
        multiplayer_mode=read(original, MULTIPLAYER_MODE, '<B'),
        human_mask=read(original, HUMAN_MASK, '<B'),
        resume_civ=read(original, RESUME_CIV, '<B'),
        round_start_flag=read(original, ROUND_FLAG),
        research_target=read(original, research + 18, '<h'),
        research_progress=read(original, research + 16, '<h'),
    )


def post_event(original, regions, player, records):
    # Prepared loader locals for the stock save path. The 16 sign controls
    # stay nonnegative. Native table reconstruction executes on this context.
    original.write(FRAME - 0xe9c, 0, '<I')
    for offset in (0x1980, 0x197e, *range(0x195e, 0x197a, 2)):
        original.write(FRAME - offset, 0, '<h')
    ranges = regions + [(address, RECORD_SIZE) for address in records]
    before = [bytes(original.cpu.mem_read(address, size)) for address, size in ranges]
    before_phase = phase(original, player)
    calls = []

    def substitute_display(cpu, address, _size, _data):
        if address == DISPLAY:
            esp = cpu.reg_read(UC_X86_REG_ESP)
            target = read(original, esp, '<I')
            calls.append(f'{target - 5:08x}')
            cpu.reg_write(UC_X86_REG_EAX, 0)
            cpu.reg_write(UC_X86_REG_ESP, esp + 4)
            cpu.reg_write(UC_X86_REG_EIP, target)

    cpu = original.cpu
    hook = cpu.hook_add(UC_HOOK_CODE, substitute_display)
    try:
        run(original, SKIP, {BEFORE_FINAL_REFRESH}, calls=(DISPLAY, 0x412DBA))
    finally:
        cpu.hook_del(hook)
    assert calls == ['004622ea', '00462c07']
    assert [bytes(cpu.mem_read(address, size)) for address, size in ranges] == before
    assert phase(original, player) == before_phase
    return {'display_call_sites': calls, 'phase': before_phase,
            'civ_unit_city_event_records_unchanged': True,
            'tiles_and_remembered_improvements_unchanged': True}


def mode_restore(original, player, *, mode, previous_mode, humans, living, status):
    # The pre-load mode is a caller-context value. The status is set by the
    # skipped refresh/file-close region. Neither is extracted from the SAV.
    original.write(FRAME - 0x149c, previous_mode, '<I')
    original.write(FRAME - 0x14a0, status, '<I')
    original.write(MULTIPLAYER_MODE, mode, '<B')
    original.write(HUMAN_MASK, humans, '<B')
    original.write(LIVING_MASK, living, '<B')
    before = phase(original, player)
    run(original, MODE_RESTORE, {EPILOGUE})
    assert original.cpu.reg_read(UC_X86_REG_EAX) == status
    return {'before': before, 'after': phase(original, player), 'pre_load_mode': previous_mode,
            'return_status': status, 'living_mask': living}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    home = Path(os.environ['CIVTOT_HOME'])
    root = Path(__file__).resolve().parent.parent
    pin = json.loads((root / 'docs/evidence/import-input-handoff.json').read_text())
    path = pin['source']['path']
    binary = (home / 'civ2.bak').read_bytes()
    save = (home / path).read_bytes()
    rules = (home / pin['rules']['path']).read_bytes()
    assert digest(binary) == pin['binary_sha256']
    assert digest(save) == pin['source']['sha256']
    assert digest(rules) == pin['rules']['sha256']
    cities = walk_save_bytes(save)['cities']
    tail = save[cities['offset'] + cities['size'] + 1853:]
    original, player, _, regions, _ = prepare(home, binary, path, exhaust=False)
    assert player == 4 and tail.startswith(b'EVNT')
    width, height = (read(original, at, '<h') for at in (0x6705A0, 0x6705A2))
    area = width * height // 2
    original.write(0x6705A4, area, '<h')
    regions.append((0x700000, 6 * area))
    regions.extend((read(original, 0x5DEAD0 + 4 * civ, '<I'), area) for civ in range(1, 8))
    loaded = trace(binary, tail, original=original, record_base=0x7F0000,
                   after_load=lambda o, records: post_event(o, regions, player, records))
    assert loaded['remaining'] == 0 and loaded['consumed'] == 1276
    assert loaded['round_flag_at_stop'] == 1
    after_event = loaded['after_load']
    assert after_event['phase'] == dict(multiplayer_mode=0, human_mask=16, resume_civ=4,
                                       round_start_flag=1, research_target=-1, research_progress=0)
    # The shared event probe also runs separate round-start controls after
    # the callback. Reestablish the pinned resume civ for the suffix trials.
    original.write(RESUME_CIV, player, '<B')
    cases = {}
    for name, mode, previous, humans, living, status, expected in [
        ('same_mode', 0, 0, 16, save[686], 0, (0, 16)),
        ('preserve_existing_human', 0, 1, 16, save[686], 0, (1, 16)),
        ('add_resume_human', 1, 0, 0, 16 | 4, 0, (1, 16)),
        ('first_living_fallback', 1, 0, 0, 4 | 32, 0, (1, 4)),
        ('failed_load_bypasses_mode_restore', 1, 0, 0, 16 | 4, 1, (1, 0)),
    ]:
        case = mode_restore(original, player, mode=mode, previous_mode=previous, humans=humans,
                            living=living, status=status)
        final_mode, final_human = expected
        assert case['after'] == dict(case['before'], multiplayer_mode=final_mode,
                                     human_mask=final_human), (name, case)
        cases[name] = case
    report = {
        'schema': 1,
        'source': {'path': path, 'sha256': digest(save)},
        'rules_sha256': digest(rules),
        'binary_sha256': digest(binary),
        'after_event_read': {'entry': f'{SKIP:08x}', 'stop': f'{BEFORE_FINAL_REFRESH:08x}',
                             'round_flag_written_by_reader': 1, **after_event},
        'mode_restore': {'entry': f'{MODE_RESTORE:08x}', 'stop': f'{EPILOGUE:08x}', 'cases': cases},
        'scope': 'Prepared native EVNT reader and post-read prefix with display substitutes. '
                 'Separate native mode restoration starts after skipped final refresh and close.',
    }
    with args.out.open('x') as stream:
        stream.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
