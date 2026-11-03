# Changelog

All notable changes to ToTEngine. The format follows Keep a Changelog; releases
are tags `vX.Y.Z` gated by the [milestone review](docs/milestone-review.md).

## [0.2.0] — Headless game loop

ToTEngine boots the original's scenarios **and standard-game saves** into a
world state and runs them headless: movement, combat, macro events and city
growth, each checked against the original game (Wine oracle + RAM access) or
against its own documentation (`Macro.txt`). **Scope (owner decision,
2026-11-03):** RG-2 (start placement from the game seed) moved to v0.3.0.
The new-game entropy source is not found yet; see evidence §RG-2.

### Engine

- **Turn loop** (`engine::turn::WorldState::run_turn`): city growth,
  TURN/TURNINTERVAL/RANDOMTURN events with their effects applied, and
  movement reset. Deterministic: same save + seed + rules gives the same
  fingerprints. Checks: asset-free 50-turn test in CI, FallRome 50-turn
  golden (`docs/evidence/fallrome-50turn.txt`), and a standard-game save
  (`standard_game_install.rs`).
- **Movement** (T1-3): Rules columns oracle-proven; thirds, roads/rail,
  ZOC, domains. One step per request: the destination must be one of the
  8 diamond-grid neighbours, for moves and attacks alike. East-west map
  wrap is not modelled.
- **Combat** (T1-6): the original's two generators, decoded and shipped:
  - the NR LCG `rand_int`, runtime-verified;
  - the MSVC CRT `rand()`, which drives combat.
  Round formula, attack/defence strength (incl. ToTPP's hooks),
  promotion, defender choice and damage persistence. **Six oracle battles
  are bit-exact, three of them predicted from seeds we wrote.** Combat is
  wired into `move_unit`.
- **Events** (T1-7): `@IF/@THEN` parser and interpreter. Matching and
  CREATEUNIT/CHANGEMONEY/GIVETECHNOLOGY/CITYTAKEN semantics were corrected
  against `Macro.txt`. Effects change the world (`engine::effects`):
  treasuries (oracle-proven field), units created. Kills fire UNITKILLED.
- **Cities** (T1-4): economics fields oracle-verified. The growth model
  replays the standard-game lab series: of 126 checked transitions (10 of
  them growths), 125 match exactly, including 9 of the growths. The one
  mismatch (Ur, turn 40→41) is listed.
- **Research/wonders** (T1-5), **rules model** (T1-1), **world state +
  checksum** (T1-2).

### Formats

- **Unit and city records are read in the game's own framing.** Both
  walkers had units 6 B late and cities 34 B late, so every city carried
  the next city's coordinates and boxes. Proof: a unit standing on a city
  is homed there 349/349 (evidence §Record framing FIXED). City owner is
  u8@10, size u8@11. Unit fields decoded: veteran, damage, orders, home,
  stack links.
- Save fields: game flags @652, human mask @687, treasury (civ block
  i32@10), civ-name slots.
- **Save writer** (T1-8): surgical `0x32` edits, a byte-identical round
  trip, and an edited save loaded by the original.
- Standard-game setup byte-map (RG-1).

### Oracle rig

- RAM-level unit driver (`unitram.py`), RNG slot tools (`rngslot.py`),
  live combat-frame polling, and `/proc/<pid>/mem` writes for `.text`
  patches.
- Standard-game lab scripts, and the derived growth series generator
  (`tools/growth_series.py`).

### Known open

- RG-2 (moved to v0.3.0).
- Combat special cases: pikemen bonus, ships, air, improvement-dependent
  bonuses (fortress, walls, coastal fortress, SAM, SDI). Wonder
  obsolescence needs per-civ techs.
- GIVETECHNOLOGY is reported but not applied (no per-civ techs).
  CREATEUNIT's "valid location" rule is the engine's own.
- Tile yields, production items, unrest. The turn-loop step order is the
  engine's choice. Parity with the original's per-turn saves is T1-10.
- The `0x33` record framing was not re-checked.

## [0.1.0] — Tier 0: format toolchain & reconnaissance

First release: ToTEngine reads the original game's data — every claim proven
against real game files, evidence committed under `docs/evidence/`.

### Formats decoded & proven

- **`Rules.txt`** — all 22 sections to JSON (`totctl rules`).
- **`.MP` maps** — both flavors: stock ToT 3-byte slots *and* the ToTPP-era
  paired-slot variant (AnticChina); terrain/river/specials/improvements/void;
  renders as PNG (`totctl map`, `--scale`, overlays via `scnmap`).
  World.mp renders as Earth with correct biome latitudes; river bit 0x80
  corrected from an earlier mis-attribution.
- **`.SAV`/`.SCN` base (`0x32`) front** — section walk verified byte-exact:
  settings, civs, transporters, turn/year; embedded map (visibility 7 B +
  6-B-stride tile fields: terrain &0x3F, special 0x40, river 0x80; road
  0x40 / rail 0x80); unit records (x/y, type→@UNITS,
  owner with 0x10 leader bit, per-save re-assigned ids — garrisons-on-cities
  145/146 measured per file by the walker); city records (name, x/y;
  the city-size-like short sits in the two bytes before each name —
  exact field boundary open).
- **ToTPP `0x33` extension** — tail framing (`LUAE` + serialized Lua state
  as plain text + `EVNT` trailer) and the mid-region, with an exact
  decomposition on the four-file Warring States ladder and falsifiable
  structural checks in the walker: `[32+15×surface per-tile arrays][40 B
  units][4 B][92 B cities][fixed post-city]`; 0x33 unit layout differs from
  the 0x32 base (id@0 sequential, logical signed coords, type@18, owner@19).
  ToTPP writes plain 0x32 for non-Lua scenarios (oracle-verified).

### Toolchain

- Rust workspace `tot-kit | engine | lua-host | totctl | totui`; MP decoder
  ported and cross-validated against the Python probe on the corpus maps —
  enforced by `make test` via `docs/evidence/crosscheck-mp.json`; terrain
  mask is per-layout (stock `0x7F`, ToTPP paired `0x3F`); CI on
  fmt + clippy + tests.
- `tools/totctl.py` — reference probe (rules/map/save/diff/scnmap),
  stdlib-only, `CIVTOT_HOME` for the install root.
- Golden-file harness (`make test`): 21-entry evidence corpus (all 8
  FallRome variants + Midgard ×2 + Warring States ×4 + maps/rules/diff),
  machine-
  normalized, self-skipping in CI.
- **Wine oracle rig** (`tools/oracle/`): the original game driven headless
  under Xvfb (custom XTest tool + Win32 SendInput injector), automated turn
  loop harvesting same-session saves; annotated diffs. Findings documented
  (launcher listbox needs double-click; X-level keys never reach the
  DirectDraw surface; classic exe's cdaudio CD-check is unrunnable headless).

### Project

- Language decision (Rust) recorded; prior art license-audited; MIT license;
  ToTPP Lua API vendored for Tier 3; public at `github.com/diegoami/totengine`.
- Docs: feasibility (evidence discipline), plan (tiers), backlog (T0 complete,
  12/12), milestone-review process, environment/provisioning guide.

### Known open (not defects)

- 0x33 refinements: per-tile band sub-layout, post-city internals, unit flag
  bytes; unit-start formula proven on one map.
- Multi-map (Midgard-style) unit scan in the 0x32 walker.
