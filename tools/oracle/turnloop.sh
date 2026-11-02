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
set -u
N="${1:?turns}"
SCEN_DIR="${2:?scenario dir}"
AUTO="${3:?autosave name}"
OUT="${4:?out dir}"

HERE="$(cd "$(dirname "$0")/../.." && pwd)"
GAME="$CIVTOT_HOME/$SCEN_DIR/$AUTO"

mkdir -p "$OUT"
i=0
while [ "$i" -lt "$N" ]; do
    i=$((i + 1))
    before=$(stat -c %Y "$GAME" 2>/dev/null || echo 0)
    for _ in 1 2 3; do
        (cd "$CIVTOT_HOME" && wine "$HERE/tools/oracle/winject.exe" key 32 >/dev/null 2>&1 &)
        sleep 0.8
    done
    (cd "$CIVTOT_HOME" && wine "$HERE/tools/oracle/winject.exe" key 13 >/dev/null 2>&1 &)
    # wait for the autosave to be rewritten (turn end) — up to 60s
    for _ in $(seq 1 120); do
        sleep 0.5
        now=$(stat -c %Y "$GAME" 2>/dev/null || echo 0)
        [ "$now" != "$before" ] && break
    done
    sleep 2   # let the write finish
    cp "$GAME" "$OUT/turn$(printf %02d "$i").sav" 2>/dev/null || { echo "turn $i: no autosave written"; exit 1; }
    echo "turn $i: $(stat -c %s "$OUT/turn$(printf %02d "$i").sav") bytes, turn-field $(python3 -c "
import struct, sys
print(struct.unpack_from('<h', open(sys.argv[1],'rb').read(), 668)[0])" "$OUT/turn$(printf %02d "$i").sav")"
done
