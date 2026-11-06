#!/usr/bin/env python3
"""Re-derive original capture loot, building-loss and population kernels in Unicorn.

Executes unmodified civ2.bak 0x53adc6, 0x53d9ac..0x53da34 and
0x53da34..0x53dc1f, plus ownership history at 0x53f522..0x53f55c.
Instruction ranges use authored caller frames, stopping
before destruction or ownership transfer. No game assets or saved records
are emitted. Not a whole capture comparison: capital relocation, tech acquisition,
civilization elimination and presentation are outside these two kernels.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
from yields_check import OriginalCity


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--vectors', type=Path, required=True)
    p.add_argument('--report', type=Path, required=True)
    args = p.parse_args()
    from unicorn.x86_const import UC_X86_REG_EBP
    binary = (Path(os.environ['CIVTOT_HOME'])/'civ2.bak').read_bytes()
    o = OriginalCity(binary)
    rows = ['# loot size treasury population result | buildings mode recapture seed input*5 output*5 final_CRT']
    loot_cases = 0
    for size in (1, 2, 3, 12, 127):
        for cash in (0, 1, 1000, 20000, 100000):
            for population in (0, 40, 127):
                o.write(0x5fd15a, 5, '<b')
                o.write(0x5fd15b, size, '<b')
                o.write(0x5f68aa+3348*5, cash, '<i')
                o.write(0x5f6914+3348*5, population, '<h')
                value = o.call(0x53adc6, 0)
                rows.append(f'loot {size} {cash} {population} {value}')
                loot_cases += 1
    building_cases = 0
    for mode in (0, 1, 2):
        for recapture in (0, 1):
            for seed in range(1, 17):
                for buildings in (bytes([255]*5), bytes([0, 18, 52, 86, 120])):
                    frame = 0x108000
                    o.write(frame+8, 0, '<I')
                    o.write(frame+16, mode, '<I')
                    o.write(frame-0x78, recapture, '<I')
                    o.write(0x5e2590, seed, '<I')
                    o.cpu.mem_write(0x5fd186, buildings)
                    o.cpu.reg_write(UC_X86_REG_EBP, frame)
                    o.cpu.reg_write(o.sp, frame-0x900)
                    o.cpu.emu_start(0x53d9ac, 0x53da34, count=10000)
                    assert o.cpu.reg_read(o.ip) == 0x53da34
                    result = list(o.cpu.mem_read(0x5fd186, 5))
                    state = struct.unpack('<I', o.cpu.mem_read(0x5e2590, 4))[0]
                    rows.append(' '.join(map(str, ['buildings', mode, recapture, seed, *buildings, *result, state])))
                    building_cases += 1
    from unicorn import UC_HOOK_CODE
    def stop_population(cpu, address, length, context):
        if address == 0x53dc51:
            cpu.reg_write(o.ip, 0x53dc1f)
            cpu.emu_stop()
    o.cpu.hook_add(UC_HOOK_CODE, stop_population)
    population_cases = 0
    for size in (1, 2, 12):
        for recapture in (0, 1):
            for humans in (0, 32, 64, 2):
                for known in (0, 2):
                    for rank in (1, 4, 7):
                        for seed in (1, 2, 7, 106):
                            frame = 0x108000
                            o.write(frame+8, 0, '<I')
                            o.write(frame+12, 6, '<I')
                            o.write(frame+16, 0, '<I')
                            o.write(frame-0x78, recapture, '<I')
                            o.write(frame-0x80, 5, '<I')
                            o.write(0x5fd15b, size, '<B')
                            o.write(0x5fd15e, known, '<B')
                            o.write(0x60765b, humans, '<B')
                            o.write(0x607771, 2, '<B')
                            o.write(0x607772+2, 3, '<B')
                            o.write(0x607772+6, rank, '<B')
                            o.write(0x5e2590, seed, '<I')
                            o.cpu.reg_write(UC_X86_REG_EBP, frame)
                            o.cpu.reg_write(o.sp, frame-0x900)
                            o.cpu.emu_start(0x53da34, 0x53dc1f, count=10000)
                            # Recapture size 1 branches past the population block.
                            if o.cpu.reg_read(o.ip) != 0x53dc1f:
                                raise AssertionError(hex(o.cpu.reg_read(o.ip)))
                            result = o.cpu.mem_read(0x5fd15b, 1)[0]
                            state = struct.unpack('<I', o.cpu.mem_read(0x5e2590, 4))[0]
                            rows.append(' '.join(map(str, ['population', size, recapture, humans, known, rank, seed, result, state])))
                            population_cases += 1
    history = []
    for old_owner in range(8):
        frame = 0x108000
        o.write(frame+8, 0, '<I')
        o.write(frame-0x80, old_owner, '<I')
        o.write(0x5fd15c, 255, '<B')
        o.cpu.reg_write(UC_X86_REG_EBP, frame)
        o.cpu.reg_write(o.sp, frame-0x900)
        o.cpu.emu_start(0x53f522, 0x53f55c, count=1000)
        assert o.cpu.reg_read(o.ip) == 0x53f55c
        previous = o.cpu.mem_read(0x5fd15c, 1)[0]
        assert previous == old_owner
        history.append({'old_owner': old_owner, 'resulting_original_owner': previous})
    args.vectors.write_text('\n'.join(rows)+'\n')
    report = {'binary_sha256': hashlib.sha256(binary).hexdigest(),
              'routines': ['0x53adc6', '0x53d9ac..0x53da34', '0x53da34..0x53dc1f', '0x53f522..0x53f55c'],
              'loot_cases': loot_cases, 'building_cases': building_cases, 'population_cases': population_cases,
              'replacement_callees': [], 'owner_history_checks': history,
              'scope': 'Authored caller inputs for loot, alternating-bit building attrition, non-barbarian military population loss and previous-owner recording only; '
                       'not complete city capture, capital handling or unit movement.'}
    args.report.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report))


if __name__ == '__main__':
    main()
