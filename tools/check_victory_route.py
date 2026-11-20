"""Shared-client ENDGAME route through real SDL keyboard input.

Private checkpoints come from installed_victory_input_fixture; the installation
is a scratch copy. Outcome checkpoints must match byte for byte, including
after terminal-input rejection and a save/load cycle.
"""
from pathlib import Path
import shutil


def run_victory(scene, key, save_dialog, load, wait, work, fixture, click_button=None, research=None):
    fixture = Path(fixture)
    initial=fixture/'victoryinitial.cth'
    if initial.exists():
        assert click_button is not None and research is not None
        shutil.copyfile(initial,work/'saves/victoryinitial.cth')
        load('victoryinitial')
        wait(lambda s:any(d.get('text')=='Choose research' for d in s['draws']))
        click_button('Save pending choice');save_dialog('savedviewinitial')
        assert (work/'saves/savedviewinitial.cth').read_bytes()==initial.read_bytes()
        research()
        key('Ctrl+s');save_dialog('savedviewplayed')
        assert (work/'saves/savedviewplayed.cth').read_bytes()==(fixture/'victorystart.cth').read_bytes()
    shutil.copyfile(fixture / 'victorystart.cth', work / 'saves/victorystart.cth')
    load('victorystart')
    key('Ctrl+n')
    if any(d.get('text') == 'End your turn with movable units?' for d in scene()['draws']):
        key('Return')
    wait(lambda s: s['phase'] == 'GameOver')
    for expected in ('Video not played:', 'Controlled victory.', 'Game over: scenario ENDGAME.'):
        assert any(expected in d.get('text', '') for d in scene()['draws']), (expected, scene()['status'])
        key('Return')
    key('Ctrl+n')  # cannot start another turn
    key('Ctrl+s')
    save_dialog('victoryafter')
    expected = (fixture / 'victoryend.cth').read_bytes()
    assert (work / 'saves/victoryafter.cth').read_bytes() == expected
    load('victoryafter')
    assert scene()['phase'] == 'GameOver'
    for _ in range(3):
        key('Return')
    key('Ctrl+s')
    save_dialog('victoryrestored')
    assert (work / 'saves/victoryrestored.cth').read_bytes() == expected
