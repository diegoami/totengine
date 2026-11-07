"""Actual SDL double-click city entry with an unchanged full checkpoint."""
from pathlib import Path
import shutil


def run_city_open(scene, key, double_click_at, save_dialog, load, wait, work, fixture):
    fixture=Path(fixture)
    shutil.copyfile(fixture,work/'saves/cityopen.cth')
    load('cityopen')
    if any(d.get('text')=='Return to map [Esc]' for d in scene()['draws']):key('Escape')
    city=next(d for d in scene()['draws'] if d.get('kind')=='sprite' and d['sprite'].startswith('city-'))
    double_click_at(city['x']+city['w']/2,city['y']+city['h']*.6)
    wait(lambda s:any(d.get('kind')=='text' and d['x']==18 and d['y']==14 and 'population' in d['text'] for d in s['draws']))
    key('Ctrl+s');save_dialog('citydouble')
    assert (work/'saves/citydouble.cth').read_bytes()==fixture.read_bytes(), 'double click changed gameplay'
    key('Escape')
