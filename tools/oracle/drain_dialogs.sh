#!/usr/bin/env bash
# drain_dialogs.sh [quiet-checks] — dismiss queued blocking windows until quiet.
# Allowlist approach: ANY visible titled window that isn't the game shell or
# an open file dialog gets a Return (default button). Turn processing stacks
# reports (Defense/Domestic advisors, captures, demands, tutorials, save
# toasts) and each one blocks the end-of-turn.
set -u
export WINEPREFIX="${WINEPREFIX:-$HOME/.wine_civ2tot}"
cd "$(dirname "$0")"

PAT=${1:-2}
ALLOW='Civilization II: Test of Time|MrTimer|Default IME|Select File Name|Select Game To Load'
clean=0
for i in $(seq 1 40); do
  D=$(timeout 15 wine ./winject.exe wins 2>/dev/null \
      | awk -F'hwnd=' 'NF>1 {print $2}' \
      | grep -vE '^\s*$' \
      | while read -r rest; do
          hwnd=$(echo "$rest" | sed -n 's/^\([0-9a-f]*\) .*/\1/p')
          title=$(echo "$rest" | sed -n 's/^[0-9a-f]* .* [0-9]+x[0-9]+ //p' | sed 's/^ //')
          echo "$hwnd|$title"
        done \
      | grep -v '(has no name)' \
      | grep -vE "\|($ALLOW)$" \
      | grep -viE '\|(Select File Name|Select Game To Load)' \
      | head -1 | cut -d'|' -f1)
  # simpler + robust: match on the title column of winject output
  D=$(timeout 15 wine ./winject.exe wins 2>/dev/null \
      | grep 'vis=1' \
      | grep -vE "($ALLOW)" \
      | grep -v 'has no name' \
      | sed -n 's/^hwnd=\([0-9a-f]*\).*/\1/p' | head -1)
  if [ -z "$D" ]; then
    clean=$((clean+1))
    if [ "$clean" -ge "$PAT" ]; then echo "drained after $i checks"; exit 0; fi
    sleep 1.5
    continue
  fi
  clean=0
  TITLE=$(timeout 15 wine ./winject.exe wins 2>/dev/null | grep "^hwnd=$D" | sed 's/^.*[0-9]x[0-9]* //')
  echo "dismiss 0x$D ($TITLE)"
  timeout 15 wine ./winject.exe focus "0x$D" >/dev/null 2>&1
  timeout 15 wine ./winject.exe key 13 >/dev/null 2>&1
  sleep 2
done
echo "still busy after 40 rounds" >&2
exit 1
