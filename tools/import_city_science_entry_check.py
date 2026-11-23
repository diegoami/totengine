#!/usr/bin/env python3
"""Trace the pinned player's city phase through its native science boundary.

Execute the original city phase on the prepared resume image, without playing
opponents or the rest of the round. The no-city-window query and redraw are
substituted. Positive controls replace only the science total at the call
from city processing into accumulation. Stops precede the selector/adviser.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from import_input_handoff_check import witness
from import_phase_check import CIV, STRIDE, read, run

CITY_PHASE = 0x4CB01D
SCIENCE_FROM_CITY = 0x4CB6AF
SCIENCE = 0x4A1EBF
SCIENCE_BUFFER = 0x6537F8
ADVICE = 0x477030
CHOOSE = 0x4A1436
RETURN = 0x100000
PRESENTATION = {0x411C50: -1, 0x467C89: 0}
CALLS = (
    0x411C50, 0x4C596B, 0x4C19E0, 0x467C89, 0x4C5268, 0x4C109F, 0x566AE2,
    0x4030F0, 0x5702CD, 0x56FFF0, 0x5691ED, 0x56909A, 0x57042E, 0x570F15,
    0x568860, 0x56B64B, 0x570169, 0x4C13FE, 0x43002E, 0x566E4F, 0x566DAD,
    0x566B9E, 0x566D26, 0x49C820, 0x4C1BDF, 0x4C18AF, 0x4C521C, 0x4C2C14,
    0x4C2BA1, 0x4CBA90, 0x4C139F, 0x4C135F, 0x4C2B1F, 0x4C22A7, 0x5716D5,
    0x570599, 0x5700FF, 0x5707A6, 0x4C2943, 0x444A28, 0x4449B0, 0x57054F,
    0x57145C, 0x4095B0, 0x4C2A7B, 0x4C506E, 0x4C34B6, 0x4C0FA1, 0x4C3919,
    0x444A61, 0x4C35A2, 0x4C3551, 0x566A30, 0x4C45FF, 0x430232, 0x5701D1,
    0x566E7C, 0x411D62, 0x4C3F0C, 0x4C1048, 0x4C1002, 0x4C3D3B, 0x568FDF,
    0x4C6208, 0x5B40E0, 0x56A1C0, 0x42FFCC, 0x435DD0, 0x569856, 0x569113,
    0x41F704, 0x57012C, 0x571972, 0x57008F, 0x571819, 0x5711C9, 0x42FE9B,
    0x490851, 0x5719C8, 0x4C92A4, 0x4C998E, 0x4CA000, SCIENCE, 0x40106B, CHOOSE,
    ADVICE, 0x4CA17C, 0x5B11D0, 0x4CA6E0, 0x4CA5AE, 0x4CAD56, 0x4796D0, 0x4793D9,
)


def trace(original, player, city_region, *, points, advice):
    cpu = original.cpu
    cpu.mem_map(0, 4096)  # production uses FS:[0] for exception registration
    address, length = city_region
    cities = [slot for slot in range(length // 92)
              if read(original, address + 92 * slot + 10, '<B') == player]
    assert cities == [5]
    city = cities[0]
    assert read(original, 0x67A6E0) == player and read(original, 0x6762F8) == 0
    options = read(original, 0x60763A, '<I')
    assert not options & 0x100
    if advice:
        original.write(0x60763A, options | 0x100, '<I')

    record = CIV + STRIDE * player

    def research():
        return {'target': read(original, record + 18, '<h'),
                'progress': read(original, record + 16, '<h'),
                'acquired': read(original, record + 24, '<B')}

    before = research()
    computed = []
    arguments = []
    substitutes = []

    def observe(emu, at, _size, _data):
        sp = emu.reg_read(UC_X86_REG_ESP)
        if at == SCIENCE_FROM_CITY:
            computed.append(read(original, SCIENCE_BUFFER))
            if points is not None:
                original.write(SCIENCE_BUFFER, points, '<i')
        if at == SCIENCE:
            arguments.append({'civ': read(original, sp + 4), 'points': read(original, sp + 8)})
        if at in PRESENTATION:
            substitutes.append(f'{at:08x}')
            emu.reg_write(UC_X86_REG_EAX, PRESENTATION[at] & 0xFFFFFFFF)
            emu.reg_write(UC_X86_REG_ESP, sp + 4)
            emu.reg_write(UC_X86_REG_EIP, read(original, sp, '<I'))

    hook = cpu.hook_add(UC_HOOK_CODE, observe)
    try:
        cpu.mem_write(0x108000, struct.pack('<II', RETURN, city))
        cpu.reg_write(original.sp, 0x108000)
        stop = run(original, CITY_PHASE, {RETURN, CHOOSE, ADVICE}, calls=CALLS,
                   instruction_limit=400000)
    finally:
        cpu.hook_del(hook)
    assert set(substitutes) == {'00411c50', '00467c89'}, substitutes
    assert computed == [0] and arguments == [{'civ': player, 'points': points or 0}]
    after = research()
    assert before == {'target': -1, 'progress': 0, 'acquired': 1}, before
    assert after == before | {'progress': points or 0}
    boundary = {RETURN: 'city-return', CHOOSE: 'choose-project', ADVICE: 'advice-before-choice'}
    return {'city_slot': city, 'computed_science': computed[0], 'supplied_science': points,
            'advice': advice, 'accumulation': arguments[0], 'before': before, 'after': after,
            'stop': f'{stop:08x}', 'boundary': boundary[stop], 'substitutes': substitutes}


def case(home, binary, path, *, points=None, advice=False):
    result = {}

    def city_phase(original, player, regions):
        result.update(trace(original, player, regions[2], points=points, advice=advice))

    witness(home, binary, path, exhaust=False, observation=True, after_idle=city_phase)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True, type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    handoff = json.loads((root / 'docs/evidence/import-input-handoff.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    path = handoff['source']['path']
    binary = (home / 'civ2.bak').read_bytes()
    assert hashlib.sha256(binary).hexdigest() == handoff['binary_sha256']
    for role in ('source', 'rules'):
        source = handoff[role]
        assert hashlib.sha256((home / source['path']).read_bytes()).hexdigest() == source['sha256']
    cases = {
        'pinned': case(home, binary, path),
        'one_science': case(home, binary, path, points=1),
        'one_science_with_advice': case(home, binary, path, points=1, advice=True),
    }
    assert [row['boundary'] for row in cases.values()] == [
        'city-return', 'choose-project', 'advice-before-choice',
    ]
    report = {
        'schema': 1,
        **{key: handoff[key] for key in ('binary_sha256', 'source', 'rules')},
        'city_phase': f'{CITY_PHASE:08x}',
        'accumulation': f'{SCIENCE:08x}',
        'cases': cases,
        'scope': __doc__.strip(),
    }
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
