#!/usr/bin/env python3
"""Measure two prepared hidden opponent founding callbacks.

Supply the checked settler, the emptied city slot, and the engine CRT
state in the pinned round-close context. The original founding callback
runs through return. Host directory and rectangle operations and city
name section/row I/O are supplied. Native names stay outside the report.
"""
import argparse
import json
import os
from pathlib import Path

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX

from import_event_tail_check import digest
from import_opponent_combat_view_check import CRT, TILES
from import_phase_check import CIV, STRIDE, read
from import_round_close_check import prepare_round
from import_unit_setup_check import UNIT_RECORDS
from totctl import parse_rules

SEED = 0x3B0CA297


def differences(before, after):
    return [(i, old, new) for i, (old, new) in enumerate(zip(before, after, strict=True)) if old != new]


CASES = {
    5: {'unit_slot': 2, 'city_slot': 9, 'location': (44, 42),
        'site': [130, 1, 0, 2, 36, 87], 'turn': 47, 'crt': 0x3B0CA297,
        'founder_fields': (), 'founder_id': None},
    6: {'unit_slot': 37, 'city_slot': 10, 'location': (27, 17),
        'site': [2, 1, 0, 2, 64, 107], 'turn': 47, 'crt': 0x3B0CA297,
        'founder_fields': ((6, 64), (8, 0), (9, 6), (10, 9), (11, 0),
                           (12, 0), (13, 4), (16, 0), (17, 12)),
        'founder_id': 49},
    2: {'unit_slot': 0, 'city_slot': 11, 'location': (66, 36),
        'site': [2, 1, 0, 2, 4, 45], 'turn': 50, 'crt': 0xB404D22D,
        'founder_fields': ((6, 64), (7, 2), (8, 0), (9, 2), (10, 6), (11, 0),
                           (12, 0), (13, 1), (16, 0), (17, 255), (18, 255)),
        'founder_id': 1},
}


