#!/usr/bin/env python3
"""Independently replay the decoded science/tax formula on game records.

CIVTOT_HOME required. Produces a small derived JSON summary, never game assets.
Rust equivalence tests live in crates/engine/tests/finance_install.rs.
"""
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess

from totctl import parse_rules, walk_save


def replay(home, save_rel, rules_rel):
    data = (home / save_rel).read_bytes()
    walk = walk_save(str(home / save_rel))
    rules = parse_rules(str(home / rules_rel))
    base, total = walk['cities']['offset'], walk['cities']['count']
    unit_base = walk['sections_derived']['blob_10240']['offset'] + 10240
    wonders = struct.unpack_from('<28h', data, 906)
    tech_ids = {s: i for i, s in enumerate(rules['CIVILIZE']['comments'])}
    expires = [row[0] for row in rules['ENDWONDER']['rows'][:28]]
    cosmic = lambda row: int(rules['COSMIC']['rows'][row][0])

    def known(civ, tech):
        return civ > 0 and tech < 100 and tech != 89 and bool(
            data[2944 + 3348 * civ + 96 + tech // 8] & (1 << (tech % 8)))

    def wonder(n):
        tech = tech_ids.get(expires[n])
        return -1 if tech is not None and any(known(c, tech) for c in range(1, 8)) else wonders[n]

    def holds(civ, n):
        return wonder(n) >= 0 and data[base + 92 * wonder(n) + 10] == civ

    counts = {'cities': total, 'exact_science_and_taxes': 0,
              'cities_with_specialists': 0, 'capitalization_cities': 0}
    for i in range(total):
        city = data[base + 92 * i:base + 92 * (i + 1)]
        civ = city[10]
        science_rate, tax_rate, government = data[2944 + 3348 * civ + 27:2944 + 3348 * civ + 30]
        trade = struct.unpack_from('<H', city, 80)[0]
        has = lambda n: bool(city[54 + n // 8] & (1 << (n % 8)))
        luxuries = min(trade, max(0, (trade * (10 - science_rate - tax_rate) + 4) // 10))
        if government == 4:
            science_rate = min(science_rate, cosmic(21))
        science = min(trade - luxuries, max(0, (trade * science_rate + 4) // 10))
        taxes = trade - luxuries - science
        if government == 4 and not data[687] & (1 << civ):
            science += luxuries
            luxuries = 0
        specialists = struct.unpack_from('<I', city, 24)[0]
        counts['cities_with_specialists'] += int(specialists != 0)
        taxes += 3 * sum((specialists >> (2 * k)) & 3 == 2 for k in range(16))
        science += 3 * sum((specialists >> (2 * k)) & 3 == 3 for k in range(16))
        if government == 4:
            science -= science * cosmic(17) // 100
        tithe = (known(civ, 56) + known(civ, 9)) * (2 if holds(civ, 5) else 1) if has(4) else 0
        if has(14):
            tithe += 3 + known(civ, 24)
        if known(civ, 55) and (has(11) or holds(civ, 10)):
            tithe += 2 + known(civ, 15) + known(civ, 82)
        if holds(civ, 15):
            tithe += 2
        if government == 4:
            taxes += tithe
        taxes += taxes * sum(has(n) for n in (5, 10, 22)) // 2
        bonus = sum(has(n) for n in (6, 12)) + int(has(26) or holds(civ, 26))
        science += science * bonus // (1 if wonder(16) == i else 2)
        if wonder(11) == i:
            science *= 2
        if city[59] == 217:  # signed -39, Capitalization
            counts['capitalization_cities'] += 1
            free = city[11] if government <= 1 else cosmic({2: 13, 3: 14, 4: 15}[government]) if government <= 4 else 0
            supported = 0
            for u in range(struct.unpack_from('<H', data, 698)[0]):
                unit = data[unit_base + 40 * u:unit_base + 40 * (u + 1)]
                if (struct.unpack_from('<H', unit, 30)[0] and unit[9] == civ and unit[18] == i
                        and int(rules['UNITS']['rows'][unit[8]][11]) < 6):
                    supported += 1
            taxes += max(0, city[83] - max(0, supported - free))
        expected = struct.unpack_from('<HH', city, 76)
        assert (science, taxes) == expected, (save_rel, i, (science, taxes), expected)
        counts['exact_science_and_taxes'] += 1
    return {'save': save_rel, 'rules': rules_rel, **counts}


def main():
    home = Path(os.environ['CIVTOT_HOME'])
    binary = home / 'civ2.bak'
    # Check the original's actual operand addresses rather than only a
    # cached disassembly. The arithmetic is cross-checked against records.
    dis = subprocess.check_output(['objdump', '-d', '-Mintel', '--no-show-raw-insn',
        '--start-address=0x4c1002', '--stop-address=0x4c45ff', str(binary)], text=True)
    instructions = {line.split(':', 1)[0].strip(): line.split(':', 1)[1].strip()
                    for line in dis.splitlines() if ':' in line}
    for addr, operand in {'4c102f': '0x5fd168', '4c3f8a': '0x5f68bb',
                          '4c3fa3': '0x5f68bc', '4c401b': '0x4', '4c4047': '0x4',
                          '4c45bb': '0x5fd19c', '4c45d5': '0x5fd19e'}.items():
        assert operand in instructions[addr], (addr, instructions[addr])
    result = {'source': 'civ2.bak, function 0x4c3f0c; specialist accessor 0x4c1002',
              'source_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'method': 'static decode and replay of game-written records; no new live oracle run',
              'checks': [replay(home, 'FallRome/ROMFALL.scn', 'FallRome/Rules.txt'),
                         replay(home, 'Original/Ph_Auto2.SAV', 'Original/Rules.txt')]}
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == '__main__':
    main()
