#!/usr/bin/env python3
"""Measure the Turn-49 trade-turn exchange and the following zone refusal.

Reuse both measured foundings so the valuation context matches the
checked engine's city list, then supply the engine's knowledge rows,
treaties, mask table, research rows, caches and CRT for civ 2's eastward
approach to the civ 5 unit at (42, 36). Native movement runs through
return with the closed-panel substitutions; the trade-turn exchange
grants both sides and the mover's next southward order is refused by the
zone of control without spending movement.
"""
import argparse
import json
import os
from pathlib import Path
import struct

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EBP, UC_X86_REG_EBX

from import_event_tail_check import digest
from import_opponent_combat_view_check import CRT, TILES, run_attack
from import_opponent_founding_check import measure as founding
from import_phase_check import CIV, STRIDE, read
from import_round_close_check import prepare_round
from import_unit_setup_check import UNIT_RECORDS
from totctl import parse_rules

CRT_STATE = 0x366E70F4
CRT_AFTER = 0x135A6199
GRANT = 0x49DEDF
TRADE = 0x5263AC
DRAW = 0x5B11D0
MOVER = bytes([39, 0, 35, 0, 0, 0, 64, 0, 16, 2, 0, 32, 0, 0, 49, 0, 0, 255,
               255, 255, 48, 0, 28, 0, 255, 255, 255, 255, 255, 255, 11, 0,
               0, 0, 2, 0, 1, 0, 0, 0])
RESIDENT = bytes([42, 0, 36, 0, 0, 0, 64, 4, 2, 5, 9, 0, 0, 5, 49, 0, 1, 255,
                  0, 0, 23, 0, 47, 0, 255, 255, 255, 255, 255, 255, 15, 0,
                  0, 0, 2, 0, 6, 0, 0, 0])
