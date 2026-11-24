#!/usr/bin/env python3
"""Run the prepared opponent hut entry after the witnessed city attack.

Supply the prior exchange, destroyed city and approach position in the
pinned Original save. Execute the stock movement and hut callback with
the witnessed outgoing CRT. Virtualize Windows rectangle writes and
the same closed-panel display and timing calls as the combat witness.
"""
import argparse
import json
import os
from pathlib import Path

from ai_research_adapter_check import prepare as prepare_values
from import_event_tail_check import digest
from import_loader_exit_check import rules_refresh
from import_opponent_combat_view_check import CRT, TILES, run_attack
from import_phase_check import CIV, FRAME, STRIDE, frame, read
from import_unit_setup_check import prepare, UNIT_RECORDS
from totctl import parse_rules


def measure(home, binary, attack, contact):
    path = attack['source']['path']
    original, viewer, _, regions, _ = prepare(home, binary, path, exhaust=False)
    frame(original)
    original.write(FRAME - 0x1990, 1, '<I')
    original.write(FRAME - 0xe9c, 0, '<I')
    rules_refresh(original, parse_rules(home / attack['rules']['path']))
    prepare_values(home, original, path)
    cpu = original.cpu
    cpu.mem_map(0, 4096)
    original.write(0, 0xffffffff, '<I')
    original.write(0x5D156C, 0, '<B')
    width, height = (read(original, at, '<h') for at in (0x6705A0, 0x6705A2))
    area = width * height // 2
    original.write(0x6705A4, area, '<h')
    original.write(0x607648, 44, '<h')
    original.write(0x607655, 6, '<B')

    def flags(x, y):
        return TILES + 6 * (y * (width // 2) + x // 2) + 1

    original.write(flags(48, 28), read(original, flags(48, 28), '<B') & ~3, '<B')
    original.write(UNIT_RECORDS + 40 * 36 + 30, 0, '<I')
    city = regions[2][0] + 92 * 9
    original.write(city + 11, 0, '<B')
    original.write(city + 86, 0, '<I')
    original.write(0x607668, 9, '<h')
    recipient = CIV + 6 * STRIDE
    original.write(recipient + 112, read(original, recipient + 112, '<h') - 1, '<h')
    prior = contact['engine_seed_control']
    for (civ, other), word in zip(((5, 6), (6, 5)), prior['treaty_words'], strict=True):
        original.write(CIV + civ * STRIDE + 40 + other * 4, word, '<I')
    for civ, advance, *_ in prior['grant_arguments']:
        address = CIV + civ * STRIDE + 96 + advance // 8
        original.write(address, read(original, address, '<B') | (1 << (advance % 8)), '<B')
        address = 0x6076D2 + advance
        original.write(address, read(original, address, '<B') | (1 << civ), '<B')
    original.write(recipient + 24, 7, '<B')
    original.write(recipient + 16, 12, '<h')
    original.write(recipient + 18, 1, '<h')

    unit = UNIT_RECORDS + 37 * 40
    assert (read(original, unit, '<h'), read(original, unit + 2, '<h')) == (29, 13)
    original.write(unit, 28, '<h')
    original.write(unit + 2, 12, '<h')
    original.write(flags(29, 13), read(original, flags(29, 13), '<B') & ~1, '<B')
    original.write(flags(28, 12), read(original, flags(28, 12), '<B') | 1, '<B')
    seed = attack['closed_panel_attack']['crt_after']
    original.write(CRT, seed, '<I')
    cache = read(original, 0x5DEAD0 + 4 * viewer, '<I')

    def snapshot():
        visible = bytes(mask & (1 << viewer) for mask in cpu.mem_read(TILES, 6 * area)[4::6])
        return ({
            'position': [read(original, unit + i, '<h') for i in (0, 2)],
            'spent': read(original, unit + 10, '<B'),
            'direction': read(original, unit + 13, '<B'),
            'advance_byte': read(original, recipient + 96, '<B'),
            'acquired': read(original, recipient + 24, '<B'),
            'research_progress': read(original, recipient + 16, '<h'),
            'research_target': read(original, recipient + 18, '<h'),
            'explored_tiles': sum(bool(mask) for mask in visible),
        }, visible, bytes(cpu.mem_read(cache, area)))

    before, explored, memory = snapshot()
    grants = []
    result, hosts, display, observes = run_attack(
        original, (37, 4, 3), memory_rects=True, grant_trace=grants,
    )
    after, explored_after, memory_after = snapshot()
    assert result == 1 and (explored, memory) == (explored_after, memory_after)
    assert grants == [[6, 1, 6, 0, 0]] and observes == [[37, 1]]
    assert (before['advance_byte'], after['advance_byte'], after['acquired']) == (0, 2, 8)
    return {
        'arguments': [37, 4, 3], 'before': before, 'after': after,
        'crt_before': seed, 'crt_after': read(original, CRT, '<I'), 'return': result,
        'grant_arguments': grants, 'observation_calls': observes,
        'viewer_map_and_memory_unchanged': True,
        'host_substitutes': dict(sorted((name, hosts.count(name)) for name in set(hosts))),
        'display_substitutes': display,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent / 'docs/evidence'
    attack = json.loads((root / 'import-opponent-city-combat.json').read_text())
    contact = json.loads((root / 'import-opponent-contact.json').read_text())
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    assert digest(binary) == attack['binary_sha256'] == contact['binary_sha256']
    for role in ('source', 'rules'):
        item = attack[role]
        assert item == contact[role] and digest((home / item['path']).read_bytes()) == item['sha256']
    report = {'schema': 1, **{key: attack[key] for key in ('source', 'rules', 'binary_sha256')},
              'hut': measure(home, binary, attack, contact), 'scope': __doc__.strip()}
    with args.out.open('x') as output:
        output.write(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
