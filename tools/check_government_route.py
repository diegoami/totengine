"""Actual native input for the science, rates and government routes (#196).

`route.json` is recorded by the ignored `generated_government_route` client test.
Each step is replayed through real keyboard/pointer input; the dialog title on
screen is checked after every step and every save is compared byte for byte.
"""
import json
from pathlib import Path
import shutil

KEYS = {'ArrowDown': 'Down', 'ArrowUp': 'Up', 'ArrowLeft': 'Left', 'ArrowRight': 'Right', 'Enter': 'Return'}


def title(scene):
    for d in scene['draws']:
        if d.get('kind') == 'text' and d['x'] == 212 and d['y'] == 125 and d['size'] == 22:
            return d['text']
    return None


def run_government(scene, key, click_button, save_dialog, load, wait, work, fixture):
    fixture = Path(fixture)
    route = json.loads((fixture / 'route.json').read_text())
    shutil.copyfile(fixture / 'govstart.cth', work / 'saves' / 'govstart.cth')
    load('govstart')
    assert title(scene()) is None, 'start checkpoint did not return to the map'
    for n, step in enumerate(route):
        op = step['op']
        if op == 'key':
            key(KEYS.get(step['key'], step['key']))
        elif op == 'button':
            click_button(step['label'])
        elif op == 'entry':
            save_dialog(step['value'])
        elif op == 'research':
            if any(d.get('text') == 'Choose research explicitly' for d in scene()['draws']):
                key('Down'); key('Return'); wait(lambda s: s['status'] == 'research-set')
        elif op == 'endturn':
            turn = int(next(d['text'] for d in scene()['draws'] if d.get('text', '').startswith('Turn ')).split()[1])
            key('Ctrl+n')
            if any(d.get('text') == 'End your turn with movable units?' for d in scene()['draws']):
                key('Return')
            wait(lambda s: s['phase'] == 'Human' and any(d.get('text', '').startswith(f'Turn {turn + 1} ') for d in s['draws']))
        elif op == 'compare':
            name = step['name']
            assert (work / 'saves' / (name + '.cth')).read_bytes() == (fixture / (name + '.cth')).read_bytes(), 'checkpoint differs: ' + name
            continue
        else:
            raise RuntimeError('unsupported government step ' + op)
        assert title(scene()) == step.get('title'), f'step {n} {step}: dialog is {title(scene())!r}'