WORLD_TECHS = {
    0: [0] * 13,
    1: [0] * 13,
    2: [2, 19, 16, 0, 16, 64, 0, 0, 0, 0, 0, 1, 0],
    3: [0] * 13,
    4: [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    5: [2, 19, 0, 0, 144, 192, 0, 0, 0, 0, 0, 1, 0],
    6: [2, 3, 0, 0, 128, 64, 0, 1, 0, 0, 64, 0, 0],
    7: [2, 19, 0, 0, 16, 192, 0, 0, 0, 0, 0, 0, 0],
}
WORLD_ACQUIRED = {0: 0, 1: 0, 2: 7, 3: 0, 4: 1, 5: 8, 6: 8, 7: 6}
WORLD_TREATIES = {(2, 5): 0x405, (5, 2): 0x405, (5, 6): 0x4405, (6, 5): 0x4405,
                  (5, 7): 0x405, (7, 5): 0x405}
TILE_ORIGIN = [5, 1, 0, 2, 36, 32]
TILE_DEST = [2, 0, 160, 2, 36, 85]
TILE_RESIDENT = [129, 1, 160, 2, 36, 86]


def differences(before, after):
    return [[i, old, new] for i, (old, new) in enumerate(zip(before, after, strict=True)) if old != new]


def entry(original, viewer, city_region, rules, city_lines):
    width = read(original, 0x6705A0, '<h')
    area = read(original, 0x6705A4, '<h')
    cpu = original.cpu

    def tile(x, y):
        return TILES + 6 * (y * (width // 2) + x // 2)

    def ROW(seat):
        return CIV + STRIDE * seat

    for civ in (5, 6):
        if civ != 5:  # the founding mapped its token page for the earlier call
            cpu.mem_unmap(0x900000, 4096)
        founded = founding(original, viewer, city_region, rules, city_lines, civ)
        assert founded['explored_tiles'] == 59

    original.write(0x607648, 49, '<h')
    original.write(0x607655, 2, '<B')
    original.write(0x5D156C, 0, '<B')
    original.write(CRT, CRT_STATE, '<I')
    caches = [read(original, 0x5DEAD0 + 4 * civ, '<I') for civ in range(1, 8)]

    # The engine's research rows, knowledge, treaties and caches.
    for seat, progress, target, acquired, techs in (
        (2, 18, 54, 7, WORLD_TECHS[2]), (5, 37, 65, 8, WORLD_TECHS[5]),
    ):
        base = ROW(seat)
        original.write(base + 16, progress, '<h')
        original.write(base + 18, target, '<h')
        original.write(base + 24, acquired, '<B')
        original.write(base + 25, 0, '<B')
        original.write(base + 26, 0, '<B')
        for i, value in enumerate(techs):
            original.write(base + 96 + i, value, '<B')
        original.write(base + 114, 0, '<h')
        original.write(base + 38, 0, '<B')
        cache = bytes(256)
        cache = cache[:2] + bytes([2 if seat == 2 else 3]) + cache[3:]
        cpu.mem_write(base + 0x5D0, cache)
    for seat, techs in WORLD_TECHS.items():
        base = ROW(seat)
        for i, value in enumerate(techs):
            original.write(base + 96 + i, value, '<B')
        original.write(base + 24, WORLD_ACQUIRED[seat], '<B')
    for (a, b), word in WORLD_TREATIES.items():
        original.write(ROW(a) + 40 + 4 * b, word, '<I')
    for advance in range(100):
        mask = 0
        for seat, techs in WORLD_TECHS.items():
            mask |= ((techs[advance // 8] >> (advance % 8)) & 1) << seat
        original.write(0x6076D2 + advance, mask, '<B')
    for slot, size in ((1, 3), (3, 2), (5, 3), (7, 1)):
        original.write(city_region[0] + 92 * slot + 11, size, '<B')

    cpu.mem_write(UNIT_RECORDS + 40 * 3, MOVER)
    cpu.mem_write(UNIT_RECORDS + 40 * 10, RESIDENT)
    cpu.mem_write(tile(39, 35), bytes(TILE_ORIGIN))
    cpu.mem_write(tile(41, 35), bytes(TILE_DEST))
    cpu.mem_write(tile(42, 36), bytes(TILE_RESIDENT))

    def knowledge():
        tiles = cpu.mem_read(TILES, area * 6)
        return (bytes(b & (1 << viewer) for b in tiles[4::6]),
                [bytes(cpu.mem_read(c, area)) for c in caches])

    def unit(slot):
        address = UNIT_RECORDS + 40 * slot
        return {
            'slot': slot,
            'position': [read(original, address + off, '<h') for off in (0, 2)],
            'previous': read(original, address + 26, '<h'),
            'next': read(original, address + 28, '<h'),
            'spent': read(original, address + 10, '<B'),
            'direction': read(original, address + 13, '<B'),
            'visibility': read(original, address + 11, '<B'),
        }

    def snapshot():
        return (bytes(cpu.mem_read(TILES, area * 6)),
                [bytes(cpu.mem_read(c, area)) for c in caches],
                bytes(cpu.mem_read(CIV, STRIDE * 8)),
                bytes(cpu.mem_read(0x6076D2, 100)),
                bytes(cpu.mem_read(city_region[0], 92 * 11)))

    def ROW(seat):
        return CIV + STRIDE * seat

    before_units = [unit(3), unit(10)]
    visible, memory = knowledge()
    before = snapshot()
    raw_before = {slot: bytes(cpu.mem_read(UNIT_RECORDS + 40 * slot, 40)) for slot in (3, 10)}
    events = {'trades': 0, 'grants': [], 'draws': [], 'scores': [], 'bases': []}

    def watch(emu, at, _size, _data):
        if at == DRAW:
            events['draws'].append(read(original, CRT, '<I'))
        elif at == TRADE:
            events['trades'] += 1
        elif at == GRANT:
            sp = emu.reg_read(original.sp)
            events['grants'].append(list(struct.unpack('<5i', cpu.mem_read(sp + 4, 20))))
        elif at in (0x5265A5, 0x526624):
            events['bases'].append(emu.reg_read(UC_X86_REG_EAX))
        elif at in (0x5265B9, 0x526638):
            ebp = emu.reg_read(UC_X86_REG_EBP)
            recipient = (2, 5)[int(at == 0x526638)]
            events['scores'].append([recipient, read(original, ebp - 0x18),
                                     events['bases'][-1] if events['bases'] else None,
                                     emu.reg_read(UC_X86_REG_EBX)])

    hook = cpu.hook_add(UC_HOOK_CODE, watch)
    try:
        result, hosts, display, observations = run_attack(original, (3, 1, 3), memory_rects=True)
    finally:
        cpu.hook_del(hook)

    after_units = [unit(3), unit(10)]
    raw_after = {slot: bytes(cpu.mem_read(UNIT_RECORDS + 40 * slot, 40)) for slot in (3, 10)}
    mover_changes = differences(raw_before[3], raw_after[3])
    resident_changes = differences(raw_before[10], raw_after[10])
    assert result == 1 and observations == [[3, 1]], (result, observations, after_units)
    assert mover_changes == [[0, 39, 41], [10, 0, 3], [13, 0, 1]], mover_changes
    assert resident_changes == [[11, 0, 4]], resident_changes
    assert after_units == [
        {'slot': 3, 'position': [41, 35], 'previous': -1, 'next': -1,
         'spent': 3, 'direction': 1, 'visibility': 32},
        {'slot': 10, 'position': [42, 36], 'previous': -1, 'next': -1,
         'spent': 9, 'direction': 5, 'visibility': 4},
    ], after_units
    after_tiles, after_caches, after_civs, after_masks, after_city_table = snapshot()
    tile_changes = differences(before[0], after_tiles)
    assert tile_changes == [
        [tile(42, 34) - TILES + 4, 32, 36],
        [tile(39, 35) - TILES + 1, 1, 0],
        [tile(41, 35) - TILES + 1, 0, 1],
        [tile(41, 35) - TILES + 5, 85, 37],
    ], tile_changes
    cache_changes = [(seat, *change)
                     for seat, (old, new) in enumerate(zip(memory, after_caches, strict=True), 1)
                     for change in differences(old, new)]
    assert cache_changes == [
        (2, (tile(41, 35) - TILES) // 6, 0, 1),
        (2, (tile(42, 36) - TILES) // 6, 0, 1),
        (2, (tile(41, 37) - TILES) // 6, 0, 1),
    ], cache_changes
    civ_changes = [[seat // STRIDE, seat % STRIDE, old, new]
                   for seat, (old, new) in enumerate(zip(before[2], after_civs, strict=True)) if old != new]
    row2_changes = [change for change in civ_changes if change[0] == 2]
    row5_changes = [change for change in civ_changes if change[0] == 5]
    assert [change[1:] for change in row2_changes] == [
        [24, 7, 8], [100, 16, 144], [163, 255, 5]], row2_changes
    assert [change[1:] for change in row5_changes[:3]] == [
        [24, 8, 9], [98, 0, 16], [144, 255, 2]], row5_changes
    # Civ 5's remaining row changes live past 2048: the runtime scratch the
    # saved model does not carry.
    assert all(change[1] >= 2048 for change in row5_changes[3:]), row5_changes
    assert struct.unpack_from('<I', after_civs, 2 * STRIDE + 40 + 4 * 5)[0] == 0x405
    assert struct.unpack_from('<I', after_civs, 5 * STRIDE + 40 + 4 * 6)[0] == 0x4405
    assert after_civs[2 * STRIDE + 24] == 8 and after_civs[5 * STRIDE + 24] == 9
    assert after_civs[2 * STRIDE + 96 + 39 // 8] & (1 << (39 % 8))
    assert after_civs[5 * STRIDE + 96 + 20 // 8] & (1 << (20 % 8))
    mask_changes = differences(before[3], after_masks)
    assert mask_changes == [[20, 4, 36], [39, 96, 100]], mask_changes
    assert differences(before[4], after_city_table) == []
    assert read(original, CRT, '<I') == CRT_AFTER
    assert events['trades'] == 1
    assert events['grants'] == [[2, 39, 5, 0, 0], [5, 20, 2, 0, 0]]
    assert len(events['draws']) == 3 and events['draws'][0] == CRT_STATE
    assert events['scores'] == [[5, 20, 5, 5], [2, 39, 6, 7], [2, 47, 5, 6]]
    after_visible, after_memory = knowledge()
    assert visible == after_visible and sum(bool(b) for b in visible) == 59
    assert [seat for seat, (old, new) in
            enumerate(zip(memory, after_caches, strict=True), 1) if old != new] == [2]

    # The mover's next southward order is refused by the zone of control.
    assert read(original, UNIT_RECORDS + 40 * 3 + 16, '<B') == 0
    crt_before2 = read(original, CRT, '<I')
    tiles3_before = bytes(cpu.mem_read(TILES, area * 6))
    result2, _hosts2, _display2, observations2 = run_attack(original, (3, 3, 3), memory_rects=True)
    raw3_after2 = bytes(cpu.mem_read(UNIT_RECORDS + 40 * 3, 40))
    tiles3_after = bytes(cpu.mem_read(TILES, area * 6))
    assert result2 == 0 and observations2 == []
    assert differences(raw_after[3], raw3_after2) == [[16, 0, 1]]
    assert tiles3_before == tiles3_after
    assert read(original, CRT, '<I') == crt_before2 == CRT_AFTER
    visible3 = bytes(b & (1 << viewer) for b in tiles3_after[4::6])
    assert sum(bool(b) for b in visible3) == 59
    return {
        'arguments': [3, 1, 3], 'return': result, 'crt_state': CRT_STATE,
        'crt_after_exchange': CRT_AFTER,
        'founding_setup': {'city_slots': [9, 10], 'explored_tiles': 59},
        'before': before_units, 'after': after_units,
        'mover_raw_changes': mover_changes, 'resident_raw_changes': resident_changes,
        'tile_changes': tile_changes, 'cache_changes': cache_changes,
        'civ_row_changes': {'2': row2_changes, '5': row5_changes},
        'mask_table_changes': mask_changes,
        'trade_calls': events['trades'], 'grants': events['grants'],
        'crt_draws': len(events['draws']), 'draw_values': events['draws'],
        'candidate_scores': events['scores'],
        'leader_modifiers_live_zero': True,
        'explored_tiles': 59, 'viewer_memory_unchanged': True,
        'observation_calls': observations, 'host_substitutes': hosts,
        'display_substitutes': display,
        'zone_refusal': {
            'arguments': [3, 3, 3], 'return': result2,
            'mover_raw_changes': [[16, 0, 1]], 'tile_changes': [],
            'crt_unchanged': True, 'explored_tiles': 59,
            'observation_calls': observations2,
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent / 'docs/evidence'
    contact = json.loads((root / 'import-opponent-contact.json').read_text())
    annual = json.loads((root / 'import-round-close.json').read_text())
    founding_evidence = json.loads((root / 'import-opponent-founding.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(binary) == contact['binary_sha256'] == annual['binary_sha256']
    for role in ('source', 'rules'):
        item = contact[role]
        assert item == annual[role] and digest((home / item['path']).read_bytes()) == item['sha256']
    rules = parse_rules(home / contact['rules']['path'])
    city_lines = (home / 'Original/City.txt').read_bytes().splitlines()
    prepared, measured = prepare_round(
        home, binary, contact['source']['path'], rules,
        lambda original, viewer, city: entry(original, viewer, city, rules, city_lines),
    )
    assert prepared['annual_update'] == annual['annual_update']
    assert measured['explored_tiles'] == 59
    with args.out.open('x') as output:
        out = {'schema': 1,
               **{key: contact[key] for key in ('source', 'rules', 'binary_sha256')},
               'founding_evidence': founding_evidence['binary_sha256'],
               'trade_contact': measured, 'scope': __doc__.strip()}
        output.write(json.dumps(out, indent=2) + '\n')


if __name__ == '__main__':
    main()
