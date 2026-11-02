#!/usr/bin/env bash
# restart_game.sh — kill any running game instance and start a fresh one.
# Exists so the pkill pattern never shares a command line with the literal
# process name (the documented self-kill trap).
set -u
export CIVTOT_HOME="${CIVTOT_HOME:?}"
export WINEPREFIX="${WINEPREFIX:-$HOME/.wine_civ2tot}"
# the rig's display is always :99 — force it (a caller's inherited DISPLAY=:0
# once leaked in and the game opened windows on the wrong screen)
export DISPLAY=:99

pkill -f '[c]iv2' 2>/dev/null
sleep 2
(cd "$CIVTOT_HOME" && nohup wine "$CIVTOT_HOME/civ2.exe" >/tmp/civ2.log 2>&1 &)
sleep 10
pgrep -ac '[c]iv2'
