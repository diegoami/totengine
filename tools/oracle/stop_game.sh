#!/usr/bin/env bash
# stop_game.sh — fully stop the game and the wine session (wineserver too).
# All process killing lives in scripts so the pattern never shares an
# interactive command line with the literal process name.
#
# NOTE: the match is on the process NAME (comm), never -f: the rig's own
# WINEPREFIX path ~/.wine_civ2tot contains the substring "civ2", so a
# -f pattern kills the caller's shell whenever it exports WINEPREFIX
# (struck repeatedly 2026-11-03 — the "mystery SIGKILL").
set -u
export WINEPREFIX="${WINEPREFIX:-$HOME/.wine_civ2tot}"
pkill -9 -x civ2.exe 2>/dev/null
timeout 10 wineserver -k 2>/dev/null
sleep 2
n=$(pgrep -c -x civ2.exe 2>/dev/null || true)
echo "remaining game processes: ${n:-0}"
