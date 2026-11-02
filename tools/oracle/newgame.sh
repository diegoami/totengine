#!/usr/bin/env bash
# newgame.sh — drive a fresh STANDARD game through the ToT wizard (RG-1).
#
#   newgame.sh <outdir> [size_downs] [difficulty_downs] [comp_downs]
#
# Assumes the game is NOT running (kills any). Defaults ride the first
# option of every dialog; the *_downs arguments move the selection before
# confirming (e.g. difficulty_downs=5 picks Deity from Chieftain).
# Produces <outdir>/turn00.sav (the load-time autosave).
set -u
OUT="${1:?outdir}"
SIZE_D="${2:-0}"
DIFF_D="${3:-0}"
COMP_D="${4:-0}"

HERE="$(cd "$(dirname "$0")/../.." && pwd)"
export CIVTOT_HOME="${CIVTOT_HOME:?}"
export WINEPREFIX="${WINEPREFIX:?}"
export DISPLAY="${DISPLAY:?}"

XT="$HERE/tools/oracle/xtool"
GAME="$CIVTOT_HOME/Original/Ca_Auto.SAV"
mkdir -p "$OUT"

focus_titled() {  # focus the window whose title matches $1; rc 0 if found
    local id
    id=$(xwininfo -tree -root 2>/dev/null | grep -F "\"$1\"" | awk '{print $1}' | head -1)
    [ -n "$id" ] || return 1
    timeout 5 "$XT" focus "$id" >/dev/null 2>&1
}

wait_title() {  # wait up to $2 s for a window title containing $1
    local i
    for ((i = 0; i < $2 * 2; i++)); do
        xwininfo -tree -root 2>/dev/null | grep -qF "$1" && return 0
        sleep 0.5
    done
    return 1
}

send_downs_then_return() {  # $1 = count of Down presses
    local i
    for ((i = 0; i < $1; i++)); do
        timeout 8 "$XT" key Down
        sleep 0.5
    done
    sleep 0.3
    timeout 8 "$XT" key Return
}

pkill -f '[c]iv2' 2>/dev/null
sleep 2
(cd "$CIVTOT_HOME" && wine "$CIVTOT_HOME/civ2.exe" >/tmp/civ2-ng.log 2>&1 &)
wait_title "The Test of Time" 30 || { echo "launcher never appeared"; exit 1; }
sleep 2
# launcher (first list): Return opens the game-type list
focus_titled "The Test of Time" && timeout 8 "$XT" key Return
sleep 3
# game-type list: Up x4 -> "Original game" (the remembered selection is the
# 5th item, "Begin the Midgard Scenario")
for i in 1 2 3 4; do timeout 8 "$XT" key Up; sleep 0.5; done
timeout 8 "$XT" key Return
sleep 3
# small untitled pre-dialog -> accept
timeout 8 "$XT" key Return
sleep 2

step() {  # $1 title, $2 downs
    wait_title "$1" 15 || { echo "dialog never appeared: $1"; exit 1; }
    sleep 1
    focus_titled "$1"
    send_downs_then_return "$2"
    sleep 2.5
}
step "Select Size of World" "$SIZE_D"
step "Select Difficulty Level" "$DIFF_D"
step "Select Level Of Competition" "$COMP_D"
step "Select Level Of Barbarian Activity" 0
step "Select Game Rules" 0
step "Select Gender" 0
step "Select Your Tribe" 0
step "Please Enter Your Name" 0
step "Select Your City Style" 0
step "In the Beginning" 0
# tutorial popups until quiet
for i in $(seq 1 12); do
    if xwininfo -tree -root 2>/dev/null | grep -q "Civ Tutorial"; then
        focus_titled "Civ Tutorial" || timeout 8 "$XT" key Return
        timeout 8 "$XT" key Return
        sleep 2
    else
        break
    fi
done
# wait for the load-time autosave, then snapshot it
before=$(stat -c %Y "$GAME" 2>/dev/null || echo 0)
for i in $(seq 1 60); do
    sleep 1
    now=$(stat -c %Y "$GAME" 2>/dev/null || echo 0)
    [ "$now" != "$before" ] && break
done
sleep 2
cp "$GAME" "$OUT/turn00.sav" || { echo "no autosave written"; exit 1; }
python3 -c '
import struct, sys
d = open(sys.argv[1], "rb").read()
print("in-world: %d bytes, turn=%d year=%d" % (
    len(d), struct.unpack_from("<h", d, 668)[0],
    struct.unpack_from("<h", d, 670)[0]))' "$OUT/turn00.sav"
