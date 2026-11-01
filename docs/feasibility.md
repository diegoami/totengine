# Feasibility Evaluation — Recreating Civ2: Test of Time Gameplay

*Evaluation date: 2026-11-01. All local facts below were verified on this machine
against the installation at `/mnt/d/games/CIVTOT` and the Wine prefix
`~/.wine_civ2tot`.*

---

## 1. Verdict (summary)

**Feasible — as a phased, incremental project — with two different answers depending
on scope:**

| Scope | Feasibility | Notes |
|---|---|---|
| Read/convert ToT data (maps, saves, rules, scenarios) | **High** — near-certain | Formats are simple or already documented; trivially verifiable |
| Play classic scenarios (FallRome-class, `Events.txt` macro, base rules) end-to-end | **High with effort** | Bounded rules engine + macro interpreter; strong validation possible via save round-trips |
| Full graphical parity with original ToT (all screens, all 4 game modes, AI) | **Medium** | Multi-year for one dev; AI parity is the hardest single item |
| Modern ToTPP/Lua scenarios (Warring States-class) | **Medium** | Requires re-implementing the ToTPP Lua API surface + extended save format; documented and bounded, but large |

The single biggest de-risking fact: **the original game runs here under Wine**, so the
recreation can be continuously differentially tested against the genuine article on
this machine, save file byte-by-byte. That converts "guess how it works" into
"measure how it works".

The single biggest risk: **no source code exists for civ2.exe or its AI** — behavioral
parity below the "what the rules files define" level must be reverse-engineered or
approximated (see §6).

---

## 2. What "replicating CivTOT gameplay" actually requires

ToT is not one game but a platform (this is why scenarios matter so much):

1. **The Civ2 rules engine** — movement, terrain, cities, production, trade, science,
   government, diplomacy, combat, wonders, events. Mostly *data-driven* from
   `Rules.txt`.
2. **ToT-specific extensions over classic Civ2:**
   - Up to **4 parallel maps** per game (surface / clouds / undersea / underground in
     Fantasy & Sci-Fi modes), with cross-map unit transport defined by
     `@MAP_TRANSPORT_RELATIONSHIPS` in `Rules.txt` (verified present, line 979).
   - Separate terrain rule tables per map layer (`@TERRAIN1..3`).
   - Extended Original mode's tech tree continuation to Alpha Centauri.
3. **The scenario platform**, which has two generations:
   - **Classic (1999–2015):** a scenario = a `.SCN` save + overridden text files
     (`Rules.txt`, `Game.txt`, `Units.bmp/gif`, `Events.txt`…). Logic comes from the
     **macro events language** (`@IF/@THEN/@THEN/@ENDIF` blocks — verified in
     `FallRome/Events.txt`: UNITKILLED handlers, CREATEUNIT, PLAYCDTRACK, JUSTONCE…).
   - **Modern (2015–now):** scenarios depend on **ToTPP** (Test of Time Patch
     Project), which adds Lua 5.3.6 scripting, extended limits (1024 cities / 4096
     units — verified in `TOTPP.ini`), extra units/techs/terrain, custom mods,
     dialogs, sprites, video. *Warring States* (2024) ships a 95 KB `events.lua`,
     custom `Static.spr` sprites, videos, per-civ Lua libraries.
4. **Assets & UX:** 8-bit GIF/BMP sprite sheets (`Units.gif`, `Terrain1/2.gif`,
   `Cities.bmp`, …), WAV/CD-audio sound, AVI video, and a large but finite UI
   (map, city screen, diplomacy, civilopedia, throne room…).

A credible recreation must pick a target generation first. **Recommendation: classic
scenarios first (Tier 1–2), ToTPP/Lua second (Tier 3)** — see the plan.

---

## 3. Evidence gathered locally

### 3.1 Installation inventory (verified 2026-11-01)

| Item | Detail |
|---|---|
| Game | ToT **1.1.0 (07-Sep-99)** — confirmed in `smeds.log` |
| Path | `/mnt/d/games/CIVTOT` (all built-in worlds present: `Original`, `ExtendedOriginal`, `Fantasy`/`Midgard`, `Scifi`) |
| Patch project | ToTPP installed (`TOTPP.dll`, `TOTLauncher.exe` from 2022-01); log shows **~90 patches/fixes applied** (`TOTPP.txt`) |
| Lua | `lua.dll` = **Lua 5.3.6** (strings-verified) → our engine should embed Lua 5.3 for API compatibility |
| Wine | Wine 10.0, prefix `~/.wine_civ2tot`; game ran today (logs `XDaemon.log`, saves `Va_Auto.SAV` 22:00) |
| Classic test scenario | `FallRome/` incl. 7 per-civ variants (`ROMFALL_*.scn` + `Rules_*.txt`) |
| Modern test scenario | `Scenario/china/` (*Warring States*, 2024): `events.lua` 95 KB, 6 MB ToTPP saves, custom BMPs, `Static.spr`, videos, Lua libraries |
| Rules data | `Rules.txt` = 22 `@SECTIONS` incl. all ToT-specific tables (verified §3.3) |
| Docs shipped | `README11.TXT`, `Macro.txt` (macro-language reference), `TOTPP/desc/*.rtf`, `lua/` helper scripts |

