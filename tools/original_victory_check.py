#!/usr/bin/env python3
"""Execute Original's victory-event kernels from the user's civ2.bak.

Controlled event records, not a live game: the RECEIVEDTECHNOLOGY evaluator
and @AND gate execute unmodified. Its dispatcher call is captured rather than
executed. A separate dispatcher check replaces only video/TEXT presentation
callees, capturing their order; ENDGAME executes unmodified in single-player
context. No executable, event prose or other original assets are emitted.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_EIP, UC_X86_REG_ESP

from yields_check import OriginalCity

OBJ, EVENT, SECOND = 0x10A000, 0x10B000, 0x10B200


def read(o, address, fmt='<I'):
    return struct.unpack(fmt, o.cpu.mem_read(address, struct.calcsize(fmt)))[0]


def returned(cpu, pop):
    sp = cpu.reg_read(UC_X86_REG_ESP)
    target = struct.unpack('<I', cpu.mem_read(sp, 4))[0]
    cpu.reg_write(UC_X86_REG_EAX, 1)
    cpu.reg_write(UC_X86_REG_ESP, sp + 4 + pop)
    cpu.reg_write(UC_X86_REG_EIP, target)


def invoke(o, routine, *args):
    o.cpu.reg_write(UC_X86_REG_ECX, OBJ)
    return o.call(routine, *args)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    o = OriginalCity(binary)
    o.write(0x607652, 0, '<B')  # single player, no network/UI notification
    o.write(OBJ + 0x698, EVENT, '<I')
    calls = []

    def capture(cpu, address, size, data):
        if address == 0x4D3AE8:
            event = read(o, cpu.reg_read(UC_X86_REG_ESP) + 4)
            calls.append(read(o, event + 0x20, '<i'))
            returned(cpu, 4)

    hook = o.cpu.hook_add(UC_HOOK_CODE, capture)
    cases = []
    for receiver in (-2, *range(1, 8)):  # Anybody and each named civ
        for count in (0, 4, 5, 6, 255):
            for alive in (False, True):
                for random_active in (False, True):
                    o.cpu.mem_write(EVENT, bytes(0x400))
                    o.write(EVENT, 0x100, '<I')
                    o.write(EVENT + 8, 0x10000100, '<I')  # @AND first + futuretech
                    o.write(EVENT + 0x20, receiver, '<i')
                    o.write(EVENT + 0x30, 5, '<B')
                    o.write(EVENT + 0x114, SECOND, '<I')
                    o.write(SECOND + 8, 0xC0000000 if random_active else 0x40000000, '<I')
                    o.write(SECOND + 0x118, EVENT, '<I')
                    civ = 3 if receiver == -2 else receiver
                    o.write(0x60765A, (1 << civ) if alive else 0, '<B')
                    o.write(0x5F68B9 + 3348 * civ, count, '<B')
                    calls.clear()
                    invoke(o, 0x4D306F)
                    assert calls == ([civ] if alive and count == 5 and random_active else [])
                    cases.append([receiver, count, alive, random_active, list(calls)])
    o.cpu.hook_del(hook)

    # Native fixed action order; keep the ENDGAME body and replace just the
    # video and text handlers, whose actual execution would require Windows UI.
    order = []

    def presentation(cpu, address, size, data):
        names = {0x5676A0: 'PLAYAVIFILE', 0x4D1BF6: 'TEXT'}
        if address in names:
            order.append(names[address])
            returned(cpu, 4)
        elif address == 0x575570:
            order.append('ENDGAME')

    hook = o.cpu.hook_add(UC_HOOK_CODE, presentation)
    endings = []
    for screens in (False, True):
        o.cpu.mem_write(EVENT, bytes(0x200))
        o.write(EVENT + 4, 0x5001, '<I')  # TEXT, AVI, ENDGAME
        o.write(EVENT + 8, 8 if screens else 0, '<I')
        o.write(0x607640, 0x100, '<H')
        o.write(0x60763E, 0x200, '<H')
        o.write(0x5F46F4, 0, '<I')
        order.clear()
        invoke(o, 0x4D3AE8, EVENT)
        assert order == ['PLAYAVIFILE', 'TEXT', 'ENDGAME']
        result = [screens, read(o, 0x607640, '<H'), read(o, 0x60763E, '<H'), read(o, 0x5F46F4)]
        assert result == [screens, 0x102, 0x200 if screens else 0x208, 6]
        endings.append(result)
    o.cpu.hook_del(hook)
    report = {
        'schema': 1,
        'binary_sha256': hashlib.sha256(binary).hexdigest(),
        'events_sha256': hashlib.sha256((home / 'Original/Events.txt').read_bytes()).hexdigest(),
        'routines': ['0x4d306f', '0x575cbe', '0x4d3ae8', '0x575570'],
        'method': __doc__.strip(),
        'trigger_columns': ['receiver (-2=Anybody)', 'future_count', 'alive', 'random_half_active', 'dispatched_receivers'],
        'trigger_cases': cases,
        'dispatcher_order': ['PLAYAVIFILE', 'TEXT', 'ENDGAME'],
        'ending_columns': ['endscreens', 'flags_at_607640', 'flags_at_60763e', 'reason_at_5f46f4'],
        'endings': endings,
        'limits': 'Controlled records for futuretech=5, single-player. Native parser mapping is a separate static decode. No live UI, video, scoring, full round or native save continuation is proved.',
    }
    args.out.write_text(json.dumps(report, indent=2) + '\n')
    print(f'{len(cases)} trigger contexts and {len(endings)} dispatcher/endgame contexts passed')


if __name__ == '__main__':
    main()
