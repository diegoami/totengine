#!/usr/bin/env bash
# stop_game.sh — fully stop the game and the wine session (wineserver too).
# All process killing lives in scripts so the pattern never shares an
# interactive command line with the literal process name.
set -u
export WINEPREFIX="${WINEPREFIX:-$HOME/.wine_civ2tot}"
pkill -9 -f '[c]iv2' 2>/dev/null
timeout 10 wineserver -k 2>/dev/null
sleep 2
n=$(pgrep -c -f '[c]iv2' 2>/dev/null || true)
echo "remaining game processes: ${n:-0}"