def measure(original, viewer, city_region, rules, city_lines, civ):
    case = CASES[civ]
    first = civ == 5
    unit_slot = case['unit_slot']
    city_slot = case['city_slot']
    location = case['location']
    site = case['site']
    seed = case['crt']
    cpu = original.cpu
    width = read(original, 0x6705A0, '<h')
    area = read(original, 0x6705A4, '<h')
    original.write(0x607648, case['turn'], '<h')
    original.write(0x607655, civ, '<B')
    original.write(0x5D156C, 0, '<B')
    original.write(0x6031B8, 1, '<h')
    original.write(CRT, seed, '<I')

    def tile(x, y):
        return TILES + 6 * (y * (width // 2) + x // 2)

    city = city_region[0] + city_slot * 92
    original.write(0x607668, city_slot, '<h')
    if first:
        original.write(city + 11, 0, '<B')
        original.write(city + 86, 0, '<I')
        original.write(UNIT_RECORDS + 36 * 40 + 30, 0, '<I')
        vacated_city_tile = tile(48, 28) + 1
        original.write(vacated_city_tile, read(original, vacated_city_tile, '<B') & ~3, '<B')
    original.write(CIV + civ * STRIDE + 112, 2, '<h')
    if civ == 2:
        # Mirror the checked engine's route city list: civ 5's and civ 6's
        # turn-47 foundings occupy slots 9 and 10, so the founding creates
        # slot 11, and the native save's own city at the founding site is
        # absent in that route. The records are the engine's staged saves.
        city9 = [
                 44, 0, 42, 0, 0, 0, 0, 0, 2, 0, 5, 1, 5, 0, 0, 0, 0, 0,
                 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 32, 51,
                 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 16, 0,
                 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 11, 0, 0, 0,
                 0, 0
        ]
        city10 = [
                 27, 0, 17, 0, 0, 0, 128, 0, 2, 0, 6, 1, 6, 0, 0, 0, 0, 0,
                 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 32, 51,
                 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 18, 0,
                 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 12, 0, 0, 0,
                 0, 0
        ]
        cpu.mem_write(city_region[0] + 9 * 92, bytes(city9))
        cpu.mem_write(city_region[0] + 10 * 92, bytes(city10))
        cpu.mem_write(city_region[0] + 10 * 92, bytes(city10))

    founder = UNIT_RECORDS + unit_slot * 40
    x, y = (read(original, founder + i, '<h') for i in (0, 2))
    prev, nxt = (read(original, founder + i, '<h') for i in (26, 28))
    if prev >= 0:
        original.write(UNIT_RECORDS + 40 * prev + 28, nxt, '<h')
    if nxt >= 0:
        original.write(UNIT_RECORDS + 40 * nxt + 26, prev, '<h')
    if prev == nxt == -1:
        flag = tile(x, y) + 1
        original.write(flag, read(original, flag, '<B') & ~1, '<B')
    if first:
        original.write(founder + 10, 0, '<B')
        original.write(founder + 13, 7, '<B')
    else:
        # Mirror the checked engine's staged record for this settler,
        # including its home, current order and transient identity.
        for off, value in case['founder_fields']:
            original.write(founder + off, value, '<B')
        if case['founder_id'] is not None:
            original.write(founder + 30, case['founder_id'], '<I')
    if civ == 6:
        # Keep the paid Phalanx after the settler in the unit table, as
        # in the checked engine sequence, to preserve the unit extent.
        phalanx = UNIT_RECORDS + 38 * 40
        cpu.mem_write(phalanx, bytes(40))
        for off, value in ((0, 53), (2, 9), (18, 5), (26, -1), (28, 26)):
            original.write(phalanx + off, value, '<h')
        for off, value in ((8, 3), (9, viewer), (10, 6), (17, 255)):
            original.write(phalanx + off, value, '<B')
        original.write(phalanx + 30, 50, '<I')
        original.write(UNIT_RECORDS + 26 * 40 + 26, 38, '<h')
        original.write(0x607666, 39, '<h')
    for off, value in ((0, location[0]), (2, location[1]), (26, -1), (28, -1)):
        original.write(founder + off, value, '<h')
    cpu.mem_write(tile(*location), bytes(site))

    # Populate the stock leader token used to select the city-name section.
    leader = read(original, CIV + civ * STRIDE + 14, '<h')
    plural = rules['LEADERS']['rows'][leader][5].encode('latin-1')
    cpu.mem_map(0x900000, 4096)
    cpu.mem_write(0x900000, b'?\0' + plural + b'\0')
    original.write(0x5e50e8, 0x900000, '<I')
    original.write(0x606f54 + 60 * leader, 1, '<h')

    root = b'C:\\CIVTOT'
    original_dir = root + b'\\Original'
    directory = root
    cpu.mem_write(0x606a30, root + b'\0')
    cpu.mem_write(0x5f5cf8, original_dir + b'\0')
    section_rows = iter(())
    rows_read = 0
    hosts = []
    caches = [read(original, 0x5DEAD0 + 4 * civ, '<I') for civ in range(1, 8)]

    site_index = (tile(*location) - TILES) // 6
    if not first:
        original.write(caches[civ - 1] + site_index, 1, '<B')
    unit_extent = read(original, 0x607666, '<h')
    assert unit_slot < unit_extent

    def snapshot():
        return (
            bytes(cpu.mem_read(TILES, area * 6)), bytes(cpu.mem_read(CIV, STRIDE * 8)),
            [bytes(cpu.mem_read(address, area)) for address in caches],
        )

    def c_string(address):
        return bytes(cpu.mem_read(address, 260)).split(b'\0', 1)[0]

    def watch(emu, address, size, _):
        nonlocal section_rows, rows_read, directory
        sp = emu.reg_read(original.sp)
        if address in (0x486659, 0x4866dc):
            if address == 0x486659:  # open the selected section
                file = c_string(read(original, sp + 4))
                section = c_string(read(original, sp + 8))
                assert file == b'CITY' and directory == original_dir and section == plural
                line = city_lines.index(b'@' + section.upper())
                section_rows = iter(city_lines[line + 1:])
                result = 0
            else:
                row = next(section_rows)
                rows_read += 1
                assert len(row) < 256
                emu.mem_write(0x62d3f0, row + b'\0')
                original.write(0x628b6c, 0x62d3f0, '<I')
                result = 0x62d3f0
            emu.reg_write(UC_X86_REG_EAX, result)
            emu.reg_write(original.sp, sp + 4)
            emu.reg_write(original.ip, read(original, sp))
            return
        instruction = bytes(emu.mem_read(address, size))
        if not instruction.startswith(b'\xff\x15'):
            return
        target = int.from_bytes(instruction[2:6], 'little')
        if target == 0x691758:
            directory = c_string(read(original, sp))
            assert directory in (root, original_dir)
            name, result, argc = 'SetCurrentDirectoryA', 1, 1
        elif target == 0x69175c:
            assert read(original, sp) > len(directory)
            emu.mem_write(read(original, sp + 4), directory + b'\0')
            name, result, argc = 'GetCurrentDirectoryA', len(directory), 2
        elif target == 0x691764:
            name, result, argc = 'SetEnvironmentVariableA', 1, 2
        elif target == 0x691a84:
            emu.mem_write(read(original, sp), bytes(emu.mem_read(sp + 4, 16)))
            name, result, argc = 'SetRect', 1, 5
        else:
            raise AssertionError(f'unexpected host import {target:08x} at {address:08x}')
        hosts.append(name)
        emu.reg_write(UC_X86_REG_EAX, result)
        emu.reg_write(original.sp, sp + 4 * argc)
        emu.reg_write(original.ip, address + size)

    before_map, before_civs, before_memory = snapshot()
    hook = cpu.hook_add(UC_HOOK_CODE, watch)
    try:
        original.call(0x4a3201, unit_slot, 0xffffffff, 0)
    finally:
        cpu.hook_del(hook)
    after_map, after_civs, after_memory = snapshot()
    assert cpu.reg_read(original.ip) == 0x100000
    assert read(original, CRT, '<I') == seed
    assert [bool(b & (1 << viewer)) for b in before_map[4::6]] == [
        bool(b & (1 << viewer)) for b in after_map[4::6]
    ]
    memory_changes = [(civ, *change)
                      for civ, (before, after) in enumerate(zip(before_memory, after_memory, strict=True), 1)
                      for change in differences(before, after)]
    assert memory_changes == [
        (seat, site_index, int(not first and seat == civ), 2) for seat in range(1, 8)
    ]
    return {
        'turn': case['turn'], 'founder': unit_slot, 'location': location, 'city_slot': city_slot,
        'city_owner': read(original, city + 10, '<B'), 'city_size': read(original, city + 11, '<B'),
        'city_flags': read(original, city + 6), 'native_build': read(original, city + 59, '<b'),
        'native_worked': read(original, city + 50), 'native_city_id': read(original, city + 86),
        'name_sha256': digest(bytes(cpu.mem_read(city + 34, 16)).split(b'\0', 1)[0]),
        'unit_extent': [unit_extent, read(original, 0x607666, '<h')],
        'founder_deleted_coordinates': [read(original, founder + off, '<h') for off in (0, 2)],
        'crt_state': seed, 'explored_tiles': sum(bool(b & (1 << viewer)) for b in after_map[4::6]),
        'map_changes': differences(before_map, after_map), 'memory_changes': memory_changes,
        'civ_record_changes': [(i // STRIDE, i % STRIDE, old, new)
                               for i, old, new in differences(before_civs, after_civs)],
        'name_rows_read': rows_read, 'host_calls': dict(sorted((name, hosts.count(name)) for name in set(hosts))),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent / 'docs/evidence'
    reference = json.loads((root / 'import-round-close.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(binary) == reference['binary_sha256']
    for role in ('source', 'rules'):
        item = reference[role]
        assert digest((home / item['path']).read_bytes()) == item['sha256']
    rules = parse_rules(home / reference['rules']['path'])
    city_file = home / 'Original/City.txt'
    names = city_file.read_bytes()
    cases = {}
    for civ, key in ((5, 'founding'), (6, 'second_founding'), (2, 'third_founding')):
        prepared, result = prepare_round(
            home, binary, reference['source']['path'], rules,
            lambda original, viewer, city: measure(original, viewer, city, rules, names.splitlines(), civ),
        )
        assert prepared['annual_update'] == reference['annual_update']
        assert result['explored_tiles'] == 59
        cases[key] = result
    report = {'schema': 1, **{key: reference[key] for key in ('source', 'rules', 'binary_sha256')},
              'city_names': {'path': 'Original/City.txt', 'sha256': digest(names)},
              **cases, 'scope': __doc__.strip()}
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
