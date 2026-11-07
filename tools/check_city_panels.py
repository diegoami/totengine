"""Actual native panel inputs and full checkpoint/visible-panel assertions."""
import json
from pathlib import Path
import shutil


def run_panels(scene, key, click_at, click_button, save_dialog, load, work, fixture):
    fixture = Path(fixture)
    for route in json.loads((fixture/'route.json').read_text()):
        name = 'panel'+route['name']
        shutil.copyfile(fixture/(route['name']+'-before.cth'), work/'saves'/(name+'.cth'))
        load(name)
        if any(d.get('text') == 'Manage city' for d in scene()['draws']):
            click_button('Manage city')
        click_button('Info')
        keys = {'ArrowDown':'Down', 'ArrowUp':'Up', 'Enter':'Return', 'PageDown':'Next', 'PageUp':'Prior'}
        for step in route['inputs']:
            item = step['input']
            if item['type'] == 'click':
                click_at(item['x'], item['y'])
            elif item['type'] == 'key':
                key(keys.get(item['key'], item['key']))
            else:
                raise RuntimeError('unsupported panel input')
            current = scene()['draws']
            assert any(d.get('kind') == 'text' and d['x'] == 744 and d['y'] == 193 and d['text'] == step['panel'] for d in current), route['name']
            if step.get('text'):
                assert any(d.get('text') == step['text'] for d in current), route['name']
            if step['modal']:
                assert any(d.get('text') == step['modal'] for d in current), route['name']
            else:
                assert not any(d.get('text') == 'Confirm improvement sale' for d in current), route['name']
                if step['pages'] > 1:
                    assert any(d.get('text') == f"List {step['page']} / {step['pages']}" for d in current), route['name']
        assert any(d.get('kind') == 'text' and d['x'] == 744 and d['y'] == 193 and d['text'] == route['panel'] for d in scene()['draws']), route['name']
        assert any(d.get('kind') == 'text' and d['x'] == 18 and d['y'] == 14 and d['text'] == route['heading'] for d in scene()['draws']), route['name']
        key('Ctrl+s'); save_dialog(name+'after')
        assert (work/'saves'/(name+'after.cth')).read_bytes() == (fixture/(route['name']+'-after.cth')).read_bytes(), name
        load(name+'after')
    key('Escape')
