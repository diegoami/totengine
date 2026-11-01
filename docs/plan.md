# Implementation Plan — civtotre

*Companion to [feasibility.md](feasibility.md). This plan is tiered so that every
phase ships a standalone, useful artifact — the project produces value long before
"full game parity".*

---

## 1. Guiding principles

1. **Compat-first.** The engine's contract is the original's *data*: read the user's
   ToT 1.1 install (optionally with ToTPP), load its scenarios/saves, and — from
   Tier 1 on — write saves the original can load back.
2. **Oracle-driven.** The Wine-run original (`WINEPREFIX=~/.wine_civ2tot wine
   /mnt/d/games/CIVTOT/civ2.exe`) is the referee. Every rule or format claim gets a
   differential test.
3. **No assets in repo.** Code + docs only; tests read from the local install via
   a configurable path (`CIVTOT_HOME`, default `/mnt/d/games/CIVTOT`).
4. **Every tier is a product.** Parsers, viewers, and diff tools are useful to the
   Civ2 community (scenario modders) immediately — not just intermediate steps.
5. **Deterministic core.** The simulation is seeded and reproducible headless;
   the GUI is a thin layer over the same core.

## 2. Target tiers & scope

| Tier | Target | "Done" means |
|---|---|---|
| **T0** | Format toolchain | `totctl` dumps/converts Rules/MP/SAV/SCN; save-diff harness works |
| **T1** | Headless classic-scenario engine | FallRome `.scn` loads, turns resolve, macro events fire, `.sav` round-trips into the original |
| **T2** | Graphical play (classic) | FallRome playable end-to-end with original art, sound, city screen, orders |
| **T3** | ToTPP layer | Lua 5.3 embedded; `0x33` saves; *Warring States* loads, its Lua runs, dialogs show |
| **T4** | Full ToT + polish | Midgard/Sci-Fi 4-map modes, editor, packaging, performance |

Out of scope (explicitly): network multiplayer, rewriting ToTPP itself, new
game-design features before T4.

## 3. Architecture

```
┌────────────────────────────────────────────────────────────┐
│ clients:  totui (SDL2 GUI)   totctl (CLI)   tests/benches  │
├────────────────────────────────────────────────────────────┤
│ scripting:  lua-host (Lua 5.3 ToTPP-compatible `civ` API)  │
│             macro-events (classic @IF interpreter)          │
├────────────────────────────────────────────────────────────┤
│ engine:  rules (Rules.txt-driven)  sim (turn kernel, RNG)   │
│          ai (plausible Civ2 AI)    events (bus/triggers)    │
├────────────────────────────────────────────────────────────┤
│ formats: tot-kit  rules-txt  mp  sav-scn (0x32/0x33)        │
│          art (gif/bmp/pal)      snd (wav/cd)  spr (ToTPP)   │
└────────────────────────────────────────────────────────────┘
```

**Stack decision (T0-5, 2026-11-01): Rust** — recorded with rationale in
[`docs/decisions/0001-language.md`](decisions/0001-language.md). The workspace
(`crates/tot-kit|engine|lua-host|totctl|totui`) is scaffolded with the proven
`.MP` decoder already ported and cross-validated against the Python probe;
CI (`.github/workflows/ci.yml`) runs fmt + clippy + tests on every push. The
Python probes remain the fast RE loop until T0-9's golden tests flip ownership.

**Why embed exactly Lua 5.3.6:** verified `lua.dll` in the local ToTPP install is
Lua 5.3.6; scenario scripts exercise 5.3 semantics.

## 4. Repository layout (target)

```
docs/            feasibility, plan, format notes, evidence
tools/           Tier-0 reference probes (Python; fast iteration)
crates|modules/  tot-kit, rules, sim, ai, events, lua-host, art, snd, totui
tests/           golden files, round-trip corpus, oracle scripts
oracle/          Wine automation: launch, key macros (xdotool), save capture
```

## 5. Phases

### Phase 0 — Recon & toolchain (T0) · 2–6 weeks
- [x] Inventory install, confirm Wine oracle, identify scenario corpus (done — see feasibility §3)
- [x] Header probes: `CIVILIZE`/version bytes, MP header, Rules sections (done)
- [x] `tools/totctl.py` (reference probe): Rules → JSON; MP → PNG+JSON; SAV/SCN
      structural walker (Mercator section map); size-tolerant save diff
      (see [evidence/](evidence/))
- [x] Walker validated on ground truth: ROMFALL civ names/turn/map header; two
      autosaves diff → "−40 bytes = 1 unit record"; ToTPP `0x33` shares front layout
- [x] MP format decoded & proven (terrain = word&0x7F, void = extra&0xF0; Earth
      biome-latitude check)
- [ ] Collect remaining specs: ToTPP Lua reference PDF/CHM, ToTPP feature docs
- [ ] Language decision (Rust vs C++), repo skeleton, CI, golden-file harness
- **Status:** core Phase-0 probing complete on day one; evidence in
  [evidence/](evidence/). **Accept criteria** (met): `totctl dump ROMFALL.scn`
  yields correct turn/civs/dims; World.mp PNG renders Earth with correct biomes;
  autosave diff isolates gameplay deltas.

### Phase 1 — Headless classic engine (T1) · 3–6 months
- Rules model from `Rules.txt` (all 22 sections; ToT tables @TERRAIN1..3,
  @MAP_TRANSPORT_RELATIONSHIPS parsed, single-map mode used first)
