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
In its root, **`civ2.bak` is the original unpatched executable** (no ToTPP;
use it for "what does the original do/import") and `civ2.exe` is the
ToTPP-patched one the oracle runs (details: `docs/environment.md`).

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

v0.1.0 (Tier 0) and **v0.2.0 "headless game loop"** are released (v0.2.0:
2 review rounds, AGREE on #61) and **v0.3.0 "reproducible new game"** are
released (v0.3.0: AGREE on #70, 1 round; RG-2 done: `seeded_start.sh`,
`engine::placement`, goody huts; map tile framing and record counts
fixed) and **v0.4.0 "minimal production"** (AGREE on #72, 1 round: tile
yields, corruption, waste, unit support in `engine::yields`; production
queue in `engine::production`; decoded from `civ2.bak`). **v0.5.0 "player
interface"** is released too (AGREE on #98, 1 round; RG-8, decision 0003:
`engine::player` with Observation, Order, Game + order log + replay,
RandomPlayer; `totctl play/replay`). **v0.6.0 "AI v0"** is released too
(AGREE on #101, 2 rounds; #102 found create_city's `0x20000` is a
transient refresh bit): `engine::ai::AiV0`, `engine::founding` (FoundCity, create_city
partly decoded), obsolete units + veterans, goody-hut outcomes in part
(`engine::huts`; the rest is #79, moved to v0.8.0). Next: v0.7.0
browser observer (#80, #81).
Roadmap: small releases, one theme each (GitHub milestones; BACKLOG.md
holds conventions only;
decision 0002; observing and playing are the two main goals): v0.3.0
reproducible new game → v0.4.0 minimal production → v0.5.0 player
interface (headless API) → v0.6.0 AI v0 (all-AI game runs) → v0.7.0
browser observer → v0.8.0 AI v1 + economy depth → v0.9.0 generated-world
campaign → v0.10.0 pluggable AIs → v0.11.0 minimal SDL2 playable. Story state:
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
framing since 2026-11-03 (evidence §Record framing FIXED). GitHub issues/milestones are the live queue,
`docs/evidence/README.md` carries every proven finding (and, for T1-6,
an explicit list of what has been ruled out), and `docs/lessons.md` the
rig/process traps.
