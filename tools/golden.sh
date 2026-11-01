#!/usr/bin/env bash
# Golden-file harness (T0-9): regenerate & compare the evidence corpus.
#
#   tools/golden.sh check   — compare current outputs against docs/evidence/golden/
#   tools/golden.sh regen   — overwrite the goldens with current outputs
#
# Requires $CIVTOT_HOME (the original game install). Skips with status 0 in CI
# when unset. Outputs are normalized: the absolute $CIVTOT_HOME becomes @HOME@,
# so goldens are machine-independent.
set -u
HERE="$(cd "$(dirname "$0")/.." && pwd)"
GOLDEN="$HERE/docs/evidence/golden"
MODE="${1:-check}"

if [ -z "${CIVTOT_HOME:-}" ]; then
  echo "golden: CIVTOT_HOME not set — skipping (game data not available here)"
  exit 0
fi
if [ ! -d "$CIVTOT_HOME" ]; then
  echo "golden: CIVTOT_HOME=$CIVTOT_HOME is not a directory" >&2
  exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# corpus: name|command-args (paths relative to CIVTOT_HOME)
CORPUS="
rules|rules|Rules.txt
world-map|map|World.mp
romfall-walk|save|FallRome/ROMFALL.scn
ere-walk|save|FallRome/SCN/ROMFALL_ERE.scn
va-auto-walk|save|Scenario/FallRome/Va_Auto.SAV
warring-walk|save|Scenario/china/Warring States.scn
romfall-scnmap|scnmap|FallRome/ROMFALL.scn
va-autosave-diff|diff|Scenario/FallRome/Va_Auto2.SAV|Scenario/FallRome/Va_Auto.SAV
anticchina-reject|map|Scenario/china/AnticChina.MP
"

normalize() {  # strip machine-specific bits: install path, temp paths
  sed -e "s#$CIVTOT_HOME#@HOME@#g" -e "s#$TMP#@TMP@#g" "$1"
}

mkdir -p "$GOLDEN"
fail=0
while IFS='|' read -r name cmd a b; do
  [ -z "$name" ] && continue
  case "$cmd" in
    map)    python3 "$HERE/tools/totctl.py" "$cmd" "$a" -o "$TMP/$name.png" --json "$TMP/$name.json" >/dev/null 2>&1 ;;
    scnmap) python3 "$HERE/tools/totctl.py" "$cmd" "$a" -o "$TMP/$name.png" --json "$TMP/$name.json" >/dev/null 2>&1 ;;
    diff)   python3 "$HERE/tools/totctl.py" "$cmd" "$a" "$b" -o "$TMP/$name.json" >/dev/null 2>&1 ;;
    *)      python3 "$HERE/tools/totctl.py" "$cmd" "$a" -o "$TMP/$name.json" >/dev/null 2>&1 ;;
  esac
  if [ ! -s "$TMP/$name.json" ]; then
    echo "FAIL $name: no output produced"
    fail=1
    continue
  fi
  normalize "$TMP/$name.json" > "$TMP/$name.norm"
  if [ "$MODE" = regen ]; then
    cp "$TMP/$name.norm" "$GOLDEN/$name.json"
    echo "regen $name"
  else
    if [ ! -f "$GOLDEN/$name.json" ]; then
      echo "FAIL $name: golden missing (run: make evidence-regen)"
      fail=1
    elif ! diff -u "$GOLDEN/$name.json" "$TMP/$name.norm" > "$TMP/$name.diff"; then
      echo "FAIL $name: output differs from golden (first hunk):"
      head -20 "$TMP/$name.diff"
      fail=1
    else
      echo "ok    $name"
    fi
  fi
done <<< "$CORPUS"

if [ "$fail" = 0 ] && [ "$MODE" = check ]; then
  echo "golden: all corpus outputs match"
fi
exit $fail