### 3.2 Binary formats (headers verified by hexdump)

| Format | Magic / layout | Difficulty |
|---|---|---|
| `.MP` map | **Decoded & proven (2026-11-01)**: `word w, word h, dword real_tiles`, then `w×h` slots of 3 bytes — terrain = `word&0x7F`, bit 0x80 = special resource, void = `extra&0xF0`. `World.mp` = 150×120, 9,000 real tiles; renders Earth with correct biome latitudes (see [evidence/](evidence/)) | **Trivial** |
| `.SAV`/`.SCN` | `"CIVILIZE"` magic, then version `0x001A`, then game version byte: `0x32` = ToT base (FallRome saves ≈ 203–226 KB), `0x33` = ToTPP-extended (Warring States ≈ 6.4 MB) | **Moderate** — full section map published (Mercator/Apolyton, §3.5); walker already validated on ROMFALL.scn (turn, 7 civ names, map header) |
| Sprite art | 8-bit indexed GIF sheets; scenarios may swap in same-layout BMP (verified: FallRome & china use `.bmp` variants) | **Easy–moderate** (palette handling, ToT's 640×480 isometric grid) |
| Sound | WAV files + CD-audio tracks (ToTPP adds DirectShow paths) | Easy |
| `Static.spr` | ToTPP custom sprite package (modern scenarios) | Moderate, ToTPP-documented |

### 3.3 Rules as data (verified)

`Rules.txt` is plain, commented text: `@COSMIC, @CIVILIZE, @IMPROVE, @ENDWONDER,
@UNITS, @TERRAIN, @TERRAIN1..3, @GOVERNMENTS, @LEADERS, @CARAVAN, @ORDERS,
@DIFFICULTY, @ATTITUDES, @CIVILIZE2, @LEADERS2, @UNITS_ADVANCED,
@MAP_TRANSPORT_RELATIONSHIPS, @INITIAL_SETTINGS, @SOUNDS, @RIVERS`. A rules engine
driven by this file *inherits* scenario customization for free — that is the crux of
scenario compatibility.

### 3.4 Prior art (web-verified)

| Project | What it proves / lacks |
|---|---|
| **axx0/Civ2-clone** (C#, 109★, active) | ToT saves *can* be loaded and rendered cross-platform today (loads `.sav`/scenarios, moves units, opens city window). Far from full gameplay; events/Lua not its focus. Useful reference for parsing & rendering. |
| **richardpenman/civ2** (C, 17★) | Native reconstruction viable in C. Early. |
| **chill117/civ2web**, **wan0net/civ2** (MGE-in-browser) | Map/unit rendering from original assets on other stacks. |
| **ToTPP** itself | The deepest documentation of ToT internals in existence (~90 hooks; save-format knowledge; Lua API; shipped as binary + docs). Original source repo is gone from the author's GitHub, but the local install ships `TOTPP.ini`, `TOTPP/desc/*.rtf`, and the Lua runtime; the Lua API reference is published on CivFanatics. |
| **Freeciv / Freeciv21** | Proof that a Civ1/2-style engine is a tractable long-term project; not ToT-compatible (different rules/save formats). |

**No existing project replicates ToT scenario gameplay** — the niche is real, and the
hardest groundwork (format knowledge) is already public.

### 3.5 External documentation (web-verified 2026-11-01)

| Source | What it gives us |
|---|---|
| **Mercator, "Civilization II SAV/SCN file format" (Apolyton thread 130935)** | Full section map of saves incl. ToT-specific sizes: Settings 1238 (incl. 640-byte unit transport-site block decoded by Catfish), Civilizations 8×3348, Transporters `172+14n` (ToT 1.1), Maps header 16 + per-map `13×surface+2`, Units 40/unit, Cities 92/city, Events `276×event` + string heap; header version constants (0x0031=ToT1.0, 0x0032=ToT1.1) — **matches our local hexdumps exactly** |
| **CivFanatics utilities section** | Existing Windows save editors for ToT formats — proof the format is fully cracked and toolable |
| **ToTPP thread 517282 + Lua tutorial 636192** | ~81 patches documented; Lua events workflow; the `civ` API surface |
| **Shipped `Macro.txt`** (local) | Complete classic events-macro command reference |
| **AGRICOLA's posts (same Apolyton thread)** | Deep mechanics encoded in saves (e.g. settler work-point accumulation) — the kind of behavioral details our oracle harness can regenerate |

This moves the save format from "reverse-engineer" to "**port a published spec,
then verify against the local Wine oracle**" — a categorical feasibility
improvement.

---

## 4. Why the Wine installation is a strategic asset

The requirement "accessible here and can be started with wine" turns the original
game into a **measurement instrument**:

1. **Save-diff reverse engineering.** Load a scenario in the original under Wine,
   save, perform exactly one action, save again → byte-diff reveals precisely what
   that action changed. Scriptable via `xdotool` against a virtual display.
2. **Round-trip compatibility tests.** The ultimate correctness check for our engine:
   a `.SAV`/`.SCN` **we write** must load in the original (and vice versa). Any
   parser/writer gap is caught immediately, not guessed at.
3. **Behavioral oracle.** Same seed + same inputs → compare outcomes (combat rolls,
   city growth, AI moves) to calibrate our engine's RNG and rules order.
4. **Asset source.** All sprites/sounds/texts for tests come from the legal install;
   nothing copyrighted ever enters our repo.

Suggested invocation for the harness:

```bash
WINEPREFIX=~/.wine_civ2tot wine /mnt/d/games/CIVTOT/civ2.exe   # or TOTLauncher.exe
```

---

## 5. Feasibility by subsystem

### 5.1 Text data files — **easy** (≥95% of rules content)
Straightforward parsers (CRLF, `;` comments, `@SECTION` headers, comma-separated
columns). FallRome's per-civ `Rules_*.txt` swapping shows how scenarios reshape the
engine through data alone.

### 5.2 Map & rendering — **moderate**
`.MP` parsing is trivial; the work is ToT's isometric tile grid, multi-map layers,
fog-of-war, and reassembling 8-bit sprite sheets into animations. Prior art
(axx0/Civ2-clone) already renders ToT maps, confirming feasibility.

### 5.3 Save/scenario format — **moderate, time-consuming tail**
Header structure known (`CIVILIZE`, version, then fixed-order blocks: units, cities,
tribes, map data, visibility bitmaps…). ToTPP extended the format (`0x33`) with its
documented limit changes. Strategy: implement incrementally, gated by round-trip
tests (§4). Expect the long tail (replay/undo data, event heaps, ToTPP extras) to
consume disproportional time — mitigate by declaring byte-perfect *read* mandatory
and byte-perfect *write* only for the fields the original round-trips.

### 5.4 Rules engine (turn loop, economy, combat, research) — **moderate**
The rules are fully specified in data files + manual; the hidden parts are exact
ordering/rounding and RNG usage. Oracle testing (§4) compensates.

### 5.5 Macro events language — **easy–moderate**
~50 commands (`@IF/@THEN/@ENDIF`, UNITKILLED, CREATEUNIT, TEXT, JUSTONCE, DELAY…),
grammar documented in the shipped `Macro.txt`. A recursive-descent interpreter is
days-to-weeks of work; edge cases (delayed events, stack limits noted in
`README11.TXT`) add polish time.

### 5.6 ToTPP Lua API — **moderate–hard, but bounded**
Embed Lua 5.3.6 (exact version verified) and re-implement the `civ.*` API surface
(objects, triggers, dialogs, text, sounds, teleporters…). The API is published and
versioned; *Warring States* is the perfect conformance suite (95 KB of real-world
usage). Risk is breadth, not difficulty — hundreds of functions/properties.

### 5.7 AI — **hard, and the least reversible-engineerable part**
No source for the original AI. However: (a) most scenario gameplay is *scripted* via
events/Lua, muting AI differences; (b) scenarios ship pre-built empires where AI
mostly manages production/war; (c) a competent Civ2-style AI is well-understood
territory (Freeciv's is a study reference). Recommend: "plausible AI faithful to
Civ2 heuristics" as the target, with fidelity flags, not instruction-level parity.

### 5.8 UI scope — **moderate, mostly volume**
Map viewport, city screen, unit orders, diplomacy, trade, civilopedia, throne room,
science/advisors, editor. Reproducible from original layout; modern scenarios also
use ToTPP Lua dialogs. SDL2 (or equivalent) + the original art keeps it faithful.

### 5.9 Audio/video — **easy**
WAV + MIDI/CD replacements (files exist in `Sound/`), AVI via libmpv/ffmpeg for
ToTPP's DirectShow paths.

---

## 6. Risk register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Save-format unknown fields block round-trips | Medium | High | Save-diff harness (§4); accept "read-only" for exotic fields initially |
| ToTPP Lua API breadth underestimated | Medium | Medium | Conformance-test against `Warring States`; ship subset per scenario needs |
| AI parity expectations | High | Medium | Declare AI target as *plausible*, not identical; scenario scripting hides most deltas |
| Scope creep ("just like the original, plus…") | High | High | Tiered scope with written acceptance criteria (plan doc) |
| Legal: assets & code | Low | High | Engine ships **no** original data; requires user's install; we document formats, never copy code; ToTPP is treated as a behavioral spec, not source to lift |
| Multiplayer (original used DirectPlay) | — | — | **Out of scope** (recommendation); hotseat covers scenario play |
| Motivation/longevity (multi-year) | High | High | Phase design where each phase yields a *usable tool*, not just groundwork |

---

## 7. Effort estimate (one experienced developer, part-time; halves with full-time)

| Tier | Deliverable | Estimate |
|---|---|---|
| 0 | Parsers + `totctl` CLI (dump SCN/MP/Rules → JSON/PNG), save-diff harness | **2–6 weeks** |
| 1 | Headless rules engine; classic scenario runs turn-by-turn; macro events; `.SAV` round-trip with original | **3–6 months** |
| 2 | Full graphical client for classic scenarios (map, city, orders, dialogs) + basic AI; FallRome playable end-to-end | **6–12 months cumulative** |
| 3 | ToTPP layer: `0x33` saves, Lua 5.3 embedded, API subset; *Warring States* loads & plays | **+6–12 months** |
| 4 | Polish: 4-map modes (Midgard/Sci-Fi), editor, audio/video parity, packaging | ongoing |

These are consistent with how long Freeciv/Julius-type projects take to reach
playability — ToT is smaller than Freeciv in scope, but scenario compatibility adds
the ToTPP long tail.

---

## 8. Legal posture (short)

- The project distributes **code only**; it reads the user's legally-owned ToT files
  (same model as OpenRA, *Julius*/*Augustus* for Caesar III, axx0/Civ2-clone).
- File formats and APIs are facts — documenting/re-implementing them is standard,
  accepted practice in game recreation projects.
- Scenario content belongs to its authors; our repo ships none, and testing uses the
  user's local copies.
- The ToT trademark/name stays out of the binary's branding ("unofficial engine
  requiring the original game").

---

## 9. Value assessment — is it worth doing?

**For the user/owner of this repo:**

| Value | When |
|---|---|
| ToT runs natively on Linux/macOS, no Wine, no 640×480 scaling pain, forever maintainable | Tier 2 |
| Modern Lua scenarios (*Warring States*-class) preserved on a platform that outlives x86 Windows/Wine quirks | Tier 3 |
| Deep, satisfying reverse-engineering project with a genuinely complete local lab (game + oracle + scenario corpus) | immediately |

**For the Civ2 community (external value — the project is not just a private toy):**

1. **No one else is doing this.** The two active recreations target MGE-style play,
   not ToT scenario compatibility; ToT's scenario scene (2015–2026 ToTPP works) has
   no long-term runtime plan beyond "keep Wine alive".
2. **Tier 0 alone is novel:** a cross-platform ToT save/map/rules inspector and
   differ. Today's modders hex-edit or depend on Windows-only tools; several
   community questions ("what did this event change?", "why does my save crash the
   editor?") are directly answered by a good differ.
3. **Tier 1 enables scenario CI:** authors could regression-test their `Events.txt`
   logic headlessly — something never possible before.
4. **Documentation externality:** whatever happens to the engine, the verified
   format notes (esp. `0x33` ToTPP saves) become the reference the community lacks.

**Cost side:** realistically 1 dev-year to Tier 2, 2+ to Tier 3 (halved with a
second contributor). The tiered design means the effort curve can stop at any tier
and the value already shipped stands. Opportunity cost is the main risk — mitigated
by Phase 0 being days-to-weeks and independently useful.

**Verdict:** the project produces real value at *every* tier, with unique (not
duplicative) value from Tier 0 onward. Worth starting; the plan sequences it so
stopping points are graceful.

## 10. Conclusion

Everything needed to *start* is present on this machine: the full game + ToTPP, two
scenario generations as conformance targets, shipped documentation (`Macro.txt`,
ToTPP docs), and a working Wine oracle for differential testing. No format or
platform blocker exists. The genuine constraints are **duration** (rules engine +
UI + Lua API is multi-year for one person) and **AI parity** (approximate by
design). With the tiered scope in [plan.md](plan.md) — tools first, headless classic
scenarios second, graphics third, ToTPP/Lua last — the project is feasible and each
phase produces standalone value.
