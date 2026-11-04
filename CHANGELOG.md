# Changelog

All notable changes to ToTEngine. The format follows Keep a Changelog; releases
are tags `vX.Y.Z` gated by the [milestone review](docs/milestone-review.md).

## [Unreleased]

### Engine (v0.4.0, minimal production)

- **Tile yields** (`engine::yields`), decoded from the original's tile
  yield function (`civ2.bak 0x4c22a7`):
  - the inputs: special resources, the grassland shield, irrigation,
    mines, roads, rivers, railroads, pollution, and the buildings,
    wonders, government and despotism penalty that apply;
  - a city's worked-tile sum matches the game-written records: 9/10
    standard-game cities exact, ROMFALL food 144/146.
- **City totals** (`WorldState::city_output`): corruption (`0x4c35a2`:
  capital distance, government factor, difficulty for human despots,
  Courthouse/Palace), production multipliers and waste (`0x4c3919`).
  - Trade after corruption is exact in 145/145 ROMFALL cities without
    routes and 10/10 standard-game cities.
  - Shields, after unit support (`unit_support`: free units by
    government from @COSMIC): 143/146 and 10/10. The road connection to
    the capital and trade routes are still open.
- **Production** (`engine::production`, in `run_turn`):
  - net shields fill the box; units and improvements complete at
    `cost × rows`;
  - human rows come from @COSMIC; AI rows from difficulty
    (runtime-confirmed);
  - the std-lab one-turn box transitions match 53/70 (the misses are
    stale stored shields, see T1-10);
  - `totctl run` prints a `built` count; FallRome 50-turn golden
    regenerated.
- `City` gains worked tiles, buildings, flags and the build item. The world gains
  per-civ government and advances, and the raw tile improvement byte (all
  in the checksum).

## [0.3.0] — Reproducible new game

A new standard game in the original is now reproducible, and the engine
reproduces its start:
- the oracle pins the four entropy inputs;
- `engine::placement` places every civ draw for draw like the original;
- goody huts come from the map seed;
- tile records and the unit/city tables are read in the game's own
  framing.

Two decoder errors from v0.2.0 are fixed: the road/rail bits (they were
fog bits) and city-chain framing on fresh games.

### Oracle / RG-2

- **Reproducible new-game starts.** A new standard game depends on four
  inputs:
  - the NR state;
  - the CRT state;
  - the five world parameters (drawn before City Style);
  - the map resource seed (`GetTickCount() & 0x7fff`).

  `tools/oracle/seeded_start.sh` pins all four, so the same pins give the
  same turn-1 autosave (evidence §RG-2 reproducible starts). The
  "seed word" at save 686/688 is the civs-in-play mask.
- `tools/oracle/randhook.py`: call-site logger for the CRT `rand`.
- `tools/oracle/launcher_pick.py`: picks the launcher's game type from the
  screen. Replaces the `Up×4` key count, which flipped between two game
  types on alternate runs.

### Formats / engine

- **Map tile records are read in the game's framing.**
  - Records start right after the 7 B/tile block; the 2 bytes after the
    maps are a trailer.
  - Index = `y·w + x/2`.
  - Terrain, river and special were already right.
  - Road and rail were not: the old bits were the next tile's fog
    (ROMFALL roads 461, not 3,497).
  - New tile accessors: unit, city, landmass, visibility, fertility, owner.
  - `SaveFile` gains `barbarian_level`, `civs_mask` and `landmass_table`.
- **`engine::placement`** (RG-2): civ start placement, decoded from the
  original. Three oracle worlds replay draw for draw (568 CRT draws,
  15 starts).
- **Goody huts** (RG-2): derived from the map seed (oracle-checked). Any
  unit entering a tile makes its civ the tile owner, which takes the hut;
  `MoveOutcome::hut` reports entries (outcomes not decoded yet, RG-11).
  The engine tracks tile owners and the resource seed (in the checksum).
- **Units and cities are framed by the save's own counts** (i16@698 /
  @700), in tot-kit and the Python walker. Fresh games with few cities now
  parse.

- City and civ names decode as Latin-1, and the save editor encodes renames
  as Latin-1 (#69).
- `engine` example `headless_demo`: boots FallRome, fights one battle and
  narrates N turns.

### Planning

- Decision 0002: every civ is driven by a player behind an interface; an
  all-AI game is the default. Observing and playing are the two main
  goals.
- Release ladder (BACKLOG.md): small releases, one theme each:
  - v0.4.0 minimal production;
  - v0.5.0 player interface;
  - v0.6.0 AI v0;
  - v0.7.0 browser observer;
  - … v0.11.0 minimal SDL2 playable client.
- `civ2.bak` in the install is the original unpatched executable
  (`docs/environment.md`).

### Known open

- Goody-hut outcomes (RG-11): the engine reports a hut entry and applies
  no effect.
- Barbarians (RG-9). The per-map huts-enabled bit (RAM `0x5f5ed6`) is not
  located in saves; the engine assumes huts on.
- Multi-map files (Midgard) still do not frame units and cities.
- Placement branches decoded but not oracle-exercised: hut and occupancy
  rejections, the 2000-try relaxations, the flat-world margin, and preset
  Earth starts (not modelled).
- Carried from v0.2.0: tile yields and production, per-civ techs, special
  combat cases, east-west wrap in movement, T1-10 parity.

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
