#!/usr/bin/env python3
"""Probe the original human research-candidate filter natively (#194).

Native routine executed: 0x49f98b(civ), the default/recommended research pick.
It applies the same candidate filter as the human project chooser built in
0x4a0ab4 (enabled advance record, not known, both prerequisites known, not both
prerequisites 'nil', tech-group gate 0x4a2098, and the human thinning rule), so
a candidate's membership can be read off by making it the unique best value.

Replaced callees (recorded): 0x5b11d0 (CRT rand, returns 1 so the human value is
unperturbed) and 0x49c95d (advance value: 100 for the probed advance, else 0).
Predicates 0x49c820 (knows), 0x49ecb6 and 0x4a2098 run natively. The advance
record table (0x5d0a14, 20-byte records: group dword +8, enabled byte +17,
prerequisites +22/+23, nil = -2) is synthetic and seeded: the Rules.txt loader
0x412f54 is a static decode only. The Python model transcribes the loop of
0x49f98b and calls native predicates; it is compared with native membership.
The dialog's extra conditions (mode 0, scenario flag 0x80) are static decode.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from rates_government_check import CIV, STRIDE, world

TABLE = 0x5D0A14
GROUP_FLAGS = 0x606F78
HUMAN = 0x60765B
DIFFICULTY = 0x607658


def setup(o, rng, civ, difficulty):
    # Prerequisite bytes (+22/+23) overflow into the next 20-byte record's first
    # four bytes, so clear the whole table before writing any field.
    o.cpu.mem_write(TABLE, b"\0" * (20 * 100 + 8))
    for a in range(100):
        base = TABLE + 20 * a
        group = rng.choice([-1, -1, -1, 0, 1, 2, 3])
        o.write(base + 8, group, "<i")
        o.write(base + 17, 0 if rng.random() < 0.08 else 1, "<B")
        p1 = -2 if rng.random() < 0.25 or a == 0 else rng.randrange(a)
        p2 = -2 if rng.random() < 0.5 or a == 0 else rng.randrange(a)
        o.write(base + 22, p1, "<b")
        o.write(base + 23, p2, "<b")
    leader = rng.randrange(4)
    o.write(CIV + STRIDE * civ + 14, leader, "<h")
    o.cpu.mem_write(GROUP_FLAGS, bytes(rng.choice([0] * 8 + [1, 2]) for _ in range(5 * 4 + 16)))
    block = CIV + STRIDE * civ
    known = bytearray(13)
    for a in range(100):
        if rng.random() < 0.45:
            known[a // 8] |= 1 << (a % 8)
    o.cpu.mem_write(block + 96, bytes(known))
    o.write(block + 24, rng.randrange(0, 45), "<B")
    o.write(HUMAN, 1 << civ, "<B")
    o.write(DIFFICULTY, difficulty, "<B")
    return leader


def native_pick(o, civ, probe):
    calls = {"rand": 0}

    def hook(uc, address, size, user):
        esp = uc.reg_read(UC_X86_REG_ESP)
        ret = struct.unpack("<I", uc.mem_read(esp, 4))[0]
        if address == 0x5B11D0:
            value = 1
            calls["rand"] += 1
        else:
            value = 100 if struct.unpack("<II", uc.mem_read(esp + 4, 8))[1] == probe else 0
        uc.reg_write(UC_X86_REG_EAX, value)
        uc.reg_write(UC_X86_REG_ESP, esp + 4)
        uc.reg_write(UC_X86_REG_EIP, ret)

    handles = [o.cpu.hook_add(UC_HOOK_CODE, hook, begin=a, end=a) for a in (0x5B11D0, 0x49C95D)]
    try:
        return o.call(0x49F98B, civ)
    finally:
        for h in handles:
            o.cpu.hook_del(h)


def model(o, civ, difficulty, thin=True):
    block = CIV + STRIDE * civ
    acquired = o.cpu.mem_read(block + 24, 1)[0]
    count = 0
    members = []
    for a in range(100):
        if not o.call(0x49ECB6, civ, a):
            continue
        base = TABLE + 20 * a
        p1, p2 = struct.unpack("<bb", o.cpu.mem_read(base + 22, 2))
        if p1 == -2 and p2 == -2:
            continue
        if not o.call(0x4A2098, civ, a):
            continue
        if thin and count != 0 and difficulty != 0 and (a - acquired) % 3 == 0:
            continue
        count += 1
        members.append(a)
    return members


def inputs(o, civ):
    """Per-advance inputs to the candidate rule, as the native predicates see them."""
    rows = []
    for a in range(100):
        base = TABLE + 20 * a
        enabled = o.cpu.mem_read(base + 17, 1)[0] != 0
        p1, p2 = struct.unpack("<bb", o.cpu.mem_read(base + 22, 2))
        rows.append((a, int(enabled), int(bool(o.call(0x49C820, civ, a))), p1, p2, int(bool(o.call(0x4A2098, civ, a)))))
    return rows


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--gold")
    p.add_argument("--vectors")
    args = p.parse_args()
    home = Path(os.environ["CIVTOT_HOME"])
    binary, o = world(home)
    rng = random.Random(194)
    lines, mismatches, thinned, empty, details = [], 0, 0, 0, []
    for case in range(48):
        civ = 1 + case % 7
        difficulty = case % 6
        setup(o, rng, civ, difficulty)
        expected = model(o, civ, difficulty)
        native = [a for a in range(100) if native_pick(o, civ, a) == a]
        if native != expected:
            mismatches += 1
        if native != model(o, civ, difficulty, thin=False):
            thinned += 1
        empty += not expected
        lines.append([case, civ, difficulty, len(native), int(native == expected), *native])
        acquired = o.cpu.mem_read(CIV + STRIDE * civ + 24, 1)[0]
        details.append(f"case {case} difficulty {difficulty} acquired {acquired}")
        details += [f"{case} " + " ".join(map(str, r)) for r in inputs(o, civ)]
    text = ["# case civ difficulty members match | members..."]
    text += [" ".join(map(str, r)) for r in lines]
    text.append("# inputs: case advance enabled known prerequisite1 prerequisite2 group_gate_open (native predicates 0x49c820/0x4a2098; known is false for 89; -1 nil, -2 no)")
    text += details
    body = "\n".join(text) + "\n"
    report = {
        "binary_sha256": hashlib.sha256(binary).hexdigest(),
        "routine": "0x49f98b (probed per advance)",
        "cases": len(lines),
        "model_mismatches": mismatches,
        "empty_candidate_sets": empty,
        "cases_where_thinning_changes_the_list": thinned,
        "replaced_callees": ["0x5b11d0 (returns 1)", "0x49c95d (100 for probe, else 0)"],
        "vectors_sha256": hashlib.sha256(body.encode()).hexdigest(),
        "scope": __doc__.strip(),
    }
    if args.gold:
        Path(args.gold).write_text(json.dumps(report, indent=2) + "\n")
    if args.vectors:
        Path(args.vectors).write_text(body)
    print(json.dumps({k: report[k] for k in ("cases", "model_mismatches", "empty_candidate_sets", "cases_where_thinning_changes_the_list")}))


if __name__ == "__main__":
    main()
