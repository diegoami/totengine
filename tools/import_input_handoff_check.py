#!/usr/bin/env python3
"""Measure native observation and input handoff during a prepared resume.

Observation executes through the tile and remembered-state update loops and
stops before notification processing. A separate control substitutes the
whole observation call to reach the first input-loop preparation call.
The report describes every invoked substitute and emits only numeric results.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from import_phase_check import CIV, STRIDE, read, run
from import_unit_setup_check import ACTIVE_SLOT, CALLS, ENTRY, prepare

OBSERVE = 0x41F704
AFTER_OBSERVATION = 0x4205A7
INPUT_PREPARATION = 0x45ABC1
DISPLAY_HELPERS = (0x5346BE, 0x40D7E7, 0x4BF9B6, 0x41AF00, 0x40D510, 0x45A4B0)
OBSERVATION_HELPERS = (
    OBSERVE, 0x57012C, 0x5700FF, 0x56FFF0, 0x571972, 0x566AE2, 0x57008F,
    0x571819, 0x5711C9, 0x42FE9B, 0x42FA90, 0x56AEBE, 0x4796D0, 0x5691ED,
    0x57042E, 0x41F63D, 0x41F5E4, 0x56B64B, 0x570599, 0x5702CD, 0x570F15,
    0x56909A, 0x568860, 0x57054F,
)


def witness(home, binary, path, *, exhaust, observation):
    original, player, _, regions, _ = prepare(home, binary, path, exhaust=exhaust)
    width, height = (read(original, addr, '<h') for addr in (0x6705A0, 0x6705A2))
    area = width * height // 2
    original.write(0x6705A4, area, '<h')
    original.write(0x5D156C, 1, '<B')  # running-game context for the input loop
    original.write(0x5DE648, 0, '<I')  # suppress observation redraw callbacks
    viewer_cache = read(original, 0x5DEAD0 + 4 * player, '<I')
    before_tiles = bytes(original.cpu.mem_read(0x700000, 6 * area))
    before_cache = bytes(original.cpu.mem_read(viewer_cache, area))
    unit_addr, unit_bytes = regions[1]
    city_addr, city_bytes = regions[2]

    def entity_knowledge():
        units = original.cpu.mem_read(unit_addr, unit_bytes)
        cities = original.cpu.mem_read(city_addr, city_bytes)
        return units[11::40], [cities[i + 14:i + 23] for i in range(0, city_bytes, 92)]

    before_units, before_cities = entity_knowledge()
    callbacks = []
    substitutes = set(DISPLAY_HELPERS)
    if not observation:
        substitutes.add(OBSERVE)

    def replace(cpu, address, _size, _data):
        if address not in substitutes:
            return
        esp = cpu.reg_read(UC_X86_REG_ESP)
        ret = read(original, esp, '<I')
        callbacks.append(f'{address:08x}')
        cpu.reg_write(UC_X86_REG_EAX, 0)
        cpu.reg_write(UC_X86_REG_ESP, esp + 4)
        cpu.reg_write(UC_X86_REG_EIP, ret)

    hook = original.cpu.hook_add(UC_HOOK_CODE, replace)
    stop = AFTER_OBSERVATION if observation else INPUT_PREPARATION
    paths = (OBSERVATION_HELPERS if observation else
             (OBSERVE, 0x473127, 0x4734B9, 0x472864, INPUT_PREPARATION))
    try:
        run(original, ENTRY, {stop}, calls=CALLS + DISPLAY_HELPERS + paths)
    finally:
        original.cpu.hook_del(hook)
    after_tiles = bytes(original.cpu.mem_read(0x700000, 6 * area))
    after_cache = bytes(original.cpu.mem_read(viewer_cache, area))
    after_units, after_cities = entity_knowledge()
    result = {
        'stop': f'{stop:08x}',
        'active_slot': read(original, ACTIVE_SLOT, '<h'),
        'display_mode': read(original, 0x67A6E8),
        'research_target': read(original, CIV + STRIDE * player + 18, '<h'),
        'research_progress': read(original, CIV + STRIDE * player + 16, '<h'),
        'explored_tiles_before': sum(bool(mask & (1 << player)) for mask in before_tiles[4::6]),
        'explored_tiles_after': sum(bool(mask & (1 << player)) for mask in after_tiles[4::6]),
        'tile_bytes_changed': sum(a != b for a, b in zip(before_tiles, after_tiles, strict=True)),
        'human_memory_bytes_changed': sum(a != b for a, b in zip(before_cache, after_cache, strict=True)),
        'unit_masks_changed': sum(a != b for a, b in zip(before_units, after_units, strict=True)),
        'city_knowledge_records_changed': sum(a != b for a, b in zip(before_cities, after_cities, strict=True)),
        'substituted_calls': callbacks,
    }
    if not observation:
        result['return_address'] = f'{read(original, original.cpu.reg_read(UC_X86_REG_ESP), "<I"):08x}'
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    setup = json.loads((root / 'docs/evidence/import-unit-setup.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    source = setup['source']['path']
    assert hashlib.sha256(binary).hexdigest() == setup['binary_sha256']
    assert hashlib.sha256((home / source).read_bytes()).hexdigest() == setup['source']['sha256']
    assert hashlib.sha256((home / setup['rules']['path']).read_bytes()).hexdigest() == setup['rules']['sha256']
    cases = {
        'native_observation': witness(home, binary, source, exhaust=False, observation=True),
        'selected_input_control': witness(home, binary, source, exhaust=False, observation=False),
        'exhausted_input_control': witness(home, binary, source, exhaust=True, observation=False),
    }
    seen = cases['native_observation']
    assert seen['active_slot'] == 6
    assert seen['explored_tiles_before'] == seen['explored_tiles_after'] == 59
    assert seen['tile_bytes_changed'] == seen['human_memory_bytes_changed'] == 0
    assert seen['unit_masks_changed'] == seen['city_knowledge_records_changed'] == 0
    assert seen['substituted_calls'] == ['005346be', '0040d7e7', '004bf9b6']
    selected = cases['selected_input_control']
    assert (selected['return_address'], selected['active_slot'], selected['display_mode']) == (
        '0047342e', 6, 1,
    )
    assert '0041f704' in selected['substituted_calls']
    exhausted = cases['exhausted_input_control']
    assert (exhausted['return_address'], exhausted['active_slot'], exhausted['display_mode']) == (
        '00473522', -1, 0,
    )
    assert '0041f704' not in exhausted['substituted_calls']
    assert all(case['research_target'] == -1 and case['research_progress'] == 0
               for case in cases.values())
    report = {'schema': 1, 'binary_sha256': setup['binary_sha256'], 'source': setup['source'],
              'rules': setup['rules'], 'entry': f'{ENTRY:08x}', 'cases': cases,
              'scope': __doc__.strip()}
    args.out.write_text(json.dumps(report, indent=2) + '\n')
    print('Native observation: 59 tiles and unchanged player memory. Both input controls reached preparation.')


if __name__ == '__main__':
    main()
