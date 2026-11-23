#!/usr/bin/env python3
"""Measure map knowledge through the native purchase price and commit.

The prepared city accessor and name formatting substitute presentation.
Execute native affordability, treasury, shield and switch-budget logic,
stopping before redraw. A second control has one gold less than the price.
"""
import argparse
import json
import os
from pathlib import Path

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP

from import_event_tail_check import digest
from import_loader_exit_check import rules_refresh
from import_phase_check import CIV, FRAME, STRIDE, frame, read, run
from import_unit_setup_check import prepare
from totctl import parse_rules

CITY = 5


def case(home, binary, path, rules, treasury=None):
    original, player, _, _regions, _ = prepare(home, binary, path, exhaust=False)
    frame(original)
    original.write(FRAME - 0x1990, 1, '<I')
    original.write(FRAME - 0xE9C, 0, '<I')
    rules_refresh(original, rules)
    width, height = (read(original, at, '<h') for at in (0x6705A0, 0x6705A2))
    area = width * height // 2
    original.write(0x6705A4, area, '<h')
    memory = read(original, 0x5DEAD0 + 4 * player, '<I')

    def knowledge():
        return (bytes(original.cpu.mem_read(0x700000, area * 6)),
                bytes(original.cpu.mem_read(memory, area)))

    record = 0x5FD150 + 92 * CITY
    seat = CIV + STRIDE * player
    item = read(original, record + 59, '<b')
    if treasury is not None:
        original.write(seat + 10, treasury, '<i')
    cash = read(original, seat + 10)
    shields = read(original, record + 30, '<H')
    original.write(0x5F5EBC, int(rules['COSMIC']['rows'][4][0]), '<B')
    frame(original)
    original.write(FRAME - 0x6E4, item, '<i')
    original.write(FRAME - 0x6F0, -item, '<i')
    original.write(FRAME - 0x6F4, player, '<i')
    before = knowledge()

    def display(cpu, ip, _size, _data):
        if ip in (0x411C50, 0x41F4B8):
            stack = cpu.reg_read(UC_X86_REG_ESP)
            cpu.reg_write(UC_X86_REG_EAX, CITY if ip == 0x411C50 else 0)
            cpu.reg_write(UC_X86_REG_ESP, stack + 4)
            cpu.reg_write(UC_X86_REG_EIP, read(original, stack, '<I'))

    hook = original.cpu.hook_add(UC_HOOK_CODE, display)
    try:
        run(original, 0x4EB474, {0x4EB5FD}, calls=(0x411C50, 0x41F4B8, 0x566A30))
        price = read(original, FRAME - 0x6E0)
        run(original, 0x4EB644, {0x4EB6C8}, calls=(0x411C50,))
        allowed = read(original, FRAME - 0x6DC) == 1
        assert allowed == (cash >= price)
        if allowed:
            run(original, 0x4EB7CF, {0x4EB83C}, calls=(0x411C50, 0x4C0EE6, 0x4C0EAC))
    finally:
        original.cpu.hook_del(hook)

    assert before == knowledge()
    result = {'city': CITY, 'item': item, 'price': price, 'admitted': allowed,
              'treasury_before': cash, 'treasury_after': read(original, seat + 10),
              'shields_before': shields, 'shields_after': read(original, record + 30, '<H'),
              'explored_tiles': sum(bool(mask & (1 << player)) for mask in before[0][4::6]),
              'tile_bytes_changed': 0, 'remembered_bytes_changed': 0}
    if allowed:
        result['rebased_shields'] = read(original, 0x6537A8)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    prior = json.loads((root / 'docs/evidence/import-saved-production.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(binary) == prior['binary_sha256']
    for key in ('source', 'rules'):
        item = prior[key]
        assert digest((home / item['path']).read_bytes()) == item['sha256']
    context = (home, binary, prior['source']['path'], parse_rules(home / prior['rules']['path']))
    pinned = case(*context)
    short = case(*context, treasury=pinned['price'] - 1)
    assert (pinned['price'], pinned['treasury_after'], pinned['shields_after'], pinned['rebased_shields']) == (19, 6, 20, 20)
    assert (short['admitted'], short['treasury_after'], short['shields_after']) == (False, 18, 12)
    report = {'schema': 1, **{key: prior[key] for key in ('source', 'rules', 'binary_sha256')},
              'pinned': pinned, 'short_treasury': short, 'scope': __doc__.strip()}
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
