#!/usr/bin/env python3
"""Retain small derived live-hut evidence and engine comparison vectors.

Inputs are JSON from oracle/hut_entry.py on fresh loads of the reference
save. The install is only read; no original game records are copied out.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--gold', required=True, type=Path)
    parser.add_argument('--vectors', required=True, type=Path)
    args = parser.parse_args()
    specs = [('gold', args.gold)] + [(name, args.directory / f'{file}.json') for name, file in (
        ('mercenary', 'mercenary-final'), ('scroll', 'scroll'), ('late-tribe', 'tribe-final'),
        ('nomad', 'nomad-final'), ('barbarians', 'barbarians-final'), ('nothing', 'nothing-final'),
        ('forced-tribe', 'barbarians'))]
    cases = []
    lines = ['# Derived from controlled live hut entries. See huts-live.json for protocol and scope.']
    for name, path in specs:
        raw = json.loads(path.read_text())
        before, after = raw['before'], raw['after']
        word = after['hook_words']
        code = word[3]
        kind = {0: 'city', 1: 'unit', 2: 'gold', 3: 'barbarians', 4: 'advance', 5: 'unit'}[code]
        value = word[5] if code == 0 else word[4] if code in (1, 5) else word[6] if code == 2 else len(after['new_units'])
        if code == 3 and not value:
            kind, value = 'nothing', 0
        if code == 4:
            acquired = [t for t in range(100) if not before['techs'][t // 8] & (1 << (t % 8)) and after['techs'][t // 8] & (1 << (t % 8))]
            assert len(acquired) == 1, (name, acquired)
            value = acquired[0]
        outcome = {'kind': kind, 'value': value, 'crt_after': word[2],
                   'gameplay_draws': after['gameplay_draws'], 'presentation_draws': after['renderer_draws'],
                   'treasury': after['treasury'], 'techs': after['techs'],
                   'new_units': after['new_units']}
        for field in ('new_city', 'explorer_home', 'explorer_stack_links'):
            if field in after:
                outcome[field] = after[field]
        cases.append({'case': name, 'setup': before, 'outcome': outcome})
        tx, ty = before['target']
        values = (name, before['seed'], before['unit'], tx, ty, before['fertility'], before['site_bits'],
                  before['turn'], before['year'], before['level'])
        lines.append('case ' + ' '.join(map(str, values)))
        for unit, home in before.get('homed_settlers', []):
            lines.append(f'home {unit} {home}')
        lines.append('result ' + ' '.join(map(str, (kind, value, word[2], after['treasury'], len(after['gameplay_draws']), *after['techs']))))
        for unit in after['new_units']:
            lines.append('unit ' + ' '.join(map(str, (unit['index'], unit['type'], unit['owner'], unit['x'], unit['y'], unit['home'], int(unit['veteran'])))))
        if 'new_city' in after:
            city = after['new_city']
            lines.append('city ' + ' '.join(map(str, (city['index'], city['size'], city['owner'], *city['buildings']))))
        if 'explorer_home' in after:
            lines.append(f"explorer-home {after['explorer_home']}")
        lines.append('end')
    home = Path(os.environ['CIVTOT_HOME'])
    sources = {name: hashlib.sha256((home / name).read_bytes()).hexdigest()
               for name in ('civ2.exe', 'civ2.bak', 'Original/Ph_Auto2.SAV', 'Original/Rules.txt')}
    args.vectors.write_text('\n'.join(lines) + '\n')
    print(json.dumps({'source': 'live Wine + ToTPP oracle; Original/Ph_Auto2.SAV freshly loaded per case',
                      'source_sha256': sources,
                      'protocol': 'CRT pinned at 0x54ab80 entry; actual original rand executes; presentation call sites 0x55852f, 0x581d96 and 0x582b9b use a separate CRT state; entry/exit/RNG/cave patches restored after each case',
                      'scope': 'surface land huts; controlled location, fertility, date, barb level and settler homes where listed; compares gameplay outcomes/CRT, unit slots/types/home/stack and tribe population/buildings; no presentation RNG parity claim',
                      'cases': cases}, indent=2))


if __name__ == '__main__':
    main()
