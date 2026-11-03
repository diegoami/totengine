#!/usr/bin/env bash
# memspool_ctrl.sh start|stop|restart — manage the memspool daemon.
# Exists so a pkill pattern never shares a command line with the literal
# 'memspool.py serve' (the trap that killed the civ2 shells generalizes to
# ANY string: pattern + matching literal in one cmdline = self-kill).
set -eu
cd "$(dirname "$0")"

stop() {
  pkill -f '[m]emspool.py serve' 2>/dev/null || true
  sleep 1
}
start() {
  # the daemon spawns the game from the configured install (no default)
  : "${CIVTOT_HOME:?set CIVTOT_HOME to the original install}"
  (setsid nohup python3 memspool.py serve >/tmp/memspool.log 2>&1 < /dev/null &)
  sleep 1.5
  pgrep -f '[m]emspool.py' >/dev/null && echo "daemon up"
}

case "${1:-}" in
  stop) stop ;;
  start) start ;;
  restart) stop; start ;;
  *) echo "usage: $0 start|stop|restart" >&2; exit 2 ;;
esac
