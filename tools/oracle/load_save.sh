#!/usr/bin/env bash
# load_save.sh <path relative to the install root> — start the game under
# the memspool daemon and load a saved game, e.g.
#   load_save.sh original/ph_auto2.sav
# The launcher's "Load a Saved Game" item is picked by launcher_pick.py.
# The file dialog opens in the last-used folder, so the path is typed
# relative to D:\ after moving up once with the address combo: the keys
# go to the filename field only after a click into it (saveturn.sh's
# trick). Lowercase letters, digits, '.', '\' and '_' are supported.
set -u
REL=${1:?path relative to the install root, e.g. original/ph_auto2.sav}
cd "$(dirname "$0")"
export CIVTOT_HOME="${CIVTOT_HOME:?}"
export WINEPREFIX="${WINEPREFIX:-$HOME/.wine_civ2tot}"
export DISPLAY="${DISPLAY:-:99}"
XT=./xtool
wi() { timeout 15 wine ./winject.exe "$@" >/dev/null 2>&1; }
python3 memspool.py restart >/dev/null 2>&1
for _ in $(seq 1 80); do xwininfo -tree -root | grep -q '"The Test of Time"' && break; sleep 0.5; done
sleep 2
id=$(xwininfo -tree -root | grep -F '"The Test of Time"' | awk '{print $1}' | head -1)
timeout 5 "$XT" focus "$id" >/dev/null; timeout 8 "$XT" key Return; sleep 3
python3 launcher_pick.py 6 || exit 1
timeout 8 "$XT" key Return; sleep 3
W=$(timeout 15 wine ./winject.exe wins 2>/dev/null | grep -i 'Select Game To Load' \
    | sed -n 's/^hwnd=\([0-9a-f]*\).*/\1/p' | head -1)
[ -n "$W" ] || { echo "no load dialog"; exit 1; }
wi focus "0x$W"; wi clickat 202 239; sleep 0.5
# the install root is D:\ in the prefix: type an absolute path
type_key() {
    case "$1" in
        .) wi key 190 ;;
        \\) wi key 220 ;;
        :) wi keydown 16; wi key 186; wi keyup 16 ;;
        _) wi keydown 16; wi key 189; wi keyup 16 ;;
        *) wi key "$(printf '%d' "'$(echo "$1" | tr a-z A-Z)")" ;;
    esac
    sleep 0.12
}
for c in $(echo "d:\\${REL//\//\\}" | fold -w1); do type_key "$c"; done
wi key 13; sleep 8
./drain_dialogs.sh 4 >/dev/null 2>&1
python3 -c "
import sys; sys.path.insert(0, '.')
import unitram as u
print('loaded: turn', u.u16(0x607648), 'units', u.u16(0x607666), 'cities', u.u16(0x607668))"
