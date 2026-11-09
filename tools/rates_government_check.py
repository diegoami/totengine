#!/usr/bin/env python3
"""Execute original rate-cap, slider-redistribution, government-eligibility and
anarchy-resolution routines from unpatched civ2.bak (#194).

Native routines (all executed from the original image, no reimplementation):

* 0x40966b  max rate (tenths) for a civ's government byte
* 0x409704  three-slider redistribution with two locks and a remembered value
* 0x525468  is a government selectable for a civ (advance / Statue of Liberty)
* 0x5292d6  per-civ anarchy resolution, run to its common exit 0x529453

Replaced callees (recorded, not executed): 0x52588e, the human revolution /
chooser trigger, because it opens modal UI. The call and its arguments are
captured and the call returns 0. Everything else in the stretch is original
code on a copied standard save. This does not cover the Shift+R handler, the
government list UI, the dialog-open cap normalization or the rate commit; those
are static decodes documented in docs/evidence/README.md.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct
import sys

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP, UC_X86_REG_ESP, UC_X86_REG_EAX

from yields_check import OriginalCity

CIV = 0x5F68A0
STRIDE = 3348
DIALOG_PTR = 0x5E45C0
DIALOG = 0x10C000
ARGS = 0x10D000
STOP = 0x529453
# advance ids, from the installed Rules.txt @CIVILIZE row order
GOV_ADVANCE = {2: 54, 3: 15, 4: 31, 5: 71, 6: 21}  # Monarchy..Democracy
GOV_NAME = ["Anarchy", "Despotism", "Monarchy", "Communism", "Fundamentalism", "Republic", "Democracy"]
SOL_WONDER = 19  # improvement 59 (Statue of Liberty) - 40


def world(home):
    binary = (home / "civ2.bak").read_bytes()
    o = OriginalCity(binary)
    o.load_saved(home, "Original/Ph_Auto2.SAV")
    return binary, o


def u8(o, address):
    return o.cpu.mem_read(address, 1)[0]


def cap_cases(o):
    lines = []
    for gov in range(256):
        o.write(CIV + STRIDE + 29, gov, "<B")
        lines.append((gov, o.call(0x40966B, 1)))
    return lines


def normalize(o, cap, x, y, z, w, ly, lz):
    o.cpu.mem_write(DIALOG, b"\0" * 0x800)
    o.write(DIALOG_PTR, DIALOG, "<I")
    o.write(DIALOG + 0x740, cap, "<I")
    for i, v in enumerate((x, y, z, w, ly, lz)):
        o.write(ARGS + 4 * i, v, "<i")
    o.call(0x409704, *(ARGS + 4 * i for i in range(6)))
    return [struct.unpack("<i", o.cpu.mem_read(ARGS + 4 * i, 4))[0] for i in range(3)]


def normalize_cases(o):
    rng = random.Random(194)
    cases = []
    # every legal opening state, each lock mask, every moved slider (0..cap)
    for cap in (6, 7, 8, 10):
        for tax in range(cap + 1):
            for sci in range(cap + 1):
                lux = 10 - tax - sci
                if lux < 0 or lux > cap:
                    continue
                cases.append((cap, tax, sci, lux))
    sample = []
    for cap, tax, sci, lux in cases:
        for _ in range(3):
            moved = rng.randrange(3)
            value = rng.randrange(cap + 1)
            locks = rng.randrange(8)
            sample.append((cap, tax, sci, lux, moved, value, locks))
    rng.shuffle(sample)
    lines = []
    # Dialog order tax=0, luxury=1, science=2. Native handlers 0x409844 (tax),
    # 0x4099d7 (science) and the luxury handler near 0x409b6a pass the moved
    # slider as X and the next two sliders in the cycle tax->science->luxury->tax
    # as (Y, Z), with W = remembered value of Y and each slider's own lock.
    cycle = {0: (2, 1), 2: (1, 0), 1: (0, 2)}
    for cap, tax, sci, lux, moved, value, locks in sample[:1500]:
        cur = [tax, lux, sci]
        lock = [locks & 1, (locks >> 1) & 1, (locks >> 2) & 1]  # tax, luxury, science
        yi, zi = cycle[moved]
        out = normalize(o, cap, value, cur[yi], cur[zi], cur[yi], lock[yi], lock[zi])
        result = [0, 0, 0]
        result[moved], result[yi], result[zi] = out
        lines.append([cap, moved, value, tax, lux, sci, locks, *result])
    return lines


def eligible(o, civ, gov, tech, sol, flag, others):
    block = CIV + STRIDE * civ
    o.cpu.mem_write(block + 96, b"\0" * 13)
    known = set()
    if tech is not None and tech:
        known.add(GOV_ADVANCE.get(gov, -1))
    if others:
        known |= {v for k, v in GOV_ADVANCE.items() if k != gov}
    for t in known:
        if t >= 0:
            o.cpu.mem_write(block + 96 + t // 8, bytes([u8(o, block + 96 + t // 8) | (1 << (t % 8))]))
    city = 0
    o.write(0x5FD150 + 92 * city + 10, civ if sol else 255, "<B")
    o.write(0x607736 + 2 * SOL_WONDER, city if sol else -1, "<h")
    o.write(0x5D0C91, flag, "<B")
    return o.call(0x525468, civ, gov)


def eligibility_cases(o):
    lines = []
    for gov in range(9):
        for tech in (0, 1):
            for others in (0, 1):
                for sol in (0, 1):
                    for flag in (0, 1):
                        lines.append([gov, tech, others, sol, flag, eligible(o, 1, gov, tech, sol, flag, others)])
    return lines


def resolve(o, civ, government, human, turn, flags, sol):
    block = CIV + STRIDE * civ
    o.write(block + 29, government, "<B")
    o.write(block, flags, "<H")
    o.write(0x60765B, (1 << civ) if human else 0, "<B")
    o.write(0x607648, turn, "<h")
    o.write(0x5FD150 + 92 * 0 + 10, civ if sol else 255, "<B")
    o.write(0x607736 + 2 * SOL_WONDER, 0 if sol else -1, "<h")
    calls = []

    def hook(uc, address, size, user):
        if address == 0x52588E:
            esp = uc.reg_read(UC_X86_REG_ESP)
            ret, a, b = struct.unpack("<III", uc.mem_read(esp, 12))
            calls.append((a, b))
            uc.reg_write(UC_X86_REG_EAX, 0)
            uc.reg_write(UC_X86_REG_ESP, esp + 4)
            uc.reg_write(UC_X86_REG_EIP, ret)

    handle = o.cpu.hook_add(UC_HOOK_CODE, hook, begin=0x52588E, end=0x52588E)
    try:
        o.cpu.mem_write(0x108000, struct.pack("<II", 0x100000, civ))
        o.cpu.reg_write(UC_X86_REG_ESP, 0x108000)
        o.cpu.emu_start(0x5292D6, STOP, count=2000000)
        assert o.cpu.reg_read(UC_X86_REG_EIP) == STOP, hex(o.cpu.reg_read(UC_X86_REG_EIP))
    finally:
        o.cpu.hook_del(handle)
    after = struct.unpack("<H", o.cpu.mem_read(block, 2))[0]
    return after, calls


def resolve_cases(o):
    lines = []
    for human in (0, 1):
        for government in (0, 1, 3):
            for turn in (0, 1, 2, 3, 4, 5, 7, 8, 99, 100, 101):
                for flags in (0x00, 0x01, 0x08, 0x49):
                    for sol in (0, 1):
                        after, calls = resolve(o, 2, government, human, turn, flags, sol)
                        lines.append([human, government, turn, flags, sol, after, len(calls), *(calls[0] if calls else (-1, -1))])
    return lines


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--gold")
    p.add_argument("--vectors")
    a = p.parse_args()
    home = Path(os.environ["CIVTOT_HOME"])
    binary, o = world(home)
    caps = cap_cases(o)
    norm = normalize_cases(o)
    elig = eligibility_cases(o)
    res = resolve_cases(o)
    text = ["# cap: government cap_tenths"]
    text += [" ".join(map(str, row)) for row in caps]
    text.append("# normalize: cap moved(0=tax,1=lux,2=sci) value tax lux sci locks(tax=1,lux=2,sci=4) | tax lux sci")
    text += [" ".join(map(str, row)) for row in norm]
    text.append("# eligible: government knows_own_advance knows_other_government_advances statue_of_liberty fundamentalism_flag | result")
    text += [" ".join(map(str, row)) for row in elig]
    text.append("# resolve: human government turn flags_in statue_of_liberty | flags_out chooser_calls arg0 arg1")
    text += [" ".join(map(str, row)) for row in res]
    report = {
        "binary_sha256": hashlib.sha256(binary).hexdigest(),
        "routines": ["0x40966b", "0x409704", "0x525468", "0x5292d6 -> 0x529453"],
        "cases": {"cap": len(caps), "normalize": len(norm), "eligible": len(elig), "resolve": len(res)},
        "replaced_callees": ["0x52588e (recorded, returns 0)"],
        "vectors_sha256": hashlib.sha256(("\n".join(text) + "\n").encode()).hexdigest(),
        "scope": __doc__.strip(),
    }
    if a.gold:
        Path(a.gold).write_text(json.dumps(report, indent=2) + "\n")
    if a.vectors:
        Path(a.vectors).write_text("\n".join(text) + "\n")
    print(json.dumps(report["cases"]))


if __name__ == "__main__":
    main()
