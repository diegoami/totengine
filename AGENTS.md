# ToTEngine — agent orientation

Open-source recreation of Civilization II: Test of Time gameplay (especially
community scenarios), in Rust. Reads the user's original ToT install; ships
none of its assets.

## Layout

- `crates/` — Rust workspace: `tot-kit` (format decoders), `engine`,
  `lua-host`, `totctl` (CLI, successor of the Python probe), `totui`
- `tools/totctl.py` — Python format probe (rules/map/save/diff/scnmap);
  stdlib only
- `tools/oracle/` — headless-Wine rig that drives the original game as a
  behavioral oracle; **read its README before touching or running it**
- `docs/` — `feasibility.md`, `plan.md` (tiers/principles), `BACKLOG.md`
  (story queue), `evidence/` (all proven format findings), `decisions/`,
  `refs/` (vendored ToTPP Lua API), `milestone-review.md` (release process),
  `environment.md` (machine setup/quirks), `lessons.md` (distilled
  process/rig lessons — read before driving the oracle)
- `docs/evidence/` + `tools/golden.sh` — golden corpus; `make
  evidence-regen` after *intentional* format changes, then commit

## Commands

```sh
make test            # cargo test + py_compile + golden corpus (self-skips w/o CIVTOT_HOME)
make fmt clippy      # CI runs these with -D warnings
python3 tools/totctl.py save $CIVTOT_HOME/FallRome/ROMFALL.scn -o /tmp/walk.json
```

`CIVTOT_HOME` points at the original install (reference box:
`/mnt/d/games/CIVTOT` — provisioning on a new machine: `docs/environment.md`).

## Hard rules

- **Never commit original game assets** (sprites, maps, saves, sounds).
  Evidence artifacts are derived data only (JSON summaries, renders we made),
  small.
- **Every format claim needs evidence** — an artifact in `docs/evidence/` or
  a check that re-derives it. No folklore from forums without verification
  against real data.
- The oracle's absolute paths stay in tools behind `CIVTOT_HOME`, never in
  library code.
- Releases are tags gated by independent review: read
  `docs/milestone-review.md` before proposing a version. **The main session
  never launches the milestone review** — the owner does. At a milestone,
  print the review-prompt template from that doc and stop; agent-launched
  reviews proved unreliable (owner decision, 2026-11-02).
- Prior art was license-audited (`docs/prior-art.md`); GPL projects are
  reference-only (no code copying). MIT vendors keep their notices.
- The checkout dir may be named `civtotre` while the remote is `totengine`;
  pass explicit working directories.

## Status shorthand

v0.1.0 (Tier 0) is released. **v0.2.0 "headless game loop" is a milestone
candidate** (CHANGELOG; RG-2 moved to v0.3.0). Story state:
T1-1/2/3/5/8/9 and RG-1 are done; `engine::turn::run_turn` is the loop. T1-6's randomness half is **done**
(the original LCG and its `rand_int` are decoded, runtime-verified and
shipped in `engine::rng`). Its combat-round half is decoded and shipped
in `engine::combat`: rolls come from a *second* generator (the MSVC CRT
`rand()`); attack/defence strength (incl. ToTPP hooks) and the promotion
roll are decoded too, six oracle battles are bit-exact, and combat is
wired into `move_unit` with a UNITKILLED context for T1-7. Open: sea/air
and improvement-dependent special cases.
Units are driven in the oracle through RAM (`tools/oracle/unitram.py`),
not by map navigation. T1-4 (cities) and
T1-7 (events) are partially landed (T1-7 effects now change the world:
treasury, CREATEUNIT). Unit and city records are read in the game's own
framing since 2026-11-03 (evidence §Record framing FIXED). `BACKLOG.md` is the live queue,
`docs/evidence/README.md` carries every proven finding (and, for T1-6,
an explicit list of what has been ruled out), and `docs/lessons.md` the
rig/process traps.
