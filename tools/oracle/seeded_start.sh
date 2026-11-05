#!/usr/bin/env bash
# seeded_start.sh <outdir> — a REPRODUCIBLE fresh standard game (RG-2).
#
# A new game is fully determined once four inputs are pinned while the
# City Style dialog is open (evidence §RG-2 reproducible starts):
#   NR      the NR LCG state           (VA 0x5dd53c)        default 0x1234
#   CRT     the MSVC CRT rand() state  (VA 0x5e2590)        default 0x5678
#   PARAMS  the five world parameters  (5 × i32 @0x5ceaa8, each -1/0/1;
#           drawn with rand()%3-1 BEFORE this dialog, so they carry the
#           startup tick seed)                              default 0,0,0,0,0
#   MAPSEED the map's resource seed: the instruction at 0x56fa6c
#           (`call GetTickCount-wrapper`) is patched to `mov eax, MAPSEED`
#                                                           default 0x1357
# Same four values ⇒ byte-identical turn-1 autosave.
# DIALOG_GUARD_DIR=<outside-Git directory> gates wizard Returns on visually
# approved screenshots (dialog_guard.py); changed dialog pixels pause again.
# HOOK_OUT=<file> [HOOK_SECS] [HOOK_ARGS="--skip 0x403000:0x40a000 --state"]
# logs the CRT rand() draws after the City Style answer (randhook.py).
# DUMP_INTRO=1 dumps tiles/landmass table/map header/unit table from RAM
# while "In the Beginning" is open (after placement, before AI moves). The wizard answers
# are newgame.sh's defaults (Downs via $2/$3 like lab_std.sh).
# Output: every *.sav the run wrote under $CIVTOT_HOME (fresh by mtime,
# named <folder>_<file>), plus placement.txt (live unit table).
# The install is left as found: existing saves are backed up first and
# restored after the harvest, and saves this run created are removed
# (other tests read e.g. Original/Ph_Auto.SAV, the std-lab autosave). The
# game is stopped before the restore; KEEP_GAME=1 leaves it running for a
# follow-up probe, and then its later autosaves land in the install.
set -u
OUT=${1:?outdir}
SIZE_D=${2:-0}
DIFF_D=${3:-0}
NR="${NR:-0x1234}"
CRT="${CRT:-0x5678}"
PARAMS="${PARAMS:-0,0,0,0,0}"
MAPSEED="${MAPSEED:-0x1357}"
mkdir -p "$OUT"
rm -f "$OUT"/*.sav "$OUT"/*.SAV "$OUT/placement.txt"
touch "$OUT/.marker"
BK="$OUT/.install_backup"
rm -rf "$BK"; mkdir -p "$BK"
(cd "${CIVTOT_HOME:?}" && find . -iname "*.sav" -exec cp -p --parents {} "$BK" \;)
restore_install() {
    # the running game autosaves later (seen 2026-11-04: Ph_Auto.SAV
    # rewritten after an exit-time restore), so stop it first unless the
    # caller keeps it for a follow-up probe (then the install may change)
    if [ -n "${KEEP_GAME:-}" ]; then
        echo "KEEP_GAME: game left running; restoring now, but later autosaves will overwrite the install's saves"
    else
        ./stop_game.sh >/dev/null 2>&1
    fi
    (cd "$BK" && find . -iname "*.sav" -exec cp -p --parents {} "$CIVTOT_HOME" \;)
    find "$CIVTOT_HOME" -iname "*.sav" -newer "$OUT/.marker" | while read -r f; do
        [ -e "$BK/${f#"$CIVTOT_HOME"/}" ] || rm -f "$f"
    done
}
trap restore_install EXIT
cd "$(dirname "$0")"
export CIVTOT_HOME="${CIVTOT_HOME:?}"
export WINEPREFIX="${WINEPREFIX:-$HOME/.wine_civ2tot}"
export DISPLAY="${DISPLAY:-:99}"
XT=./xtool
focus_titled() {
    local id
    id=$(xwininfo -tree -root 2>/dev/null | grep -F "\"$1\"" | awk '{print $1}' | head -1)
    [ -n "$id" ] || return 1
    timeout 5 "$XT" focus "$id" >/dev/null 2>&1
}
wait_title() {
    local i
    for ((i = 0; i < $2 * 2; i++)); do
        xwininfo -tree -root 2>/dev/null | grep -qF "$1" && return 0
        sleep 0.5
    done
    return 1
}
send_downs_then_return() {
    local i
    for ((i = 0; i < $1; i++)); do
        timeout 8 "$XT" key Down; sleep 0.5
    done
    sleep 0.3
    if [ -n "${DIALOG_GUARD_DIR:-}" ]; then
        python3 dialog_guard.py "$GUARD_TITLE" "$DIALOG_GUARD_DIR" || exit 1
    fi
    timeout 8 "$XT" key Return
}

# 1. game under the daemon
python3 memspool.py restart >/dev/null 2>&1
wait_title "The Test of Time" 40 || { echo "launcher never appeared"; exit 1; }
sleep 2
focus_titled "The Test of Time" && timeout 8 "$XT" key Return
sleep 3
# game-type list: it remembers the last choice and Up/Down wrap, so pick
# by reading the selected radio (GAME: 0 = Original, 2 = Fantasy, ...)
python3 launcher_pick.py "${GAME:-0}" || exit 1
timeout 8 "$XT" key Return
sleep 3
timeout 8 "$XT" key Return
sleep 2

step() {
    wait_title "$1" 15 || { echo "dialog never appeared: $1"; exit 1; }
    sleep 1
    focus_titled "$1"
    GUARD_TITLE="$1"
    send_downs_then_return "$2"
    sleep 2.5
}
step "Select Size of World" "$SIZE_D"
step "Select Difficulty Level" "$DIFF_D"
step "Select Level Of Competition" 0
step "Select Level Of Barbarian Activity" 0
step "Select Game Rules" 0
step "Select Gender" 0
step "Select Your Tribe" 0
step "Please Enter Your Name" 0
wait_title "Select Your City Style" 15 || { echo "no city style"; exit 1; }
sleep 1
# Inspect before pinning: a potentially long visual check must not introduce
# a presentation-dependent interval between the pinned RNGs and generation.
focus_titled "Select Your City Style"
if [ -n "${DIALOG_GUARD_DIR:-}" ]; then
    python3 dialog_guard.py "Select Your City Style" "$DIALOG_GUARD_DIR" || exit 1
fi
python3 - "$NR" "$CRT" "$PARAMS" "$MAPSEED" <<'PY' || exit 1
import sys, struct
sys.path.insert(0, '.')
import unitram as u
nr, crt, ms = int(sys.argv[1], 0), int(sys.argv[2], 0), int(sys.argv[4], 0)
params = [int(v) for v in sys.argv[3].split(',')]
assert len(params) == 5 and all(v in (-1, 0, 1) for v in params), params
call = bytes.fromhex('e86fb4eaff')  # call 0x41aee0 (tick wrapper)
cur = u.read(0x56fa6c, 5)
assert cur == call or cur[:1] == b'\xb8', f"unexpected code at 0x56fa6c: {cur.hex()}"
print("params before", struct.unpack('<5i', u.read(0x5ceaa8, 20)))
u.write(0x5dd53c, struct.pack('<I', nr))
u.write(0x5e2590, struct.pack('<I', crt))
u.write(0x5ceaa8, struct.pack('<5i', *params))
u.write(0x56fa6c, b'\xb8' + struct.pack('<I', ms))
print(f"pinned NR={nr:#x} CRT={crt:#x} params={params} mapseed={ms:#x}")
PY
focus_titled "Select Your City Style"
if [ -n "${HOOK_OUT:-}" ]; then
    # log the CRT draws this answer triggers (randhook.py presses Return)
    # shellcheck disable=SC2086
    python3 randhook.py "${HOOK_SECS:-25}" "$HOOK_OUT" --return ${HOOK_ARGS:-} || exit 1
else
    timeout 8 "$XT" key Return
fi
sleep 2.5
if [ -n "${DUMP_INTRO:-}" ]; then
    # post-placement, pre-AI-move world: dump it from RAM while the intro
    # dialog holds the game (evidence §RG-2 placement)
    wait_title "In the Beginning" 15 || { echo "no intro"; exit 1; }
    python3 - "$OUT" <<'PY' || exit 1
import sys, struct
sys.path.insert(0, '.')
import unitram as u
out = sys.argv[1]
w2, h = struct.unpack('<hh', u.read(0x6705a0, 4))
area = (w2 // 2) * h
open(out + '/intro_tiles.bin', 'wb').write(u.read(u.u32(0x5deaa8), 6 * area))
open(out + '/intro_landmass.bin', 'wb').write(u.read(0x61bca8, 10240))
open(out + '/intro_maphdr.bin', 'wb').write(u.read(0x6705a0, 16))
units, _ = u.bases()
open(out + '/intro_units.bin', 'wb').write(u.read(units, 40 * 64))
print(f"intro dump: {w2}x{h}, {area} tiles")
PY
fi
step "In the Beginning" 0
sleep 4
python3 - "$OUT" <<'PY'
import sys, struct
sys.path.insert(0, '.')
import unitram as u
units, _ = u.bases()
rows = []
for i in range(60):
    r = u.read(units + 40 * i, 40)
    if r[30:32] == b'\0\0' and r[0:4] == b'\0' * 4:
        continue
    x, y = struct.unpack_from('<hh', r, 0)
    rows.append(f"{i} {x} {y} type={r[8]} owner={r[9]}")
txt = (f"settings {u.read(0x607658, 6).hex()}\n"
       f"maphdr {u.read(0x6705a0, 16).hex()}\n" + "\n".join(rows) + "\n")
open(sys.argv[1] + '/placement.txt', 'w').write(txt)
print(txt)
PY
# the turn-1 autosave (rotating slots: harvest everything newer than the marker)
sleep 2
find "$CIVTOT_HOME" -iname "*.sav" -newer "$OUT/.marker" | while read -r f; do
    cp "$f" "$OUT/$(basename "$(dirname "$f")")_$(basename "$f")"
done
n=$(find "$OUT" -maxdepth 1 -iname "*.sav" | wc -l)
[ "$n" -gt 0 ] || { echo "no fresh save"; exit 1; }
find "$OUT" -maxdepth 1 -iname "*.sav" -exec ls -l {} +
