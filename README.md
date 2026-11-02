# ToTEngine — Civilization II: Test of Time, recreated

Goal: an open, cross-platform program that **replicates the gameplay of Civ2: Test
of Time (ToT)**, with first-class support for the community's **scenarios** (both the
classic 1999-era macro-events kind and the modern ToTPP/Lua kind).

The engine is a *recreation*, not a bundle: it requires the user's original ToT
installation and reads its data files directly.

| Document | Purpose |
|---|---|
| [BACKLOG.md](BACKLOG.md) | Working queue: epics, stories, difficulty, acceptance criteria |
| [docs/milestone-review.md](docs/milestone-review.md) | Release process: independent milestone review, AGREE/BLOCK verdict, tag-on-reviewed-SHA |
| [docs/feasibility.md](docs/feasibility.md) | Evidence-based feasibility evaluation: formats, prior art, risks, effort, value assessment, verdict |
| [docs/plan.md](docs/plan.md) | Phased implementation plan: architecture, milestones, testing strategy |
| [docs/evidence/](docs/evidence/) | Tier-0 probe results — measured format findings, rendered map, save walks |

## Local reference environment (verified)

Civ2 Test of Time 1.1 + ToTPP at `/mnt/d/games/CIVTOT`, running under Wine 10
(prefix `~/.wine_civ2tot`) as a behavioral *oracle* for differential testing.
Full provisioning for a new machine — toolchain, scratch space, rebuild
steps for the gitignored oracle binaries, box quirks — lives in
[docs/environment.md](docs/environment.md); the oracle rig recipe in
[tools/oracle/README.md](tools/oracle/README.md).

## Status

**Tier 0 complete** — v0.1.0 candidate (see [CHANGELOG.md](CHANGELOG.md) and
[BACKLOG.md](BACKLOG.md)):

- Engine language decided: **Rust** ([decision 0001](docs/decisions/0001-language.md));
  workspace scaffolded (`crates/tot-kit|engine|lua-host|totctl|totui`), CI on
  fmt + clippy + tests
- `.MP` decoder ported to Rust, cross-validated against the Python probe
- ToTPP Lua API contract collected offline ([docs/refs/](docs/refs/))
- SAV/SCN walker: front sections + city blocks + unit-count arithmetic proven

```bash
# Rust CLI (successor of the Python probe)
cargo run -- mp /mnt/d/games/CIVTOT/World.mp

# Full local suite: Rust tests + Python probe + golden evidence corpus
export CIVTOT_HOME=/mnt/d/games/CIVTOT   # golden part self-skips when unset
make test
make evidence-regen                       # after an intentional format change

# Python format probe: Rules.txt / .MP maps / .SAV+.SCN walks / save diffs
python3 tools/totctl.py rules $CIVTOT_HOME/Rules.txt -o /tmp/rules.json
python3 tools/totctl.py map   World.mp -o /tmp/world.png --json /tmp/world.json
python3 tools/totctl.py scnmap FallRome/ROMFALL.scn -o /tmp/romfall.png
python3 tools/totctl.py save  FallRome/ROMFALL.scn -o /tmp/walk.json
python3 tools/totctl.py diff  <a.sav> <b.sav> -o /tmp/diff.json
```

Highlights from the first session (details in [docs/evidence/](docs/evidence/)):
`.MP` format decoded and *proven* (World.mp renders as Earth with correct biome
latitudes); ROMFALL.scn walked — all 7 civ names decoded; ToTPP `0x33` saves share
the base front layout; the save-differ identified a "−1 unit record" delta between
two real autosaves.
