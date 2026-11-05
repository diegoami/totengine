# Changelog

All notable changes to ToTEngine. The format follows Keep a Changelog; releases
are tags `vX.Y.Z` gated by the [milestone review](docs/milestone-review.md).

## [Unreleased] — AI v1 + economy depth (v0.8.0)

- **Calendar:** turns now update the raw date using the original difficulty
  tables and decoded scenario increments/start dates (324 native vectors,
  both reference dates exact). Undecoded 0x33 scenario calendars retain
  their date. Runtime research now respects leader-group restrictions,
  checked against both original research/hut gates.

- **Hut outcomes (#79, in progress):** original global-tech mercenary
  choices, late-year gold doubling, tribe site gates and late tribe
  population/buildings, saved landmass dispatch, leader-specific nomads,
  scroll legality and surface barbarian rings. 1,664 native vectors cover
  outcomes, conditional CRT draws and spawn requests. Tribe creation
  preserves the explorer without a temporary settler; new hut units get
  full movement. Full entry/state parity and runtime AI-cache refresh
  remain open.

- **Treasury and upkeep:** city taxes now credit the civ treasury outside
  anarchy/disorder; original human-mask upkeep, Barracks adjustments,
  Fundamentalism/Adam Smith exemptions and bankruptcy building sales are
  implemented. Formula checks cover 1,000 synthetic and 490 saved-input
  original-x86 cases, plus the original bankruptcy mutation path.
- **Happiness and disorder (#86):** decoded happiness stages, luxury and
  specialist balancing, improvements/wonders and military inputs; city
  happiness now refreshes before production and can stop or resume it.
  The arithmetic matches 1,000 original-x86 cases and native recomputation
  on 156 saved city inputs. Fifteen FallRome stored values also differ
  when original bytes recompute them; full-turn parity remains open.
- **Wonder lifecycle (#88, in progress):** player orders can select unbuilt
  wonders; completion registers the city, discards overflow and prevents
  duplicate or destroyed wonders from being rebuilt. Darwin grants two
  discoveries with subsequent choices left to the player. Human ordinary
  improvement completion now also discards overflow, as the original does.
- **AI production rows (#89):** corrected default difficulty rows and
  decoded the turn-200 adjustment using the saved power leader, wonder
  state and live knowledge masks. All 1,108 arithmetic cases match the
  original x86 fragment, including signed masks for civ 7.
- **Recurring trade-route income (#85 slice):** decoded partner trade caches,
  commodity/partner slots, domestic rounding and connection/building bonuses;
  route arrows now enter city trade and finance. The original x86 loop agrees
  on 288 synthetic cases. Route creation, delivery rewards, supply/demand
  refresh and non-surface routes remain open.
- **Roadmap amendment:** v0.10.0 targets a built-in AI resembling the
  original game's behavior. External AI protocols and tournaments move
  after the playable client; their version is unassigned. v0.11.0 begins
  UI discovery, owner-reviewed design and milestone decomposition; client
  implementation spans later milestones whose count/versions remain open.
- **Science and tax split (#87):** decoded city rates, specialist slots,
  rounding, improvement/wonder multipliers, Fundamentalism and
  Capitalization; city science/tax outputs refresh each turn. Every
  FallRome science/tax record (146 cities) and the current standard save
  (10 cities) match from stored trade/shields. Trade routes and maintenance
  remain separate work.
- **Per-civ research (#82 dependency):** decoded research targets,
  progress, cost counters and cost calculation; `SetResearch` orders,
  discoveries from city science, and shared hut advance bookkeeping.
  The cost formula matches the original x86 routine on 1,014 cases;
  recorded saves check 20 increments and four discovery resets.
- **AI v1, first increment:** `totctl play/observe --ai-v1` adds
  deterministic research choices to AI v0. A 50-turn real-save run gains
  14 advances and replays identically; full AI v1 remains in progress.
  Saved comparison rankings, late spaceship bonuses, discovery rewards/
  hooks and ToTPP scenario-tail research are not yet implemented.
- **Capital connections (#84):** road/rail paths and airport fallback now
  scale shield waste. The decoded land-path calculation matches original
  x86 execution on 123 saved city/capital pairs and 12 invented cases.
  FallRome shield totals improve from 143/146 to 145/146 exact; Aventicum
  remains one shield low. Non-surface paths and scenarios replacing the
  hard-coded land pathfinding unit type remain open.
- **Review follow-up #105:** replace the stale autosave diff reproduction
  with the existing T4PAIR inputs and remove its dead golden mapping.

## [0.7.0] — Browser observer

- **Observation stream (#80):** read-only boot/per-turn snapshots with
  spectator and per-civ fog views, canonical orders/results and checksums;
  verified order-log replay emits the same frames.
- **Browser observer (#81):** `totctl observe` serves a local viewer with
  original geometric tiles, units, cities, turn stepping, play/pause,
  timeline, fog selection, and portable NDJSON replay download/import.
  A new seek or restart cancels pending generation after the current turn;
  malformed archives leave the current replay intact.
  `--replay` watches a verified order log; `--stream` exports offline.
- Synthetic and real-save 100-turn stream/replay equality checks;
  optional Playwright control/render smoke test. See
  [browser observer](docs/browser-observer.md).

### Known limits

- Surface map only, with the existing engine fog rules.
- Events in frames are canonical orders/results and turn records, not an
  exhaustive list of internal simulation effects.
- The local server and browser cache history in memory; very long games
  and large maps can use substantial memory.
- Replay from an order log needs the same save, rules, events and build;
  portable archives contain every fog view and the full map by design.

## [0.6.0] — AI v0

An all-AI game now runs headless. AI v0 (an engine-designed player)
explores, founds cities and builds; production switches obsolete units
and makes veterans; goody huts pay out. A 100-turn all-AI game on a
standard save grows 10 cities to 50, and replays bit-exact from its log.

### Engine

- **Goody-hut outcomes** (#79, `engine::huts`), decoded in part from
  `hut_outcome` (`0x54ab80`):
  - **decoded and implemented:** the CRT draws, `rand()%5` with the
    human `rand()%3`, the case dispatch and its redirects, metals (25/50/
    100 by difficulty) and scrolls (`rand()%100`, then a scan for a
    researchable advance);
  - **simplified, flagged:** tribe (a size-1 city), mercenaries (the best
    defender), nomads (Settlers), barbarians/nothing (nothing);
  - entering a hut applies the outcome (`MoveOutcome::hut_outcome`).

- **Obsolete units** (#77) and **veterans** (#78), decoded from the
  production step (`0x4c63b0`, `0x4c6728`):
  - a unit build switches to the same-role successor once the civ knows
    the obsoleting advance;
  - new units are veteran from Barracks (or wonder 7) on land, from a
    Port Facility (or wonder 3) at sea, and from an Airport in the air.

- **AI v0** (`engine::ai::AiV0`, #75/#76):
  - a deterministic engine-designed player: it explores, founds cities,
    garrisons them, and builds defenders and Settlers;
  - a 100-turn all-AI game on the standard save takes 10 cities to 50,
    every civ founds cities, and the game replays bit-exact
    (`crates/engine/tests/ai_install.rs`);
  - `totctl play --ai`.
- **`FoundCity` order** (`engine::founding`), decoded in part from
  `create_city` (`0x432810`):
  - decoded: the record's initial values, the first city's Palace, and
    the first build (the best defender per cost). Flag `0x20000` is a
    transient refresh-pending bit: the city-totals pass clears it, so the
    engine clears it in the turn's production step (review #102);
  - engine rules, flagged: the name, the first worked tile, founding
    validity and the Settlers population cost.

### Known open

- **Hut outcomes (#79), partly:** tribe city size/buildings, the
  mercenary unit, case 3 (barbarians vs nothing), the landmass-claim
  forcing and the site-value redirect. Barbarians go with RG-9 (#83).
- **City founding:** the name lists, the first worked tile, founding
  validity and the Settlers population cost are engine rules. The
  large-water flag `0x200000` is not set (the engine map has no landmass
  table).
- **Type-table bytes not yet mapped to Rules:** +0x14 (availability) and
  +5 & 0x10 (veteran exemption).
- **AI v0:** simple by design. Research choice waits for per-civ
  research progress (v0.8.0).

## [0.5.0] — Player interface

Every civ is now driven by a player behind a headless interface. A player
gets an observation of what its civ sees and returns validated orders; a
game is recorded as an order log and replays bit-exact from save + seed +
log (decision 0003, story #74). This is the boundary that AI v0, the
browser observer, external AIs and the SDL2 client plug into.

### Engine

- **`engine::player`:** every civ is driven by a `Player`.
  - **Observation:** own units and cities (with output), what the civ
    sees (per-civ visibility, tile byte +4), and foreign units in sight.
  - **Orders:** `Move` (or attack), `SetProduction` and `EndTurn`, each
    validated; a rejected order changes nothing.
  - **`Game`:** drives turns and writes a text order log (orders, results,
    per-turn checksums). `Game::replay` re-drives a game from the save,
    seed and log, and stops at the first divergent line.
- **`RandomPlayer`:** random orders, legal and not; its own generator,
  so it cannot change the world's dice.
- **Replay tests:** all-random play replays bit-exact on the synthetic
  save (CI), FallRome with events, and a standard-game save.
- **`totctl play` / `totctl replay`.**
- Fog is the engine's rule, not decoded: a civ sees distance 1 around its
  units and the city radius, and seen tiles stay seen. Visibility is in
  the checksum (run50 regenerated, checksums only).

### Known open

- **Orders not yet available:** `FoundCity` and `SetResearch` come with
  AI v0 (#75). Founding a city needs a city-creation decode.
- **Fog:** the engine's own rule, not the original's reveal rules.
- **Turn order:** civs act in index order, an engine choice.
- **Logs:** the order log is text; the JSON protocol is #92.
- Carried from v0.4.0: see the issues on milestones v0.6.0 and v0.8.0.

## [0.4.0] — Minimal production

Cities now produce headless: tiles yield food, shields and trade as the
original computes them, corruption, waste and unit support are applied,
and shields fill the box until units and improvements complete. Every
formula is decoded from the unpatched `civ2.bak` and replayed against
game-written city records (evidence §T1-4).

### Engine

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

### Oracle

- `tools/oracle/load_save.sh`: load a save through the launcher.
- `tools/oracle/corrhook.py`: log calls of the corruption function.

### Known open

- **Road connection to the capital** (`0x471a13`): it scales waste.
  ROMFALL shields replay 143/146 without it.
- **Trade routes.**
- **Fundamentalism's free-unit flag.**
- **Production gaps:**
  - obsolete-unit switching;
  - wonders (ids ≥ 40);
  - veteran units;
  - the AI rows adjustment after turn 200;
  - AI build choice: an item, once chosen, repeats.
- **Turn order:** the original recomputes shields at processing time,
  after growth; exact turn-pair parity is T1-10.
- **Not modelled yet:** happiness/disorder, science and tax.

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
