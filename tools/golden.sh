#!/usr/bin/env bash
# Golden-file harness (T0-9): regenerate & compare the evidence corpus.
#
#   tools/golden.sh check   — compare current outputs against docs/evidence/golden/
#   tools/golden.sh regen   — overwrite the goldens with current outputs
#
# Requires $CIVTOT_HOME (the original game install). Skips with status 0 in CI
# when unset. Outputs are normalized: the absolute $CIVTOT_HOME becomes @HOME@,
# so goldens are machine-independent.
#
# Review finding #26: the harness also refreshes/compares the TOP-LEVEL
# artifacts in docs/evidence/ (the README "Files" table) and hashes every PNG
# render, so a decoder change can never leave a stale picture behind. It also
# runs the Rust↔Python MP crosscheck (#28) and walks the 0x33 ladder (#30.4).
set -u
HERE="$(cd "$(dirname "$0")/.." && pwd)"
GOLDEN="$HERE/docs/evidence/golden"
EV="$HERE/docs/evidence"
MODE="${1:?check|regen}"

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
world-map-rivers|map|World.mp||scale3
romfall-walk|save|FallRome/ROMFALL.scn
ere-walk|save|FallRome/SCN/ROMFALL_ERE.scn
romfall-fra-walk|save|FallRome/SCN/ROMFALL_FRA.scn
romfall-hun-walk|save|FallRome/SCN/ROMFALL_HUN.scn
romfall-sax-walk|save|FallRome/SCN/ROMFALL_SAX.scn
romfall-van-walk|save|FallRome/SCN/ROMFALL_VAN.scn
romfall-vis-walk|save|FallRome/SCN/ROMFALL_VIS.scn
romfall-wre-walk|save|FallRome/SCN/ROMFALL_WRE.scn
va-auto-walk|save|Scenario/FallRome/Va_Auto.SAV
midgard-walk|save|Midgard/Midgard.SCN
midgard-auto-walk|save|Midgard/Oc_Auto.SAV
warring-walk|save|Scenario/china/Warring States.scn
warring-auto-walk|save|Scenario/china/Hu_Auto.SAV
warring-yan-walk|save|Scenario/china/deiYan.sav
warring-qin-walk|save|Scenario/china/deiQin.sav
romfall-scnmap|scnmap|FallRome/ROMFALL.scn
t4-pair-diff|diff|Scenario/FallRome/T4PAIR_A.SAV|Scenario/FallRome/T4PAIR_B.SAV
anticchina-map|map|Scenario/china/AnticChina.MP||rules:Scenario/china/Rules.txt
"

# top-level artifacts (README "Files" table): corpus name -> files under
# docs/evidence/ that must equal the corpus entry's outputs (json = the
# normalized output; png = the render bytes)
artifacts_for() {
  case "$1" in
    rules)             echo "rules-summary.json:json" ;;
    world-map)         echo "world-mp.json:json world-mp.png:png" ;;
    world-map-rivers)  echo "world-mp-rivers.png:png" ;;
    romfall-walk)      echo "romfall-walk.json:json" ;;
    va-auto-walk)      echo "va-auto-walk.json:json" ;;
    warring-walk)      echo "warring-states-walk.json:json" ;;
    romfall-scnmap)    echo "romfall-scnmap.png:png" ;;
    anticchina-map)    echo "anticchina-map.png:png" ;;
  esac
}

normalize() {  # strip machine-specific bits: install path, temp paths
  sed -e "s#$CIVTOT_HOME#@HOME@#g" -e "s#$TMP#@TMP@#g" "$1"
}

