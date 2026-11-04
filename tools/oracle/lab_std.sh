#!/usr/bin/env bash
# lab_std.sh <outdir> [size_downs] [diff_downs] — fresh STANDARD game under
# the memspool daemon (RAM access), then N turns of: save + RAM snapshot.
# The wizard flow is newgame.sh's; the game process is spawned by the daemon
# so memspool can read it (yama ptrace_scope).
set -u
OUT=${1:?outdir}
SIZE_D=${2:-0}
DIFF_D=${3:-0}
N="${N_TURNS:-10}"
mkdir -p "$OUT"
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
step "Select Your City Style" 0
step "In the Beginning" 0
for i in $(seq 1 12); do
    if xwininfo -tree -root 2>/dev/null | grep -q "Civ Tutorial"; then
        focus_titled "Civ Tutorial" || timeout 8 "$XT" key Return
        timeout 8 "$XT" key Return
        sleep 2
    else
        break
    fi
done
echo "in-world; starting turn loop"

# 2. per turn: end turn -> drain -> save -> RAM snapshot
G=$(timeout 15 wine ./winject.exe wins 2>/dev/null \
    | grep -i 'Civilization II' | sed -n 's/^hwnd=\([0-9a-f]*\).*/\1/p' | head -1)
[ -n "$G" ] || { echo "no game window"; exit 1; }
for t in $(seq 1 "$N"); do
    timeout 15 wine ./winject.exe focus "0x$G" >/dev/null 2>&1
    timeout 60 wine ./winject.exe seq 32 12 200 >/dev/null 2>&1   # few units early
    timeout 15 wine ./winject.exe key 13 >/dev/null 2>&1
    sleep 12
    ./drain_dialogs.sh 3 >/dev/null 2>&1
    ./saveturn.sh 7$t >/dev/null 2>&1 || true
    ./drain_dialogs.sh 2 >/dev/null 2>&1
    F=$(ls -t "$CIVTOT_HOME"/Original/*.SAV "$CIVTOT_HOME"/Original/*.sav 2>/dev/null | head -1)
    if [ -n "$F" ]; then
        cp "$F" "$OUT/turn$(printf %02d "$t").sav"
        python3 memspool.py snapshot dir="$OUT/ram$t" >/dev/null 2>&1 || true
        T=$(python3 -c "
import struct
d = open('$F','rb').read()
print(struct.unpack_from('<hh', d, 668)[0])" 2>/dev/null)
        echo "turn $t: saved (counter=$T) + RAM snapshot"
    else
        echo "turn $t: NO SAVE"
    fi
done
