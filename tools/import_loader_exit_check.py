#!/usr/bin/env python3
"""Trace the loader from the pinned save's successful EVNT read to its epilogue."""

import argparse
import json
import os
from pathlib import Path

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from import_event_tail_check import RECORD_SIZE, ROUND_FLAG, SKIP, digest, trace
from import_phase_check import CIV, FRAME, STRIDE, frame, read, run
from import_unit_setup_check import prepare
from totctl import parse_rules, walk_save_bytes

BEFORE_FINAL_REFRESH = 0x462E0D
MODE_RESTORE = 0x462EC7
EPILOGUE = 0x4630E3
CALLER_ENTRY = 0x463E69
CALLER_SUCCESS = 0x463F54
CALLER_FAILURE = 0x463E96
DISPLAY = 0x46D7ED
MULTIPLAYER_MODE = 0x607652
LIVING_MASK = 0x60765A
HUMAN_MASK = 0x60765B
RESUME_CIV = 0x5D1570
SECTION = 0x486659
ROW = 0x4866DC
CLOSE = 0x5B1950
REFRESH_CALLS = (
    0x412F54, 0x413358, 0x4134FC, 0x486300, SECTION, ROW, CLOSE,
    0x48683C, 0x4868BF, 0x486960, 0x490440, 0x4AC61E, 0x4AC590,
    0x52C190, 0x5B1A40, 0x5B4050, 0x5B1220, 0x5B3FD0, 0x5B3EB0,
    0x5B1630, 0x4129FD, 0x566A30, 0x402A10, 0x402A30,
)


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


def rules_refresh(original, rules):
    """Feed pinned sections/rows; native field parsing and writes still run."""
    expected = {'CIVILIZE': 100, 'IMPROVE': 68, 'ENDWONDER': 28, 'UNITS': 80}
    section = None
    counts = {}
    closed = []

    def supply_io(cpu, address, _size, _data):
        nonlocal section
        if address not in (SECTION, ROW, CLOSE):
            return
        esp = cpu.reg_read(UC_X86_REG_ESP)
        ret = read(original, esp, '<I')
        result = 0
        if address == SECTION:
            name = read(original, esp + 8, '<I')
            section = bytes(cpu.mem_read(name, 16)).split(b'\0')[0].decode('ascii')
            assert section in expected and section not in counts
            counts[section] = 0
        elif address == ROW:
            index = counts[section]
            assert index < expected[section]
            row = ','.join(rules[section]['rows'][index]).encode('latin1')
            assert len(row) < 256
            cpu.mem_write(0x62D3F0, row + b'\0')
            original.write(0x628B6C, 0x62D3F0, '<I')
            counts[section] += 1
            result = 0x62D3F0
        else:
            assert ret == 0x462EC4 and read(original, esp + 4, '<I') == 1
            closed.append(f'{ret - 5:08x}')
        cpu.reg_write(UC_X86_REG_EAX, result)
        cpu.reg_write(UC_X86_REG_ESP, esp + 4)
        cpu.reg_write(UC_X86_REG_EIP, ret)

    original.write(FRAME - 0x149c, 0, '<I')  # pre-load single-player mode
    cpu = original.cpu
    hook = cpu.hook_add(UC_HOOK_CODE, supply_io)
    try:
        run(original, BEFORE_FINAL_REFRESH, {EPILOGUE},
            calls=REFRESH_CALLS, instruction_limit=2000000)
    finally:
        cpu.hook_del(hook)
    assert counts == expected and closed == ['00462ebf']
    assert original.cpu.reg_read(UC_X86_REG_EAX) == 0
    assert read(original, 0x60766A, '<H') == 89
    return {'entry': f'{BEFORE_FINAL_REFRESH:08x}', 'stop': f'{EPILOGUE:08x}',
            'rows_by_section': counts, 'enabled_advances': 89, 'close_call_sites': closed,
            'substituted_helpers': [f'{addr:08x}' for addr in (SECTION, ROW, CLOSE)],
            'return_status': 0}


def post_event(original, regions, player, records, rules):
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
    refresh = rules_refresh(original, rules)
    assert [bytes(cpu.mem_read(address, size)) for address, size in ranges] == before
    assert phase(original, player) == before_phase
    return {'display_call_sites': calls, 'phase': before_phase,
            'civ_unit_city_event_records_unchanged': True,
            'tiles_and_remembered_improvements_unchanged': True,
            'final_refresh': refresh}


def mode_restore(original, player, *, mode, previous_mode, humans, living, status):
    # The pre-load mode is a caller-context value. The status is set by the
    # refresh/file-close region. Neither control is extracted from the SAV.
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


def caller_options(original, saved, configured, status):
    # At this entry the loader has returned, leaving its argument at ESP.
    # The caller overlays process preferences before even branching on status.
    frame(original)
    original.write(0x60763A, saved, '<I')
    original.write(0x5F5E0E, configured, '<I')
    original.cpu.reg_write(UC_X86_REG_EAX, status)
    stop = run(original, CALLER_ENTRY, {CALLER_SUCCESS, CALLER_FAILURE})
    merged = read(original, 0x60763A, '<I')
    assert merged == ((saved & 0xc000) | (configured & ~0xc000))
    assert stop == (CALLER_FAILURE if status else CALLER_SUCCESS)
    result = {'saved': f'{saved:08x}', 'configured': f'{configured:08x}',
              'merged': f'{merged:08x}', 'stop': f'{stop:08x}', 'return_status': status}
    if status == 0:
        # Drive only the advice gate in human preparation, with this same
        # options word. Stop before processing any advisory state.
        branch = run(original, 0x473BBE, {0x473BE0, 0x473E6B})
        assert branch == (0x473BE0 if merged & 0x100 else 0x473E6B)
        result['human_advice_gate'] = f'{branch:08x}'
    return result


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
    rules_path = home / pin['rules']['path']
    rules = rules_path.read_bytes()
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
    parsed_rules = parse_rules(rules_path)
    loaded = trace(binary, tail, original=original, record_base=0x7F0000,
                   after_load=lambda o, records: post_event(o, regions, player, records, parsed_rules))
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
    saved_options = int.from_bytes(save[654:658], 'little')
    caller = {
        'entry': f'{CALLER_ENTRY:08x}',
        'retained_saved_bits': '0000c000',
        'cases': {
            'same_preferences': caller_options(original, saved_options, saved_options, 0),
            'different_advice_bit': caller_options(original, saved_options,
                                                  saved_options ^ 0x100, 0),
            'saved_bits_win': caller_options(original, saved_options | 0x4000,
                                             saved_options | 0x8000, 0),
            'failed_load': caller_options(original, saved_options, 0, 1),
        },
    }
    report = {
        'schema': 2,
        'source': {'path': path, 'sha256': digest(save)},
        'rules_sha256': digest(rules),
        'binary_sha256': digest(binary),
        'after_event_read': {'entry': f'{SKIP:08x}', 'stop': f'{BEFORE_FINAL_REFRESH:08x}',
                             'round_flag_written_by_reader': 1, **after_event},
        'mode_restore': {'entry': f'{MODE_RESTORE:08x}', 'stop': f'{EPILOGUE:08x}', 'cases': cases},
        'caller_options': caller,
        'scope': 'Prepared native EVNT reader through return with display, Rules section/row, '
                 'and file-close substitutes. Separate mode and caller-preference controls.',
    }
    with args.out.open('x') as stream:
        stream.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