mkdir -p "$GOLDEN"
fail=0
while IFS='|' read -r name cmd a b opt; do
  [ -z "$name" ] && continue
  scale_args=""
  [ "${opt:-}" = "scale3" ] && scale_args="--scale 3"
  rules_args=""
  case "${opt:-}" in
    rules:*) rules_args="--rules $CIVTOT_HOME/${opt#rules:}" ;;
  esac
  case "$cmd" in
    map)    python3 "$HERE/tools/totctl.py" "$cmd" "$a" -o "$TMP/$name.png" --json "$TMP/$name.json" $scale_args $rules_args >/dev/null 2>&1 ;;
    scnmap) python3 "$HERE/tools/totctl.py" "$cmd" "$a" -o "$TMP/$name.png" --json "$TMP/$name.json" $scale_args $rules_args >/dev/null 2>&1 ;;
    diff)   python3 "$HERE/tools/totctl.py" "$cmd" "$a" "$b" -o "$TMP/$name.json" >/dev/null 2>&1 ;;
    *)      python3 "$HERE/tools/totctl.py" "$cmd" "$a" -o "$TMP/$name.json" >/dev/null 2>&1 ;;
  esac
  if [ ! -s "$TMP/$name.json" ]; then
    echo "FAIL $name: no output produced"
    fail=1
    continue
  fi
  normalize "$TMP/$name.json" > "$TMP/$name.norm"

  # PNG hash golden sidecar (map/scnmap entries render a PNG)
  png_ok=1
  if [ -f "$TMP/$name.png" ]; then
    sha=$(sha256sum "$TMP/$name.png" | cut -d' ' -f1)
    if [ "$MODE" = regen ]; then
      echo "$sha" > "$GOLDEN/$name.png.sha256"
    elif [ ! -f "$GOLDEN/$name.png.sha256" ]; then
      echo "FAIL $name: png hash golden missing (run: make evidence-regen)"
      fail=1; png_ok=0
    elif [ "$(cat "$GOLDEN/$name.png.sha256")" != "$sha" ]; then
      echo "FAIL $name: png render differs from golden hash"
      fail=1; png_ok=0
    fi
  fi

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
      [ "$png_ok" = 1 ] && echo "ok    $name"
    fi
  fi

  # top-level artifacts must stay identical to the corpus outputs (#26)
  for spec in $(artifacts_for "$name"); do
    file="${spec%%:*}"; kind="${spec##*:}"
    if [ "$kind" = json ]; then
      if [ "$MODE" = regen ]; then
        cp "$TMP/$name.norm" "$EV/$file"
      elif ! diff -q "$EV/$file" "$TMP/$name.norm" >/dev/null 2>&1; then
        echo "FAIL $name: top-level artifact docs/evidence/$file is stale (run: make evidence-regen)"
        fail=1
      fi
    else
      if [ "$MODE" = regen ]; then
        cp "$TMP/$name.png" "$EV/$file"
      elif ! cmp -s "$EV/$file" "$TMP/$name.png"; then
        echo "FAIL $name: top-level render docs/evidence/$file is stale (run: make evidence-regen)"
        fail=1
      fi
    fi
  done
done <<< "$CORPUS"

# Semantic enforcement (#53): the rules entry's own semantic_checks must
# PASS before it can be blessed or compared — a regen must never silently
# accept a shifted parse (e.g. the #44 first-data-row bug).
if [ -s "$TMP/rules.norm" ]; then
  if ! python3 -c '
import json, sys
d = json.load(open(sys.argv[1]))
sc = d.get("semantic_checks", {})
sys.exit(0 if sc.get("pass") is True else 1)
' "$TMP/rules.norm" 2>/dev/null; then
    echo "FAIL rules: semantic_checks.pass is not true — refusing to bless/compare a shifted parse"
    fail=1
  fi
fi

# T1-9: deterministic 50-turn headless run (Rust engine) vs golden
# per-turn fingerprints. Runs wherever the install exists; normalized to
# @HOME@ like the rest of the corpus.
RUN50="$EV/fallrome-50turn.txt"
if ! ( cd "$HERE" && "$(command -v cargo 2>/dev/null || echo "$HOME/.cargo/bin/cargo")" run -q -p totctl -- run "$CIVTOT_HOME/FallRome/ROMFALL.scn" --turns 50 ) > "$TMP/run50.raw" 2> "$TMP/run50.err"; then
  echo "FAIL run50: totctl run failed:"
  head -5 "$TMP/run50.err"
  fail=1
else
  normalize "$TMP/run50.raw" > "$TMP/run50.norm"
  if [ "$MODE" = regen ]; then
    cp "$TMP/run50.norm" "$RUN50"
    echo "regen run50 (rust 50-turn)"
  elif [ ! -f "$RUN50" ]; then
    echo "FAIL run50: golden missing (run: make evidence-regen)"
    fail=1
  elif ! diff -u "$RUN50" "$TMP/run50.norm" > "$TMP/run50.diff"; then
    echo "FAIL run50: 50-turn fingerprints differ from golden (first hunk):"
    head -20 "$TMP/run50.diff"
    fail=1
  else
    echo "ok    run50 (rust 50-turn deterministic)"
  fi
fi

# Rust tot-kit vs Python probe crosschecks (#28 mp, T1-1 rules)
python3 "$HERE/tools/crosscheck_mp.py" "$MODE" || fail=1
python3 "$HERE/tools/crosscheck_rules.py" "$MODE" || fail=1

if [ "$fail" = 0 ] && [ "$MODE" = check ]; then
  echo "golden: all corpus outputs match"
fi
exit $fail