- Sim kernel: turn phases, movement/terrain, cities (food/shields/trade), research,
  government, combat (Civ2 rounds, veterans, walls…), wonders, hut outcomes, unrest
- RNG: identify Civ2's PRNG via oracle experiments (seed location in save → reproduce
  battle sequences); fall back to flagged "approximate RNG"
- Macro events interpreter (grammar from `Macro.txt`; ~50 commands; delayed events)
- Save writer: byte-compatible 0x32 saves → original loads them (round-trip gate)
- **Accept:** scripted 50-turn FallRome run: cities grow by Rules.txt math, events
  fire, produced save loads in Wine original; CI runs it against golden saves.

### Phase 2 — Graphical classic client (T2) · +6–12 months
- SDL2 client: isometric map from `Terrain1/2` sheets, units, fog, minimap
- City screen, unit orders/shortcuts (from `@ORDERS`), menus, dialogs per
  `Menu.txt`/`Labels.txt`; WAV sound; civilopedia from `Pedia.txt`/`Describe.txt`
- Plausible AI v1 (expansion, defense, scripted-scenario awareness)
- **Accept:** a human completes FallRome start-to-end without the original running;
  playtesters report no rule discrepancies beyond the published known-differences list.

### Phase 3 — ToTPP layer (T3) · +6–12 months
- `0x33` save variant (extended limits: 1024 cities / 4096 units from `TOTPP.ini`)
- Lua host + `civ` API in slices: core objects → triggers → UI (dialogs, text,
  sound) → teleporters/mods; conformance = *Warring States* `events.lua` runs
- `Static.spr` sprites, custom BMP resources, ToTPP feature flags honored
- **Accept:** *Warring States* loads, its intro Lua executes, opening dialog and
  custom UI appear; save/load round-trips under ToTPP.

### Phase 4 — Full ToT & polish (T4) · ongoing
- 4-map modes (Midgard/Sci-Fi) end-to-end, Extended Original AC chain
- Scenario editor assist tools (build on tot-kit), replays, performance
  (4096-unit turns < original's time), packaging (AppImage/Flatpak, Windows)

## 6. Testing strategy

| Level | Mechanism |
|---|---|
| Unit | Rules math (food/shields/beakers) vs hand-computed + oracle values |
| Golden files | Parsed JSON snapshots for Rules/MP/SAV corpus (FallRome ×8, Midgard, china ×3) |
| Round-trip | Our `.SAV` → original loads (Wine); byte-diff of re-saved file |
| Differential | Save-diff after scripted single actions (xdotool macros over Xvfb) |
| Scenario smoke | Each release boots the full local scenario corpus |
| Determinism | Same seed+inputs ⇒ identical save bytes (sim-level checksum) |

## 7. Key references (collected)

- **ToTPP Lua API (machine-readable, offline):** [`docs/refs/totpp-api-zerobrane.lua`](refs/totpp-api-zerobrane.lua)
  — the full `civ.*`/`totpp.*` contract for Tier 3, with behavioral notes
- **SAV/SCN structural spec:** Mercator's Apolyton thread "Civilization II SAV/SCN
  file format" (section sizes for ToT; transport-site block decoded by Catfish;
  settler work-point bytes by AGRICOLA) —
  https://apolyton.net/forum/civilization-series/civilization-i-and-civilization-ii/130935-civilization-ii-sav-scn-file-format
- **ToTPP main thread & downloads:** CivFanatics
  https://forums.civfanatics.com/threads/the-test-of-time-patch-project.517282/
- **ToTPP Lua events tutorial:** CivFanatics thread 636192
- **Macro language reference:** shipped `Macro.txt`; events behavior in README11.TXT
- **Prior art:** axx0/Civ2-clone (C#, loads ToT saves — check license before any code
  reuse), richardpenman/civ2, Freeciv (AI/engine patterns)
- **Local oracle:** `/mnt/d/games/CIVTOT` + `~/.wine_civ2tot` (verified working)

## 8. Open questions (tracked, not blocking)

1. ~~Exact `.MP` header variant in ToT~~ — **resolved 2026-11-01** (see
   [evidence/](evidence/)): 8-byte header + 3-byte slots; terrain `word&0x7F`,
   void `extra&0xF0`. *Sub-question left:* the `AnticChina.MP` variant from a
   ToTPP-era tool doesn't fit this layout — needs its own decode.
2. Unit/city count fields' exact offsets (Mercator sizes imply layout; verify by
   construction against known saves — the 40-byte unit-record delta is already
   confirmed by the differ).
3. Civ2 PRNG algorithm & seed persistence (oracle experiments; several community
   write-ups exist to cross-check).
4. ToTPP `0x33` extension layout (front section proven shared; derive the tail by
   diffing `0x32`↔`0x33` saves of the same scenario).
5. axx0/Civ2-clone license status (reference-only until verified).

## 9. Value produced along the way

- **T0 (now):** first *cross-platform* ToT save/map/rules inspector & differ —
  immediately useful to scenario modders (today they hex-edit or use Windows-only
  tools).
- **T1:** headless scenario runner → automated scenario testing/CI, "scenario
  linters", playthrough analytics.
- **T2:** ToT natively on Linux/macOS without Wine, forever-maintainable.
- **T3:** modern Lua scenarios preserved & playable platform-independently; the
  community's 2015–2026 masterpieces stop being Wine-dependent.
- **Throughout:** the most complete open documentation of ToT formats in existence.
