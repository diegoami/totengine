#!/usr/bin/env bash
# saveturn.sh <digits-label> — save the current game via Ctrl+S with retries.
# The dialog's suggested name arrives pre-selected; typing digits replaces it.
set -u
export WINEPREFIX="${WINEPREFIX:-$HOME/.wine_civ2tot}"
export DISPLAY="${DISPLAY:-:99}"
LABEL=${1:?digits-only label}
cd "$(dirname "$0")"

for attempt in 1 2 3 4; do
  # any queued dialog blocks the menu: drain first
  ./drain_dialogs.sh 2 >/dev/null
  G=$(timeout 15 wine ./winject.exe wins 2>/dev/null \
      | grep -i 'Civilization II' | sed -n 's/^hwnd=\([0-9a-f]*\).*/\1/p' | head -1)
  [ -n "$G" ] || { echo "no game window"; exit 1; }
  timeout 15 wine ./winject.exe focus "0x$G" >/dev/null 2>&1
  timeout 15 wine ./winject.exe keydown 17 >/dev/null 2>&1
  sleep 0.2
  timeout 15 wine ./winject.exe key 83 >/dev/null 2>&1
  timeout 15 wine ./winject.exe keyup 17 >/dev/null 2>&1
  sleep 2.5
  W=$(timeout 15 wine ./winject.exe wins 2>/dev/null \
      | grep -i 'Select File Name' | sed -n 's/^hwnd=\([0-9a-f]*\).*/\1/p' | head -1)
  [ -z "$W" ] && { echo "attempt $attempt: no dialog, retrying"; continue; }
  timeout 15 wine ./winject.exe focus "0x$W" >/dev/null 2>&1
  # click into the field first — that selects the suggested name wholesale
  timeout 15 wine ./winject.exe clickat 202 239 >/dev/null 2>&1
  sleep 0.5
  for c in $(echo "$LABEL" | fold -w1); do
    timeout 15 wine ./winject.exe key "$((48 + c))" >/dev/null 2>&1
    sleep 0.25
  done
  timeout 15 wine ./winject.exe key 13 >/dev/null 2>&1
  sleep 3
  # report what actually got saved (name + turn counter) so the caller can
  # verify the turn advanced
  F=$(ls -t /mnt/d/games/CIVTOT/Scenario/FallRome/*.SAV /mnt/d/games/CIVTOT/Scenario/FallRome/*.sav 2>/dev/null | head -1)
  python3 - "$F" << 'PYEOF'
import struct, sys
d = open(sys.argv[1], 'rb').read()
print('saved', sys.argv[1], 'turn/year', struct.unpack_from('<hh', d, 668))
PYEOF
  exit 0
done
echo "failed after retries" >&2
exit 1
