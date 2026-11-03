# Tier-0 Evidence — first format probe results (2026-11-01)

Artifacts produced by `tools/totctl.py` (Python reference probe) against the local
install `/mnt/d/games/CIVTOT`. These validate the feasibility claims in
[../feasibility.md](../feasibility.md) with measured data.

#### T1-8 save writer (2026-11-03): surgical edits accepted by the original

- `tot-kit::sav::SavEditor` — surgical `0x32` writer: validates the file
  with the same gates as the parser, then applies typed edits to proven
  offsets: turn@668, year@670, difficulty@684, rivals@689, unit x@6/y@8/
  type@14 (40-B records), city name@0 (16-B slot, ≤15 bytes), city
  x@58/y@60 (92-B records). A no-edit round trip is **byte-identical**
  on ROMFALL.scn, ROMFALL_WRE.scn and Va_Auto.SAV (env-gated test).
- **Oracle acceptance**: a Va_Auto.SAV copy with city 0 renamed
  "Roma"→"TESTCITY" (`EDITEST2.SAV`, 8 bytes changed) was loaded by the
  original through its own *Load a Saved Game* dialog; the game's
  **Find City list shows `TESTCITY (OBJECTIVEx3)`** (the OBJECTIVE tag
  proving the edit landed in the right record), and the game's own
  *Save Game* (`429dec.sav`, same 226,336 bytes) **preserves
  TESTCITY** through a full save cycle.
- **Re-save diff is field-explained**: our input vs the game's re-save =
  974 differing bytes in 161 regions: one byte per unit record (diffs at
  a 40-byte stride — the game's per-unit bookkeeping byte), front bytes
  655–658/674, and one map byte at 34663 (fog/visibility). No structural
  drift, identical size, identical turn/year.
- Finding: the displayed scenario date ("Dec 429") is **not** driven by
  the year field@670 — the edited −3000 year left the display unchanged.
  Scenario calendar source still open (likely the scenario date
  settings; matters for T1-7/T1-4 displays).
- Multi-map saves (Midgard.SCN) are correctly **refused** by the parser
  (the unit-region gate fails; the walker cannot locate them either) —
  no fabricated tables.

### T1-4 live-RAM city struct + the @58 coordinate problem (2026-11-03 late)

RAM-diff experiment (toggle one worked tile in Madrid's city view, diff
full 383 MB address space — `tools/oracle/ramdump.py`):

- **The live city-view econ struct** sits at ~0x653848 for the viewed
  city: F/S/T as **u32s in 32-bit layout** (F@+0, S@+4), four bytes
  00→01 nearby (worked-tile flags), a 02→00 (entertainer count) — the
  record array at 0x19c0xxx ALSO updated on the toggle (F@48 6→4→6,
  S@49, owner/lux byte@51), so **@48/@49/@46 live-sync with the game**.
- The @58 scare (record (43,37) vs apparent live (53,9)) is **resolved:
  @58 is the true position** (see the settled note above). The 0x653848
  live-struct finding itself stands: F/S/T u32s + worked-flag bytes +
  entertainer count, live-syncing with @48/@49/@46 in the record.
- The worked-tile flags found in RAM are NOT a map-parallel 0xFF array;
  they are per-view-struct bytes (only the opened city's struct
  appears; Carthage's F/S/T signature sits 0x280E0 away — cache, not
  an array).
- Experiment plan (next session): open each city's view once (Find City
  + dblclick), toggle a tile, diff — collect every city's live struct +
  true position, then map record↔live (slot permutation? second table?)
  and decode the worked-tile set per city.

### T1-4 tile yields: first fits + open geometry (2026-11-03 night)

- **@TERRAIN fields 3/4/5 = food/shields/trade** (classic layout,
  Original/Rules.txt); special resources are further rows of the same
  table with the same columns (Oasis 3,1,0; Gold 0,1,6; Fish 3,0,2 …).
- **Madrid exact fit**: center Grassland (2,1,0) + 2 worked grass tiles
  = the recorded (F6, S3, T0) — with rivers NOT adding trade (its tiles
  carry the 0x80 river flag yet T=0). But Persepolis (T3) fits only
  WITH river-trade (+1T) — and Carthage/Little Bighorn/Utica fit under
  no rule tried. Conclusion: the radius-shape offsets and/or the
  special-id decode are still wrong; city-sum achievability is too weak
  a channel. Next: per-tile ground truth (city-screen yield labels,
  tile by tile) before more fitting.
- **Settled by patch-and-prove (final)**: @58/@60 IS the city's true
  map position. Patching Madrid's record x 43→44 in RAM changed nothing
  on screen (the record array is the save buffer, not the render
  source) — but patching the "live" 0x653860 field (53→54) ALSO changed
  nothing, including after reopening the view: that struct's 53/9 is
  cursor/goto data (a unit path list through (53,9) sits at 0x5f73b0).
  The city view's radius matches the (43,37) decode (ocean NNW/NNE,
  mountain NW, grass E), and the panel's "Loc (53,9)" is the SELECTED
  PHALANX's position (home Madrid ≠ on Madrid). The screen-vs-save
  conflict was a vision mirage after all; the original verdict stands.
  Lesson: settle screen-vs-save with a memory patch, not with more
  screen reads.
- The standard-game save's unit coords are **@6 (x,y)** in the walker
  frame (the @10 pair is something else) — the garrison check passes
  10/10 with @6 vs cities@58; the earlier "0/10" used the wrong field.

### T1-4 walker fixes + young-city quirk (2026-11-03 night)

- **Two-character city names are real**: "Ur" in the standard-game corpus
  broke the name-chain at the 3-char minimum, hiding every later city
  (Malaca sat undetected behind it). Both walkers now accept 2..15-char
  names (Rust `sav.rs`, Python `totctl.py`).
- **Young city records are half-initialized**: a freshly founded city
  carries placeholder coords (x=1, y=256-pattern) and zeroed boxes until
  its first full processing (Madrid @turn 25, Malaca @turn 43 — same
  signature). Not corruption; do not reject on it.
