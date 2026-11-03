#!/usr/bin/env bash
# cityloop.sh <expected-new-turn> [n_units] — advance ONE game turn and save.
# Skips all units (SPACE x n), ends the turn (RETURN), waits out the AI,
# drains queued event dialogs, then saves via saveturn.sh. Verifies the
# save's turn counter reached the expected value; retries the end-turn key
# if not. Exits 0 with the newest save path on stdout.
set -u
export WINEPREFIX="${WINEPREFIX:-$HOME/.wine_civ2tot}"
export DISPLAY="${DISPLAY:-:99}"
EXPECT=${1:?expected new turn number}
NUNITS=${2:-170}
SCENDIR="${SCENDIR:-${CIVTOT_HOME:?set CIVTOT_HOME}/Scenario/FallRome}"
export SCENDIR  # saveturn.sh harvests from the same directory
cd "$(dirname "$0")"

G=$(timeout 15 wine ./winject.exe wins 2>/dev/null \
    | grep -i 'Civilization II' | sed -n 's/^hwnd=\([0-9a-f]*\).*/\1/p' | head -1)
[ -n "$G" ] || { echo "no game window"; exit 1; }

# 1. skip units then end turn
timeout 15 wine ./winject.exe focus "0x$G" >/dev/null 2>&1
timeout 180 wine ./winject.exe seq 32 "$NUNITS" 130 >/dev/null 2>&1
timeout 15 wine ./winject.exe key 13 >/dev/null 2>&1
sleep 20
./drain_dialogs.sh 2

# 2. retry end-turn until the saved counter moves
for try in 1 2 3 4 5; do
  ./saveturn.sh 9$try >/dev/null 2>&1 || true
  F=$(ls -t "$SCENDIR"/*.SAV "$SCENDIR"/*.sav 2>/dev/null | head -1)
  T=$(python3 -c "
import struct
d = open('$F','rb').read()
print(struct.unpack_from('<hh', d, 668)[0])" 2>/dev/null || echo err)
  echo "try $try: $(basename "$F") turn=$T (want $EXPECT)"
  if [ "$T" = "$EXPECT" ]; then echo "$F"; exit 0; fi
  timeout 15 wine ./winject.exe key 13 >/dev/null 2>&1
  sleep 15
  ./drain_dialogs.sh 2
done
echo "turn did not reach $EXPECT" >&2
exit 1
