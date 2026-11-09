#!/usr/bin/env python3
"""Author second-government fixtures from the installed reference save (#194).

Derives two private saves from Original/Ph_Auto2.SAV into --out (never
committed): `monarchy.SAV` gives the human civ Monarchy and its prerequisite
closure while it stays in Despotism, and `anarchy.SAV` additionally puts it in
Anarchy. Only hashes, the patch description and native route vectors are
retained as evidence.

Native routines are executed on the derived saves exactly as in
rates_government_check.py: 0x525468 (eligibility, chooser list), 0x40966b (cap),
0x409704 (redistribution) and 0x5292d6 (anarchy resolution; 0x52588e recorded
and returns 0). The advance record table is not loaded by the harness, so the
enabled byte of the five government advances is set from the installed Rules.txt
(prerequisite 1 not nil). Live load of the derived saves in the original game
has NOT been exercised.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from totctl import parse_rules
from yields_check import OriginalCity
from rates_government_check import CIV, STRIDE, STOP, normalize

SOURCE = "Original/Ph_Auto2.SAV"
GOV_ADVANCE = {2: 54, 3: 15, 4: 31, 5: 71, 6: 21}
TABLE = 0x5D0A14
OFF_CIVS = 2944
OFF_TURN = 668
OFF_HUMAN = 687


def closure(rules, target):
    rows = rules["CIVILIZE"]["rows"]
    ids = {s: i for i, s in enumerate(rules["CIVILIZE"]["comments"])}
    seen, stack = set(), [target]
    while stack:
        t = stack.pop()
        if t in seen:
            continue
        seen.add(t)
        for p in rows[t][3:5]:
            if p in ids:
                stack.append(ids[p])
    return sorted(seen)


def derive(data, civ, techs, government):
    out = bytearray(data)
    b = OFF_CIVS + STRIDE * civ
    for t in techs:
        out[b + 96 + t // 8] |= 1 << (t % 8)
    count = sum(bin(x).count("1") for x in out[b + 96 : b + 109])
    out[b + 24] = count  # acquired-advance counter follows the known set
    out[b + 29] = government
    return bytes(out)


def load(binary, home, name):
    o = OriginalCity(binary)
    o.load_saved(home, name)
    o.cpu.mem_write(TABLE, b"\0" * (20 * 100 + 8))
    return o


def enable_government_advances(o, rules):
    rows = rules["CIVILIZE"]["rows"]
    for adv in GOV_ADVANCE.values():
        assert rows[adv][3] != "nil", adv
        o.write(TABLE + 20 * adv + 17, 1, "<B")


def chooser(o, civ):
    return [g for g in range(1, 7) if o.call(0x525468, civ, g)]


def resolve(o, civ, turn):
    o.write(0x607648, turn, "<h")
    calls = []

    def hook(uc, address, size, user):
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
    finally:
        o.cpu.hook_del(handle)
    return calls


def rate_state(o, civ):
    b = CIV + STRIDE * civ
    sci, tax = o.cpu.mem_read(b + 27, 2)
    return [tax, 10 - sci - tax, sci]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--out", required=True, type=Path, help="private output directory")
    p.add_argument("--gold", type=Path)
    p.add_argument("--vectors", type=Path)
    a = p.parse_args()
    home = Path(os.environ["CIVTOT_HOME"])
    binary = (home / "civ2.bak").read_bytes()
    rules = parse_rules(str(home / "Rules.txt"))
    source = (home / SOURCE).read_bytes()
    civ = next(c for c in range(8) if source[OFF_HUMAN] & (1 << c))
    assert source[OFF_HUMAN] == 1 << civ
    grant = closure(rules, GOV_ADVANCE[2])
    a.out.mkdir(parents=True, exist_ok=True)
    link = a.out / "Rules.txt"
    if not link.exists():
        link.symlink_to(home / "Rules.txt")
    fixtures = {
        "baseline": source,
        "monarchy": derive(source, civ, grant, 1),
        "anarchy": derive(source, civ, grant, 0),
    }
    for name in ("monarchy", "anarchy"):
        (a.out / f"{name}.SAV").write_bytes(fixtures[name])

    lines = [f"# fixture human_civ={civ} granted_advances={' '.join(map(str, grant))}"]
    report = {"binary_sha256": hashlib.sha256(binary).hexdigest(), "source": SOURCE,
              "source_sha256": hashlib.sha256(source).hexdigest(), "human_civ": civ,
              "turn": struct.unpack_from("<h", source, OFF_TURN)[0],
              "granted_advances": grant, "fixtures": {}}
    lines.append("# chooser: fixture government -> eligible governments (0x525468, civ 1..6 list)")
    for name in ("baseline", "monarchy"):
        if name == "baseline":
            (a.out / "baseline.SAV").write_bytes(source)
        o = load(binary, a.out, f"{name}.SAV")
        enable_government_advances(o, rules)
        lines.append(f"chooser {name} " + " ".join(map(str, chooser(o, civ))))
        report["fixtures"][name] = {"sha256": hashlib.sha256(fixtures[name]).hexdigest()}
    (a.out / "baseline.SAV").unlink()
    report["fixtures"]["anarchy"] = {"sha256": hashlib.sha256(fixtures["anarchy"]).hexdigest()}
    base = OFF_CIVS + STRIDE * civ
    for name in ("monarchy", "anarchy"):
        # byte offsets inside the human civ block; nothing outside it changes
        changed = [i - base for i in range(len(source)) if source[i] != fixtures[name][i]]
        assert all(0 <= c < STRIDE for c in changed), name
        report["fixtures"][name]["changed_civ_block_offsets"] = changed

    lines.append("# cap: government -> cap_tenths for the human civ (0x40966b)")
    o = load(binary, a.out, "monarchy.SAV")
    for gov in (0, 1, 2):
        o.write(CIV + STRIDE * civ + 29, gov, "<B")
        lines.append(f"cap {gov} {o.call(0x40966B, civ)}")

    lines.append("# rate route: government moved value | tax lux sci after 0x409704, saved start 4/0/6 (tax lux sci)")
    start = rate_state(o, civ)
    lines.append("# saved rates tax/lux/sci " + " ".join(map(str, start)))
    for gov in (1, 2):
        o.write(CIV + STRIDE * civ + 29, gov, "<B")
        cap = o.call(0x40966B, civ)
        for moved, value in (("sci", 7), ("sci", 8), ("tax", 7), ("lux", 3)):
            cur = dict(zip(("tax", "lux", "sci"), start))
            cycle = {"tax": ("sci", "lux"), "sci": ("lux", "tax"), "lux": ("tax", "sci")}[moved]
            clamped = min(value, cap)
            y, z = cycle
            out = normalize(o, cap, clamped, cur[y], cur[z], cur[y], 0, 0)
            result = {moved: out[0], y: out[1], z: out[2]}
            lines.append(f"rate {gov} {moved} {value} | {result['tax']} {result['lux']} {result['sci']} clamped={int(clamped != value)}")

    lines.append("# anarchy: turn -> chooser opened (0x5292d6, 0x52588e recorded), anarchy fixture, civ flags as saved")
    o = load(binary, a.out, "anarchy.SAV")
    for turn in range(43, 53):
        calls = resolve(o, civ, turn)
        flags = struct.unpack("<H", o.cpu.mem_read(CIV + STRIDE * civ, 2))[0]
        lines.append(f"anarchy {turn} {len(calls)} flags_after={flags:#x}")
        o.write(CIV + STRIDE * civ, struct.unpack_from("<H", source, OFF_CIVS + STRIDE * civ)[0], "<H")

    text = "\n".join(lines) + "\n"
    report["vectors_sha256"] = hashlib.sha256(text.encode()).hexdigest()
    report["scope"] = __doc__.strip()
    if a.gold:
        a.gold.write_text(json.dumps(report, indent=2) + "\n")
    if a.vectors:
        a.vectors.write_text(text)
    print(text)


if __name__ == "__main__":
    main()