- The "**post-city block**" interpretation from T0-6 is revised: at least
  part of it is the *extended city array* (cities beyond the walked chain)
  followed by end-game quote strings ("…Theory is to be…", "WE WON THE
  GAME!") — not a fixed per-city structure.
- `xtool resize` added (XResizeWindow, no WM needed): the game WINDOW
  resizes but the DirectDraw surface stays 600×440 — city views stay
  clipped; not a path to bigger screenshots.
- @TERRAIN fields 3/4/5 = food/shields/trade (classic layout); the city
  screen's worked-tile labels agree on the samples read (grassland tile
  "2F+1S" = 2,1,0). Full verification needs the worked-tile array, which
  is NOT a 0xFF-sentinel map-parallel array (scan found none) — next
  experiment: RAM-diff while toggling a worked tile via the city screen.

### T1-4 city model (2026-11-03 evening): growth math closed on a peaceful standard-game lab

A fresh STANDARD game driven turn-by-turn through the original under the
memspool daemon (`tools/oracle/stdloop.sh`), autosaves harvested per turn
(`~/review-tmp/oracle-runs/std1/`): 5 AI capitals + a human-founded Madrid,
turns 4–43, zero war noise.

**Growth model (7 growth events, turn-by-turn samples, 4 cities):**
- **Food box = u16@86**; ticks by surplus (`gross_food@48 − food_per_citizen×size
  − settler_food×settlers`) each turn.
- **Growth capacity = (rows − 1) × (size + 1)** with `@COSMIC` rows=10 →
  **18 at size 1, 27 at size 2**. The `@COSMIC` comment
  ("rows * city_size+1 = box") is garbled — the measured rule is
  (rows−1)×(size+1).
- **Growth is strictly greater**: a box sitting exactly at 18/27 persists
  (Madrid fb 18@t36, LB/Carthage 18@t22/27@t40 — all sat a full turn).
  Overflow carries: `box −= cap` (Madrid 18+4=22 → grow → 4 ✓ t38;
  Carthage 27+2=29 → grow → 2 ✓ t41).
- `engine::cities::{CityRules, CityEcon}` implements exactly this
  (unit tests cite each oracle sample); `CityRules::from_rules` reads the
  `@COSMIC` constants (typed accessors in tot-kit).
- **Shield box = u16@88**: ticks by net shields (S − unit support), resets
  on production completion (Carthage 35→3, 15→3, 31→12…). Production
  items themselves not located yet — open.
- **Settlers eat** (`@COSMIC` rows 6/7) and are INVISIBLE in the save:
  Madrid's box only decodes with 2 supported settlers (surplus 6−2−2=+2),
  LB's post-t42 ticks need a settler too. Full-series replay tests are
  therefore impossible without settler counts — transitions are covered
  by unit tests instead.
- **u16@90 is NOT size** (Babylon stays 3 through a growth; correlates
  with size+capital for most cities — open).
- **City size = u16 at city_base − 13** holds for WRE cities in
  FallRome (8/8 vs the City Status report) but reads 0 for standard-game
  cities and some AI civs — per-civ encoding open. The walker exposes it
  as `CityRecord::size` (0 = unknown).
- FallRome/standard contradiction resolved as **war contamination**: the
  earlier FallRome turn-1 "shield box at @86" fit was a coincidence on
  small deltas; the peaceful lab is authoritative.
- Standard-game saves now parse (chain gate ≥8 → ≥2; orig-A turn01/02 =
  2/6 AI-capital cities, arithmetic gate 11514 = 40×287+34 intact).

**Rig lessons (std lab):** the autosave rotates Ph_Auto/Ph_Auto2 per turn
(harvest by mtime); Ctrl+S saves into the LAST-USED folder (FallRome!) and
colliding names stack overwrite prompts that wedge everything; SPACE does
NOT skip units (arrow-move settlers to exhaust them, then Return; Return
only ends the turn when no unit has moves); AI turns under Xvfb take
minutes with dialog waves — `drain_dialogs.sh` + patience.

### T1-4 city records (2026-11-03): economics fields oracle-verified

Oracle channel: the game's own **City Status report** (Advisors→City Status,
F1) lists the human civ's cities in *save-record order* with size + gross
Food/Shields/Trade — cross-read against saves and RAM (memspool, city array
at RAM stride 92 identical to the save record).

Proven (7/7 rows match, FallRome/Va_Auto state):
- **u8@48 = gross food**, **u8@49 = gross shields**, **u16@46 = gross
  trade** — exactly the City Status columns.
- **u16@42 = net science**, **u16@44 = net taxes** (city-screen Net values;
  trade×rates then improvement multipliers: Roma 29T @7.0.3 → 20/9, market
  +50% → 30 tax, library → 13 sci).
- **u16@50 = owner** (1..7 like units), **u16@52 = city id** (sequential,
  i+1).
- **City size = u16 at (city_base − 13)** — duplicated byte at −14/−13 for
  WRE cities (15,5,11,6,12,4,11,10 all verified); for some AI/other-civ
  cities those bytes read 0/bogus → the *encoding* there is still open.
- `@COSMIC` (FallRome): **citizens eat 3 food/turn** (Roma 45F = 15×3 →
  surplus 0 = city screen Net Food 0), settlers eat 2, food/shield box =
  "10 rows × size(+1)", aqueduct >8, sewer >12.
- Independent-run comparison (two turn-0→1 runs from the same base): F/S/T
  + sci/tax are **deterministic** (tile economics); the short accumulators
  @86/@88/@90 tick differently — **@86 behaves like the shield box**
  (net = S − support on 7/7 sampled cities), @88 ~+1/turn typical with
  unexplained big jumps on war-event cities, @90 mostly static. Exact
  accumulator semantics open — best answered in a peaceful standard-game
  lab (war events distort FallRome turn 1+).
- Famine signal: Chersonesus @86 went 72 → −2 (s16) in one turn — the food
  box draining past zero (famine/capture), consistent with @86-as-storage
  but the dual use vs shields is unresolved.

Rig lessons (this session): the "Viewing Pieces" viewer swallows keys —
`v` exits it; queued AI-turn dialogs (Defense/Domestic Minister, Civ
Tutorial after capital loss, Game-saved toasts) block turn end AND Ctrl+S
— `drain_dialogs.sh` (allowlist dismiss, return-to-accept); Ctrl+S only
works with a movable unit active; a FallRome turn end takes minutes under
Xvfb (dialog waves) — patience + drain beats retrying; turn@668 of the
newest save is the only reliable turn counter (panel date is event-driven,
frozen "Dec 429" for turns).

