"""Real-input two-unit selection route shared by native Linux checker helpers."""
import json
from pathlib import Path
import shutil
import time


def run_selection(scene, key, click_at, save_dialog, load, work, fixture, pixel, resize_window, wait):
    fixture=Path(fixture);meta=json.loads((fixture/'route.json').read_text())
    saves=work/'saves';a,b=meta['source']['slot'],meta['target']['slot']
    for name in ['before','stack']:shutil.copyfile(fixture/(name+'.cth'),saves/('selection'+name+'.cth'))
    def has(text):return any(d.get('text')==text for d in scene()['draws'])
    def unit(slot):return next(d for d in scene()['draws'] if d.get('kind')=='unit_sprite' and d['unit']==slot)
    def expect(slot,moving=True):
        assert unit(slot)['selected'],('selected identity',slot)
        assert has('Moving [V]' if moving else 'Viewing [V]'),('input mode',scene()['status'])
    def point(slot,part=1):
        u=unit(slot);p=meta['hit_points'][part];return u['x']+p['x']*u['w'],u['y']+p['y']*u['h']
    def pointer(slot,part=1):click_at(*point(slot,part))
    def compare(name,wanted):
        key('Ctrl+s');save_dialog(name)
        assert (saves/(name+'.cth')).read_bytes()==(fixture/(wanted+'.cth')).read_bytes(),name
    load('selectionbefore');expect(a)
    pointer(b,0);expect(b)
    assert unit(a)['moves_left']==3 and unit(b)['moves_left']==3
    revision=scene()['revision'];pixels=set();stable=set();pb=point(b);pa=point(a)
    for _ in range(12):
        pixels.add(pixel(*pb));stable.add(pixel(*pa));time.sleep(.1)
    assert len(pixels)>1 and meta['hit_points'][1]['rgb'] in pixels and len(stable)==1,('sprite blink',pixels,stable)
    assert scene()['revision']==revision,'blink generated input'
    # Wait for the body to disappear, then click that hidden body.
    for _ in range(100):
        if pixel(*pb)!=meta['hit_points'][1]['rgb']:break
        time.sleep(.02)
    else:raise AssertionError('unit did not enter hidden blink phase')
    pointer(b);expect(b)
    assert scene()["status"].startswith("Unit activated."),"hidden-phase click missed the unit"
    pointer(a,2);expect(a);key('v');expect(a,False)
    pointer(b,2);expect(b);compare('selectioninert','before')
    resize_window(1280,900);key('z');pointer(a);expect(a);pointer(b);expect(b)
    key('minus');resize_window(1024,768);compare('selectionresized','before')
    load('selectionstack');u=next(d for d in scene()['draws'] if d.get('kind')=='unit_sprite' and d['own'])
    pointer(u['unit']);assert has('Activate a unit')
    for _ in range(20):
        key('Down')
        if any(d.get('text','').startswith('●') and '#'+str(b)+' ·' in d['text'] for d in scene()['draws']):break
    else:raise AssertionError('stack member absent')
    key('Return');expect(b);compare('selectionstackinert','stack')
    load('selectionbefore');pointer(b);expect(b)
    u=unit(b);dx,dy=meta['move_delta'];click_at(u['x']+u['w']/2+dx*32,u['y']+u['h']*.75+dy*16)
    expect(a);assert unit(b)['moves_left']==0;compare('selectionmove','after-move')
    pointer(b);expect(b,False);assert not unit(b)['blink']
    key('v');expect(a);key('space');assert has('End of turn')
    compare('selectionskip','after-skip');key('Return')
    wait(lambda s:s['phase']=='Human' and any(d.get('text','').startswith('Turn '+str(meta['turn']+1)+' ') for d in s['draws']))
    if has('Choose research explicitly'):key('Down');key('Return')
    assert any(d.get('kind')=='unit_sprite' and d['selected'] and d['blink'] and d['moves_left']>0 for d in scene()['draws'])
    compare('selectionturn','after-turn')
