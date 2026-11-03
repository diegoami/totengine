#!/usr/bin/env bash
# stdloop.sh <outdir> <first> <last> — standard-game turn loop that harvests
# the game's own rotating autosaves (Ph_Auto/Ph_Auto2) each turn end.
# NO Ctrl+S: the save dialog's remembered folder + overwrite prompts wedge
# the game (2026-11-03); the autosave is written by the game at turn end.
set -u
export WINEPREFIX="${WINEPREFIX:-$HOME/.wine_civ2tot}"
export DISPLAY="${DISPLAY:-:99}"
export CIVTOT_HOME="${CIVTOT_HOME:?}"
OUT=${1:?outdir}; FIRST=${2:?first}; LAST=${3:?last}
cd "$(dirname "$0")"
mkdir -p "$OUT"

for t in $(seq "$FIRST" "$LAST"); do
  G=$(timeout 15 wine ./winject.exe wins 2>/dev/null \
      | grep -i 'Civilization II: Test' | sed -n 's/^hwnd=\([0-9a-f]*\).*/\1/p' | head -1)
  [ -n "$G" ] || { echo "no game window"; exit 1; }
  timeout 15 wine ./winject.exe focus "0x$G" >/dev/null 2>&1
  # exhaust movers: rotate all 4 directions (settlers pinned on map edges
  # by fixed directions keep their moves and Return then cycles units
  # instead of ending the turn — 2026-11-03 std lab)
  for k in 39 40 37 38 39 40 37 38; do
    timeout 15 wine ./winject.exe key $k >/dev/null 2>&1; sleep 0.7
  done
  ./drain_dialogs.sh 2 >/dev/null 2>&1
  timeout 15 wine ./winject.exe focus "0x$G" >/dev/null 2>&1
  timeout 15 wine ./winject.exe key 13 >/dev/null 2>&1; sleep 3
  timeout 15 wine ./winject.exe key 13 >/dev/null 2>&1; sleep 10
  ./drain_dialogs.sh 3 >/dev/null 2>&1
  NEW=$(ls -t "$CIVTOT_HOME/Original/Ph_Auto.SAV" "$CIVTOT_HOME/Original/Ph_Auto2.SAV" 2>/dev/null | head -1)
  cp "$NEW" "$OUT/turn$(printf %02d "$t").sav"
  T=$(python3 - "$NEW" << 'PYEOF'
import struct, sys
d = open(sys.argv[1], 'rb').read()
print(struct.unpack_from('<hh', d, 668)[0])
PYEOF
)
  echo "turn $t: $(basename "$NEW") counter=$T"
done