### T1-3 movement (2026-11-02 night): Rules columns + kernel

- **`@UNITS` field 3 (written `N.`) = movement rate** — proven twice:
  (a) the game's unit panel reads **"Movers 1"** for standard Settlers
  (`Settlers, Exp, 0, 1.,0, …`); (b) editing that field to `4.` in both
  the global and `Original/Rules.txt` and starting a fresh standard game
  made the panel read **"Movers 4"** (install restored + sha256-verified
  afterwards). Cross-checked against classic Civ2 values across ~25
  units: Trireme 3, Caravel 3, Galleon 4, Destroyer 6, Transport 5,
  Cruise Missile 12, Nuclear Missile 16, Diplomat 2, Spy 3, Caravan 1.
- **`@UNITS` field 9 = shield cost / 10** (Trireme 40, Ironclad 60,
  Battleship/Carrier 160, Settlers 40, Spy 30 — all classic).
  **Field 10 = transport capacity (hold)**: Trireme 2, Caravel 3,
  Galleon 4, Transport 8, everything else 0.
  Fields 5–8 = attack/defense/hits/firepower (`Na`/`Nd`/`Nh`/`Nf`).
- **`@TERRAIN` field 1 = movement cost** — global file matches classic
  Civ2 on all 11 base terrains (Desert/Plains/Grassland 1, Forest/Hills/
  Tundra/Glacier/Swamp/Jungle 2, **Mountains 3**, Ocean 1; FallRome
  tunes Mountains to 4). **Field 2 = defense bonus** (classic relative:
  Plains 2 < Forest/Swamp/Jungle 3 < Hills 4 < Mountains 6).
- **`@COSMIC` "Road movement multiplier" = 3** (three road tiles per
  movement point) — comment-keyed row.
- Engine kernel (`engine::movement`): movement tracked in thirds of a
  point; terrain = 3×cost thirds; road = 1 third (multiplier 3);
  railroad = 0; classic "any movement left ⇒ one tile" rule; classic ZOC
  (moving from enemy-adjacent to enemy-adjacent tile ends movement);
  domain gating (sea↔ocean, land↔non-ocean). ROMFALL boots with 641/710
  units movable.
- In-world input on the rig: winject's SendInput was being discarded
  until a **`winject focus <hwnd>`** command (thread-attach +
  SetForegroundWindow + SetActiveWindow + SetFocus) made the game the
  Wine foreground window — then keys land. This is the fix for the
  post-reboot silence; keep it in every in-world recipe (focus, then
  key). XTest remains useless for the DirectDraw surface as documented.
- **Movement cost confirmed live**: with Settlers at move 4 (the rules
  edit), stepping onto **Forest** dropped the panel to **Movers 2** —
  exactly the parsed `@TERRAIN` Forest cost (field 1 = 2). Terrain cost
  deduction is oracle-verified end-to-end.

## T1-2 boot (2026-11-02 night): ROMFALL headless + checksum

- `totctl boot <file.sav|scn>` parses the proven 0x32 layout (Rust port of
  the Python walker) and boots the engine world state; `--turns N` / `--seed S`
  drive the kernel skeleton. ROMFALL.scn boots to **710 units / 146 cities /
  9000-tile 75×120 stored map**, civil names extracted as printable runs
  (leader first: "Valentianus III", "West Romans", "West Roman").
- **Checksum reproducible** (env-gated acceptance test
  `crates/totctl/tests/boot_romfall.rs`, runs in `make test`): boot is pure
  (same file + seed ⇒ same 64-bit FNV checksum); kernel turns are
  deterministic given the seed; the skeleton consumes no randomness yet.
