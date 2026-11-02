#!/usr/bin/env bash
# turnloop — end N turns in the running oracle game, snapshotting the
# autosave after each turn (T0-10: same-session consecutive saves).
#
#   turnloop.sh <n-turns> <scenario-dir> <autosave-name> <out-dir>
#
# Requires: the game already in-world under Xvfb :99, CIVTOT_HOME set,
# WINEPREFIX pointing at the game prefix. Keys are injected Win32-level via
# winject (X-level keys never reached the game's DirectDraw window): three
# SPACEs (no-orders for any active units) then RETURN (end turn).
#
# Failure discipline (issue #47): injector failures abort; an autosave that
# is never rewritten aborts; a captured save whose turn field did not
# advance aborts (set ALLOW_STALLED=1 to accept a deliberately stalled
# turn, e.g. for input-methodology probes).
set -u
N="${1:?turns}"
SCEN_DIR="${2:?scenario dir}"
AUTO="${3:?autosave name}"
OUT="${4:?out dir}"

HERE="$(cd "$(dirname "$0")/../.." && pwd)"
GAME="$CIVTOT_HOME/$SCEN_DIR/$AUTO"

mkdir -p "$OUT"

inject() {  # inject <vk-code>
    (cd "$CIVTOT_HOME" && wine "$HERE/tools/oracle/winject.exe" key "$1") \
        >/dev/null 2>&1
}

turn_field() {  # turn_field <save>
    python3 -c '
import struct, sys
print(struct.unpack_from("<h", open(sys.argv[1], "rb").read(), 668)[0])
' "$1"
}

prev_field=""
i=0
while [ "$i" -lt "$N" ]; do
    i=$((i + 1))
    before=$(stat -c %Y "$GAME" 2>/dev/null || echo 0)
    for _ in 1 2 3; do
        inject 32 || { echo "turn $i: injector (space) failed rc=$?"; exit 1; }
        sleep 0.8
    done
    inject 13 || { echo "turn $i: injector (return) failed rc=$?"; exit 1; }
    changed=0
    for _ in $(seq 1 120); do  # wait up to 60s for the autosave rewrite
        sleep 0.5
        now=$(stat -c %Y "$GAME" 2>/dev/null || echo 0)
        if [ "$now" != "$before" ]; then changed=1; break; fi
    done
    if [ "$changed" != 1 ]; then
        echo "turn $i: autosave never rewritten (game blocked on a dialog or injection failed) — FAILING"
        exit 1
    fi
    sleep 2   # let the write finish
    cap="$OUT/turn$(printf %02d "$i").sav"
    cp "$GAME" "$cap" || { echo "turn $i: copy failed"; exit 1; }
    field=$(turn_field "$cap") || exit 1
    if [ -n "$prev_field" ] && [ "$field" = "$prev_field" ] \
       && [ "${ALLOW_STALLED:-0}" != "1" ]; then
        echo "turn $i: turn field did not advance ($field) — FAILING (set ALLOW_STALLED=1 to accept deliberately)"
        exit 1
    fi
    prev_field=$field
    echo "turn $i: $(stat -c %s "$cap") bytes, turn-field $field"
done