- Layout finding from the port: the transporter section starts at
  **29728** (`168 + 4 + 14×N`), so the map header sits at **29900 only
  when N = 0** — which is every standard-game/ROMFALL file measured so
  far (Midgard's N=12 shifts it by 168).
- Civ-name blocks are extracted as printable runs (Python `null_strings`
  semantics); **per-slot field semantics stay unproven** — ROMFALL reads
  `leader, plural, adjective` but no offset→field mapping is claimed.
- Kernel skeleton deliberately omits the year-advance schedule (no
  oracle-verified table yet — folklore banned); year stays put until a
  turnloop experiment lands in T1-3. RNG: `GameRng` trait + placeholder
  MMIX LCG; the original PRNG identity is T1-6's open problem.

## RG-1 findings (2026-11-02, eighth+tenth sessions — standard-game setup state)

Materials: rig-driven **standard Original-game** new games with
**vision-verified wizard answers**, load-time autosaves
(`Original/Ca_Auto*.SAV` — the standard game autosaves into the game folder,
prefix from the ruler name; slots ROTATE, see the staleness lesson in
`../lessons.md`). Verified family (ninth session): **C2** = Large/Emperor/5civ,
**D2** = Large/Emperor/3civ, **E2** = Normal/Emperor/5civ, **S2** = Small/
Emperor/5civ, plus **S2b/S2c** repeats of S2's exact answers
(`/home/diego/review-tmp/oracle-runs/orig-{C2,D2,E2,S2,S2b,S2c}`). Older
A/B/C runs (all-Return rides, answers inferred): A = Chieftain + out-of-box
defaults, B = Deity + defaults, C = Emperor + defaults.

### Decoded setup fields

1. **`save[684] = difficulty level`** (0=Chieftain … 5=Deity, Rules.txt
   @DIFFICULTY order) — A/B/C = 0/5/4; C2/D2/E2/S2/S2b/S2c all = 4 (Emperor).
2. **`save[689] = rivals`** (number of AI civilizations) — competition
   dialog answer minus one: 5 Civs → 4 (C2/E2/S2 agree across three
   different worlds), 3 Civs → 2 (D2), 7 Civs → 6 (A and C, whose
   competition was the out-of-box default — see the dialog defaults below).
3. **Map dimensions at 29900 = `(2×height, width, height×width)`** —
   confirmed on all three sizes: Small 40×50 → `(80, 50, 2000)`; Normal
   50×80 → `(100, 80, 4000)`; Large 75×120 → `(150, 120, 9000)`. The wizard
   dialog's "40x50 squares" reads height×width. Save size tracks world size:
   ~70,047 (Small) / ~96,6xx (Normal) / ~162,8xx (Large).
4. **No front-section byte encodes the size ANSWER** — the three-way
   isolation (C2/D2/E2, plus S2) shows size only manifests structurally
   (dims at 29900, file length, map block); every other front-section
   candidate (660, 807, 826, 853, 894, 962 …) wobbles across same-size
   worlds, i.e. is world-noise.
5. **`save[698]` is world-derived, not a settings field** — 5–13 across
   worlds including same-answer repeats (S2=6, S2b, S2c differ); earlier
   "civs placed" reading retired. `save[655]` = 0x32/0x33 and 814/815, 842,
   853, 871, 892/893, 962 move with the world too.
6. Load-time saves all read **turn=1, year=−4000**.

### World generation is NON-deterministic (CORRECTION)

**S2, S2b, S2c — three runs with identical, vision-verified answers —
produced three DIFFERENT worlds** (~3.5k differing bytes each; same
structure: length 70,047, dims, 684, 689). The world-gen seed comes from a
non-settings source (time / PRNG state at load).

An earlier revision of this section claimed the opposite ("C ≡ D ≡ E
byte-identical ⇒ deterministic"). That was an artifact: D's and E's harvests
copied C's STALE autosave (the fresh saves had rotated into the other
`Ca_Auto*` slot — E's true save was a 162,927-byte Large world, initially
mistaken for a "stray"). The lesson: harvest checks must compare mtimes of
all rotating slots AND byte-compare against the previous baseline
(`docs/lessons.md`). Consequence for planning: RG-2 starting placement is
NOT reproducible from wizard settings alone; the T1-6 RNG-state hunt (RAM,
via `memspool.py`) is the path to reproducibility.

### RNG / seed state (T1-6 notes, 2026-11-02 night)

- **Save 684–689 is a byte-for-byte copy of a RAM globals block at
  `0x607658`**: `difficulty | 0x02 | seed-word | seed-word | rivals`
  (the seed word is `0x02XX`, i.e. one random byte + constant 02,
  duplicated). Verified live: in-world RAM at 0x607658 reads exactly the
  save's 684–689 bytes (S2f: `04 02 db 02 db 04`).
- The seed word is a **placeholder (`0x0200`) while the City Style dialog
  is open, set by the time the "In the Beginning" intro opens, and stable
  through world-gen into the save** (pre-gen vs post-gen RAM snapshots,
  runs S2d/S2f).
- **Patching it at City-Style gets overwritten by gen** (write 0x0237 →
  save came out 0x02db) — 0x60765a is a serialized MIRROR, not the live
  generator state. And two runs with the same final seed word (P1 vs S2f,
  both 0xdb) still produced different worlds → the 685–688 word is the
  classic combat/event-replay seed, NOT the world-gen state.
- **The game imports NO `rand`/`srand`/`time`**: exactly one
  `GetLocalTime` (call site 0x463369 — date-string formatting), one
  `GetSystemTime` (0x5b598a, adjacent to another GetLocalTime — a
  formatting pair), and three `GetTickCount` calls (0x58fc60/0x58fca5/
  0x592a66 — all inside ToTPP's own appended code, timing helpers).
  World-gen entropy therefore does NOT come from a time import in the
  original code — candidate sources: uninitialized memory, timer-driven
  UI event counts, or a hardware-derived value. This also explains how
  identical-answer rides can still diverge.
- LCG constant scan of `.text`: `214013/2531011` (MSVC rand) found only at
  **0x5b11f1** and `0x19660D` at **0x554a4f** — both in ToTPP's appended
  code region (its bundled msvcrt `rand`, e.g. for Lua `math.random`),
  NOT in the original game's code. The original RNG's constants are
  unknown — next step: disassemble the writer of the 0x607658 block (find
  code storing to 0x607658..0x60765d) and walk back to the generator.
- Exe layout notes for that hunt: `.text` 0x401000–0x5c3fde (original game
  code roughly below ~0x5a0000, ToTPP appended at the end), `.rdata`
  0x5c5000, `.data` 0x5cc000, `.idata` 0x691000, `totpp` section
  0x6b6000 holds the import table. Game runtime globals cluster at
  0x6076xx (between `.data` end and `.idata` — an unlisted block; code
  at 0x463322/0x46334a reads globals at 0x60764a/0x607652).
- **0x607658 is the global SETTINGS struct, not an RNG variable**: a
  store-site scan of `.text` finds hundreds of references (reads like
  `mov al, [0x60765b]`, writes like `mov [0x60765a], al`) scattered
  across the whole codebase — every subsystem touches the block, and the
  save serializes a copy of it at 684–689. So the 685–688 word is a
  *field of the settings struct* whose meaning (probably the game's
  combat-replay seed) cannot be inferred from the block alone. The true
  world-gen PRNG (which produced different worlds for identical
  settings) still has not been located; candidates remain uninitialized
  memory and a timer-driven event count. Next concrete step: trace the
  function that *initializes* the settings block on new-game start
  (write sites with immediate/zero stores) rather than reading it
  everywhere.

### Wizard reference (all items + defaults, vision-read)

- **Game-type list** (remembers last selection): 1 Original Game, 2 Science
  Fiction Game, 3 Fantasy Game, 4 Extended Original Game, 5 Begin the
  Midgard Scenario, 6 Begin a Scenario, 7 Load a Saved Game., 8 Play a Mod.
  The list WRAPS (Up from 1 → 8).
- **Start-mode menu** (after game-type, titled "The Test of Time"):
  Start a New Game (default) / Start on Pre-made World / Customize World.
- **Size of World**: Small 40×50 / Normal 50×80 / Large 75×120 (+ Custom
  button).
- **Difficulty Level**: Chieftain/Warlord/Prince/King/Emperor/Deity; the
  list wraps (5 Downs from Deity → Emperor — retro-explains C=4).
- **Level of Competition**: 7/6/5/4/3 Civilizations (+ Random); out-of-box
  default **7 Civilizations**.
- **Barbarian Activity**: Villages Only / Roving Bands / Restless Tribes
  (remembered default) / Raging Hordes (+ Random).
- **Game Rules**: Use Standard Rules (default) / Customize Rules.
- **Gender**: Male (default) / Female. **Tribe**: 21 tribes in 3 columns,
  Romans default. **Name**: Caesar. **City Style**: Bronze Age Monolith /
  Classical Forum (default) / Far East Pavilion / Medieval Castle.
- **Dialogs remember previous answers** across restarts (difficulty, size,
  competition, game-type all verified remembering). Never assume defaults —
  the remembered value IS your baseline; restore knobs changed by earlier
  variants before isolating a new one.
- **Arrow keys can be silently swallowed** when racing a dialog's opening
  (two early knob variants silently produced baseline settings). Reliable
  pattern: wait for the window TITLE → focus the NEW dialog HWND → keys
  ≥0.7 s apart → vision-verify the highlighted item → Return.
- **The new-game wizard chain**: game-type list → start-mode menu → Select
  Size of World → Select Difficulty Level → Select Level Of Competition →
  Select Level Of Barbarian Activity → Select Game Rules → Select Gender →
  Select Your Tribe → Please Enter Your Name → Select Your City Style →
  "In the Beginning" intro → Civ Tutorial popups → in-world (autosave
  written at world load). Plain Return confirms every dialog.

### Methodology

**Vision-model screen reading** (screenshots → PNG → a vision-capable model
reads dialogs literally) and **live memory access** (`tools/oracle/memspool.py`
— the game runs as a descendant of a spawner daemon, so `/proc` reads work;
snapshot/diff/scan commands). Both documented in `tools/oracle/README.md`.

## Files

| File | Produced by | What it shows |
|---|---|---|
| `rules-summary.json` | `totctl rules Rules.txt` | All 22 `@`-sections parsed; `@COSMIC` values, terrain/unit/government tables |
| `world-mp.png` / `world-mp.json` | `totctl map World.mp --rules Rules.txt` | ToT world map rendered to PNG (terrain-colored, voids black) |
| `world-mp-rivers.png` | `totctl map World.mp --scale 3` | River overlay (word bit `0x80`) — dendritic Earth river networks |
| `romfall-scnmap.png` | `totctl scnmap FallRome/ROMFALL.scn` | Embedded scenario map with river/road/railroad overlays |
| `anticchina-map.png` | `totctl map Scenario/china/AnticChina.MP` | ToTPP-era paired-slot `.MP` flavor: China with river networks + improvement flags |
| `romfall-walk.json` | `totctl save ROMFALL.scn` | Structural walk of a classic scenario (ToT 1.1, `0x0032`) |
| `warring-states-walk.json` | `totctl save "Warring States.scn"` | ToTPP-extended save (`0x0033`) walked: `ext033` units/cities/structural checks (this pristine scenario has no `LUAE` — the Lua tail framing appears in the deiYan/deiQin ladder rows) |
| `golden/warring-auto-walk.json`, `golden/warring-yan-walk.json`, `golden/warring-qin-walk.json` | `totctl save` on the 0x33 ladder | regression-protected turns 1/2/11 walks (T0-7) |
| `crosscheck-mp.json` | `tools/crosscheck_mp.py` (runs both decoders) | Rust `tot-kit` vs Python probe: layout/dims/top-6 histogram must match |
| `va-autosave-diff.json` | `totctl diff Va_Auto2.SAV Va_Auto.SAV` | Annotated diff of two real autosaves (oracle-methodology demo) |

## Findings (verified)

1. **`.MP` map format (ToT variant) — decoded.**
   8-byte header: `word width, word height, dword real_tile_count`; then
   `width×height` slots of **3 bytes**: `word` (little-endian) + 1 extra byte.
   - Terrain id = `word & 0x7F`; bit `0x80` = **river** *(corrected 2026-11-01,
     third session — initially mis-attributed to special resources; see
     finding 7 below for the proof)*
   - Void/non-existent tile = `extra & 0xF0` (marks Civ2's round-world voids)
   - `World.mp`: 150×120 = 18,000 slots; declared real-tile dword = 9,000,
     **counted non-void slots = 8,568** (both decoders agree — the 432-tile
     gap between dword and count is open), 90-byte tail

2. **The decode is provably correct — geography check.** Terrain histogram of real
   tiles: Ocean 5,633 (66%), Grassland 673, Plains 518, Forest 508, Tundra 279,
   Desert 273, Hills 210, Jungle 179, Glacier 143, Mountains 121, Swamp 31 — Earth's
   surface fractions. Mean-latitude per terrain: Glacier 0.081 and Tundra 0.144
   (polar), Forest 0.235 (boreal belt), Desert ~0.49 (mid-latitudes), Jungle 0.670
   (tropical). The rendered PNG is Earth with correct biomes.

3. **`Rules.txt` is fully machine-readable.** `@TERRAIN` rows 0–10 are the base
   terrains in id order (`Desert, Plains, Grassland, Forest, Hills, Mountains,
   Tundra, Glacier, Swamp, Jungle, Ocean`) — matching the MP bit decode exactly;
   subsequent rows are special-resource overlays.

4. **SAV/SCN structural walk works on real scenarios.** ROMFALL.scn (`0x0032`):
   turn 0; 7 civ name blocks decode — the walker extracts every NUL-run in
   each block (e.g. `Hengist`/`ief`/`Anglo-Saxons`/`Anglo-Saxon`: leader,
   an encoding-artifact fragment, plural and singular adjective); 56 techs
   pre-discovered; 10 wonders
   built, 1 destroyed; map header `[150, 120, 9000, 1, …]` consistent with the MP
   findings.

5. **ToTPP `0x33` saves keep the base layout at the front.** *Warring States.scn*
   walks successfully through settings + civ names ("Xiao", "Qin") at the same
   offsets as `0x32` — extension differences start later. (The `0x33` tail was
   decoded later the same day — see §T0-7.)

6. **The save-differ turns file sizes into semantics.** Two real autosaves 1.5 h
   apart: size delta exactly **−40 bytes = one ToT unit record** (a unit died or was
   disbanded); first changed region is offset 668–677 — precisely the `game` block
   where turn/year live. This is the oracle methodology of docs/plan.md working
   end-to-end on day one.

### T1-6 PRNG identified statically: Numerical-Recipes LCG at RVA 0x554a4f (2026-11-03 late)

- The exe contains **`state = state * 1664525 + 1013904223`** (Numerical
  Recipes constants, adjacent at file offsets 0x153e4f/0x153e56, RVA
  0x554a4f) operating on the global at **0x5dd53c**; the post-step value
  is used as a **float in [0,1)** (`fild; mov [ebp-0x1c],0x4f000000`
  trick at RVA 0x554a64-0x554a94).
- **0x5dd53c is a swap slot, not the turn RNG**: callers follow a
  save/swap/roll/restore pattern — save [0x5dd53c] to a local, load a
  per-context state from **[ctx+0x100]** into the global, roll, restore
  (RVA 0x5602ea load / 0x560cc2 restore). Only 9 references to
  0x5dd53c exist in .text.
- Runtime: writing state=1 into 0x5dd53c survived a full turn end
  unchanged, and no LCG^n(1) (n ≤ 200) appeared anywhere in low RAM —
  turn processing does not roll through the global; the live states are
  the per-context [ctx+0x100] fields. **Next hunt: identify the ctx
  struct (likely per-unit or the combat context) and the new-game
  seeding site; the save-serialized combat seed is one of those +0x100
  fields.**
- The setup seed (save 685-688) is constant across all 24 std-corpus
  saves (f510f502) — confirmed NOT the evolving state.

## T0-6 findings (2026-11-01, second probe session)

`totctl save` now walks deep into `0x32` saves. Verified on ROMFALL.scn, Va_Auto.SAV
and the ROMFALL_{ERE,HUN,VIS} variants:

1. **Section arithmetic closes exactly.** After the map header at 29,916:
   map data `13×9000+2 = 117,002` → blob `2×38×30 = 2,280` → `10,240` blob ends at
   **159,438**, which is precisely where ROMFALL's long zero run ends.
2. **`turn`/`year` proven.** Offset 668/670: ROMFALL turn 0 → Va_Auto2 turn 1 with
   year 5159 → 5164 (+5/turn for FallRome's 5-year turns); Warring States starts
   year −4000. The field is a turn counter — the *display* year ladder
   (Game.txt/initial settings) is still to be mapped.
3. **City block fully located.** 92-byte records, 16-byte name at record start;
   ROMFALL: **146 cities** (all named), Roma first, block [187,872, 201,304).
   One grid member, *Arsinoë* (Latin-1 `0xEB`), breaks ASCII-only scanning — the
   walker accepts Latin-1. Pre-name bytes belong to the previous record's tail
   (city-size-like shorts: Roma 29, Ravenna 8, Lambaesis 1 … per-field decode open).
4. **Unit section bounds & count via exact arithmetic.** Region between the 10,240
   blob and the city block: ROMFALL `28,434 = 34 + 40×710` and Va_Auto
   `28,474 = 34 + 40×711` (one more unit in-game — matches the autosave size delta).
   So: 34-byte preamble + N×40-byte records. The unit count is *not* in the preamble
   (only a state u16 at +30 changed: 2134 scenario vs 1 in-game).
5. **Unit field semantics remain open.** Records show x/y-like columns at bytes
   6–10 (values fit 150×120) and variant bytes at 14–19, but type/owner guesses did
   not validate — the next step is the in-game cheat screen and/or a
   move-one-unit save-diff pair. (Resolved by T0-6 — issue #3, closed.)
6. **Va_Auto saved at turn 0 with 711 units vs the scenario's 710** — consistent
   with a SCENARIOLOADED event creating a unit on load (Events.txt has
   CREATEUNIT-on-load logic); worth confirming via the events interpreter in T1-7.

## T0-11 findings (2026-11-01, third probe session)

1. **`.MP` word bit `0x80` is RIVER, not special-resource — correction.** Evidence:
   on `World.mp` the 246 bit-0x80 tiles are 100% on land (ocean specials would
   exist if these were specials), form chains up to 10 tiles (105 adjacent pairs
   vs ~60 expected for random isolated dots), and an ASCII render places them in
   dendritic networks matching Earth: a long southward chain in eastern North
   America (Mississippi system), clusters across Africa and Australia (Murray),
   one chain wrapping the x=0/x=149 seam. Special resources are **not stored in
   `.MP` at all** — they are RNG-placed per game (map tail likely holds seeds).
2. **The embedded SCN/SAV map section is decoded (ROMFALL, `0x32`).** Per Mercator's
   ToC the per-map block is Visible Improvements (7 B/tile) + Map (6 B/tile) + 2.
   The stored grid is **dense**: `width = hdr0/2 × hdr1 = surface` tiles
   (ROMFALL 75×120 = 9,000; `hdr0` = 150 is the *display* width — ToT tiles
   span 2 display columns). **Byte-position correction (2026-11-02, review
   #35):** the 6-B-stride tile fields read cleanly at fixed phases —
   terrain|river|special byte at `map_base + 6t + 4`, road/rail bits at
   `map_base + 6t + 14`; the exact interleaving of the fields inside the
   section (and the role of the 2-byte values Mercator places after the
   maps) is open. The original "+2 before the tile blocks" note came from a
   mis-phased reader; the corrected phases reproduce every histogram below.
3. **Terrain byte: terrain = `&0x3F`, `0x40` = special resource, `0x80` =
   river.** ROMFALL terrain histogram under the `0x3F` mask: ocean
   2,838 (including 70 special-resource ocean tiles), plains 1,756 / forest
   1,448 / grass 1,050 / hills 870 / desert 800 … A `0x7F` mask
   mis-decodes those 70 special ocean tiles as a nonexistent terrain id 74
   (`0x40|10`) — fixed 2026-11-02. Rivers: 534 tiles.
4. **Improvements: road = bit `0x40`, railroad = bit `0x80` at
   `map_base + 6t + 14`.** Verified spatially on ROMFALL: 3,497 road tiles
   form a connected empire-wide network; 413 railroad tiles cluster densely
   in the Italy/Gaul core. Of the rail tiles, 128 carry the road bit too
   (rails DO set `0x40` when a road is also present) and 285 are rail-only.
   ASCII renders show the Mediterranean, Italy's boot and the road/rail
   networks in historically sensible places. Remaining bits
   (`0x02` base, `0x08/0x10/0x20` variants — irrigation/mine/farmland/fortress
   candidates) are open; oracle save-diffs will pin them.
5. **The 7 visibility bytes per tile show near-identical per-byte distributions**
   (~530 nonzero each on ROMFALL) — consistent with per-civ fog-of-war snapshots
   (7 civs); unproven, needs a fog-changing oracle pair.
6. **`totctl` UX (T0-11):** `CIVTOT_HOME` env resolves relative input paths and
   defaults `--rules`; `--scale` on renders; new `scnmap` subcommand renders the
   embedded scenario map with river/road/railroad overlays (artifacts:
   `romfall-scnmap.png`, `warring-states-scnmap.png` where applicable).

## T0-8 findings (2026-11-01/02, fourth probe session)

**`AnticChina.MP` (ToTPP-era `.MP` flavor) decoded** — artifact:
`anticchina-map.png` + `golden/anticchina-map.json`.

1. **Layout:** `u16 display_width=224, u16 height=250, u16 real=28000,
   u16 maps=1` + a 90-byte front blob (`44 ae 38 00 3f 00` + 84×`0xFF`) +
   `6 × real` tile bytes + `real` per-tile flag bytes. Exact:
   `196,098 = 98 + 7×28,000`.
2. **Per stored tile: 6 bytes = [data slot][pad slot].** Data slot = `u16`
   word + 1 extra byte — the extra byte takes exactly 8 values in 0x20
   steps: 0x00 (25,955) / 0x20 (657) / 0x40 (305) / 0x60 (215) / 0x80 (253)
   / 0xA0 (199) / 0xC0 (169) / 0xE0 (247); its low 5 bits are always 0 and
   its meaning is open. Pad slot = `u16` word + 1 extra byte: word 0x003F
   on 22,488 records, 0x0001 on 4,293, and 22 further values (mostly
   `0xXX3F`: 0x023F 340, 0x043F 166, 0x103F 153, 0x803F 133) on the
   remaining 1,219; extra byte: 0xF0 on 25,882, 0xFF on 1,219, high-bit
   variants of 0xF0 (0xF7 268, 0xFE 150, 0xFD 85, 0xFC 57, …), and 264
   records with a LOW high-nibble (0x10 82, 0x20 37, 0x40 30, 0x70 26,
   0x50 19, 0x60 19, 0x30 16, 0x00 6, plus two-digit oddities like
   0x77/0x37/0x47) — semantics open. Dominant
   combinations: (0x003F, 0xF0) 20,884 and (0x0001, 0xF0) 4,209. The stored
   grid is dense `112×250` (display width 224
   = 2× stored — the ToT half-tile display convention, matching the SCN map
   headers).
3. **Data word = terrain | flags**, unifying with the stock `.MP` word:
   terrain = `word & 0x3F` (Warring States' own `Rules.txt` uses ids > 10:
   Town, Nomad Settlement, Deep Desert…), special resource = `0x40`
   (2,008 tiles — coastal clustering, fish-type specials *are* stored in this
   flavor), river = `0x80` (3,454 tiles), improvement flag = `0x3000`
   (304 tiles, 43% river-adjacent; exact meaning open).
4. **Verified spatially:** the decoded 112×250 grid renders as China — the
   Yellow River and Yangtze `0x80`-river networks, ocean east, Himalaya
   mountain chains west, `0x3000` improvement clusters in the river valleys.
5. **Open:** pad-slot low bits, the per-tile flag byte (mostly 0; values 1
   [240×] and 2 [180×] observed — start-position candidates), the 90-byte
   front blob (mirrors the stock format's 90-byte *tail*).

## T0-6 findings (2026-11-02, fifth probe session — unit & city records decoded)

Controlled experiments over ROMFALL + its playable-civ variants (ERE/HUN/VIS)
+ the Va_Auto session pair closed the unit/city records:

1. **Unit record (40 B) field map, proven:**
   - `u16@6 = x`, `u16@8 = y` on the **logical** 150×120 grid
   - `u8@14 = unit type` — index into the scenario's `Rules.txt @UNITS`
     (ROMFALL's 39 distinct types map to sensible rosters: WRE/ERE Legion &
     Infantry, Hun Warrior, B. Warriors …)
   - `u8@15 = owner` — **civ 0 = barbarians, civs 1–7 = the players**
     (`civ_names[n-1]`), bit `0x10` = leader unit (the single owner-16 unit
     is *Pope Leo I*)
   - `u16@36 = unit id` — **re-assigned on every save** (ROMFALL_ERE differs
     from ROMFALL at bytes 36–37 of 709/710 unit records — per-save ids.
     Outside the unit section the two files differ in 324 more bytes:
     **292** in the city records (all 146 cities differ at bytes 52–53,
     2 B each — owner re-assigned to the ERE civ, the ERE save being a
     later game state) and **32** elsewhere (5 settings bytes: 674,
     679–681 = human civ 1→6, 687; 3 civ-block bytes; 2 unit-preamble
     bytes; 22 post-city bytes). Total 1,418 + 292 + 32 = 1,742
     differing bytes.
2. **City record (92 B): `u16@58 = x`, `u16@60 = y`** (logical grid, same
   space as units). Tail shorts @86–91 measured, e.g. Roma (5, 5, 8) —
   **not** the city size: a size-like short sits in the two bytes
   immediately BEFORE each name (Roma 29 = its in-game size; see item 3 of
   the second session above), i.e. at the final short of the preceding
   92-B slot; the exact field boundary is open.
3. **The map geometry is settled.** The terrain grid stored in the map
   section is **75×120 dense** (`hdr0/2 × hdr1`); unit/city coordinates live
   on the **logical 150×120** grid. Mapping (proven by a unique alignment
   optimum): `sx = (x − (y&1))//2 − 1, sy = y` — diamond tiles spanning 2
   logical columns with 2 border columns at the west edge.
4. **Verification (measured per file by the walker — `garrisons_measured` /
   `city_terrain_measured` fields):**
   - 145/146 cities have a garrison unit on their exact tile (holds for
     ROMFALL, ROMFALL_ERE and Va_Auto alike);
   - under the diamond mapping, 141/145 cities sit on land, 0 on ocean
     (4 on desert — North-African cities), at the unique optimum dx=−1;
   - of 710 units, the **32 sea-domain units** (per the scenario's own
     `Rules.txt @UNITS`: Scan. Warship ×5, Roman Warship ×13, V. Warship
     ×7, Heavy Warship ×7) **all sit on ocean tiles** under the mapping,
     and all 678 remaining units sit off ocean — the domain check
     supports the mapping. (Independently re-verified with a raw decode
     in both round-2 reviews; a round-1 objection to this check used a
     wrong domain mapping — from the global instead of the scenario
     Rules.txt — and was withdrawn.)
5. **The Va_Auto/Va_Auto2 autosaves are from different play sessions** (the
   id ranges 2..711 vs 1452..2171 prove re-registration) — they are *not* a
   clean one-turn diff pair; true same-session pairs need the T0-10 oracle.

## T0-7 findings (2026-11-02, sixth session — 0x33 tail framing)

Materials: `deiQin.sav` (turn 11) / `deiYan.sav` (turn 2), Warring States.

1. **The `0x33` tail is framed by magics.** After the base-`0x32` front
   (which applies unchanged: map arithmetic `13×28,000+2`, header
   `[224, 250, 28000, 1, …]`, 10,240-blob ending at 411,214) comes the
   ToTPP extended state, then **`LUAE`** + the **serialized Lua state as
   plain readable text** (`return { ["tableTileCulture"] = { ["tile(18:4)"]
   = {…} … } … }`), ending with an **`EVNT` + 4-byte trailer** at EOF.
2. **The Lua blob is the scenario's persistent state**: deiQin carries
   5,477,281 bytes of it with 23,969 `tile(x:y)` entries (86% of the
   28,000-tile map) under tables like `tableTileCulture`/`cityTileWealth` —
   this is why Lua-scenario saves are megabytes.
3. **ToTPP writes plain `0x32` (no tail) for scenarios without Lua events** —
   verified via the T0-10 oracle on Midgard. So `0x33` ⇔ Lua/extension data
   present, which also scopes T1/T3 save writing.

## T0-7 findings (2026-11-02, seventh session — mid-region decoded)

Materials: the four-file Warring States ladder — `Warring States.scn`
(pristine, turn 1), `Hu_Auto.SAV` (oracle rig autosave, turn 1),
`deiYan.sav` (turn 2), `deiQin.sav` (turn 11). All offsets relative to the
end of the base front (411,214 here):

| Range | Content |
|---|---|
| `[+0, +32+15×surface)` | extended per-tile arrays, mostly sparse; a dense **6 B/tile band** (168,960 B = 6×28,160 — 960 B beyond 6× the 28,000-tile surface; exact structure open) and a **10,240-B blob** sit inside; sub-layout open |
| `[+32+15×surface, …)` | **unit records, 40 B each, count-based** (no fixed cap written) |
| `+4 B` | interlude |
| next | **city records, 92 B each, count-based** (same record layout as 0x32: name@0, x u16@58, y u16@60) |
| `[city end, LUAE/EOF)` | fixed-size post-city block (~12.9 KB here; trade-route-like records + sequential u16 id lists; internal layout open) |

**The 0x33 unit record is a different layout from the 0x32 base record**:

- `@0 u16 id` — sequential, **re-assigned on every save** (proof: scn ids
  6527..7967 → Hu_Auto ids 10..1450, sequential from 10 with **17 records
  carrying id 0**; same 1,442 units, of which **1,352** sit at an (x,y)
  that occurs somewhere in the .scn after load+autosave);
- `@10/@12 s16 x/y` — **logical** grid coords (not the stored/diamond
  space); off-board units carry negative coords (−100…−700 observed:
  −200 deiQin, −700 deiYan/Hu_Auto);
- `@18 u8 type` — Rules.txt @UNITS index (190 rows in Warring States,
  values 0..188 observed);
- `@19 u8 owner` — 0=barbarians, 1..7 = civs (exactly 8 distinct values);
- `@28 u16` optional link, 0xffff = none (home city?);
- rest flags/counters (open).

**Decomposition** (walker `ext033`; the arithmetic is *descriptive* — the
post-city term is the leftover by construction — so validity rests on the
falsifiable structural checks in `ext033.checks`: the formula landmark
sits inside the measured block, city-0 name is printable, and no competing
92-B name grid exists
outside the walked one; corrupting a unit id flips them to FAIL):

```
deiQin   510821 = 419992 + 40*1647 + 4 + 92*132 + 12801
deiYan   501981 = 419992 + 40*1449 + 4 + 92*121 + 12893
scn      501513 = 419992 + 40*1442 + 4 + 92*119 + 12889
Hu_Auto  501513 = 419992 + 40*1442 + 4 + 92*119 + 12889
```

The unit-start formula `32+15×surface` is a *landmark* that lands one
record into the block: an id-0/f4-0 slot precedes the f4=2 bulk in all four
files (whether it belongs to the unit array or terminates the preceding
per-tile section is open). deiQin − deiYan = **+198 units (+7,920) + 11
city slots (+1,012) − 92 in the post-city block = +8,840 exactly**. The
turn-1→2 delta (+468) decomposes as +7 units, +2 city slots, +4 bytes in
post-city. The landmark formula held for all four files (one map);
re-verify on a second 0x33 map before trusting it generally.





## Reproduce

```bash
G=/mnt/d/games/CIVTOT   # or export CIVTOT_HOME=...
python3 tools/totctl.py rules  $G/Rules.txt -o /tmp/rules.json
python3 tools/totctl.py map    $G/World.mp --rules $G/Rules.txt \
        -o /tmp/world.png --json /tmp/world.json
python3 tools/totctl.py save   $G/FallRome/ROMFALL.scn -o /tmp/walk.json
python3 tools/totctl.py diff   $G/Scenario/FallRome/Va_Auto2.SAV \
                                $G/Scenario/FallRome/Va_Auto.SAV -o /tmp/diff.json
```
