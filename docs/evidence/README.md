# Tier-0 Evidence — first format probe results (2026-11-01)

Artifacts produced by `tools/totctl.py` (Python reference probe) against the local
install `/mnt/d/games/CIVTOT`. These validate the feasibility claims in
[../feasibility.md](../feasibility.md) with measured data.

#### Record framing FIXED: units and cities in the game's own framing (2026-11-03)

**Supersedes every older unit/city offset in this file** (T0-6, T1-4, T1-8,
the T1-6 "unit-record corrections"). Both walkers (`tot-kit` `sav.rs`,
`tools/totctl.py`) framed records wrongly: unit records 6 bytes late, city
records 34 bytes late. The region arithmetic `blob end → first name =
40·N + 34` was always right; the 34 bytes are the **head of city record
0**, not a unit preamble. So the units are exactly `40·N` from the blob
end, and each city record starts 34 bytes before its name.

Proof (re-derived by `crates/tot-kit/tests/framing_install.rs` and reported
per file as `cities.homes_measured` in the walk JSONs):

- **Home city.** Every homed unit standing on a city tile is homed in that
  city: ROMFALL **349/349** in true framing, **0/342** in the old one.
  Standard-game lab turn 44: 10/12 vs 0/10.
- **Owner.** City u8@10 equals the owner of every unit in it (ROMFALL
  116/116). The old "owner u16@50" (true @84) is not the owner: Roma
  reads 3 there.
- **Geography.** In the old framing each name carried the *next* city's
  coordinates, which put Brundisium west of Roma. True framing gives Roma
  (47,79), Brundisium (54,84) east-south-east, Syracusae (47,93) south.
- **Size = u8@11.** It reads the City Status values for all 8 WRE cities
  (15,5,11,6,12,4,11,10) **and** reads cleanly in standard games and for
  AI civs, where the old "u16 at base−13" read 0.
- **The Madrid "mirage" was real.** True framing puts Madrid at **(53,9)**,
  the position the screen showed; (43,37) is Pasargadae. The T1-4 note that
  "settled" (43,37) was the mistake (its RAM patch proved nothing either
  way).

True layouts (the classic Civ2 records shifted by two bytes):

| Unit (40 B) | | City (92 B) | |
|---|---|---|---|
| x u16@0, y u16@2, z @4 | | x u16@0, y u16@2, map i16@4 | |
| attr u16@6 (0x2000 veteran) | | owner u8@10, size u8@11, original owner u8@12 | |
| type u8@8, owner u8@9 | | food box u16@28, shield box u16@30, u16@32 open | |
| moves spent @10, visibility @11 | | name 16 B @34 | |
| damage @12, last dir @13 | | net science u16@76, net taxes u16@78 | |
| orders @17, home city @18 | | gross trade u16@80, food u8@82, shields u8@83 | |
| stack prev/next i16@26/@28, id u32@30 | | u8@84 open (old "owner"), city id u32@86 | |

Identity widths above correct the superseded u16 claims: native dword writes
and cases above 65,535 are recorded in §Live hut-entry parity and unit allocation
and [unit-allocation.json](unit-allocation.json) (review follow-up #109).

Old-to-new translation: unit `old − 6` with index `old + 1`, and old
record 0's bytes 0..5 were the previous unit's tail. City fields at old
`@0..57` → `+34`, same record. Old `@58..91` (x/y, the boxes, "tail90") →
`−58` **in the next city's record**, so every old per-city box value
belonged to the following city.

Consequences: the engine's unit index now equals the game's (the battle-2
defender is `units[510]`, game unit 510). The old walker invented a fake
last unit out of city 0's head (a "leader" at (47,79)) and dropped real unit
0. The engine's city owners and positions were wrong for every city.
**Open:** the T1-4 growth/box samples (Madrid, Carthage, Little Bighorn
turn citations) were read in the old framing, i.e. off by one city. The
growth rules still fit true-framing data (Carthage turn 13→14: size 1→2,
box 18→2 within one record), but the per-sample citations need
re-reading. The 0x33 (ToTPP) records were not re-checked: their unit
layout differs (id@0, x@10) and the home test is inconclusive there (2/3).

#### Headless turn loop (v0.2.0, 2026-11-03)

`engine::turn` (`WorldState::run_turn`) runs one turn in three steps:
1. Cities grow or starve on the replayed T1-4 model. Gross food is the
   save's value, held constant.
2. TURN, TURNINTERVAL and RANDOMTURN events fire and apply. RANDOMTURN
   rolls `next_below(n) == 0` on the kernel generator.
3. The turn advances and movement resets.

**Step order and the RANDOMTURN generator are engine choices, not
decoded**; TURNINTERVAL is modelled as `turn % interval == 0`. The loop is
deterministic. Checks:
- `turn::tests` (asset-free, CI): 50 turns, same seed ⇒ same
  fingerprints.
- `fallrome-50turn.txt` (golden.sh run50): FallRome with its own Rules and
  Events.
- `standard_game_install.rs`: a standard game, `Original/Ph_Auto.SAV`,
  50 turns.

Fixed on the way: `totctl run` read FallRome's non-UTF-8 Rules.txt with
`read_to_string`, so the old 50-turn golden had silently run with **no
rules**.

#### T1-4 growth replayed on the whole std1 series (2026-11-03)

With true record framing (§Record framing FIXED), **settlers are visible**:
they are units of type 0/1 homed in the city (u8@18). The old note "settlers
are invisible in the save" was a framing artifact.
`tools/growth_series.py` writes the lab's 26 saves as
`docs/evidence/t1-4-growth-series.tsv` (turn, city, size u8@11, box u16@28,
gross food u8@82, settlers). The engine test
`cities::tests::std1_series_replays` steps the model across every save
pair:

- **126 transitions checked (10 of them growth events), 125 exact, including 9 of the growths**.
  Pairs where the settler count changes mid-span, or where growth
  re-assigns tiles before the last step, are skipped. One mismatch: Ur,
  turn 40→41, box 1 observed vs 2 predicted, while its F moved 4→5
  across the growth.
- Food eaten = 2 × size + 1 per settler, the standard `@COSMIC` rows
  "food each citizen eats" = 2 and "Settlers eat (govt <= Monarchy)" = 1.
- Growth when the box **exceeds** `(rows − 1)(size + 1)` (18/27/36).
- **On growth the box restarts at that turn's surplus, not the overflow.**
  This corrects the old "carry" rule, which was fitted on mis-paired
  cities. Utica, turn 41: 28 + 1 = 29 at size 1 → size 2, box 1 (carry
  would give 11). Persepolis, turns 22→27 (box 22 after a settler
  shrink): grows, then +1 × 4 = 5 (carry would give 9).
- Building a settler shrinks the city by one and keeps the box (Persepolis
  turn 20, Utica turn 40).

#### T1-7 treasury + effects applied to the world (2026-11-03)

- **Treasury = i32@10 of each civ-properties block.** There are 8 blocks
  of 3348 B at save @2944, indexed by owner (0 = barbarians), and they end
  exactly at the transporter section @29728. ROMFALL:
  `[4050, 10000, 200, 100, 100, 200, 10000, 208]`.
  **Oracle patch-and-prove:** the live game keeps the same table at VA
  `0x5F68A0`, located by scanning for ROMFALL's untouched civ 0/6 block
  bytes. The WRE value read 9919 and the status panel showed
  "9919 Gold". Writing 12345 there and pressing W made the panel show
  "12345 Gold"; the value was then restored.
- **Three more interpreter bugs found against Macro.txt and FallRome's
  Events.txt:**
  - GIVETECHNOLOGY was read through a `tech=` key that never occurs (the
    file writes `technology=<index>` + `receiver=`), so it never fired.
  - CHANGEMONEY dropped its `receiver=`.
  - CITYTAKEN's `city=` was not matched at all, so taking any city fired
    all 9 CITYTAKEN events.
  - Action civ parameters now resolve `TRIGGERATTACKER` /
    `TRIGGERDEFENDER` (Macro.txt) against the trigger context; FallRome
    uses both.
- **`engine::effects`** applies the effects: `WorldState::apply_effects`,
  `fire_event` (fire + apply), and `move_unit_with_events` (a kill fires
  UNITKILLED). CHANGEMONEY clamps at 0 (Macro.txt). CREATEUNIT places
  `count` units at the first valid location. **"Valid" is the engine's
  rule, not oracle-checked**: on the map, terrain fits the domain, no
  other civ's units. GIVETECHNOLOGY is reported as skipped (no per-civ
  techs in the state). `crates/engine/tests/events_install.rs` fires the
  real Events.txt against the booted ROMFALL: Merchant kill → WRE
  10000 → 10050; Theodoric I's death → a veteran Theodoric II for the
  Visigoths at (24,60), once.

#### T1-7 events: matching and CREATEUNIT corrected against Macro.txt (2026-11-03)

The game ships its own macro-language reference, `$CIVTOT_HOME/Macro.txt`.
Three interpreter behaviours were wrong against it, and the two env-gated
event tests asserted content that is not in the install's FallRome
Events.txt (`owner=TRIGGERDEFENDER`, 7 locations). The real file, identical
under `FallRome/` and `Scenario/FallRome/`, has `owner=Visigoths` and 6.

- **Names match exactly** (case-insensitive), not by prefix. Macro.txt:
  "unit types, civilization names, and so on must match the corresponding
  names in the rules.txt file exactly". Prefix matching made a Theodoric I
  kill also fire FallRome's "Theodoric II dies" event.
- **`locations … endlocations` belongs to the preceding CREATEUNIT**. The
  parser used to emit it as a separate action, so the interpreter created
  nothing. Points may carry an optional `,z` (map); it is parsed and
  dropped.
- **CREATEUNIT creates `Count` units (default 1) at the first valid
  location**, with later ones as fallbacks. The interpreter used to emit
  one unit per location. `Randomize` and `InCapital` are not modelled.
- UNITKILLED: "Attacker is the civilization that killed the unit, defender
  is the one who owned the unit", and it fires whichever side initiated
  unless `Defender Only` is given (used by `combat::Battle::unitkilled_ctx`).

#### T1-8 save writer (2026-11-03): surgical edits accepted by the original

- `tot-kit::sav::SavEditor` — surgical `0x32` writer: validates the file
  with the same gates as the parser, then applies typed edits to proven
  offsets: turn@668, year@670, difficulty@684, rivals@689, unit x@0/y@2/
  type@8 (40-B records), city name@34 (16-B slot, ≤15 bytes), city
  x@0/y@2 (92-B records) — game framing, see §Record framing FIXED (the
  offsets here were old-framing until 2026-11-03). A no-edit round trip is **byte-identical**
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

### T1-4 tile yields: first fits + open geometry (2026-11-03 night) — SUPERSEDED by §T1-4 tile yields DECODED (2026-11-04)

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
- **REVERSED by §Record framing FIXED**: Madrid is at (53,9); the
  verdict below read the next city's coordinates.
- *(superseded)* **Settled by patch-and-prove (final)**: @58/@60 IS the city's true
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

### Historical T1-4 city-model hypothesis (2026-11-03 evening, superseded)

A fresh STANDARD game driven turn-by-turn through the original under the
memspool daemon (`tools/oracle/stdloop.sh`), autosaves harvested per turn
(`~/review-tmp/oracle-runs/std1/`): 5 AI capitals + a human-founded Madrid,
turns 4–43, zero war noise.

**Historical hypothesis, not the current algorithm.** The following notes
used the old record framing and selected series observations. Native growth,
worker and city-phase checks below supersede the capacity/collection claims.
The approximation and its self-replay tests have been removed from the engine.

**Growth model (7 growth events, turn-by-turn samples, 4 cities):**
- **Food box = u16@86** (true framing: u16@28 of the *next* record — §Record framing FIXED; sample citations below are off by one city); ticks by surplus (`gross_food@48 − food_per_citizen×size
  − settler_food×settlers`) each turn.
- **Growth capacity = (rows − 1) × (size + 1)** with `@COSMIC` rows=10 →
  **18 at size 1, 27 at size 2**. The `@COSMIC` comment
  ("rows * city_size+1 = box") is garbled — the measured rule is
  (rows−1)×(size+1).
- **Growth is strictly greater**: a box sitting exactly at 18/27 persists
  (Madrid fb 18@t36, LB/Carthage 18@t22/27@t40 — all sat a full turn).
  Overflow carries: `box −= cap` (Madrid 18+4=22 → grow → 4 ✓ t38;
  Carthage 27+2=29 → grow → 2 ✓ t41).
- The former `engine::cities` approximation implemented this hypothesis; it
  has been removed in favor of the checked native phase.
- **Shield box = u16@88** (true: u16@30, same caveat): ticks by net shields (S − unit support), resets
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
- *(superseded by §Record framing FIXED: owner is u8@10, size u8@11, all
  offsets below +34)* **u16@50 = owner** (1..7 like units), **u16@52 = city id** (sequential,
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
  MMIX LCG. *(Superseded: the original generator was identified and
  runtime-verified in T1-6 on 2026-11-03 — see §T1-6 above.)*

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

### City founding: `create_city` decoded in part (2026-11-04, v0.6.0)

Source: `civ2.bak` `0x432810` = `create_city(x, y, civ, map)`, 6
callers. Engine: `engine::founding`.

**Decoded:**
- **Slot:** a free slot (city id @86 = 0) is reused, else the city count
  (RAM `0x607668` = save @700) grows, up to 255.
- **Record:**
  - x@0, y@2, map@4, owner@10, original owner@12;
  - city id @86 from a global counter (`0x5d1504`);
  - flags @6 = `0x20000` (`mov` at `0x432b2f`), a **transient
    refresh-pending bit**. The city-totals function `0x4c45ff` calls
    `0x430232` (`0x4c46b7`), which runs its body when that bit is set
    (`test byte @8, 0x2` at `0x43024b`), when the global `0x607657` is set,
    or every 16th turn per city (`(turn + city) & 0xF == 0`). It clears the
    bit first (`and …, 0xfffdffff` at `0x43028c`). Two other writers set
    it too (`0x433503`, `0x433570`; not identified). So the bit never
    survives to a save: 0 of 1,480 city records in 51 saves carry it, including six cities
    saved 0–2 turns after founding (review #102). The engine sets it in
    `found_city` and clears it in the turn's production step;
  - size @11 = 1;
  - boxes @28/@30, u16@32, worked tiles u32@50 = 0;
  - improvements @54 cleared.
- **Civ:** the civ's city count (civ block u16@112) increments, and the
  founding turn is stored at civ @22.
- **Tile:** gets the city bit (byte +1 `0x02`).
- **First city:** the civ's first city gets the Palace (`0x4300af`).
- **Build item @59:** the buildable unit of role 1 (defend) with the best
  defence / cost, ties to the later unit (`0x432d47`..`0x432e02`). The
  tribe's default unit is overridden whenever any defender is buildable.
- **Flags:** `0x80` when a neighbour is ocean, plus `0x200000`
  (`0x4332c8`) when a neighbouring water tile's body has more than one
  tile in the landmass table (`0x61bcac`). The std1 data confirms both on
  all five coastal foundings (review #102). **The engine does not set
  `0x200000` yet**: its map keeps no landmass table. Nothing in the engine
  reads it.

**Not decoded, and the engine's rules instead** (flagged in
`founding.rs`):
- the name (`0x4323af` uses per-tribe lists);
- the first worked tile (the original reassigns workers later);
- founding validity (land, no city within distance 1);
- the population cost of Settlers: no size write was found in the
  production step, so it is in the unit-creation path or elsewhere. The
  engine uses the classic one population, and a size-1 city waits.

### T1-4 production DECODED (2026-11-04): shield box, rows, completion

Source: `civ2.bak`. The production step is `0x4c6210`; shield rows come
from `0x4c1bdf`. Engine: `engine::production` (`production_step`,
`shield_rows`), wired into `run_turn` after growth.

**The step:**
- **Net shields** = the city's shields (after multipliers and waste) −
  unit support. 0 while flag `0x1` (disorder) is set; never negative.
- **Box:** grows by net, except when the build item (record i8@59) is −39
  (Capitalization).
- **Cost:** Rules cost × rows.
  - Units: @UNITS field 9; improvements: @IMPROVE field 1.
  - **Human rows:** @COSMIC row 4.
  - **AI rows (corrected 2026-11-05, #89):** `13 − d`, +1 if d < 3,
    +1 if d = 0. Only if cosmic != 10: multiply by cosmic/10, then round
    up to even. Default rows by difficulty are **15/13/12/10/9/8**.
  - **Runtime check:** the live global `0x6537fc` reads 10 at
    difficulty 4.
- **Completion:** `box ≥ cost` → the box drops by the cost; the unit is
  created (homed in the city), or the improvement bit set (`0x4300af`).
- **Not modelled:**
  - obsolete-unit switching (the build moves to the newer unit once the
    civ knows its advance);
  - wonders (ids ≥ 40; one path resets the box to 0);
  - veteran units;
  - the Settlers-at-size-1 case at Chieftain;
  - AI build choice.

**Replay** on the std-lab series, the one-turn pairs only (turn t → t+1,
using t's stored shields and computed support): **53/70** boxes exact.
The misses are explained by shields that the original recomputes at
processing time, after growth: the stored @83 of turn t is the previous
evaluation. They all fit rows = 10 with a net 1–2 higher. Exact
turn-pair parity is T1-10.

**AI rows and late adjustment (#89, 2026-11-05):**
`production-rows.json` and `production-row-vectors.txt` re-derive 1,108
cases by executing original `0x4c1c2c..0x4c1dd3` and its clamp callee.
This fragment begins after unit support and ends before the unit loop;
there are no call stubs. Reproduce with `CIVTOT_HOME=... PYTHONPATH=...`
`python3 tools/production_check.py --vectors /tmp/production-rows.txt`.

At turn >= 200 and difficulty > 1, if the power leader is human, start
adjustment at 2. With wonder slot 23 != -1 and some AI knowing advance 73,
subtract 1, and another 1 unless humans know both 73 and 58. With wonder
slot 25 != -1 and some AI knowing advance 76, subtract 1, and another 1
unless humans know 76. Clamp adjustment to 0..2 and subtract from AI rows
before optional cosmic scaling. The original sign-extends the knowledge
mask bytes before `mask & ~humans`; preserve this for civ 7. Destroyed
wonders (-2) satisfy the original != -1 test.

RAM `0x60777a..0x607782` maps to file bytes 974..982: the power order,
weakest to strongest, populated from civ ranks at `0x46e238..0x46e2ab`.
The row code reads its last slot at `0x607781`. Global advance-owner masks
start at RAM `0x6076d2` (file 806); discovery sets the owner's bit at
`0x49e091..0x49e0c0`. All 100 masks equal the corresponding per-civ tech
bitfield union in the standard and FallRome saves. The engine derives
these masks from live knowledge and retains the saved power order in its
checksum. Updating the power ranking itself remains separate work.

### Happiness and disorder (#86, 2026-11-05)

Sources: `civ2.bak 0x4c4925..0x4c4fa9`, normalization `0x4c3d3b`,
unit unrest `0x4c1bdf`, disorder `0x4c999a` (happy minus unhappy < 0).
`happiness.json`, `happiness-vectors.txt` and `happiness-saved-vectors.txt`
are derived artifacts. Reproduce with optional external Unicorn:

```
CIVTOT_HOME=... PYTHONPATH=... python3 tools/happiness_check.py --vectors /tmp/happiness.txt --saved-vectors /tmp/happiness-saved.txt
```

The 1,000 invented cases execute original happiness/normalization/building/
wonder callees, supplying luxury arrows, military unrest and garrison
counts. The garrison lookup is explicitly excluded in those cases. The
156 saved-input checks additionally execute original unit support, unrest,
finance and the actual garrison lookup, with original saved records and
decoded Rules type fields. They cover governments 0/1/2 on the surface map;
fundamentalist type flags and scenario-specific air exceptions are excluded.
Tile trade and shield totals are supplied from stored records, and specialist
counts from the packed specialist slots. This is not a complete-turn claim.

- Human initial unhappiness is `size + 1 - (cosmic[7] - difficulty)`.
  A Palace with no military unrest or home military count floors the
  content term to 2 when it would be lower. Empire factor is
  `max(1, (cosmic[8] - 2*difficulty + (game_flags[660]&4 ? 2 : 0)) *
  ((government>>1)+2) / 2)`. Except under Communism, add
  `(saved_civ_city_count - factor + city_index % factor) / factor`, with
  signed truncation. AI initial unhappiness is `size + 4 - cosmic[7]`.
- Track overflow unhappiness separately as angry citizens. Normalize
  before and after each effect group, including the specialist population
  cap, cancellation between happy/unhappy and angry-citizen conversion.
  Luxury happiness starts at `luxury_arrows >> 1`.
- Colosseum (14) removes `3 + knows(24)` unhappiness. With advance 55,
  Cathedral (11) or wonder 10 removes `3 - knows(15) + knows(82)`.
  Temple (4) removes `knows(56)+knows(9)`, doubled by wonder 5.
  A Courthouse or Palace adds one happy citizen under Democracy.
- Fundamentalism clears unhappiness/anger. Governments below Republic
  use martial law: up to 3 armed garrison units, or twice their number up
  to 6 under Communism. Republic exempts its first unrest unit; Democracy
  doubles unrest. Police Station (33) or wonder 21 subtracts one from that
  multiplier. Aircraft/away units and nearby fortresses enter through
  the separate unrest computation; fortress distance is the original
  wrapped Manhattan sum / 2 (`0x566c40`).
- Wonder 1 adds one happy citizen throughout the owner's cities and two
  more locally; wonder 27 adds one; local wonder 13 clears unhappiness;
  wonder 15 removes two unhappy citizens. All use active-wonder lookup.

The engine matches native recomputation on **156/156** inputs. Stored
happiness matches **10/10 standard** and **131/146 FallRome**. Original
bytes reproduce the same 15 FallRome stored discrepancies, listed in the
artifact; their historical cause is unresolved. Do not relabel this as
exact save/turn parity. `run_turn` writes happy/unhappy bytes and updates
disorder before production; an install test proves production stops and
can resume when the mood changes.

Civ i16@112 (RAM `0x5f6910`, stride 3348) is the saved city count used
for empire unhappiness: founding increments it at `0x4329ee`, removal
decrements it at `0x434750`. Retain it in the world/checksum and increment
on founding. Some FallRome saved counters differ from the current record
owner count, so silently replacing them with a recount loses source state.
Military input checks beyond the saved governments, celebration/turn-order
effects and the independent #13 city-turn gate remain outstanding.
Ten additional invented military cases execute full original `0x4c1bdf`
under Democracy: home/own/foreign cities, open ground, near/far/foreign
fortresses and aircraft roles. Rust world-level cases match those results.
The scenario-specific air exception and non-surface cases remain unmodelled.

### Treasury and maintenance (2026-11-05)

Sources: income `0x4c9d58..0x4c9d95`, upkeep `0x4ca5ae`, payment loop
`0x4ca6e0`, caller `0x4cb6b7..0x4cb6e5`. The caller charges upkeep
only to civs in the saved human mask. Income is skipped under anarchy or
negative happy-minus-unhappy. Assigning an engine AI player leaves this
mechanics mask unchanged.

`maintenance.json`, `maintenance-vectors.txt` and
`maintenance-saved-vectors.txt` are derived by:

```
CIVTOT_HOME=... PYTHONPATH=... python3 tools/maintenance_check.py --vectors /tmp/maintenance.txt --saved-vectors /tmp/maintenance-saved.txt
```

@IMPROVE field 2 supplies table byte `0x5f6685 + 8*id` (loader
`0x4133d5`). Barracks (2) subtracts one at difficulty < 2 if its base is
nonzero, adds one for advance 35, and another for the enabled advance 53.
The original follows a disabled advance's first prerequisite before the
knowledge check; `no` (-2) returns false. The Rules loader sets availability
from the first prerequisite at `0x413146..0x413170`. The current Rules
fixtures exercise both ordinary and disabled slots. Other costs are direct.
An active wonder 17 makes an adjusted cost of exactly 1 free. Under
Fundamentalism, Temple (4), Cathedral (11) and Colosseum (14) are free.

Payment skips anarchy, visits improvements 1..39 in ascending order (the
original building accessor rejects 36+), and subtracts each applicable fee.
If the balance becomes negative, it is reset to 0, that improvement is
removed, and its full Rules cost times shield rows is credited. This can
pay for subsequent improvements in the same pass. The executable check
starts at 0 with Barracks/Granary/Temple charging one each: Barracks is
removed, its 40-shield cost credited, the later two fees paid, leaving 38.
It sets original global `0x6031b8=1` to suppress notifications; monetary
and building mutations execute from original bytes without call stubs.

All **1,000 invented fees** and **490 saved Rules/civ fees** match the
original. Rust also checks AI/anarchy gates, bankruptcy order and two
successive tax/upkeep turns. Current turn integration credits the existing
pre-production finance result, then charges upkeep after production.
Original overall phase order and exact city-turn parity remain #13;
these fee checks do not establish either.

### Wonder completion (#88 in progress, 2026-11-05)

`wonder-completion.json` is re-derived by `tools/wonders_check.py` using
original registration `0x4c7910..0x4c7936` and box reset
`0x4c7f97..0x4c7fae`, across all 28 slots. It checks these isolated writes,
not the UI or entire completion dispatch. Table entries are city indices,
-1 unbuilt and -2 destroyed; only -1 can be built. The successful
single-player path sets city flag 0x100 and resets the entire box, losing
overflow. Subsequent successful unit/improvement production clears 0x100
(`0x4c6575` / `0x4c7777`). Human ordinary improvements also take the box
reset path; ordinary AI improvements branch to `0x4c7ff5`, preserving
overflow. The engine formerly preserved human improvement overflow too.

Completion dispatch `0x4c8f7d..0x4c8fa3` calls `0x4a145e(civ,0)` twice
for wonder slot 18 (Darwin). The engine grants the selected target, clears
progress, and keeps the remaining choice pending for `SetResearch`; with
no target both choices are pending. This interaction state is engine-owned,
checksummed and visible through `ResearchView.free_advances`. Existing
active-wonder consumers provide yield, finance, veteran and happiness
effects when the table changes. Other one-off completion effects remain
open: do not close #88 on this lifecycle slice.

### T1-4 city totals DECODED (2026-11-04): corruption, waste, multipliers, unit support

Static decode from `civ2.bak`. The city-totals function `0x4c45ff`
stores the record fields:

| Record field | Holds | Source |
|---|---|---|
| @80 | trade after routes and corruption | — |
| @82 | food | global `0x653848` |
| @83 | shields | global `0x65384c` |
| @76 | science | `0x6537f8` |
| @78 | taxes | `0x6537d4` |
| @84 | happy count | `0x6537d0` |
| @85 | unhappy count | `0x653828` |
| u16@32 | trade − corruption, before trade routes | — |

The globals `0x653848/4c/50` are the food/shield/trade sums of the tile
loop (`0x4c2b81`).

**Corruption / waste** (`0x4c35a2(city, amount, waste?, mode)`):
- `amount ≤ 0` gives 0.
- **Non-communist:** `amount · d · 3 / (f · 20)`, where:
  - `d` = the distance (`0x566dad`) to the nearest same-civ city with a
    Palace (improvement 1); 0 in the capital, 32 with no capital
    (`0x4c13fe`);
  - for waste `d` is capped at 16;
  - under anarchy/despotism, a human civ uses
    `d = clamp(difficulty + d, 0, 32)`;
  - `f = 4 + (g≥1) + (g≥2) + (g≥3) + (g≥5)` (`0x4c3551`), where g is the
    government, +1 while celebrating.
- **Communism** (g = 3): `rate · amount · 3 / (f · 20)`, with the rate
  byte at RAM `0x5f5ec8`.
- Clamped to [0, amount], then halved by a Courthouse (7) or Palace (1).
- Trade corruption is zeroed under Fundamentalism (4) and Democracy (6).

**Shields** (`0x4c3919`):
- **Multipliers:** with `q` = 2 per Factory (15) and Manufacturing Plant
  (16), and `p` = 2 with any power plant (19/20/21/29) or Hoover Dam
  (wonder 22) but capped at `q`:
  `shields += shields·p/4 + shields·q/4`.
- **Waste:** `w = corruption(shields − support, waste)` (support =
  `0x6537e8`, a unit-support count from a separate function), then:
  - `w = w · (3 − conn) / 3`;
  - `w = w / ((g>>1) + 1)`;
  - capped at the shields;
  - zero for barbarians, Democracy or Fundamentalism.

  `conn` (`0x6537f4`) is 1 in the capital (+1 with Railroad), 1 when both
  cities have improvement 32, else the road/rail route test `0x471a13` to
  the capital.

**Replay status (2026-11-04, after an oracle hook):** the formula is
`amount · d · 3 / (f · 20)`.

The `· 3` (`lea eax,[eax+eax*2]` at `0x4c36c3`) had been dropped by a
disassembly filter. A hook on `0x4c35a2` in the live game (lab save
loaded, one turn ended) logged Utica's call as amount 4, d 10, which
needs corruption 1; the decode without `· 3` gives 0. With it:

- **Trade after corruption:** exact in **145/145** ROMFALL cities without
  trade routes and **10/10** standard-game cities.
- **Shields after multipliers and waste:** exact in **141/146** ROMFALL
  and **10/10** standard, with unit support taken as 0 and the road
  connection as capital-only. Each of the 5 misses is the model one
  shield low (waste too high), the direction support and connection
  would correct. "Homed units minus N free" fits no better than 143/146,
  so support needs its own decode.

**Unit support** (`0x4c19e0` loop + `0x4c18af`):
- A unit counts when it is alive (id ≠ 0), owned by the city's civ,
  homed in the city, and of Rules role < 6.
- The n-th counted unit costs a shield when n exceeds the free
  allowance:
  - anarchy/despotism: the city size;
  - monarchy/communism/fundamentalism: @COSMIC rows 13/14/15 (0-based;
    RAM `0x5f5ec5..c7`; rows start at `0x5f5eb8`);
  - republic/democracy: none.
- Fundamentalism exempts unit types with type-table flag `0x08` (not yet
  mapped to Rules).

At the initial support decode, shields replayed **143/146** (ROMFALL)
and 10/10. The subsequent connection decode below improves this to
145/146; Aventicum remains one shield low. The initial hypothesis that
connections alone explained all three differences was not borne out.

Engine: `WorldState::city_output`, `corruption`, `capital_distance`,
`unit_support`. Tests: `crates/engine/tests/yields_install.rs`. Recurring
route arrows were subsequently decoded below; the connection test follows.

### Recurring trade-route arrows DECODED (#85 slice, 2026-11-05)

Sources: original `civ2.bak` city totals `0x4c45ff`, route loop
`0x4c46bf..0x4c4890`. Derived artifacts: `trade-routes.json` and
`trade-route-vectors.txt`; reproduction:

```
CIVTOT_HOME=... PYTHONPATH=... python3 tools/trade_routes_check.py --emulate --vectors /tmp/route-vectors.txt
```

Unicorn is an optional development dependency outside the workspace. This
probe executes the original loop with its real building/connection callees;
it starts after city refresh and stops before final corruption. It therefore
checks route arithmetic, not a complete original-game turn.

- City record i16@32 (RAM `0x5fd170`, stride 92) is tile trade minus the
  first corruption pass (`0x4c46ab`). Source trade is refreshed before the
  loop; partner trade is read from that partner's cached record.
- i8@60 (`0x5fd18c`) is the route count; three i8 commodity slots start at
  @67 (`0x5fd193`), parallel to three i16 partner city indices at @70
  (`0x5fd196`). These indices are city array slots, not city IDs.
- Base arrows: `(source_cache + partner_cache + 4) >> 3`, using signed
  values. Connection factor starts at `0x471a13` (0/1/2). If both cities
  have Airport (32), it is at least 1; source Superhighways (25) adds 1.
  Add `(base * factor) >> 1`. A negative commodity makes the result zero;
  a same-owner partner halves the result afterward with arithmetic shift.
- Arrows are added to **raw tile trade**, not to the already corrupted
  source cache. Final corruption then runs on the combined trade at
  `0x4c489e` (mode 1). With the network-bookkeeping global disabled, the
  mode changes no returned corruption arithmetic; Fundamentalism and
  Democracy explicitly zero it afterward. Network statistics are unmodelled.

All **288 synthetic cases** match original execution, covering three
connection factors, airports, Superhighways, domestic/foreign ownership,
negative/zero/positive commodity IDs and rounding boundaries. The current
standard save has no routes. FallRome city slot 16 has one route to slot
14, commodity -1, cached trade 7/12, and zero arrows; original execution
agrees. This saved route supplies no positive-income campaign validation.

`engine::trade_routes` retains these fields in the checksum and uses cached
partners without recursive city evaluation. `run_turn` refreshes the source
cache before finance, following the existing engine city order. Tests cover
mutual routes, multiple slots, cache changes, invalid/dead partners,
route-inclusive science/tax and install-gated cache refresh. The run50
fingerprints change intentionally with the new state; its gameplay rows are
otherwise unchanged. Source supply/demand refresh (`0x430232`), creation,
delivery rewards, non-surface routes and original turn-order parity remain
open. Do not close #85 based on this isolated income slice.

### Capital road/rail connections DECODED (#84, 2026-11-04)

Source: unpatched `civ2.bak`, connection `0x471a13`, path search
`0x48c9c5`, capital wrapper `0x4c1736..0x4c18aa`. Artifact:
`connections.json`; derived Rust cases: `connection-vectors.txt`.
Re-derive with:

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn python3 tools/connections_check.py \
  --emulate --vectors /tmp/connection-vectors.txt > /tmp/connections.json
CIVTOT_HOME=... cargo test -p engine --test connections_install --test yields_install
```

The probe loads original PE sections, runs the connection and all its
callees from their original bytes, bounds execution at 20 million
instructions and requires return to a sentinel. There are no call stubs
and no new Wine run. Inputs come from surface tiles, saved diplomatic
words and Rules, with type 2's land domain asserted. The passability
input is normalized to its effective blocked-terrain set; other unit
properties do not affect this uniform-cost search. **123 saved pairs**
(5 standard, 118 FallRome) and **12 invented cases** match the decoded
Python implementation. Rust independently matches every saved-pair vector.

Newly retained fields: city i16@4 is its map ID (loads `0x4c1444`,
`0x4c1824`, comparison `0x4c1857`); civ u32@40+4*other holds the word
whose bit `0x08` permits crossing another civ's occupied tile (RAM
`0x5f68c8`, tests `0x471bc8`, `0x471c20`). No broader diplomatic meaning
is assigned here. Tile +3 was already proven as landmass (`0x5701d1`);
it now survives engine boot. All three join the world fingerprint.

The live engine uses its current city/unit objects for occupied-tile
checks: movement and deaths must not leave a route blocked by the boot
save's unit-presence bit. A regression moves and removes a blocker without
altering the boot tile flags. This is the engine state representation of
occupancy, rather than an additional original-game rule.

The helper returns 0/1/2 for none/road/rail. It first requires matching
landmass IDs (ocean is -1) and wrapped `(abs(dx)+abs(dy))/2 < 23`.
It searches backwards from the destination over passable land with
uniform edge costs, in the 48-by-48 diamond work area, then chooses a
decreasing-cost neighbour, breaking ties by smaller geometric distance
and then direction order. Road flags do **not** guide this search. The
chosen path is then checked for roads, rails and occupied-tile permission.
An own intermediate city immediately succeeds. Endpoints and that city
need no road and do not downgrade rail. The original selector checks its
raw neighbour bounds before wrapping; a synthetic map-seam route is
therefore rejected even when a road exists. The probe preserves this.

Rules loader `0x4141f4` writes terrain field 15 to terrain-record +22;
the terrain token decoder `0x4904f4` maps `no` to -1, `yes` to -2.
`0x57073a` excludes -2 unless unit advanced field 6 bit `0x10` is set
(loader `0x413a52..5d`). FallRome marks terrain 9 impassable. The engine
tests exclusion and the unit override without game assets.

Capital wrapper: Palace factor is 1, plus 1 with Railroad advance 67.
Otherwise it uses the nearest alive own Palace. Two Airports (32) give
factor 1 across differing landmasses; on the same landmass the road test
replaces that factor. `city_output` now uses this factor in
`waste * (3 - connection) / 3`. FallRome shield totals improve to **145/146**;
Aventicum remains engine 5 versus stored 6, cause unproven. Standard save
totals retain the existing exact comparisons. Scope: surface map and land
type 2; sea/air type replacements and non-surface paths remain open.

`make evidence-regen` intentionally refreshes `fallrome-50turn.txt`:
all fingerprints include the new state, and 13 per-turn build-count rows
shift because reduced waste changes production timing. Total completed
items, growth counts and event counts over that run are unchanged.

### Science and tax split DECODED (#87, 2026-11-04)

Source: unpatched `civ2.bak`, `0x4c3f0c` (normal totals mode 0).
Artifact: `t1-4-finance.json`. Re-derive with
`CIVTOT_HOME=... python3 tools/finance_check.py`; Rust record comparisons:
`CIVTOT_HOME=... cargo test -p engine --test finance_install`.
This is a static decode checked against game-written records, not a new
live oracle run.

**Fields:** civ byte @27 = science rate, @28 = tax rate, in tenths
(RAM `0x5f68bb/bc`, loads at `0x4c3f8a`/`0x4c3fa3`). Government
remains @29. City u32@24 is sixteen two-bit specialist slots
(`0x4c1002`, load `0x4c102f` from `0x5fd168`): 0 unused, 1 entertainer,
2 tax collector, 3 scientist. Records remain in the already proven framing.

**Calculation** (`T` = trade after routes/corruption):

- Luxuries = `clamp((T * (10 - science_rate - tax_rate) + 4) / 10, 0, T)`.
- Under Fundamentalism, cap the science rate at @COSMIC row 21, after
  computing luxuries. Science =
  `clamp((T * science_rate + 4) / 10, 0, T - luxuries)`.
- Taxes receive the remainder `T - science - luxuries`. Thus the half-way
  case rounds down: one arrow at 50% science gives zero science, one tax.
  The +4 rounding instructions are `0x4c401b` and `0x4c4047`.
- A Fundamentalist AI transfers allocated luxuries to science first.
  Specialists then add 2 luxuries / 3 taxes / 3 science per matching slot.
  Fundamentalism subtracts `science * @COSMIC[17] / 100`, including
  specialists (`0x4c41dc` onward).
- Fundamentalist tithes (`0x4c4208`–`0x4c4371`): Temple adds the number of
  known advances 56/9, doubled by active wonder 5; Cathedral adds 3,
  +1 with advance 24; advance 55 plus a Colosseum or active wonder 10
  adds `2 + knows(15) + knows(82)`; active wonder 15 adds 2.
- Marketplace/Bank/Stock Exchange (5/10/22) add
  `base * count / 2` to taxes and luxuries, including specialists/tithes
  (`0x4c43c9`–`0x4c43e7`). Science improvements 6/12 and either 26 or
  active wonder 26 give the same additive half-base bonus. In the city
  with active wonder 16, that improvement bonus uses the full base
  (`0x4c44b0`), not half. Active wonder 11 doubles the resulting science.
- Capitalization (build item -39) adds positive shields after unit support
  to taxes **after** multipliers in normal mode 0 (`0x4c4513`). Alternate
  UI preview modes and network-game bookkeeping are outside this API.
- Active-wonder lookup (`0x444a28`/`0x4449b0`): negative city slots have
  no effect; any player's discovery of the @ENDWONDER advance obsoletes
  the wonder. Read physical row IDs, retaining repeated scenario labels.

**Record replay:** exact science and taxes in **146/146** ROMFALL cities
(including 14 with specialists and 2 building Capitalization), and **10/10**
current `Original/Ph_Auto2.SAV` cities. Inputs are their stored trade and
shields; this isolates finance from still-open trade-route/waste inputs.
Fundamentalism and the science-wonder ordering additionally have synthetic
arithmetic tests backed by the static decode; no claim of live runtime
coverage for those branches is made.

Engine: `engine::finance`. `run_turn` refreshes the city science/tax fields
before production completion using the current engine's computed trade.
Trade routes remain #85; per-civ research accumulation, maintenance and
actual treasury income are not supplied by this slice. New rates and
specialists join the checksum; run50 fingerprints are intentionally
regenerated, with all non-checksum columns unchanged.

### Per-civ research DECODED (v0.8.0, #82 dependency)

Source: unpatched `civ2.bak`: cost `0x4a1ae0`, accumulation `0x4a1ebf`,
completion `0x4a145e`, advance grant `0x49dedf`, city science `0x4ca000`.
Derived artifacts: `research.json`, `research-cost-vectors.txt`.

**Saved inputs:** per-civ properties (same @2944 + civ*3348 framing):

| Offset | Meaning | Original access |
|---|---|---|
| i16@16 | science progress | `0x4a1eee`/`0x4a1f09` |
| i16@18 | target (-1 none; 89 repeatable future tech) | `0x4a1f21` |
| u8@24 | acquired-advance cost counter | `0x4a1afb` |
| u8@25 | future-tech counter | `0x49e0db` |
| u8@26 | additional cost counter, wider meaning open | `0x4a1b14` |

The acquired counter is persisted, not inferred from known-tech bit count:
starting grants at turn zero do not increment it (`0x49e0e2`–`0x49e137`).
A normal grant sets the known bit; a future grant increments @25 instead;
post-turn-zero grants increment @24, and a matching target is cleared.

Game word **@660** is the raw flags word read at `0x607640` (research uses
0x80/0x04/0x08); word **@702** is the enabled-advance count
(`0x60766a`), and byte **@964** is the comparison slot (`0x607770`). The
advance loader zeros the count at `0x412f84` and increments it for enabled
advance rows at `0x4132a3`; it is **89 in Original and 52 in ROMFALL**.
It is not a map dimension. The cost routine uses the saved comparison slot
as a civ index; its broader ranking policy is not implemented by the engine.

For classic 0x32 scenarios, writer `0x4600ba`–`0x46053a` places a 100-byte
scenario block after the cities and: 21*3 terrain bytes, 4 cursor bytes,
60 bytes per human leader (one when game byte @678 is zero), 1200 bytes,
106 bytes and 8*10 bytes. Thus the block starts at
`city_end + 1453 + 60*leaders`. Research uses its i16@82 paradigm and
i16@84 skip-adjustment setting (RAM `0x5f5ea2/a4`). ROMFALL reads
**(100, -5)**. The parser requires the whole block to fit, and does not
apply the classic tail formula to 0x33 scenarios. Unknown scenario cost
settings yield `None` and disable research accounting/orders for that
scenario, rather than using fabricated defaults. Non-scenario games use
@COSMIC row 11 and need no scenario tail.

**Cost arithmetic**, in original order (each division truncates toward zero):

1. `raw = counter24 + counter26`, `n = max(raw, 1)`,
   `d = min(difficulty, 4)`. Base factor is `2*d + 6` for a human slot,
   otherwise `14 - d` (`0x4a1b34`–`0x4a1b7c`).
2. Unless scenario flag 0x80 and its skip-adjustment setting are both set:
   compare the saved comparison civ's raw counters with n. If ahead,
   subtract 1 except on difficulty 0; subtract a second 1 on difficulty 5
   when ahead by more than 4 and turn > 150. Otherwise add `(n-ref)/3`.
   For n >= 20 additionally add `clamp(n-turn/8, 0, 6)`.
3. Multiply factor by the applicable paradigm / 10. Add a bonus of
   `(factor*3)>>2`, scaled by `raw/20` while raw < 20, with division only
   after multiplying (`0x4a1d54`–`0x4a1de2`).
4. When enabled-advance count > 67, scale by 67/count. Flags 0x04 and
   0x08 scale by 5/4 and 4/5 respectively, sequentially. For humans clamp
   factor upward to `11-n`.
5. Cost = factor*n; results outside 1..32000 become 32000, including
   zero/negative results (`0x4a1e8a`–`0x4a1eb2`).

**Accumulation/completion:** positive city science is added to the signed
16-bit progress word (`0x4a1ef6`, low-word store). With a selected target,
progress >= cost triggers completion. Completion writes progress = 0 and
target = -1 (`0x4a154b`/`0x4a1565`): excess science is discarded. An
external grant (e.g. hut scroll) clears a matching target but preserves
progress. The city helper skips anarchy and doubles Chieftain human
science toward an already globally discovered non-future advance. The
late spaceship-related AI doubling path is not implemented.

**Checks:**

- `tools/research_check.py --emulate` loads the user's original PE into
  Unicorn and executes the actual cost routine and its original clamp
  callee, with a verified return and bounded instruction count. The decoded
  formula matches **1,014 cases** (14 civ/save input sets and 1,000 seeded
  generated cases across difficulty, counters, flags, paradigm and scaling
  boundaries). This is original-code emulation, not a new Wine run.
- 128 derived cases are committed as text for asset-free Rust regression
  checks. The source executable's SHA-256 is in `research.json`; no
  executable bytes are committed.
- Existing `std1` saves contribute **20** exact one-turn progress additions
  with stable positive city science and **four** discovery/reset boundaries.
  One is 33+3 science against cost 35, then progress 0: overflow is lost.
  The artifact records the selected transitions and selection method;
  this is not a claim to replay the whole original turn loop.
- Engine integration: a 50-turn AI v1 increment on the current standard
  save discovers **14** additional advances and replays the complete world
  and research state identically. Install tests:
  `crates/engine/tests/research_accounting_install.rs`.

Re-derive (Unicorn is optional development tooling, not a runtime dependency):

```sh
uv --cache-dir /tmp/research-uv-cache pip install --target /tmp/research-deps unicorn
CIVTOT_HOME=... PYTHONPATH=/tmp/research-deps python3 tools/research_check.py \
  --emulate --vectors /tmp/research-cost-vectors.txt --series /path/to/std1
cmp docs/evidence/research-cost-vectors.txt /tmp/research-cost-vectors.txt
```

Without `--series`, only cost checks run. Without `--emulate`, the script
reports decoded costs without claiming execution parity. The archived
std1 series is local oracle data, not distributed assets.

Engine policy boundaries: `SetResearch` accepts prerequisite-valid choices
and preserves progress on a change; the original rotating UI subset and
ToTPP tech groups are not recreated. A discovered target stays unset until
the next player order phase; intervening science is banked. The saved
comparison slot is preserved rather than recomputing original rankings.
Diplomacy rewards, first-discovery rewards and technology-event hooks are
not implemented by the shared grant helper. AI v1's initial research policy
uses Rules AI value with lowest-ID tie breaks; it is our policy, not a
claim of original-AI parity. Run50 goldens were regenerated intentionally:
only checksum columns changed in this run.

### T1-4 tile yields DECODED (2026-11-04): the original's tile-yield function

Source: the unpatched `civ2.bak`, function `0x4c22a7(city, radius index k,
kind)` (0 food, 1 shields, 2 trade), with its helpers. Engine:
`engine::yields` (`WorldState::tile_yield`, `worked_yield`).

**City side** (the RAM city table `0x5fd150` uses the save's 92-B record
layout):
- worked tiles: u32@50, bit k = radius tile k;
- the radius offset tables: `0x5d1620` (dx) / `0x5d1650` (dy), 21 entries,
  index 20 = the centre;
- improvements: bitfield @54 (`0x43002e`);
- flags: u32@6, bit `0x2` = celebrating.

**Civ side** (civ block, RAM `0x5f68a0` + civ·3348):
- government: byte @29;
- advances: bitfield @96 (`0x49c820`).

**The yield:**
- **Base:** `Rules @TERRAIN[special·11 + terrain]` fields 3/4/5 (RAM
  table `0x5d11f2`, 24-B rows).
- **Special** (`0x5707a6`): none if tile byte +0 has `0x40` (so `0x40`
  is a *suppression* flag) or the tile is grassland. Otherwise, with
  `a=(x+y)>>1`, `b=x−a`: special iff
  `(a&3) + 4(b&3) == (seed + 11(a>>2) + 13(b>>2)) & 15`. Its kind is 2 if
  `a&k == b&k` with `k = 1 << ((seed>>4)&3)`, else 1.
- **Ocean:** +1 food with Harbour (`0x1e`), +1 shield with Offshore
  Platform (`0x1f`), +1 trade with Colossus (wonder 2) in this city.
- **Food:** irrigated (`+1 & 0x04`) or the centre adds the terrain's
  irrigation bonus (field 7). Supermarket (`0x18`) then adds +50% on
  farmland (`0x08` too) or the centre.
- **Shields:**
  - mine only (`& 0x0C == 0x08`), or the centre on a terrain with no
    irrigation bonus, adds the mine bonus (field 11);
  - grassland keeps its shield only where `(-x - a) & 2 == 0`
    (`0x4095b0`);
  - the centre makes at least 1.
- **Trade:** +1 river; +1 road or centre where the terrain is desert,
  plains or grassland or the tile already yields trade; +1 Colossus where
  there is trade.
- **Then, in order:**
  - +1 shield with King Richard's (wonder 8);
  - shields +50% on railroad, or on the centre once the civ knows
    Railroad (advance `0x43`);
  - −1 on any yield above 2 under anarchy/despotism (government ≤ 1)
    unless celebrating;
  - +1 trade where government ≥ 5, or ≥ 2 while celebrating;
  - trade +50% on road or centre with Superhighways (`0x19`);
  - pollution (`+1 & 0x82 == 0x80`) halves, rounding up; floor 0.

**Replay against game-written city records** (sum over worked tiles vs
record food u8@82, shields u8@83, trade u16@80):
- **Standard-game lab** (std1 turn 26, `Original/Ph_Auto2.SAV`): 9/10
  cities exact in all three; Utica's trade is 1 lower in the record.
- **ROMFALL:** food exact in 144/146. Trade is below the tile sum in 100
  cities and never above. Shields differ both ways (21 above, 11 below).
- Reading: the record holds shields and trade **after** waste,
  corruption and the production multipliers. Those are the next decode.

Tests: `crates/engine/tests/yields_install.rs`. The irrigation and mine
bonuses are read from Rules as written; zeroing them for "no" terrains
changes no match.

### Map tile framing FIXED (2026-11-04): the game's own tile record

The 6-B tile records start right after the 7 B/tile block, at
`map_data + 7·area`. The 2 bytes Mercator lists after the maps are a
trailer, a copy of the resource seed (ROMFALL: `0c42` at both places).
This is Mercator's order. The 2026-11-02 "phase correction" put the 2
bytes first, so every record was read 2 bytes late.

Proof:
- In a live standard game (RG-2 pl1), the tile array at `[0x5deaa8]`
  equals the save's block at `map_data + 7·area` byte for byte (12,000 B).
- The accessors are read from the binary:
  - tile pointer `0x56fff0`: `base + 6·(w·y) + 6·(x>>1)`, so the index
    is `y·w + x/2`;
  - bounds `0x4030f0`: `x < raw[0]`, `y < raw[1]`.

| byte | content | evidence |
|---|---|---|
| +0 | terrain `& 0x0F`, `0x40` special flag, `0x80` river | `0x5700ff` (`== 10` is ocean, `0x57012c`); ROMFALL 146/146 cities on land |
| +1 | `0x01` unit, `0x02` city, `0x10` road, `0x20` railroad; `0x04`/`0x08`/`0x40` other improvements | ROMFALL: `0x01` on exactly the 423 unit tiles, `0x02` on exactly the 146 city tiles |
| +2 | top 3 bits: a site value (`0x57024c`, start placement) | meaning open |
| +3 | landmass id; the 10240-B blob is the landmass table (256 × 40 B, RAM `0x61bca8`) | `0x5701d1`; RAM table = save blob |
| +4 | per-civ visibility bits | `0x489b66` tests it against the human mask |
| +5 | fertility `& 0x0F`, owner `>> 4` (`0xF` = none) | `0x570390`/`0x570169`; ROMFALL 146/146 city tiles carry the city's owner |

How the road and rail bits were identified:
- Road `0x10` and railroad `0x20` were identified by network shape.
- **Road (ROMFALL):** 84 % of `0x10` tiles have one or two `0x10`
  neighbours. The `0x04`/`0x08` bits cluster instead.
- **Railroad (Warring States):** all 297 `0x20` tiles also carry `0x10`,
  and 99 % have one or two rail neighbours.
- Neither bit is oracle-checked.

What the old framing got right and wrong:
- **Terrain, river and special were right.** The 2-byte shift and the old
  `6t+4` + index `−1` cancel out: the old terrain equals byte +0 of the
  true tile on 8,880/8,880 ROMFALL tiles.
- **Road and rail were wrong.** The old "road/rail" byte at `6t+14` is the
  *next tile's visibility byte* (8,880/8,880). So the "3,497-tile road
  network" was the fog of civs 6 and 7. Real ROMFALL roads: 461; railroads: 0.

Consequences:
- The engine's road and rail flags change (FallRome 50-turn fingerprints
  regenerated; only the checksums moved).
- All six oracle battles still pass. Battle 2's defender stands on
  Grassland + river; the old decode said Forest, and both give ×1.5.

### RG-2 goody huts, oracle-checked (2026-11-04)

Huts are not stored; the seed and the tile decide them (`0x57093f`). A
tile holds a hut iff all of these hold:
- it is land;
- it is unowned (tile byte +5 `>> 4 == 0xF`);
- huts are enabled on its map (RAM `0x5f5ed6` bit; save location not
  found);
- with `a = (x+y)>>1` and `b = x − a`:
  `(a&3) + 4(b&3) == (seed + 13(b>>2) + 11(a>>2) + 8) & 31`.

The hash selects one of the 32 (a, b) positions in each 4 × 4 block, so
about 1 tile in 32. The pinned default world (seed `0x1357`) has 31 huts
on 975 land tiles.

**Oracle check** (pinned default world, human civ 4, settlers at (62,26)):
- The formula predicts exactly one hut visible to civ 4, at (63,25).
- Moving a settler there (RAM select + numpad-9) opened the "Village"
  dialog: *"You have discovered valuable metal deposits worth 100 gold."*
  The treasury went from 0 to 100.
- Moving the other settler to (61,25), predicted hut-free, gave no dialog
  and no gold.

**How a hut is taken:**
- *Any* unit entering a tile sets the tile's owner nibble to its civ:
  (61,25) `f0` → `40`.
- The owner stays after the unit leaves: (62,26) keeps owner 4.
- So a hut is gone once any civ's unit has stepped on it. There is no
  separate "taken" flag.

Engine (`MapState::hut_at`, `MoveOutcome::hut`) follows this.

**Not decoded:** the hut *outcome* (gold / tech / units / barbarians /
nothing) and its random draws. The engine reports the entry and applies
no effect (RG-11).

### Record counts: i16@698 = units, i16@700 = cities (2026-11-04)

The game block carries the table sizes:
- i16@698 = unit count: RAM `0x607666`, the loop bound of start placement
  and of the unit loops.
- i16@700 = city count.

Evidence:
- Both equal the counts the old name-chain scan derived, on every save it
  could frame: ROMFALL 710/146, the std-lab series 10/4, Ph_Auto2 38/10.
- They also frame a fresh turn-1 game with 7 units and 4 cities (Moscow,
  Kyoto, Beijing, Paris), which the chain scan could not anchor on. The
  cities match the starts of the same pins (fixture world pl2) as
  follows:
  - Moscow (43,41), Kyoto (2,12) and Beijing (30,24) stand exactly on the
    starts of civs 1, 2 and 5.
  - Paris (45,9) is one diamond step from civ 3's start (46,8); civ 3's
    settler had moved there before founding it (run record o1 lists it at
    (45,9)).
  - That save's own draw log is not committed; the match rests on the
    pl2 fixture, which uses the same pins (reviewer finding #71).

Both decoders now frame units (`40·N` after the 10240 blob) and cities
(`92·M` after the units) from these counts. Every city slot must hold a
NUL-padded name at +34 (the #49/#58 anti-fabrication gate).

Multi-map files (Midgard) still do not frame: their blob end is not
located yet.

### RG-2 start placement DECODED, oracle-exact (2026-11-04)

`engine::placement::place_starts` reproduces the original's start
placement draw for draw. Every CRT draw, call site and state, matches the
`randhook.py` log of three pinned worlds:

| world | CRT pin | draws | civs placed (tries) |
|---|---|---|---|
| pl2 | 0x5678 | 218 | 1 (6), 2 (10), 3 (1), 4 (53), 5 (8) |
| pl3 | 0x9999 | 214 | 3 (4), 4 (28), 5 (2), 6 (30), 7 (12) |
| pl4 | 0x2468 | 136 | 1 (13), 3 (8), 4 (4), 5 (7), 7 (5) |

Fixture: `docs/evidence/rg2-placement-fixture.txt`, derived by
`tools/oracle/placement_fixture.py`. It holds the setup, the CRT states,
the starts, and the 190 sampled tiles (not a map).

How the fixture was captured:
- `seeded_start.sh` with `DUMP_INTRO=1` dumps the tiles from RAM at
  "In the Beginning": after placement, before any AI move.
- The turn-1 save is not a pre-placement map. AI civs found cities on
  turn 1, which rewrites fertility and site bits around them.

The algorithm:
- **Civ loop** (`0x48c5ce`): one `rand() % 50`, then `0x488499(civ)` for
  civ 0..7. Civ 0 (barbarians) draws nothing.
- **Per civ 1..7** (`0x488499`):
  - 7 attitude draws, one per civ 1..7 (`rand()%80 + 10`, or toward a
    human civ `clamp(rand()%80 + 5·difficulty + 10, 10, 75)`);
  - for an AI civ, a tribe pick (`rand() % candidates`) and a `rand() % 3`;
  - for a civ in the civs-in-play mask (save @686), location tries.
- **A try:** `x = rand() % w2`, `y = rand() % h`; if x is odd, x −= 1; if
  y is odd, x += 1. With n = the try number, a try is rejected when:
  - the tile is ocean;
  - `site < 12 − n/32`, where site (`0x5703be`) = `+2 >> 5` if set, else
    `max(fertility, 8)` if fertility > 0, else 0;
  - `near < 16 − n/128`, where near = distance to the nearest placed
    settler (30 when none); also `near < 2` while n ≤ 2000, or `near < 1`;
  - `landmass[+2] < 32 − turn/16 − n/64`;
  - a hut (`0x57093f`) while n ≤ 2000;
  - the tile is occupied (`0x570599`).
- **After 2000 tries** an AI civ accepts any tile.
- **Distance** (`0x566dad`): with dx wrapped unless flat, `m = (dx+dy)>>1`;
  then `dy − ((m−dx+1)>>1)` if dy ≥ dx, else `dx − ((m−dy+1)>>1)`.
- **Huts** derive from the resource seed: `a=(x+y)>>1`, `b=x−a`; hut iff
  `(a&3) + 4(b&3) == (seed + 13(b>>2) + 11(a>>2) + 8) & 31`.

What the data checks:
- The oracle tries exercised the ocean, site, distance and landmass tests.
- Mutating the distance threshold, the `rand()%3` draw or the parity fix
  breaks the replay.
- The site and landmass constants (`0xc`, `0x20`) are binary constants
  the samples do not bracket: no try had site 9–11.
- The hut, occupancy and 2000-try branches, and the flat-world margin
  (`rand()%(w2−16)+8` when bit `0x8000` of `0x607638` is set and w2 > 80),
  are decoded but not exercised.
- Preset (Earth-map) starts are not modelled; their tables are −1 on
  random maps.

Two decodes from the same work:
- `0x607638 & 0x8000` = flat world. The distance wrap (`0x566b9e`) and the
  margin branch both test it.
- The tile record framing: see §Map tile framing FIXED.

Still open for RG-2 (2026-11-04 later: huts done, §RG-2 goody huts;
barbarians moved to RG-9):
- what the AI civs do on turn 1 (cities on their starts).

### RG-2 reproducible new-game starts: the four entropy inputs (2026-11-04)

**A new standard game is fully determined by four values, all writable
while the City Style dialog is open.** Two runs with the same four values
give the same turn-1 autosave; changing one changes the world. Run record:
`docs/evidence/rg2-seeded-start.txt`. Tool: `tools/oracle/seeded_start.sh`.

| Input | Where | Why it varied |
|---|---|---|
| NR LCG state | `0x5dd53c` | startup `exchange_state(GetTickCount())` |
| CRT `rand()` state | `0x5e2590` | startup `srand(GetTickCount())` |
| 5 world parameters | 5 × i32 at `0x5ceaa8` (each −1/0/1) | drawn `rand()%3 − 1` at `0x41223f`, **before** City Style, from the tick-seeded CRT |
| Map resource seed | u16 `0x6705a8` (save 29914) | `0x56fa66`: `GetTickCount() & 0x7fff`, during generation |

The map seed cannot be written in advance (generation overwrites it), so
the tool patches the 5-byte `call 0x41aee0` at `0x56fa6c` into
`mov eax, MAPSEED`.

Results:
- **Original Game:** o1 and o2 match in the front section, the map, and
  the cities/units block. They differ only in 15 tail bytes, which are
  serialized heap pointers (`0x02fbxxxx` vs `0x02f9xxxx`).
- **Fantasy Game:** pa1, pa2 and ss1 (ss1 made by the committed tool) are
  byte-identical, 192,851 B.
- **Controls:**
  - a different CRT value gives a different world and different starts;
  - a different MAPSEED changes the map and derived bytes, not the starts.
- The pre-pin parameters differed on every run (e.g. `(0,0,1,0,−1)` vs
  `(0,0,−1,1,0)`). That is why pinning NR and CRT alone failed on
  2026-11-03.

How it was found:
- `tools/oracle/randhook.py` hooks CRT `rand` and logs each draw's call
  site.
- Two runs with only NR and CRT pinned agree for 40 draws. At draw 40 one
  run makes an extra `rand()` at `0x405164`, guarded by
  `[0x5ceaac] + 1 > 0`, which is world parameter 2.
- Who draws during setup (`0x415b74`, the new-game function, and its
  callees), all from the CRT `rand`:
  - rivals: `rand()%5 + 2` when random;
  - barbarian level (`0x607659`): `rand()%2`/`%3` + difficulty-dependent
    offset;
  - the **civs-in-play mask** (`0x60765a` = human mask | 1, then one
    `rand()%8` bit per rival, retrying taken bits);
  - the map generator (`0x403xxx`–`0x409xxx`, called from `0x4174dd`);
  - start placement (`0x488499` per civ: `rand()%width`, `rand()%height`
    until valid).
- While a wizard dialog sits idle, neither generator moves (sampled at
  0.5 s for 10 s). Draw counts are not timing-driven.

**Correction to §RNG / seed state below:** save 684–689 =
`difficulty | barbarian level | civs-in-play mask | human mask | mask copy
(set at 0x48c65e) | rivals`. The "seed word" was the mask: `0x3f` = civs
0–5 for 4 rivals + human civ 4 (`0x10`), and S2f's `0xdb` decodes the
same way. It is not a seed.

Still open for RG-2 (engine side):
- ~~reproduce the placement algorithm~~ done: §RG-2 start placement;
- the map generator itself is RG-3 (our own generator, flagged);
- why the Original tail holds heap pointers (not game state; the T1-8
  writer should treat them as opaque).

### RG-2 new-game seeding: what reseeds, and a negative experiment (2026-11-03) — EXPLAINED 2026-11-04 (above)

Static (live `.text`, ToTPP build):
- **`0x41562b` seeds both generators from the clock**:
  `srand(GetTickCount())` (CRT, `0x4156dc`) and `exchange_state(GetTickCount())`
  (NR LCG, `0x4156ea`). `0x554a00` is an **XOR-swap**: it installs the
  argument as the new NR state and returns the old one. The only caller is
  `0x41a6f9`, in an init-style function (`0x41a68c`).
- **Local, deterministic reseeds** save the NR state, install a seed and
  restore it afterwards:
  - `0x445eb2`: seed = `(city x << 8) + city y + u16@0x6705a8`, per city.
  - `0x5602d3`: seed = `object[+0x100]`, restored at `0x560cc1`.
  These are not world-generation sources.
- `srand(time())` at `0x598423` (CRT, startup).

**Negative experiment** (scratch script driving the `lab_std.sh` wizard
flow; placement read from the live unit table):
1. At the City Style dialog, write NR := `0x1234` and CRT := `0x5678`.
2. Optionally patch `0x41562b` to `ret` through `/proc/<pid>/mem` (new
   `memspool.py` write fallback for read-only pages).
3. Finish the wizard.

Result: **two runs still produced different start positions**, with and
without the patch. Even the settings seed word differed (`0x103f` vs
`0x103b`). So pinning both known generators at City Style does not
determine the new game: entropy enters elsewhere (earlier wizard steps,
another tick read, or timing). RG-2 "same-seed oracle starts" needs that
source found first.

### RNG / seed state (T1-6 notes, 2026-11-02 night) — "seed word" CORRECTED 2026-11-04 (§RG-2 reproducible starts: it is the civs-in-play mask)

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
| `research.json` / `research-cost-vectors.txt` | `tools/research_check.py --emulate` | Per-civ research: original-x86 costs, existing saved increments/discoveries, asset-free cost vectors |
| `t1-4-finance.json` | `tools/finance_check.py` | Science/tax record replay: 146/146 FallRome, 10/10 standard cities; static original-binary operand checks |
| `crosscheck-mp.json` | `tools/crosscheck_mp.py` (runs both decoders) | Rust `tot-kit` vs Python probe: layout/dims/top-6 histogram must match |

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

### T1-6 combat round formula DECODED and oracle-exact (2026-11-03, late night)

Artifact: [`t1-6-combat-oracle.json`](t1-6-combat-oracle.json) (six live
battles, all derived numbers). Replayed by `engine::combat` tests
`oracle_battle_1..6`.

- **Combat randomness is the MSVC CRT `rand()`, not `rand_int`.** VA
  `0x5b11d0` (`srand` `0x5b11b0`), state at VA **`0x5e2590`**:
  `x = x·214013 + 2531011`, return `(x >> 16) & 0x7fff` (the multiplier is
  built by a lea/shl chain; restated in a unit test). It has **269** static
  call sites, which is why the 58-site `rand_int` survey below found nothing
  combat-shaped. How it was found: `0x5dd53c` stayed at the seeded `0x1`
  through a whole battle, so `.data`+heap were snapshotted around an attack
  and every u32 tested for `after = step^n(before)`. Only `0x5e2590`
  matched (23 MSVC steps), and it is quiet when the game is idle.
- **Round loop** (inside the combat function `0x541425(attacker, dir,
  real)`, VA `0x543b3d..0x543c43`):
  `a = A>1 ? rand()%A : 0; d = D>1 ? rand()%D : 0; attacker wins iff a > d`
  (strict). The winner deals its firepower as damage (unit +0x0c). The loop
  runs while `hp·10 − damage > 0` for both sides. An optional defender
  re-roll (`[ebp-0xc8]`, set only in barbarian-attacker/difficulty
  branches) rolls a second pair and overturns an attacker win if `a2 < d2`.
  **Exactly one more `rand()` follows the last round**: the promotion roll
  (below). Draws per battle = 2·rounds + 1 (+2 per re-rolled round).
- **`real = 0` is the AI's odds probe.** It returns `min(A·8/(D+1), 0x400)`
  with no rolls and no partial-move scaling.
- **Attack strength** (`0x53f604`), fully decoded: `attack·8`, then `+= >>1`
  for veteran (attr `0x2000`) and again for attr bit `0x10` (meaning open).
  In the combat function: `A·min(moves_left, M)/M` with `M` = byte
  `@0x5f5eb8` (when `real`), and, when the defender is a barbarian, `×2` if
  the attacker's civ has wonder 6 (`0x444a61(civ, 6)`). The latter applies
  to every battle here (48 → 96, 16 → 32).
- **Defence strength** (`0x53f67a` + TOTPP.dll hooks), decoded for the
  land path and runtime-proven: `def·4·(terrain_def + river)` (ToTPP base
  hook `0x76c14f86` → terrain hook `0x76c14f23`: Rules @TERRAIN defence
  column in halves, plus the tile's river bit 7; air/sea defenders use it
  too when ToTPP's two options are on, which they are here, else a flat 2).
  Then a multiplier in halves (`0x65b49c`). It starts at 2. Fortified land
  unit (orders 2) → 3. Fortress tile (flags `&0x42 == 0x40`) vs a non-air
  attacker that doesn't negate walls → 4, *replacing* fortified. In a city
  with City Walls (improvement 8) **or** the defender civ owns wonder 6, vs
  a land attacker that doesn't negate walls → 6. Non-land defenders reset
  to 2. The ToTPP hooks store the configurable values 3/4/6 (read live from
  TOTPP.dll data). Veteran ×1.5 last. Observed D: 54 (vet def 3 on
  Grassland/Plains + river), **81 with the defender fortified** (battle 5,
  set via RAM, predicted statically before the fight), 24 (non-vet,
  Grassland, no river; battle 6).
- **Wonder 6** is FallRome's **Hadrianus' Wall** (Great Wall slot, in
  Eburacum, WRE): A ×2 vs barbarian defenders, barbarian A halved vs its
  owner, walls everywhere. It is why every WRE-vs-barbarian battle had A ×2,
  and why the Anglo-Saxon battle 6 did not.
- **The trailing draw is the promotion roll** (VA `0x544554`/`0x5445dc`):
  `r = rand() % (A+D)`. The winner is promoted if `r <=` the **loser's**
  strength (underdog wins promote more often), or always if the winner's
  civ has wonder 7 (FallRome: Warrior Culture, the Sun Tzu slot).
  Promotion (`0x53ff5e`) sets attr `0x2000` unless already veteran or
  one-shot (type flags bit 12). **Battle 6 predicted it**: seed `0x1008`
  chosen so a non-veteran defender wins and passes the roll, and the
  original set its attr `0 → 0x2000`.
- Unit-type flags (u32 at type +4) are the Rules flag string read right to
  left: bit 6 negates walls, bit 10 pikemen, bit 12 one-shot, bit 4
  can-attack-air.
- **Not implemented** (decoded only as far as their branch conditions):
  pikemen ×1.5 vs land units with exactly 2 move points, barbarian
  Archers/Legion types 4/5 halved, coastal fortress (0x1c) vs ships, SAM
  (0x11) vs air, SDI (0x1b), fighters vs air, air defenders in cities.
- **Oracle proof, six battles, all bit-exact** (outcome, damage, round
  count, CRT end state). Battles 4–6 were **predictive**: we wrote the CRT
  seed (`0x12345678`, `0x0badf00d`, `0x1008`) and the engine's prediction
  matched the original's outcome and end state. In battles 3–6, A, D and
  individual rolls were **polled from the live combat stack frame** during
  the animation (the frame stays up for over a second), and the polled
  rolls sit at the predicted round positions.

**Turn-loop wiring facts** (decoded for `WorldState::attack`; acceptance
test `crates/engine/tests/combat_install.rs` replays battle 2 from the real
ROMFALL.scn + Rules: A 96, D 54, outcome, CRT end state and the dying unit
all equal the oracle):

- **Defender choice** (`0x53fa30`): walk the tile's stack list (game
  record +0x1a prev / +0x1c next, game indices; totctl i16@32/@34) and
  skip land units on an ocean tile. Rate each unit
  `defense_strength(unit, attacker)`, scaled by `hp_left/hp_max` when game
  flag `0x10` is set. Pikemen get +1. Vs an air attacker, AEGIS gets ×5
  (missile) or ×3, and a fighter-flag unit vs a fighter-flag attacker
  gets ×2. Ships in a city are halved, but ×2 vs air when the city lacks
  improvement 0x1b. The **last** unit with `rating >= best` defends.
  ROMFALL (22,30): list 689 → 510, and 510 defended battle 2, 689 battle
  3, which matches.
- **Game-flags word = save @652** (RAM `0x607638` aligns byte for byte with
  the save from 652 on; tot-kit called 652..667 "menu"). Bit `0x10`: damage
  persists. Without it the HP helper `0x568d3e` zeroes damage before use.
  ROMFALL `0x81bf`. Same alignment: **active unit = save @674**,
  **difficulty @684** (`0x607658`, used as `(diff+1)·A/4` for barbarian
  attacks), **human-players bitmask @687** (`0x60765b`; ROMFALL `0x02` =
  WRE). So @687 is not part of a "seed word".
- **The combat RNG is seeded from the wall clock**: `srand(time())` at VA
  `0x598423` (`0x5b5970` = GetLocalTime-based `time`), plus a second
  `srand` at `0x4156dc`. A save does not determine combat; the engine
  seeds `combat_rng` from its own seed.
- **`has_wonder(civ, w)`** (`0x444a61`): false when the wonder is obsolete.
  That is decided by `0x4449b0`: ANY civ 1..7 knows the obsoleting tech
  (ToTPP hook `0x76c2b575`). Otherwise the owner byte (+6) of city
  `wonders[w]` (save @906) is compared. Obsolescence is not modelled yet
  (no per-civ techs in the world state).
- **Attack cost**: the attacker stays on its tile and spends one movement
  point (battles 2–4: spent 0 → 15 = one FallRome point). The only
  observed case is 1-move units.
- **Civ-name block slots** (242 B @1250): leader @2, plural @26, adjective
  @50, 24 B each, NUL-terminated. All 7 ROMFALL plurals read clean and are
  exactly the names FallRome's Events.txt uses.
- **UNITKILLED semantics** from the game's own `Macro.txt`: "Attacker is
  the civilization that killed the unit, defender is the one who owned the
  unit". It fires whichever side initiated, unless `Defender Only` is
  given. `Battle::unitkilled_ctx` builds exactly that, and the acceptance
  test fires it against the real FallRome Events.txt.

**Unit-record corrections that fell out of this** (these supersede the
40-byte record notes elsewhere in this file; the framing itself is now
fixed in both walkers — §Record framing FIXED, which also covers cities):

- totctl/tot-kit frame each unit record **6 bytes late**. The true record
  starts at the units region start (ROMFALL 159438; the "34-byte preamble"
  is record 0 minus its first 6 bytes), with **x@0, y@2, z@4, attr u16@6,
  type@8, owner@9, moves spent@10, visibility@11, damage@12, last
  dir@13, orders@17, home@18, id u16@30**. In totctl offsets that is +6, so
  totctl's x@6/type@14/owner@15/id@36 are right *within its framing*, but
  its index is the game's index − 1 and its bytes 0..5 belong to the
  previous unit. The game's own index (the active-unit field) proves the
  true framing.
- **u8@16 (totctl framing) = moves spent, not a per-owner value.** It is
  in units of the byte `@0x5f5eb8`, which is **15 in FallRome** (Rules says
  road multiplier 3, so ToTPP aggregates movement multipliers). The panel
  shows "Moves: 0 12/15" for 3 spent. The "3 WRE / 15 ERE / 30 emperors"
  pattern is spent movement in the saved turn. This matters for T1-3: the
  engine's "thirds" equal the original only when that byte is 3.
- **Damage = true@12** (totctl u8@18), HP = Rules `h`·10. Dead units get
  x/y/z = −200 (killed attacker) or −100 (killed defender) and id 0. A lost
  defence killed **only the defender**; its stack-mates survived (FallRome,
  open field).
- attr bit `0x0100` is set on fortified/sentried units and keeps them out of
  the `W` rotation; orders `0xff` none / `1` fortifying / `2` fortified.

**Live addresses** (ToTPP relocates the unit and unit-type tables to the heap
and patches the `.text` immediates, so stock-exe disassembly shows different
addresses; read them from the live `.text`, as `tools/oracle/unitram.py`
does): units base = imm32@`0x5414de` − 9 (`0x19e0028` this run), types base
= imm32@`0x53f623` − 0xc (`0x76c5a430`; stride 40: +10 move, +12 att, +13
def, +14 hp·10, +15 fp, +16 cost, +17 hold, +18 role, +19 prereq). Cursor:
i32 y@`0x5f46f8`, x@`0x5f46fc`. Active unit: u16@`0x60764e` (in the RAM
copy of the game block; written by the game, not a selector). Map view
struct @`0x61fb34` (i16 centre x/y, then i32 left/top/right/bottom, then
tile px 64×32).

**Still open for T1-6**: the special cases listed as not implemented;
fortress/walls/palace/SDI need tile and city improvements in the world
state; wonder obsolescence needs per-civ techs; the engine's
stack-insertion point for moved units; `Defender Only`.

### T1-6 combat: still OPEN, and what has been ruled out (2026-11-03 night) — SUPERSEDED

*Superseded by §T1-6 combat round formula DECODED above. Kept for the record.
Two claims below were wrong: combat does **not** go through `rand_int` (it uses
the CRT `rand()` at 0x5b11d0), and `u8@16` is moves spent, not a per-owner value.
Navigation was not the blocker either: units are driven through RAM
(`tools/oracle/unitram.py`).*

The combat round formula is **not** decoded yet. Recorded so the next session
does not repeat the dead ends:

- All gameplay randomness funnels through `rand_int(lo, hi)` at VA `0x554aa0`
  (58 static call sites; the float draw `0x554a43` has no other caller, and
  `0x5dd53c` is touched only by the RNG functions and the context-swap sites).
  So combat *must* consume randomness through `rand_int` — but **none of the 58
  call sites passes combat-shaped bounds** (most are literal small ranges
  0..7 or one variable bound with `lo=0`). Either combat's rolls are among the
  variable-bound sites and the bounds are built by a helper we have not traced
  (candidates: `0x428fc5` `hi=[ebp-0x710]-1`; `0x51a1bc`
  `hi=ecx-table[eax*4+0x5f68bd]`; `0x53213f` `hi=ecx`), or there is a second
  RNG entry point reached by an indirect call.
- `0x5602d3` (the "combat context" function previously noted at 0x5602cf —
  which is actually the `pop esi` of the *previous* function) is a **UI/dialog
  builder**: it saves `0x5dd53c`, loads `[this+0x100]`, calls `0x55fb2c` with
  `0x5ddea8`, then issues a long run of `0x57de0c` control-creation calls. It
  is a good model of the context-swap pattern, not the combat math.
- `0x5dea8` stays all-zero in `.data`, so it is a scratch object, not a
  pre-seeded table.
- Unresolved prerequisite: the **unit record's HP field is not identified**.
  Of the 0x32 40-byte unit record (`totctl` proves the region arithmetic
  exactly: ROMFALL `159472 + 710*40 = 187872` = city block start), bytes
  16..35 are still open; `u8@16` looks like a per-owner value (3 for WRE, 15
  for ERE, 0 for most barbarians, 30 for emperors), **not** HP. Without HP the
  round-by-round outcome cannot be read back, which is why a save-diff around
  a single attack is the decisive experiment.
- The experiment is blocked only by **rig navigation** (see
  `tools/oracle/README.md` §Navigation, 2026-11-03): there is no reliable way
  yet to place the cursor on a known tile, and a session of blind clicking
  consumes every unit's orders. Once the view origin is readable from RAM, the
  plan is: seed `0x5dd53c=1`, save, press the attack direction, read the new
  state (draw count), save again, diff the two unit records — that yields both
  the HP field and the per-round outcome for a known roll sequence.

### T1-6 the whole RNG *API* decoded from the binary (2026-11-03 night)

Static decode of the four functions around VA 0x554a00 (`civ2.exe`, `objdump
-M intel`; `.text` VMA 0x401000 / file 0x400):

| VA | role | semantics |
|---|---|---|
| `0x554a00` | context exchange | `x ^= a; a ^= x; x ^= a` on `0x5dd53c` — i.e. store `a`, return the old value. The per-context swap primitive. |
| `0x554a43` | **the draw** | `state = state*0x19660d + 0x3c6ef35f`; returns a `double` `state / 4294967295.0` (`fild` of the 64-bit-zero-extended state over 0xFFFFFFFF) |
| `0x554aa0` | **`rand_int(lo, hi)`** | the only random function gameplay code calls (58 static call sites) |
| `0x5b4f70` | float→int | truncation (the `(int)` in `rand_int`) |

`rand_int(lo, hi)` in full:

```text
if lo == hi:            rnd(); return lo           # burns one draw anyway
if hi <  lo:            swap(lo, hi)
span = hi - lo
return lo + trunc( (state / 4294967295.0) * (span + 0.999999) )
```

`0.999999` is a `double` constant at VA **0x5c62f0** (`e9 0b 21 e7 fd ff ef 3f`),
read directly from `.rdata`. The range is **inclusive of `hi`**, and the
`lo == hi` case still consumes a draw — so every call is exactly one draw,
which is what makes draw-count parity checkable.

**Shipped**: `engine::rng::GameRng::next_between(lo, hi)` implements that
formula verbatim; `next_below(n)` is now `next_between(0, n-1)`, replacing the
previous multiply-shift *approximation* (the doc comment said so). Tests
restate the binary formula independently and check the degenerate-range draw
burn. Golden `run50` fingerprints regenerated for the (now exact) mapping.

### T1-6 PRNG RUNTIME-VERIFIED (2026-11-03 night) — the original generator is in the engine

- **Patch-and-prove, end to end**: writing `state = 1` into the live
  game's `0x5dd53c` and reading it later returned **0x0438e694 — exactly
  LCG step 9 from 1** (`x = x·1664525 + 1013904223 mod 2³²`). Game rolls
  flow through this recurrence and the post-roll state persists in the
  global (the earlier "survived unchanged" reading was a mid-swap
  restore leftover, not the live state).
- `engine::rng::OrigRng` now implements exactly this (with
  `set_state`/`state` for the original's save/swap/roll/restore context
  pattern); the unit test reproduces the oracle step-9 value. The
  placeholder MMIX LCG is gone (`Lcg64` is now an alias).
- Golden corpus regenerated for the semantic change (run50
  fingerprints) — all checks green.
- Still open for T1-6: which ctx (`ctx+0x100`) feeds combat vs turn
  processing, the per-action roll counts, and the combat round formula
  itself. **Correction (2026-11-03):** the function previously filed here as
  "combat at RVA 0x5602cf" is `0x5602d3` — a UI/dialog builder, and 0x5602cf
  is just the `pop esi` of the preceding function. It is a *model* of the
  save/swap/roll/restore pattern (load `[this+0x100]`, call `0x55fb2c` with
  `0x5ddea8`, then a long run of `0x57de0c` control creation), not the combat
  math. See §T1-6 combat: still OPEN for the current state of that hunt.

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
2. *(Framing and items 4–5 SUPERSEDED 2026-11-04, §Map tile framing FIXED:
   records start at `map_data + 7·area`, terrain is byte +0, road/rail are
   byte +1 `0x10`/`0x20`; the `6t+14` "road" bits were the next tile's
   visibility.)* **The embedded SCN/SAV map section is decoded (ROMFALL, `0x32`).** Per Mercator's
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
2. *(superseded by §Record framing FIXED — x/y are u16@0/@2 of a record
   that starts 34 B before the name)* **City record (92 B): `u16@58 = x`, `u16@60 = y`** (logical grid, same
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





## Hut mercenaries, metals and tribe additions (#79, partial)

`tools/huts_check.py` loads the user's unpatched executable into Unicorn
and executes three supplied-dispatch fragments, their original CRT calls
and the original building-bit callee (no replacement callees):

- `54bae5..54bd12`: mercenary candidates use **global** advance ownership
  for 11/64/55/42/17 and 34/35/39. Random draws precede later candidate
  overrides; preserving only the final unit would lose RNG parity.
  Modes 0/1 use classic candidates; mode 2 uses fantasy candidates and
  map-1/map-2 overrides. The engine currently runs the surface map only.
- `54b453..54b4d1`: 25/50/100 gold by the decoded difficulty rolls; double
  only when saved year >1000; the treasury addition wraps as original x86.
- `54af81..54b03c`: at year **>=1000**, size `rand()%4+1`, Temple always,
  Market/Granary/Library when subsequent `%2`/`%3`/`%4` is zero. Earlier
  years retain size 1 and do not consume these four draws.

The probe executes 1,000 contexts per fragment and commits 256 of each
in [hut-outcome-vectors.txt](hut-outcome-vectors.txt), with the next CRT
state. Rust compares every committed vector, including that state.
[hut-outcomes.json](hut-outcomes.json) records executable hash and scope.

The additional 128 cases execute complete `5703be` with real map-access
callees on invented tiles. The site value is tile +2 >>5 if nonzero;
otherwise fertility (+5 &15), raised to 8 when nonzero. A tribe needs
value >=12 (`54af24..54af3c`). MapState now retains these proven tile
fields and fingerprints them. Nearest-city lookup `435dd0` uses the
existing `566dad` distance, respects flat-world wrapping and excludes
empty city slots. Tribe creation calls `432810` directly (`54af76`):
it does not create or consume an explorer/settler unit.

The game-family byte is file @982 / RAM `607782`, following the saved
power order. Original loader `41924f` sets 0 for Original, `4192bf` sets
1 for SciFi, `41932f` sets 2 for Fantasy (the preceding path strings
identify these families); both reference saves carry 0. It is retained
and fingerprinted. Mode-specific non-surface outcomes are not exercised
by the engine yet.

Reproduce (optional Unicorn dependency installed outside the repo):

```sh
CIVTOT_HOME=... PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/huts_check.py --vectors /tmp/hut-vectors.txt
```

### Hut dispatch, nomads and barbarian ring (#79/#83)

The same probe now additionally executes 1,000 contexts each for selection
`54acbf..54adb7`, nomad redirects `54b09b..54b1ac`, barbarian gates
`54b520..54b704` and ring placement/type selection `54b704..54babd`.
Rust compares 256 selection, 128 nomad, 256 barbarian gate and 128 ring
vectors: [dispatch](hut-dispatch-vectors.txt),
[nomads](hut-nomad-vectors.txt), [gates](hut-barbarian-gate-vectors.txt),
[ring](hut-barbarian-vectors.txt). Selection and gate vectors compare the
next CRT state, including the conditional AI-assistance and turn-30/50/75
draws. Ring vectors compare every spawn request in order on invented
maps with blocked, rough, water and impassable neighbours, edge clipping
and east-west wrapping. The ring probe **records** `56a1c0` creation
requests assuming success and **skips** `467d19` redraw; other placement
and type helpers execute original bytes. It does not assert original
unit-record mutation or allocation-cap parity.

Selection keeps the original `%5` draw and human `%3` draw, then forces
tribe when the saved city count is nonzero, the landmass city cache is
zero and either the low-seven-bit flag test is zero or nearest-city
distance is >12. These caches are retained, not inferred from ownership:

| Saved field | Civ-block/file location | Original use/writer |
|---|---|---|
| leader-table index | civ i16 +14 | `54b1df` indexes a 60-byte leader row |
| landmass city-count bytes | civ +0x5d0, 256 bytes | `54ad4b`; statistics pass clears at `507437`, increments own land-city count at `5078e1` |
| landmass flags | civ +0x8d0, 256 bytes | `54ad6f` tests `&127`; surface refresh is proven in §Hut-facing landmass strategy flag lifecycle |
| power-reference byte | file @965 / RAM `607771` | barbarian assistance at `54b584` |
| per-civ power ranks | file @966..974 / RAM `607772` | rank comparison at `54b58b..54b59e` |

`SavedAiState` preserves and fingerprints all of these fields. They do
not yet refresh through a full original AI statistics pass; that is an
open sustained-play gate, alongside power refresh in #89.

Nomads redirect to gold when any civ owns advance 28; otherwise they
redirect to scrolls when unhomed settler-role units exceed saved city
count >>3. Surface nomad type is `INITIAL_SETTINGS[leader][1]`, loaded to
`606f81 +60*leader` by `413b95`, selected by `54b1df..54b1f6`. Without
that section the original initializes it to 0 (`413b02`). Scrolls now
exclude disabled advances and apply `4a2098`'s `CIVILIZE2`/`LEADERS2`
group restriction: group -1 passes, otherwise the leader's group byte
must be zero. Loader paths `415192` and `41526a` establish these inputs.
The integrated regression checks leader-dependent nomads and a disabled
scroll candidate. Original new-unit movement-spent byte is zero
(`56a594`), hence hut units receive their full Rules movement allowance.

Barbarian huts redirect to mercenaries near any city (<4), or before turn
50 with no city. For AI civs, a stronger referenced power rank can replace
the result with a tribe on `rand()%8 < difficulty`. Below barbarian level
3, nearby own-city/landmass checks can suppress the encounter in early
turns. Early surviving encounters spawn one unit; later rings step by
`clamp(4-city_count,1,4)`, starting at turn%8. Rough terrain (>2 movement)
chooses infantry; other valid land chooses cavalry. Types use global
advances 51/34 and the **Rules prerequisite of unit 22** (read via
`5f4a83`), with fantasy surface variants. The ring excludes occupied,
water, impassable and out-of-map tiles.

Reproduce all retained vectors:

```sh
CIVTOT_HOME=... PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/huts_check.py --vectors /tmp/hut-outcomes.txt \
  --dispatch-vectors /tmp/hut-dispatch.txt --nomad-vectors /tmp/hut-nomads.txt \
  --ring-vectors /tmp/hut-ring.txt --barbarian-gate-vectors /tmp/hut-gates.txt
```

Still open: complete same-seed hut-entry/state comparisons, runtime cache
refresh, non-surface scenario cases and turn-time barbarian arrivals
(#83). These checks do not close either issue by themselves.

## Original calendar and leader technology groups (v0.8 integration)

The round-close caller `470305..47034e` runs barbarian arrivals before
incrementing the turn, then computes/stores the year with `46da89`.
`tools/calendar_check.py` executes that complete routine and its real
clamp; [calendar.json](calendar.json) records the four original six-stage
tables at `5d4fa8`, executable hash, 2,068 executions and two saved-input
comparisons. Both reference dates match exactly: Original turn 43,
year -1900; FallRome turn 0, raw date 5159. The latter is a scenario date
with increment -5/start 5160, rather than a conventional AD year integer.
Rust compares all 324 retained [vectors](calendar-vectors.txt).

Difficulty is adjusted by flags +660 bit4 (-1) and bit8 (+1), then
`clamp(adjusted-1,0,3)` selects the table. Turn 0 and 1 both use the first
date; standard calendars skip year zero. With scenario flag 0x80 and
nonzero increment/start, the date is `turn*abs(increment)+start-1`.
The signed word file @672 / RAM `60764c` bounds the calendar's effective
turn when nonnegative; subsequent elapsed turns add one raw year each.
Scenario increment/start are parameters i16@84/@86 (`5f5ea4`/`5f5ea6`).
This also identifies the research-cost tuple's second word: the original
cost routine uses calendar increment !=0 to skip catch-up adjustments.

World turn advancement now updates the date using these inputs, including
signed-word turn wrapping. Unlocated 0x33 scenario parameters are retained
as an unknown calendar and keep their date; no custom-calendar bytes are
guessed. This is a documented remaining decoder limit.

`tools/tech_groups_check.py` executes both original group gates, 1,000
contexts, with 128 retained [vectors](tech-group-vectors.txt) and provenance
in [tech-groups.json](tech-groups.json). `4a2098` permits a hut advance
only for group -1 or a zero leader-group flag; `4a2111` excludes research
only when that flag is 2. Thus flag 1 blocks huts but permits research.
The shared engine lookup reads `CIVILIZE2` and the saved leader's
`LEADERS2` row; runtime research choices and hut scrolls apply their
respective gates. No replacement callees are used in these probes.

```sh
CIVTOT_HOME=... PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/calendar_check.py --vectors /tmp/calendar.txt
CIVTOT_HOME=... PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/tech_groups_check.py --vectors /tmp/tech-groups.txt
```

## Live hut-entry parity and unit allocation (#79, v0.8)

[Eight controlled live entries](huts-live.json) compare the actual engine
WorldState dispatcher with the running ToTPP oracle: gold 50, mercenary
Horsemen, advance 65, a late size-3 tribe with Temple/Granary, Settlers
nomads, a barbarian Horseman, an empty encounter, and a landmass-forced
size-1 tribe. Every gameplay CRT draw count/state matches, as do the
resulting treasury/knowledge and created-unit slot/type/location/home/
veteran/movement and tribe population/improvements. The retained
[plain vectors](huts-live-vectors.txt) run in `huts_install.rs` against
the user's reference save/Rules. Each case reloads that save before the
listed RAM-controlled inputs; executable/save/Rules hashes are retained.

The live probe `tools/oracle/hut_entry.py` pins CRT at hut entry and
records callers/pre-draw states through exit. It separates three
**presentation** callers (`55852f`, `581d96`, `582b9b`) into their own CRT
state while executing the original rand routine. The first two render
reward windows; `582b72..582bbc` generates a nonzero UI object token at
object+0x408. A gold entry otherwise consumes 176 additional presentation
draws. This controlled stream is necessary for a reproducible headless
comparison; there is no claim of matching the UI's random progression.
All hooks/cave bytes are restored in finally. The oracle README gives
entry/collection commands and the explicit patch controls.

The captures also establish creation details missed by the earlier port:
new units reuse deleted **slots**, appear at the stack **head** (mercenary
slot 34: prev=-1/next=9; explorer 9: prev=34/next=-1), and start with zero
movement spent. The human mercenary inherits nearest city 5 as home;
nomads explicitly clear home. Constructor `56a51d..56a579` uses the nearest
city (of any civ) and retains it only if owned by the recipient. The hut's
AI/non-displayed path clears home at `54bd93`; the headless engine uses
the preserved human mechanics mask for this distinction.

`tools/allocation_check.py` executes the original first-free/capacity
fragment in 124 contexts, including 2,040/2,048 slot limits and the 1,948
military-count boundary. It stops before allocation UI/initialization.
[unit-slot-vectors.txt](unit-slot-vectors.txt) compare those decisions;
alive identities in the probe exceed 16 bits. Original `56a45c` and load
writer `45f0d7` store **dwords** at unit+30 and city+86; four retained
[identity write cases](unit-allocation.json), including 65,536 and
0xfffffffe, correct the old u16 accessor claims. The loader (`45f03b..e4`)
renumbers alive identities on each load. The engine preserves imported
identities and uses a fingerprinted monotonic counter for its own units;
it does not compare those transient numeric IDs across oracle loads.
Production/events/huts share slot allocation. Blocked engine production
retains its completed box/population; full original capacity-failure city
behavior is not established by the allocation-only fragment.

Still open for sustained campaigns: updating the saved AI statistics/
landmass caches, turn-time barbarian arrivals (#83), and broader city-turn
parity (#13). The live surface entry checks do not assert non-surface
scenario behavior.

## Reproduce

```bash
G=/mnt/d/games/CIVTOT   # or export CIVTOT_HOME=...
python3 tools/totctl.py rules  $G/Rules.txt -o /tmp/rules.json
python3 tools/totctl.py map    $G/World.mp --rules $G/Rules.txt \
        -o /tmp/world.png --json /tmp/world.json
python3 tools/totctl.py save   $G/FallRome/ROMFALL.scn -o /tmp/walk.json
python3 tools/totctl.py diff   $G/Scenario/FallRome/T4PAIR_A.SAV \
                                $G/Scenario/FallRome/T4PAIR_B.SAV -o /tmp/diff.json
```


### Round-end barbarian arrival phase (#83, partial)

`engine::barbarians` ports original `0x46e6cd..0x46f6a2`; the round-close
caller at `0x470314` invokes it before advancing the turn (except turn zero).
`tools/barbarians_check.py` executes that original routine, CRT and map/city
helpers in Unicorn. It substitutes successful allocation records at `0x56a1c0`,
passenger sentry at `0x5692f7`, ship stack reordering at `0x569c88`, and UI string,
notification and redraw calls. Notification suppression uses the original
`0x6031b8` gate. This is request/state parity at those boundaries, **not** a
full original turn, real capacity-failure or sustained live-game comparison.

```sh
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/barbarians_check.py --gold /tmp/barbarians.json \
  --vectors /tmp/barbarian-vectors.txt
cmp docs/evidence/barbarians.json /tmp/barbarians.json
cmp docs/evidence/barbarian-vectors.txt /tmp/barbarian-vectors.txt
cargo test -p engine --lib barbarians
```

[barbarians.json](barbarians.json) records provenance and counts;
[barbarian-vectors.txt](barbarian-vectors.txt) retains 1,024 derived synthetic
contexts, 123 with arrivals (734 requested units), on 64×48 land, water and
mixed maps. Rust compares every request's type, coordinates, sentry order and
veteran flag plus the final CRT state. Cases vary difficulty, level, turn,
family, flat/wrapped maps, technology masks, human mask, city size, treasury,
population cache, government and original owner.

Notable literal behavior: the city lottery advances to the slot *after* the
selected surface slot before checking it; land x offsets are `2*(rand()%13)-6`;
level 1/2 group-size cases fall through to the level 3 size assignment. Sea
search exhaustion aborts the entire phase after the 1,001st pair of draws.
Sea passengers are sentried and the ship has a 50% veteran roll; land groups
include a Diplomat before the raiders. Runtime per-civ population is the saved
signed word at civ +116 (`0x53ae5e`, denominator cache+1); the sea size gate uses
landmass table word +4 (corrected by §Sustained live barbarian arrivals).
Runtime city statistics and 64 sustained live phases are now checked below;
non-surface arrivals are not represented by this engine map.


### Runtime city statistics and power ranking (#79/#83/#89)

The native per-civ statistics pass is `0x505df1`, called at `0x472756` for
human and AI slots alike, after its city processing call (`0x470b69` →
`0x4cb01d`). Its own-city cache slice filters deleted records, invalid
coordinates, and foreign owners (`0x507491..0x50754c`), increments the city
count and signed population cache (`0x507794..0x5077ee`), then counts each
non-ocean city's landmass (`0x507845..0x507908`). Counts wrap as a byte;
population wraps as a signed word. `engine::statistics` refreshes these
fields after the engine's city processing. Hut-facing surface landmass flags
now refresh too (§Hut-facing landmass strategy flag lifecycle); the complete
original per-civ city/movement phase schedule is not claimed.

`tools/statistics_check.py` executes the admitted-city slice and its real
landmass helper. [city-statistics.json](city-statistics.json) and
[city-statistics-vectors.txt](city-statistics-vectors.txt) retain 128 cases,
including ocean exclusion, landmass 0/255, and 260 cities causing byte-count
and signed-population wrapping. The Rust test also checks deleted/invalid
records against the enclosing original filters. Non-surface city populations
are counted, but their landmass is unavailable to the surface-only model.

`engine::power` ports `0x46de84..0x46e354`. Standard score is acquired
advances×3 + cached population×8 + (treasury arithmetic-shift-right 5).
The original computes a unit-strength sum but does not add it to the score.
Scenario flag bit 2 instead scores twice the objective-city count, adding
wonders if bit 4 is also set. Objective is city flag `0x04000000`; the scenario
flags are word +0 of the already-proven scenario parameter block (same base
as calendar/research fields). For unlocated 0x33 tails, score refresh waits
rather than guessing scenario behavior. The alive mask is saved byte @686,
the byte read at `0x46dea8` (RAM `0x60765a`). It remains the original saved
mask; complete civ elimination/revival behavior is not yet implemented.

The pass ranks all seven player slots including inactive zero-score slots,
resolving ties in favor of the lower civ number for the stronger position.
It writes only live slots into the order table, then finds strongest/weakest
human **rank indices**, stored in game bytes @964/@965. Consumers use these
bytes literally even where they subsequently index civ data. Refresh runs
after barbarian arrivals and before advancing the turn, except on turn zero,
as the round-close caller `0x47030e..0x470337` does. Its later diplomacy/war
adjustments are excluded.

[power.json](power.json), [power-vectors.txt](power-vectors.txt), and
[power-saved-vectors.txt](power-saved-vectors.txt) record 256 native synthetic
executions and recomputation from `Original/Ph_Auto2.SAV` and
`FallRome/ROMFALL.scn`. The latter loads the actual civ/game/city/scenario
inputs internally; no original records are emitted. Rust checks scores,
ranks, order and both reference bytes. These are native recomputations,
not assertions that all saved caches are fresh.

```sh
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/statistics_check.py --gold /tmp/city-statistics.json \
  --vectors /tmp/city-statistics-vectors.txt
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/power_check.py --gold /tmp/power.json \
  --vectors /tmp/power-vectors.txt --saved-vectors /tmp/power-saved-vectors.txt
cmp docs/evidence/city-statistics.json /tmp/city-statistics.json
cmp docs/evidence/city-statistics-vectors.txt /tmp/city-statistics-vectors.txt
cmp docs/evidence/power.json /tmp/power.json
cmp docs/evidence/power-vectors.txt /tmp/power-vectors.txt
cmp docs/evidence/power-saved-vectors.txt /tmp/power-saved-vectors.txt
CIVTOT_HOME=/mnt/d/games/CIVTOT cargo test -p engine --test power_install
```


### Wonder one-off effects and remembered world state (#88)

`engine::wonders` now implements Leonardo (14), Eiffel Tower (20), Manhattan (23)
and surface Apollo (25), in addition to Darwin. `tools/leonardo_check.py`
executes original `0x49d4f9` including active-wonder/expiry/knowledge tests
and the entire candidate scan, suppressing only redraw and selecting AI
inputs (so upgrade notification branches are absent). [leonardo.json](leonardo.json)
and [leonardo-vectors.txt](leonardo-vectors.txt) retain 256 synthetic cases,
106 with a changed type. Cases include deleted/foreign units, unavailable
wonders, knowledge, civilization masks, candidate roles/costs, damaged and
veteran units. Sixteen unit types vary; remaining native slots are unavailable.

The original chooses the **last** qualifying type, with prerequisite equal
to the known obsoleting advance, equal role, and cost at least the old cost.
For role 1, a weaker defence than slot 7 plus knowledge of advance 35 forces
that obsoleting advance. An upgrade clears veteran, preserves damage and
leaves the original spent-move byte untouched. Rust preserves spent moves
within its remaining-movement representation (already-overdrawn imported
moves are clamped by that existing model). Completion and Rules-aware
research/hut grants invoke this path; the raw `grant_advance` API remains
bookkeeping-only. Capture-triggered upgrades await the city capture path.

The unit civilization mask is @UNITS_ADVANCED field 0: the loader at
`0x4138a7` selects that section; `0x4138e5` defaults +20 to 255 when absent;
`0x413993..0x41399e` parses/stores its binary field. `0x486960` converts the
binary token, and the upgrade routine tests the civ bit at `0x49d65d`.
Rust's native-vector tests parse each synthetic mask through this Rules field.
Obsolete production also tests the mask before selecting its successor.

`tools/diplomatic_wonders_check.py` executes full Eiffel Tower `0x4c6121` and
Manhattan `0x4c8ddf..0x4c8f7d`, with the existing notification-suppression
flag. [diplomatic-wonders.json](diplomatic-wonders.json) and
[diplomatic-wonder-vectors.txt](diplomatic-wonder-vectors.txt) retain 256 cases.
Eiffel Tower acts once (civ flags +0, bit 16): halves its civ's byte +38,
subtracts 25 from other civs' attitudes toward it (clamped 0..100), and
clears their treaty bit 16. Attitudes are civ bytes +72..80, read/set by
`0x4546ae`/`0x4546dc`. Manhattan replaces **all seven** player bytes +38 with
`min(6, ceil(old/2))`, including inactive slots. Flags, reputation byte +38,
and attitudes are now retained/fingerprinted; these checks prove their
mutations, not a complete diplomacy model or government-selection interface.

`tools/apollo_check.py` executes `0x4c8fcf..0x4c91d7` with **no replacement
callees**, stopping before completion UI. [apollo.json](apollo.json) and
[apollo-vectors.txt](apollo-vectors.txt) retain 32 synthetic 8×4 surface maps.
Every tile's visibility becomes 255, all seven civs' remembered-improvement
bytes receive tile byte +1, and each live city gets knowledge mask 255 and
its current size in all eight remembered-size slots. Dead city slots remain
unchanged. Rust compares all these fields. It retains city byte +14 and
sizes +15..23; creation initializes global knowledge only when the Apollo
table entry is nonnegative (`0x432b4f..0x432ba9`).

This also identifies the formerly unnamed 7 B/tile save block: seven
remembered-improvement arrays in civ 1..7 order. The original file loop
`0x56fbb6..0x56fc07` transfers each civ array, one byte per tile, before the
6-B tile-record block. `0x57008f` indexes its per-civ pointer plus tile index;
Apollo uses that real accessor through `0x571819`. Boot retains these arrays
and fingerprints them. General movement-time updates of remembered city/map
state and non-surface maps remain separate work; this is not fog-of-war UI
parity or full completion-dialog parity.

```sh
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/leonardo_check.py --gold /tmp/leonardo.json --vectors /tmp/leonardo-vectors.txt
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/diplomatic_wonders_check.py --gold /tmp/diplomatic-wonders.json \
  --vectors /tmp/diplomatic-wonder-vectors.txt
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/apollo_check.py --gold /tmp/apollo.json --vectors /tmp/apollo-vectors.txt
cmp docs/evidence/leonardo.json /tmp/leonardo.json
cmp docs/evidence/leonardo-vectors.txt /tmp/leonardo-vectors.txt
cmp docs/evidence/diplomatic-wonders.json /tmp/diplomatic-wonders.json
cmp docs/evidence/diplomatic-wonder-vectors.txt /tmp/diplomatic-wonder-vectors.txt
cmp docs/evidence/apollo.json /tmp/apollo.json
cmp docs/evidence/apollo-vectors.txt /tmp/apollo-vectors.txt
cargo test -p engine --lib wonders
```


### Native growth preparation (#13)

The former `engine::cities::CityEcon` was an observed-series approximation,
not the original growth algorithm. `engine::growth` holds independently checked
preparation arithmetic and is now integrated by `run_city_phase`/`run_turn`.
The controlled comparisons below do not close #13's live-save acceptance gate.

`tools/growth_check.py` executes food-row routine `0x4c19e0` (including its
original clamp) and positive-box growth `0x4c5cdb` through explicit stops
before UI/worker assignment. Its growth-limit helper `0x434ce5`, building
lookup and Pyramids ownership/expiry helpers run from original bytes, without
replacement callees. [city-growth.json](city-growth.json),
[food-row-vectors.txt](food-row-vectors.txt), and
[city-growth-vectors.txt](city-growth-vectors.txt) retain 1,024 cases each.
Rust compares row results, growth thresholds, granary/Pyramids storage and
Aqueduct/Sewer blocking with the original results.

Confirmed ordering: `0x4cb15d` prepares growth using the **starting** food box,
then `0x4cb17d` refreshes city totals/workers; `0x4cb1da..0x4cb23d` collects
food and charges citizens and homed settlers. This explains why the observed
series appeared to carry the current surplus after growth. The original
threshold is starting box >= `(size+1)*civ_food_rows`, not the old approximation
of adding food then comparing against `(cosmic_rows-1)*(size+1)`. AI food rows
follow the shield-row arithmetic, except the late adjustment begins at
**turn >200** (shield rows use >=200). Government >2 selects the high settler
food cost. Standard-game lab difficulty is 4, so its early AI rows are 9;
that must not be generalized to every civ or game.

An allowed growth increments size and resets the box to zero, or to
`(new_size+1)*floor(raw_cosmic_rows/2)` with Granary (improvement 3) or active
Pyramids (wonder 0). Aqueduct (9) and Sewer (23) checks use the pre-growth
size against the respective cosmic limits; a blocked growth rolls back size
**after** resetting storage. Negative food boxes use a separate original
starvation path and must not be clamped away before its next processing.

```sh
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/growth_check.py --gold /tmp/city-growth.json \
  --rows-vectors /tmp/food-row-vectors.txt --growth-vectors /tmp/city-growth-vectors.txt
cmp docs/evidence/city-growth.json /tmp/city-growth.json
cmp docs/evidence/food-row-vectors.txt /tmp/food-row-vectors.txt
cmp docs/evidence/city-growth-vectors.txt /tmp/city-growth-vectors.txt
cargo test -p engine --lib growth::tests
```


### Native workers and combined city totals (#13/#84/#85/#86)

`engine::workers` implements original allocation `0x4c2c14`, with unavailable
radius slots supplied separately. [workers.json](workers.json) and
[worker-vectors.txt](worker-vectors.txt) cover 1,024 native executions: worker
preservation/reselection, food preference, weighted remaining choices, surplus
citizens, flags and signed food-debt normalization. Only `0x4c22a7` tile-yield
calls are replaced with supplied synthetic triples. Scoring divides by current
city totals before multiplying candidate yields. The center is not collected
twice when it was preserved; the original temporary array aliases its center
marker. Packed specialist normalization (`0x4c34b6`) preserves existing types,
fills free zero-type slots with entertainers and clears unused trailing slots.

[worker-availability.json](worker-availability.json) and its
[vectors](worker-availability-vectors.txt) cover all 156 saved cities using
original radius/overlap routines `0x4c109f` and `0x4c13fe..0x4c1710`, with no
replacement callees. Visibility, hostile stacks, city markers and neighboring
cities' worked tiles all affect availability. [tile-yields.json](tile-yields.json)
and [tile-yield-vectors.txt](tile-yield-vectors.txt) cover 3,276 original per-radius
food/shield/trade triples, all matching `WorldState::tile_yield`. The original
also disagrees with two stored FallRome food values; saved caches alone do not
prove an arithmetic bug.

[city-totals.json](city-totals.json) and
[city-total-vectors.txt](city-total-vectors.txt) execute radius, capital,
support, allocation (auto mode), food routes, specialists, trade, finance and
happiness in native city-index order. All callees execute original bytes.
Rust matches all 14 fields for 312 recomputations: 156 fixed workers and 156
automatically assigned workers, across standard and FallRome inputs. This
includes all 156 fixed-worker shield totals: Aventicum recomputes to 5 shields
in both implementations, while its stored cache is 6. Two food differences
required the native `0x4c506e` transfers: each outgoing negative-commodity route
costs one food; each incoming one adds one food, independently of ownership.
No claim is made for growth, collection into boxes, production, treasury,
full-turn ordering or 50-turn parity. Worker helpers are now called through `run_city_phase` by `run_turn`.

```sh
# The external Unicorn dependency is documented above; original bytes remain local.
export CIVTOT_HOME=/mnt/d/games/CIVTOT
export PYTHONPATH=/tmp/totengine-research-deps
python3 tools/workers_check.py --gold /tmp/workers.json --vectors /tmp/worker-vectors.txt
python3 tools/availability_check.py --gold /tmp/worker-availability.json --vectors /tmp/worker-availability-vectors.txt
python3 tools/yields_check.py --gold /tmp/tile-yields.json --vectors /tmp/tile-yield-vectors.txt
python3 tools/city_totals_check.py --gold /tmp/city-totals.json --vectors /tmp/city-total-vectors.txt
# Compare each report/vector pair with its corresponding docs/evidence artifact.
cargo test -p engine --lib workers::tests
cargo test -p engine --test workers_install --test yields_install --test city_totals_install
```


### Native starvation decisions (#13)

[city-starvation.json](city-starvation.json) and
[city-starvation-vectors.txt](city-starvation-vectors.txt) retain 1,024 executions
of the negative-box branch `0x4c59b9..0x4c5c97`. The first live role-5 unit
homed in the city is selected in slot order, without an additional owner check.
If the outgoing-food count is positive, the city cancels every negative-commodity
route to the selected partner instead of losing a settler or population.
The selected partner is the last outgoing-food partner from `0x4c506e`.
Route deletion `0x433444` executes unchanged, compacts live slots without
clearing the tail and sets the refresh marker `0x20000`.

Otherwise, a homed settler is removed if the city belongs to a human-mask civ
or its known-to mask intersects that mask. An AI city unknown to humans retains
the settler and population. With no homed settler, population decreases by one
and zero requests city destruction. Non-destroyed cities reset food storage to
zero. The vectors cover 127 unit-removal requests and nine destruction requests.
`engine::growth` matches these decisions and route compaction.

The check skips notification-string setup, records the unit-removal helper
boundary without executing it and stops before city destruction. It supplies
the preceding totals pass's outgoing-food globals, including deliberately
inconsistent combinations to distinguish the branch inputs. It does not prove
unit/city destruction side effects, full turns or live-game parity. The turn loop now uses these decisions through `run_city_phase`.

```sh
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/starvation_check.py --gold /tmp/city-starvation.json \
  --vectors /tmp/city-starvation-vectors.txt
cmp docs/evidence/city-starvation.json /tmp/city-starvation.json
cmp docs/evidence/city-starvation-vectors.txt /tmp/city-starvation-vectors.txt
cargo test -p engine --lib growth::tests
```


### Controlled 50-turn city-phase sequence (#13)

`tools/city_phase_check.py` executes the full original `0x4cb01d` city routine
for 50 consecutive city rounds, retaining its gameplay callees. It starts from
the standard reference world's terrain, city positions and governments, with
explicit controls: human mask 255, difficulty 0, all cities reset to size one,
empty food boxes, only their saved Palace retained, center-only workers,
Capitalization, 100% taxes and all units unhomed. The original produces 500
city transitions, retained in [city-phase.json](city-phase.json) and
[city-phase-vectors.txt](city-phase-vectors.txt). Only the open-city-window query
`0x411c50` and map redraw `0x467c89` boundaries were reached/replaced; the
notification boundaries listed in the script were not reached.

An install-backed Rust comparison using `city_growth_context`, native growth
preparation, `refresh_city_totals`, food collection, existing production,
post-production totals and treasury matches all economic fields for all 500
transitions: size, signed boxes, workers, specialists, pre-route trade,
science/taxes/trade, food/shields, happiness and treasury. It also compares city
flags except `0x80000`: the original's end-of-city settler-job planner
`0x4cad56` sets that flag and its work-request state, which are not ported.
The original flag remains in the artifact. Balanced cities set `0x4000` when
net shields are positive and the collected food surplus is nonnegative
(`0x4c9c2d..0x4c9c8c`).

This verifies the positive-growth/two-totals-pass sequence in a controlled
setting; **the original sequence test calls `run_city_phase`; a second, civ-ordered
sequence now also checks `run_turn`**. It is not a Wine/autosave comparison, ordinary unit/building
production sequence, starvation/destruction or disorder/celebration replay,
research campaign, or complete original turn/PRNG parity. Those remain part
of the outstanding #13 integration and live acceptance work.

```sh
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/city_phase_check.py --gold /tmp/city-phase.json \
  --vectors /tmp/city-phase-vectors.txt
cmp docs/evidence/city-phase.json /tmp/city-phase.json
cmp docs/evidence/city-phase-vectors.txt /tmp/city-phase-vectors.txt
CIVTOT_HOME=/mnt/d/games/CIVTOT cargo test -p engine --test city_phase_install --test city_totals_install
```


### Surface city destruction (#13)

`engine::removal::destroy_surface_city` follows `0x43456d` for retained state:
remove city identity, decrement its civ's city count, handle homed units in
reverse slot order, compact incoming routes, mark hosted wonders destroyed
(-2), clear the city's tile marker and invalidate nearby fertility/site caches.
Type 9 becomes homeless instead of dying. A non-human unit in another live
city can be rehomed there if it has role 1 and is the only role-1 unit in that
stack; otherwise the other city gets flag `0x20` before removal. This test is
about role-1 stack count, not total stack size. Human homed units are removed.

[city-destruction.json](city-destruction.json),
[city-destruction-vectors.txt](city-destruction-vectors.txt) and
[city-destruction-map-vectors.txt](city-destruction-map-vectors.txt) cover
30 native surface removals: all ten standard cities under human masks 0, 16
and 255, with every saved unit homed to the target, the first unit changed to
type 9, synthetic incoming routes and wonder table. Original unit removal,
route compaction and map mutation helpers execute. Only map redraw/city-window
callbacks are replaced; the engine's represented identity/home, city flags,
routes, wonder slots, counts and all surface improvement/site/fertility/owner
arrays match. The map artifact records changed numeric fields, not map assets.
The original 45-offset neighborhood is retained as derived coordinates.

Interior dead city slots remain available for reuse. Last-slot extent
reduction is checked separately below. Non-surface refresh, sea-transport cargo,
unit-known-by fields and caller civilization elimination are outside this
comparison. The surface removal is now integrated into the city phase. These fixtures do not close #13's live-save acceptance gate.

The original caller `0x470aa5..0x470b69` visits a civ's cities in **descending**
city-index order for each map, unlike the older engine's ascending global loop.
The negative-box branch also reads food-route globals from the preceding totals
calculation; those transient inputs are explicit in the starvation vectors.
The controlled 50-turn sequence above supplies a different, explicitly stated
city schedule and does not claim original whole-round scheduling.

```sh
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/city_destruction_check.py --gold /tmp/city-destruction.json \
  --vectors /tmp/city-destruction-vectors.txt --map-vectors /tmp/city-destruction-map-vectors.txt
cmp docs/evidence/city-destruction.json /tmp/city-destruction.json
cmp docs/evidence/city-destruction-vectors.txt /tmp/city-destruction-vectors.txt
cmp docs/evidence/city-destruction-map-vectors.txt /tmp/city-destruction-map-vectors.txt
CIVTOT_HOME=/mnt/d/games/CIVTOT cargo test -p engine --test city_destruction_install
```


### End-of-city unrest and celebrations (#13/#86)

`engine::happiness::transition` now follows `0x4c998e`, which runs **after**
production and a second city-total refresh. [mood-transitions.json](mood-transitions.json)
and [mood-transition-vectors.txt](mood-transition-vectors.txt) retain 2,048
native cases with supplied city totals, covering unrest escalation/clearance,
income, celebration start/end, additional population and revolution requests.
Original Aqueduct/Sewer checks execute unchanged. Presentation, original AI
production selection and revolution dispatch are replaced at explicit call
boundaries; the latter two requests are recorded and checked, not performed.

The first unrest sets `0x4001`; continued unrest sets `0x2000`, and an already
set `0x2000` also sets `0x100000`. Balance clearing unrest removes `0x102001`.
A balanced city with positive net shields and nonnegative collected food
surplus sets `0x4000`; other non-rioting cases clear it. Taxes credit the
civ only outside anarchy and with nonnegative happy-minus-unhappy; barbarian
owner zero returns without processing. Human Democracy with continued unrest
requests revolution, except a scenario with parameter bit `0x10`; its civic
flags end with bit 0 (`0x1`) set around dispatch. Original AI build selection is
requested on new/continued unrest and when ending disorder.

Celebration requires zero unhappy, size >2, happy >= `(size+1)/2`, and a
non-anarchy government. The first qualifying turn sets flag `0x2`. On an
already-celebrating turn, Republic/Democracy with food strictly above citizens
and settlers' consumption gains an additional citizen if growth limits allow;
this does not reset the food box. A lost celebration clears flag `0x2`.
The controlled 500-transition city-sequence test uses this checked transition
helper too. `run_turn` integration, actual revolution/government mutation,
original AI production selection and live-save parity remain open. Runtime
now applies these arithmetic transitions in `run_city_phase`.

```sh
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/mood_transition_check.py --gold /tmp/mood-transitions.json \
  --vectors /tmp/mood-transition-vectors.txt
cmp docs/evidence/mood-transitions.json /tmp/mood-transitions.json
cmp docs/evidence/mood-transition-vectors.txt /tmp/mood-transition-vectors.txt
cargo test -p engine --lib original_mood_transitions
```


### City unit support and disband selection (#13)

`engine::support` follows `0x4c18af` support charging and `0x4c92a4` disband
selection. [support-disband.json](support-disband.json) and
[support-disband-vectors.txt](support-disband-vectors.txt) retain 1,026 native
synthetic surface contexts. Both the pure selector and the WorldState context
adapter match native removal order, final city flags and shield-box results.
The native check only replaces presentation and the deletion boundaries
(`0x56c652`/`0x56a8c2`), marking selected slots dead; support, eligibility,
distance comparisons and restart loops execute original bytes. Unit deletion
itself has the separate saved-state city-removal comparisons above.

A shield shortage selects the farthest live homed role <6 unit on the same
map, using original `0x566dad`; equal distances keep the first slot. The city
restarts its support scan after each removal. Original AI Republic/Democracy
can also disband during disorder when `(turn+unit_index)%8 == 0`; this path
excludes settlers, city tiles and fortresses within `0x566c40` distance three
of an own city. The unrest disband requires distance >0 and clears riot flags
`0x102001`. The two distance functions differ: `0x566c40` is wrapped
`(abs(dx)+abs(dy))/2`, while `0x566dad` uses the already decoded diamond metric.

Fundamentalism's exemption is type-table byte `+5 & 0x08`, which is Rules
unit flag `0x800` (the full flag word is already decoded at +4).
`WorldState::unit_support` now honors it. Exempt types still consume an ordinal
in the free-unit allowance; exemption is applied when charging that unit.
The selector takes pre-production shields/rows explicitly so new buildings or
advances cannot silently change the current pass's cached inputs.

The original contains a shield-refund branch after unrest selection, but the
containing-city lookup repeats the city-marker helper that normally excluded
that candidate. These vectors produce **zero refunds**. Dedicated cases with
city+fortress bits verify that bypassing the first city-owner check does not
invent a refund: the second lookup fails too. The native raw flags and branch
conditions are retained; positive refunds are not claimed proven here.
Sea cargo, unit-map retention, the game-options AI-stat branch and full city
phase integration remain outside these contexts.

```sh
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/support_disband_check.py --gold /tmp/support-disband.json \
  --vectors /tmp/support-disband-vectors.txt
cmp docs/evidence/support-disband.json /tmp/support-disband.json
cmp docs/evidence/support-disband-vectors.txt /tmp/support-disband-vectors.txt
cargo test -p engine --lib support::tests
```


### Forced human anarchy (#13/#86)

`WorldState::force_human_anarchy` follows the human zero-government dispatch
`0x52588e -> 0x52522f`. [anarchy.json](anarchy.json) and
[anarchy-vectors.txt](anarchy-vectors.txt) retain 49 native executions over
all seven civs and starting governments 0..6 on the standard reference world.
The human mask selects the target civ, incoming treaty words are supplied,
and all city queues start with literal unit 8. **No callees are replaced**:
original city total/worker refresh runs for every affected city. Rust matches
resulting government, all eight incoming treaty words and thirteen retained
fields for each of ten cities, including the build queue.

A changed government becomes Anarchy (0), clears bit `0x10` from incoming
treaties, and refreshes own live surface cities in ascending index order.
Queued unit 8 changes to literal unit 11 when outside Fundamentalism, matching
the original's Fanatics-to-Riflemen switch. Already-Anarchy does not refresh
cities or clear those treaties. The city unrest caller separately clears civic
flag `0x1` before dispatch and sets it after returning; the mood-transition
vectors cover that request/flag boundary.

This supports forced human Democracy collapse from city unrest. It does not
implement player-selected revolutions, non-surface refresh, the original AI's
diplomatic notification path, or later government selection. City-phase runtime integration now invokes this dispatch on a revolution request.

```sh
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/anarchy_check.py --gold /tmp/anarchy.json --vectors /tmp/anarchy-vectors.txt
cmp docs/evidence/anarchy.json /tmp/anarchy.json
cmp docs/evidence/anarchy-vectors.txt /tmp/anarchy-vectors.txt
CIVTOT_HOME=/mnt/d/games/CIVTOT cargo test -p engine --test anarchy_install
```


### Integrated surface city phase and controlled full-round replay (#13)

`WorldState::run_turn` now calls `run_city_phase` in civ order 0..7, descending
city slots within each civ, replacing the removed observed-series approximation.
It processes live surface cities even when their old stored food cache is zero.
Each city prepares growth/starvation from its starting signed box, recomputes
workers/totals, collects food, produces using the previous disorder flag,
enforces support/disband rules, recomputes totals again, resolves unrest and
celebration, collects science, and accounts for taxes/upkeep. A completed Library
therefore affects the same city's post-production science, and newly detected
unrest stops the **next** production phase. Forced human Democracy collapse
runs the checked Anarchy dispatch. Deleted cities leave observations and no
longer block founding or accept production orders.

[city-round.json](city-round.json) and [city-round-vectors.txt](city-round-vectors.txt)
repeat the controlled standard-world experiment in civ/descending-city order.
The install test now calls the actual `run_turn` for 50 rounds and matches all
500 city economic transitions plus end-of-civ treasury balances. Native
`0x4cb01d` and its gameplay callees execute; only open-city-window query and map
redraw boundaries are replaced. As above, the native settler-job-planner flag
`0x80000` remains in the artifact but is excluded from the economic comparison.
Barbarian level is zero in the headless comparison; the native probe invokes
city processing only. The original ascending-city sequence also still matches
through the same `run_city_phase` helper.

The round begins with empty transient food-route inputs; each totals pass
updates them, and the subsequent city's starvation branch reads those prior
inputs, as native `0x4c596b` does. This initialization, civ scheduling after
all player orders and turn-event dispatch remain headless engine policy.
Pollution/settler-job planning and their CRT draws, original AI build selection,
non-surface cities and sea cargo are not ported by this change. Surface landmass flag lifecycle is now checked below. City-slot
reuse/truncation is checked below. These scoped checks are not a live Wine/autosave comparison, complete
original-round RNG replay, or completion of #13's live-save gate.

```sh
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/city_phase_check.py --round-order --gold /tmp/city-round.json \
  --vectors /tmp/city-round-vectors.txt
cmp docs/evidence/city-round.json /tmp/city-round.json
cmp docs/evidence/city-round-vectors.txt /tmp/city-round-vectors.txt
CIVTOT_HOME=/mnt/d/games/CIVTOT cargo test -p engine --test city_phase_install
```


### City-slot allocation and deletion extent (v0.8.0)

`tools/city_allocation_check.py` executes the original allocation fragment
`0x432927..0x4329de`, stopping before city initialization or capacity UI.
[city-allocation.json](city-allocation.json) and
[city-allocation-vectors.txt](city-allocation-vectors.txt) contain 46 allocation
cases: reuse the first dead slot regardless of current extent, otherwise append
only below 255 slots. Human-mask selection changes notification, not admission.
Cases include imported extents above the classic limit; an existing dead slot
can still be reused. Patched ToTPP capacity is outside this proof.

Fifteen original `0x4348d7..0x4348ef` cases show that destruction reduces the
extent by exactly one if the deleted city was last. It does not trim earlier
dead records. The engine's city vector now follows that extent, preserving
interior indices; barbarian city-slot sampling therefore uses the reduced count.
The older full destruction fixture captures inactive storage too, which its
comparison reconstructs outside the active extent.

Four `0x432ad7..0x432af6` cases confirm full-width identity writes and increments
including 65,536 and 4,294,967,294. As for unit identities, the engine retains
imported IDs and initializes its creation counter above their maximum, rather
than emulating loader renumbering. That counter survives deletion and is part
of the deterministic fingerprint.

Capacity failure leaves a founding settler intact. The native hut caller at
`0x54af7b` stores the allocation result and proceeds directly to record writes
without checking for -1 (`0x54afa5..0x54afb2` in the later-year branch). The
engine returns `HutOutcome::Nothing` instead of writing outside the city table;
this deliberate safety behavior is not claimed as native outcome parity.

```sh
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/city_allocation_check.py --gold /tmp/city-allocation.json \
  --vectors /tmp/city-allocation-vectors.txt
cmp docs/evidence/city-allocation.json /tmp/city-allocation.json
cmp docs/evidence/city-allocation-vectors.txt /tmp/city-allocation-vectors.txt
```


### AI v1 campaign development (#82)

`crates/engine/examples/ai_curve.rs` exports numeric metrics without original
records or names. [ai-v1-engine-curve.tsv](ai-v1-engine-curve.tsv) covers 50
rounds from the archived standard campaign's turn-4 `turn01.sav`, seed 3,
with AI v1 controlling every living player. The original difficulty/mechanics
mask are retained. [ai-v1-oracle-curve.tsv](ai-v1-oracle-curve.tsv) measures
archived original saves by their decoded counters; harvest labels are not
turn numbers. Repeated turn-4 labels and the final turn-43 save are deduplicated.
[ai-v1-curves.json](ai-v1-curves.json) retains source hashes and aggregates;
[ai-v1-power.svg](ai-v1-power.svg) plots the verified power formula over time.

The common initial totals are 4 cities, 4 population, 7 units, 5 acquired
advances and power 47. At turn 20, original/AI-v1 power is 108/179; at turn 43,
267/287. At that final comparable turn both have 10 cities and 27 advances;
original/AI-v1 population is 21/24, units 37/25 and improvements 5/12. AI v1
expands faster early and invests more in buildings, with fewer military units.
This supports plausible campaign growth under #82; original human actions
and policy differ, so neither score equality nor original-like AI is claimed.
AI v1 reaches turn 54 with 11 cities, 27 population and power 328. The
install-backed AI test independently runs 50 rounds twice, checks exact logs,
replays them, and requires actual founding, investments and research gains.

```sh
CIVTOT_HOME=/mnt/d/games/CIVTOT cargo run -q -p engine --example ai_curve -- \
  --saved /home/diego/review-tmp/oracle-runs/std1
CIVTOT_HOME=/mnt/d/games/CIVTOT cargo run -q -p engine --example ai_curve -- \
  /home/diego/review-tmp/oracle-runs/std1/turn01.sav 50 3
```

The economic view exposes only this civ's own cities. A regression checks
that observation leaves fingerprints unchanged and foreign private building
and finance caches cannot alter the own-city economic view. Normal unit
production now applies the already decoded @UNITS_ADVANCED civilization mask,
as obsolete production and Leonardo upgrades already do.


### Live city-save comparison (#13)

The earlier native-only city fixtures are now accompanied by
[city-live.json](city-live.json) and [city-live-vectors.txt](city-live-vectors.txt):
50 consecutive controlled surface city rounds in the owned Wine game, with
all ten reference cities retained between rounds. The game itself writes each
save through its ordinary save dialog; derived vectors are decoded from those
files, rather than copied directly from live RAM. The report records every
save's SHA-256, the untouched reference save and runtime executable hashes.
Raw saves stay under `review-tmp/oracle-runs/v08-city-series`, outside Git.

`full_round_city_math_matches_fifty_live_original_saves` compares actual
`run_turn` output against all 500 saved city transitions: population, signed
food/shield boxes, workers/specialists, flags (apart from the documented
planner bit 0x80000), route cache, science/taxes/trade, food/shields, mood,
and each civ's final treasury. All match. This supplies the controlled
50-save city-math gate; independent growth/debt, support, mood and production
fixtures cover additional branches not exercised by this quiet campaign.

Fixture inputs match `city_phase_check.py --round-order`: existing standard
terrain/positions/governments, all-human mechanics mask, Chieftain, size-one
zero-box start, center workers, saved Palaces only, Capitalization, 100% tax,
and unhomed units. Process civs in order and city slots descending. Only
`0x411c50` (open city-window query) and `0x467c89` (map redraw) are replaced
while calling the live city routine; original gameplay callees run. The helper
suspends the GUI thread during each isolated call and restores code/control
inputs before saving. No original AI policy, pollution, ordinary unit builds,
calendar advancement or complete round RNG sequence is being compared here.
The saved turn counter advances once per controlled city round.

```sh
# Owned, backed-up rig only; read tools/oracle/README.md and docs/lessons.md.
/home/diego/review-tmp/llvm-mingw/bin/clang --target=x86_64-pc-windows-gnu \
  -O2 -o /tmp/totengine-call-game.exe tools/oracle/call_game.c -luser32
CIVTOT_HOME=/mnt/d/games/CIVTOT DISPLAY=:99 \
  WINEPREFIX=/home/diego/.wine_civ2tot tools/oracle/load_save.sh original/ph_auto2.sav
CIVTOT_HOME=/mnt/d/games/CIVTOT DISPLAY=:99 \
  WINEPREFIX=/home/diego/.wine_civ2tot python3 tools/oracle/city_series.py \
  --helper /tmp/totengine-call-game.exe \
  --output /home/diego/review-tmp/oracle-runs/v08-city-series \
  --gold /tmp/city-live.json --vectors /tmp/city-live-vectors.txt
```

The tool reserves numeric save labels 8701 onward, refuses occupied labels,
and removes only its just-created install-side numeric save after copying it
outside the install. Stop the owned rig and restore its initial SAV/INI backup
after experiments, as the launcher itself can update INI state.


### Combat defence context and stable casualty slots (v0.8.0)

`tools/combat_defense_check.py` executes the complete original `0x53f67a`,
including its original city, building and wonder lookups, without replaced
callees. [combat-defense.json](combat-defense.json) and
[combat-defense-vectors.txt](combat-defense-vectors.txt) retain 1,536 derived
contexts across domains, terrain/river, veterans, fortification, fortress,
City Walls and attacker wall-negation/interception/missile flags. The pure
Rust strength calculation and its World/Rules adapter both match every case.
In-city intercepting air defenders gain the original multiplier against air
attackers, except one-shot missiles; fortified/fortress/walls precedence is
retained. This supersedes the earlier claim that the city/air branch was absent.

Combat now reads actual fortress and City Walls state and Palace protection,
and uses globally active wonders rather than raw ownership. Dead cities are
excluded from lookups and city counts. The ship-selection branch reads the
actual building 27 (SAM Missile Battery; the earlier text called it SDI).
The later combat-only SAM/SDI/coastal-fortress strength and naval-firepower
branches remain outside this defence-helper proof.

`tools/combat_land_check.py` executes `0x5416fa..0x5417bc` and
`0x541cf4..0x541d82` with original movement, maximum-HP and wonder helpers,
without replacements. [combat-land.json](combat-land.json) and
[combat-land-vectors.txt](combat-land-vectors.txt) retain 1,024 cases.
The pikemen flag adds half defence only against a land attacker with exactly
two movement points **and 10 maximum HP**, before barbarian reductions.
Barbarian type 4 always has halved defence (minimum 1); type 5 does too unless
any human-mask civilization knows advance 39 (Iron Working). Synthetic World
battles additionally check four dedicated native contexts through `attack`.

Casualties now retain a dead unit slot instead of shifting later indices;
existing native-checked land removal clears empty tile markers and compacts
remaining stack positions. Removed coordinates use the documented -100/-200
sentinels. The real FallRome battle-2 integration still matches A/D, rounds,
damage and CRT end state, while checking that both the survivor and casualty
keep their original slots. Movement and defender selection ignore dead units.
The campaign curve was regenerated after this identity fix; its last turn
has one fewer newly founded city, while the turn-43 comparison is unchanged.

```sh
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/combat_defense_check.py --gold /tmp/combat-defense.json \
  --vectors /tmp/combat-defense-vectors.txt
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/combat_land_check.py --gold /tmp/combat-land.json \
  --vectors /tmp/combat-land-vectors.txt
```


### Sustained live barbarian arrivals (#83)

[barbarian-live.json](barbarian-live.json) and
[barbarian-live-vectors.txt](barbarian-live-vectors.txt) record 64 consecutive
arrival phases (turns 128..191), a continuous mechanics CRT seeded to 1, and
200 actual new units in the owned Wine game. Cities and created units persist
between phases. `same_seed_arrivals_match_sixty_four_live_original_phases`
starts from the same reference save and matches every slot/type/position,
orders, veteran flag, and per-phase final CRT; final state is 3,502,493,852.
Native creation, reused dead slots, stack insertion and order-setting execute;
the earlier 1,024 synthetic vectors stopped at a successful allocation boundary.

This caught an import error hidden by the uniform synthetic maps: the sea
admission check at `0x46e8ce` reads **landmass-table word +4** (`0x61bcac`),
whereas boot had supplied word +2 (the separately decoded start-placement
input). `MapState::landmass_values` now imports +4, and the complete 64-phase
comparison passes, including all naval arrivals. The deterministic fingerprints
are intentionally regenerated for the corrected imported field.

The fixture fixes level 3, difficulty 4, human mask 254 (all player civs;
barbarians excluded), treasury 32000 for civs 1..7 and the original quiet
notification context `0x6031b8=1`. Terrain, technologies, governments, city
records and existing units come from the reference save. It executes only
arrivals; no player/AI, city, pollution, research, calendar or full-round RNG
parity is claimed. The preceding phase's final CRT is restored before the next
phase, excluding unrelated repaint draws between calls. Original network/UI
callbacks `0x569c88`, `0x40cef0` and `0x467d19` are suppressed; text helper
`0x42095e`, allocation and order-setting execute unchanged.

`tools/oracle/call_gui.c` temporarily redirects the suspended GUI thread to a
wrapper in the dead startup cave, preserving registers/flags and resuming at
the exact interrupted instruction. It restores its wrapper only after the
thread has left the cave. This avoids a suspended-GUI loader-lock deadlock in
the worker-thread calling method. The native quiet-context switch suppresses
visible-arrival alerts (the diagnostic 64-phase attempt blocked at turn 185
without it). A timeout stops the owned game before any cave restoration.

```sh
/home/diego/review-tmp/llvm-mingw/bin/clang --target=i686-pc-windows-gnu \
  -O2 -o /tmp/totengine-call-gui.exe tools/oracle/call_gui.c -luser32
CIVTOT_HOME=/mnt/d/games/CIVTOT DISPLAY=:99 \
  WINEPREFIX=/home/diego/.wine_civ2tot tools/oracle/load_save.sh original/ph_auto2.sav
CIVTOT_HOME=/mnt/d/games/CIVTOT DISPLAY=:99 \
  WINEPREFIX=/home/diego/.wine_civ2tot python3 tools/oracle/barbarian_series.py \
  --helper /tmp/totengine-call-gui.exe --turns 64 --seed 1 \
  --gold /tmp/barbarian-live.json --vectors /tmp/barbarian-live-vectors.txt
```

Both live acceptance runs are finished. The owned game was stopped, all 50
numeric city-test saves were removed after checking their saved hashes, and
the initial 25 SAV/INI backup hashes were reverified. No original assets enter
Git. City economic observations also now resolve prerequisite choices once
per civ observation instead of rebuilding the tech tree for every city/item;
the existing 100-turn AI v0 and two-run/replay AI v1 checks pass in about 30 s
combined on this machine, versus about 365 s before that equivalent refactor.

### Positive route income with final city corruption (#85/#86)

[city-route-totals.json](city-route-totals.json) and
[city-route-total-vectors.txt](city-route-total-vectors.txt) execute the complete
original `0x4c45ff`, with original callees and no replacements. Ten standard
cities are tested under seven governments and human masks 0/254. Each has
three positive commodity routes (count @60, commodities @67, partner slots
@70), with controlled partner trade caches. All 140 cases have positive route
arrows and 40 also have positive final corruption. The World adapter matches
all 16 retained city totals, including route arrows, final corruption, mood,
shields, taxes, luxuries and science. This complements the earlier isolated
route arithmetic and checks their composition. Route creation/delivery and
non-surface maps remain outside #85's recurring-income implementation.

```sh
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/city_route_totals_check.py --gold /tmp/city-route-totals.json \
  --vectors /tmp/city-route-total-vectors.txt
```

### Wonder integration audit (#88)

Wonder IDs follow physical Rules row order: **19 is Statue of Liberty; 20 is
Eiffel Tower**. Earlier prose mislabeled the native-checked slot-20 reputation
and attitude completion effect; code and native vectors already used 20.
`diplomatic-wonders.json` and its probe now use the correct name.

[Library vectors](library-vectors.txt) and [library.json](library.json) retain
512 executions of the original discovery follow-on slice
`0x49ebb5..0x49ec5c`, including original owner/expiry/knowledge/group helpers.
There are 29 grant requests. A newly granted advance propagates to an active
Great Library owner when at least two player civilizations know it, the owner
does not, and its technology-group flag is not 2. This is a discovery-triggered
follow-on, not a round polling policy. Reserved Future Technology 89 does not
qualify. The Rust test checks every recipient and exercises those 29 grants
through the shared discovery path (which also runs Leonardo).

[wonder-yields.json](wonder-yields.json) and
[wonder-yield-vectors.txt](wonder-yield-vectors.txt) record 168 native tile-yield
triples: 21 radius tiles under eight combinations of Colossus/King Richard
ownership and global foreign-civ expiry. Original `0x4c22a7` and its callees
execute; all triples match the World/Rules adapter. Finance, tile bonuses,
Hoover power, veteran eligibility and combat now use the common active-wonder
lookup instead of treating a completed but obsolete wonder as active.

[production-veterans.json](production-veterans.json) and
[production-veteran-vectors.txt](production-veteran-vectors.txt) retain 512 native
eligibility checks at `0x4c6728..0x4c68aa`, with original improvement/wonder
helpers. One-shot flag `0x1000` prevents automatic veterans even with the
corresponding Barracks/Port/Airport/Sun Tzu/Lighthouse context. The unit
production adapter matches every vector.

[movement-allowance.json](movement-allowance.json) and
[movement-allowance-vectors.txt](movement-allowance-vectors.txt) retain 1,024
complete `0x568da2` calls without replacement callees. Sea movement gains one
point from Nuclear Power, two from Magellan and one from active Lighthouse
unless type flag 32 excludes it. Persistent damage scales non-air movement,
rounding upward to a whole point, with minima of one point on land/two at sea;
zero-base-movement types stay immobile. The engine and its World/Rules adapter
match all cases with the engine's three-thirds quantum. Turn resets, created
units, discovery/Leonardo changes and post-combat remaining movement use this
allowance. The pikemen check uses the current effective maximum movement.
Custom ToTPP movement quanta, transport and trireme sinking remain separate.

The shipped effect inventory is: city food storage, tile food/shields/trade,
happiness/disorder, science multipliers, upkeep exemptions, defence, automatic
veterans, naval movement, Library discoveries and the decoded Darwin,
Leonardo, Eiffel, Manhattan and surface Apollo one-offs. Marco Polo, Statue of
Liberty and UN effects require diplomacy/government-selection workflows that
this headless API does not yet expose. They can be built and registered, but
those interactions are not claimed here. Pollution, space victory and later
naval/air combat-only modifiers are also outside this release's effect proof.

```sh
export CIVTOT_HOME=/mnt/d/games/CIVTOT
export PYTHONPATH=/tmp/totengine-research-deps
python3 tools/library_check.py --gold /tmp/library.json --vectors /tmp/library-vectors.txt
python3 tools/wonder_yield_check.py --gold /tmp/wonder-yields.json --vectors /tmp/wonder-yield-vectors.txt
python3 tools/veteran_check.py --gold /tmp/production-veterans.json --vectors /tmp/production-veteran-vectors.txt
python3 tools/movement_allowance_check.py --gold /tmp/movement-allowance.json --vectors /tmp/movement-allowance-vectors.txt
```

### Hut-facing landmass strategy flag lifecycle (#79)

`engine::statistics::refresh_landmass_flags` now refreshes the flags consumed
by hut selection instead of retaining stale imported flags. Original military
counts include admitted types with role <=4; strength sums their signed attack
bytes (all admitted roles), excluding types with attack >=99. Both sums wrap
as words. Surface landmasses 1..254 are compared using original city counts,
treaty and attitude gates. Bits 1/2 describe non-allied city presence and bits
4/8 military presence, split by treaty bit 4. Bit 16 uses the original hostile
or treaty-32 strength/city test. Landmasses 0/255 and barbarian flags stay clear.

[landmass-flags.json](landmass-flags.json) and
[landmass-flags-vectors.txt](landmass-flags-vectors.txt) retain 512 native
contexts. `tools/landmass_flags_check.py` executes admitted-unit accumulation
at `0x506e5e..0x506ec7` and `0x50733d..0x5073f7`, then flag calculation at
`0x507a87..0x507ff4` and the bit-16 write at `0x508275..0x5082be`.
Original diplomacy/attitude callees execute, with no replacements. The skipped
intervening calculation changes the separate landmass build-strategy byte,
not these flags. The Rust World adapter builds units/cities from each numeric
fixture and matches every flag, also excluding deleted/invalid unit slots.

The headless loop refreshes city caches and flags at round close. It does not
claim the original's complete per-civ scheduling or AI planning, and the
surface-only unit model cannot resolve non-surface military caches. Earlier
sections identifying this lifecycle as open are superseded for surface flags.

```sh
CIVTOT_HOME=/mnt/d/games/CIVTOT PYTHONPATH=/tmp/totengine-research-deps \
  python3 tools/landmass_flags_check.py --gold /tmp/landmass-flags.json \
  --vectors /tmp/landmass-flags-vectors.txt
```

### Engine-designed world generator prototype (#90, v0.9)

`engine::worldgen` is **our algorithm**, not a decoded original generator.
It uses integer smoothed fields and ranked sea level, latitude/moisture
biomes and sea-rooted drainage; east/west neighbors wrap. `totctl
worldgen` exports the proven stock `.MP` slot shape for the existing Rust and
Python decoders. No unknown original metadata or game assets are copied.
Successful decoder round trips do not claim original-game import.

[worldgen-v1.json](worldgen-v1.json) records the first calibration data,
original source hashes and limitations. [worldgen-v1-metrics.tsv](worldgen-v1-metrics.tsv)
contains raw numeric counts for 16 engine seeds and the retained RG-2 original
worlds pl2/pl3/pl4. Only pl2/pl3 share default world parameters; pl4 is a
changed-settings control and is not pooled into the default reference.
These original inputs were already produced by `seeded_start.sh` and used
for start-placement evidence; no new live oracle run is claimed.

The default land target is 47%, versus original pl2/pl3 mean 47.225%.
**Statistical acceptance is not met:** prototype tundra/glacier cover
13.8063% of real tiles versus original 6.025%, rivers cover 0.4188% versus
4.025%, and hills/mountains are too sparse. The original baseline also needs
more controlled seeds. [Seed 7 render](worldgen-v1-seed7.png) uses Original
Rules terrain names/colors and was inspected: separated land regions, polar
bands and river marks are present, with the above calibration gaps visible.

The generator's dimensions and land settings are validated. Regressions
check repeatability/different seeds, every exported terrain/river cell via
the actual `.MP` decoder, invalid input rejection and CLI refusal to overwrite
an existing map. These are format/determinism checks, not original generator
parity or a complete campaign. Generated maps do not yet initialize a game.

```sh
cargo run -q -p totctl -- worldgen /tmp/generated-seed7.mp --seed 7
python3 tools/totctl.py map /tmp/generated-seed7.mp \
  --rules "$CIVTOT_HOME/Original/Rules.txt" -o /tmp/generated-seed7.png --scale 6
# RG2_RUNS points to retained original seeded-start captures, outside Git:
cargo run -q -p engine --example worldgen_metrics -- \
  "pl2=$RG2_RUNS/pl2/Original_Ph_Auto.SAV" \
  "pl3=$RG2_RUNS/pl3/Original_Ph_Auto.SAV" \
  "pl4=$RG2_RUNS/pl4/Original_Ph_Auto.SAV"
```

#### Algorithm v2: climate and basin drainage

The v1 artifacts above are historical measurements of commit `245d70f`.
Current exports identify algorithm **v2**, whose deterministic bytes differ.
V2 narrows polar bands and adjusts moisture/relief thresholds. It floods
outward from every ocean tile, assigning each discovered land tile an
already visited downstream parent at its lowest spill height. Reverse
traversal accumulates integer rain (1 + moisture/250); flow of at least 12
marks a river. Parent order prevents drainage cycles, and increasing flow
keeps marked channels continuous to the sea. These are engine policies.

[V2 numeric counts](worldgen-v2-metrics.tsv) contain the same three retained
original worlds, 16 calibration seeds (0–15), and 64 fresh engine seeds
(16–79). Original source hashes were rechecked against
[the v1 manifest](worldgen-v1.json); no new Wine run was performed.
[The reproducible report](worldgen-v2.json) separates the cohorts, keeps pl4
as a changed-settings control, and records means, sample standard deviations,
ranges and mean differences. Percentages use all 2,000 real tiles as the
denominator and give each map equal weight.

| Measure | Original pl2/pl3 mean | V2 calibration mean | V2 fresh-seed mean |
| --- | ---: | ---: | ---: |
| Land | 47.225% | 47.000% | 47.000% |
| Plains | 8.750% | 8.3688% | 7.8742% |
| Hills | 9.625% | 8.950% | 9.350% |
| Mountains | 4.050% | 4.350% | 4.4102% |
| Tundra | 2.800% | 2.8875% | 2.7617% |
| Glacier | 3.225% | 3.0812% | 2.7797% |
| Rivers | 4.025% | 3.7625% | 3.4312% |

The remaining biome means are in the report; all fresh-seed biome means
are within 0.9 percentage points of the small original reference. Individual
maps vary substantially (e.g. seed 79 has 8.55% jungle), so matching means
alone does not establish statistical parity. **#90 acceptance remains open:**
the two original maps are still insufficient, and spatial coherence has not
yet been measured against a broader matched-settings original cohort.

Inspected renders: [seed 7](worldgen-v2-seed7.png),
[fresh seed 31](worldgen-v2-seed31.png), and
[fresh seed 79](worldgen-v2-seed79.png). All show separated land regions and
climate/relief patches; seed 79 is visibly wetter with more tropical terrain.
The decoder's alternating padding slots remain visible in these diagnostic
renders. Regression checks independently flood river cells from the ocean
to reject isolated inland networks, including an artificial depression with
a flat rim and generated maps up to 256×256 logical dimensions at 80% land.

```sh
cargo run -q -p engine --example worldgen_metrics -- --seeds 80 \
  "pl2=$RG2_RUNS/pl2/Original_Ph_Auto.SAV" \
  "pl3=$RG2_RUNS/pl3/Original_Ph_Auto.SAV" \
  "pl4=$RG2_RUNS/pl4/Original_Ph_Auto.SAV" \
  > /tmp/worldgen-v2-metrics.tsv
cmp /tmp/worldgen-v2-metrics.tsv docs/evidence/worldgen-v2-metrics.tsv
python3 tools/worldgen_compare.py docs/evidence/worldgen-v2-metrics.tsv \
  --reference pl2 --reference pl3 > /tmp/worldgen-v2.json
cmp /tmp/worldgen-v2.json docs/evidence/worldgen-v2.json
python3 -m unittest discover -s tools -p 'test_worldgen_compare.py'
# Render each of 7, 31, 79 with the command above and these Rules:
# Original/Rules.txt SHA256 640ae61d6c6e871131a01647099983913e1a1bb98ce1e6503be5b9bc9da8adef
# Generated MP SHA256s:
# 7: ff08e7103e805883c0897141f293f5fbf1f78c8391a65b619acea7d1617c12bf
# 31: 1dff44f6c2d4cb2653e65359acf5c8e0880d093fbfb34090fc81723c090b8ebb
# 79: 0577a847e24bb982db3a02b58fa2531b8c5857660200655baf2b4d46dced342f
```

### Fresh generated-world campaign (#91, v0.9)

`engine::newgame` constructs a world directly from Rules and generator v2;
no stock save, city, unit or cache is imported. Setup resources, synthetic
civ names, separate land/sea components and food-based fertility estimates
are explicit initialization policy v1. The decoded start sampler operates
on those inputs. Power/statistics caches derive from the fresh state.
[Usage, defaults and limits](../generated-campaign.md) give the complete
contract; original new-game initialization parity is not claimed.

[generated-campaign-v1.json](generated-campaign-v1.json) records Rules hashes,
setup and validation context. [The full authored-rules checkpoint golden](generated-campaign-v1.ctc)
is regenerated for engine v0.11.0; only the exact-version header changes from
the v0.10 fixture, with metadata hash/version refreshed. It still contains every
order and round fingerprint for 100 rounds of AI v1, including
the final fingerprint `b22e4340b6ab8bf6`. It reaches 20 cities, 27 population,
28 units and 35 acquired advances. [The authored trajectory](generated-authored-campaign.tsv)
and [Original Rules trajectory](generated-standard-campaign.tsv) contain
numeric-only round data. With the user's Original Rules, the same setup
reaches 16 cities, 39 population, 53 units and 35 acquired advances, fingerprint
`76814781c4e92589`. These trajectories compare two Rules inputs on our world;
they are not comparisons against original-game AI.

The authored test Rules are original project data with synthetic balance;
no original Rules or save bytes are committed. Checkpoints are explicitly
engine replay files, not native `.SAV` exports. Restore replays AI from setup
and requires identical transcript/state, restoring private AI state as well
as the world. The 32-seed, 100-round suite compares uninterrupted runs with
37 rounds + checkpoint restore + 63 rounds and independently reapplies every
logged order through the generated-game replay API. The install-backed
Original Rules test verifies the same continuation and replay properties.

```sh
cargo test -p engine --test generated_campaign --release -- --include-ignored
cargo test -p totctl --test campaign
CIVTOT_HOME=/path/to/CIVTOT cargo test -p engine --test generated_campaign_install
cargo run -q -p engine --example generated_campaign_metrics -- \
  crates/engine/tests/fixtures/campaign-rules.txt 7 100
cargo run -q -p engine --example generated_campaign_metrics -- \
  "$CIVTOT_HOME/Original/Rules.txt" 7 100
```

The generated checkpoint's engine package version is part of its identity.
Intentional engine/version changes require regenerating the authored golden
and its hash; the per-round fingerprints continue to verify mechanics state.

#### Expanded original-map comparison and start inputs (#90)

[The frozen protocol](worldgen-comparison-protocol.json) tests generator v2
without further tuning: eight new original worlds (varying the declared CRT
seeds, fixing NR/resource seed and five world parameters) plus retained pl2/pl3,
versus the 64 engine seeds 16–79 excluded from tuning. All are the 80×50 logical
Original profile. Pl4 remains an excluded changed-settings control.
Live captures use the standard Wine/ToTPP oracle build; the manifest records
both its executable hash and the unpatched executable's identity.
[Source identities and native region counts](worldgen-original-cohort.json)
record ten distinct source hashes, matching turn-1/difficulty/rival/barbarian/
family/resource-seed/advance-count fields, and all new save terrain/river cells
equal to their pre-AI RAM capture. The install's 25 save/configuration hashes
match the pre-capture backup; the game was stopped before verification.
Only numeric summaries/hashes and our renders are committed.

[Expanded raw counts](worldgen-v2-expanded-metrics.tsv) and
[the complete comparison](worldgen-v2-expanded.json) retain ranges, sample
deviations, calibration/holdout separation and every coverage-budget check.
**The predeclared mean-coverage gate passes.** Land means are original 48.175%
versus engine 47.000%; river means 3.635% versus 3.4312%. All ten land biomes
pass `max(0.75 percentage points, 25% of original mean)`. Land's absolute
budget is 3 percentage points and rivers' 1.5 points. This is practical
coverage parity under one controlled profile, not a claim of identical
distributions or a recreation of the original algorithm. Renders inspected:
engine seeds 7/31/79 above and original captures [0](worldgen-original-0.png)
and [7](worldgen-original-7.png). The original diagnostic uses stored pixels;
MP diagnostics include alternating padding slots, so pixel scales differ.

**Spatial parity is not met or claimed.** Directed neighboring land/water
transitions average 27.5906% in the original cohort versus 10.1172% for the
engine holdout. Same-terrain edges are 50.6633% versus 69.0932%; largest land
component shares are 89.1324% versus 80.8910%; component counts average 7.2
versus 3.4219. Our coasts and biome patches are smoother, and the component
distribution differs. These diagnostics were declared separately from #90's
written land/biome-coverage acceptance. They are a material limit for review,
not an additional passed claim. Other settings/sizes and original MP import
remain outside the comparison.

Across all **89 occupied native regions** in the eight new pre-AI captures,
table words +0/+2/+4 equal respectively non-ocean count, plains-plus-grassland
count, and ocean count of tiles carrying that region ID. Every comparison
passes. Some native IDs associate land with coastal water. The fresh-game
constructor uses the proven plains/grassland input meaning, with its own
separate connected land/sea region policy. Original Rules have 100 advance
rows but 89 enabled rows (neither prerequisite `no`); the saved count is 89
in every cohort member. These are measured input meanings, not claims that
the original region-assignment or fertility algorithms have been decoded.

Reproduction (`CAPTURES` and `RG2_RUNS` point outside Git):

```sh
# Captures use seeded_start.sh + DUMP_INTRO=1, the protocol's seeds/settings,
# and DIALOG_GUARD_DIR outside Git; stop the game and verify install restoration.
python3 tools/worldgen_capture_manifest.py "$CAPTURES" "$RG2_RUNS" \
  > /tmp/worldgen-original-cohort.json
cmp /tmp/worldgen-original-cohort.json docs/evidence/worldgen-original-cohort.json
# worldgen_metrics --spatial --seeds 80, with label=save arguments from the
# manifest in source order, plus pl4 as a control, re-derives the retained TSV.
python3 tools/worldgen_compare.py docs/evidence/worldgen-v2-expanded-metrics.tsv \
  --reference original-0 --reference original-1 --reference original-2 \
  --reference original-3 --reference original-4 --reference original-5 \
  --reference original-6 --reference original-7 --reference pl2 --reference pl3 \
  --protocol docs/evidence/worldgen-comparison-protocol.json \
  > /tmp/worldgen-v2-expanded.json
cmp /tmp/worldgen-v2-expanded.json docs/evidence/worldgen-v2-expanded.json
```

## Original research selection (v0.10, #106)

`ai-research-choice.json` records 512 executions of the original selector
`0x49f98b` and CRT `0x5b11d0`. Winner, final CRT state and draw count agree
with the decoded implementation. The probe supplies availability (`0x49ecb6`),
group permission (`0x4a2098`) and context-dependent advance value (`0x49c95d`);
these are explicit replaced decision boundaries, not a full original AI check.
The native two-`no` prerequisite rejection executes unchanged.

AI selection assigns each admitted advance `rand() % value` for value > 1,
otherwise zero without drawing; the earliest maximal score wins. The human
advisor branch adds `rand() % 3 - 1` and above Chieftain omits later admitted
IDs congruent to the acquired count modulo three. All six difficulties, empty,
singleton, zero/tied candidates, mixed gates and signed arithmetic boundaries
are checked. `crates/engine/tests/fixtures/ai-research-choice.txt` retains 128
native outputs; the asset-free Rust test compares winner and randomness.
`engine::ai_research` is not yet wired into campaigns: full native advance
valuation remains the next step (decision 0007).

Reproduce with Unicorn installed outside the repository:

```sh
CIVTOT_HOME=/path/to/CIVTOT PYTHONPATH=/path/to/unicorn:tools \
  python3 tools/ai_research_choice_check.py --vectors /tmp/ai-research-choice.txt \
  > /tmp/ai-research-choice.json
cmp /tmp/ai-research-choice.json docs/evidence/ai-research-choice.json
cmp /tmp/ai-research-choice.txt crates/engine/tests/fixtures/ai-research-choice.txt
cargo test -p engine --test ai_research_choice
```

## Complete original research valuation (v0.10, #106)

`ai-research-value.json` records **12,800 advance valuations and 128 complete
research selections** from authored numeric Rules/cache contexts. The probe
`tools/ai_research_value_check.py` executes 0x49c95d and 0x49f98b with **no
replacement callees**. Native dependency traversal (0x49c8d4), packed knowledge
(0x49c820), wonder expiry/ownership (0x4449b0/0x444a28/0x444a61), eligibility,
group gates and CRT all execute original bytes. The retained numeric fixture
`crates/engine/tests/fixtures/ai-research-value.txt` contains no original names,
assets, executable bytes or saves. The asset-free Rust regression compares all
values, complete-choice winners and final CRT states. Eligibility bits supplied
to the Rust selector are native outputs; this does not prove a future adapter.

The value routine uses signed byte Rules values and leader modifiers, including
an effective-modifier reduction for stronger human civilizations with treaty
bit 8192 and shared city landmasses. It adds priorities from prerequisite chains
of unit types 32/33/35/43, the sea-unit count divided by four for improvement 34's
advance, city growth limits for improvements 9/23, active wonder expiry and
human ownership of nuclear/space technology. It applies 21 leader-specific
cases, an unowned-advance bonus and penalties for known direct dependents, then
clamps to at least one. Future Tech (89) never passes the native packed-knowledge
helper, even if its bit is set. The supplied global owner-mask cache is kept
consistent with packed knowledge; stale-cache divergence is outside this test.

The signed sea-unit count is civ +114 (`0x5f6912 +3348*civ`): the statistics
writer at 0x5072ba..0x5072f7 increments it for unit domain 2, whose Rules field
mapping is separately checked by the existing availability/movement probes.
The growth check ignores deleted city slots and uses signed city size.

At 0x49cc67 the inner foreign-civ loop reads the **own** landmass byte again;
the surrounding own-presence guard makes the "isolated" flag unreachable.
The port retains this behavior. It does not substitute foreign presence based
on an interpretation of the apparent intent. Dependency traversal is bounded
for malformed cyclic Rules; comparisons use acyclic trees. All 21 leader cases,
human/AI masks, signed modifiers, active/expired wonders, city deletion and
landmass IDs 1/2/254/255 are exercised across 128 contexts. Selection contexts
span all six difficulties; valuation itself does not read difficulty directly.

The kernel is not yet a campaign policy. Its native context includes foreign
information which cannot silently enter the fog-limited player observation;
Rules/world adapters and the policy/checkpoint identity remain open (decision
0007). No live original-AI round or whole-campaign parity is claimed.

```sh
CIVTOT_HOME=/path/to/CIVTOT PYTHONPATH=/path/to/unicorn:tools \
  python3 tools/ai_research_value_check.py --vectors /tmp/ai-research-value.txt \
  > /tmp/ai-research-value.json
cmp /tmp/ai-research-value.json docs/evidence/ai-research-value.json
cmp /tmp/ai-research-value.txt crates/engine/tests/fixtures/ai-research-value.txt
cargo test -p engine --test ai_research_value
```

## Research Rules/world adapters (v0.10, #106)

`ai-research-adapter.json` records 1,400 native advance scores, 1,400 eligibility
results and 84 complete research choices on the retained Original and FallRome
positions (seven civs, six difficulties). The install-backed Rust test decodes
these saves, parses their own Rules, constructs `ValueRules`/`ValueContext`, and
checks scores, gates, choices, final CRT state and unchanged world checksum.
Native scoring, eligibility, group and selection routines execute unchanged.
The numeric fixture contains only paths and derived numbers, with no assets.

The context uses the flags word **file @660 / native 0x607640**, not the distinct
options word at file @652. The fresh-world regression deliberately sets these
two words differently. Ownership masks and domain-2 live unit counts are
reconstructed before native execution; other caches are copied from the saved
inputs. The report preserves saved/refreshed naval totals: FallRome differs.
This tests the prepared mechanics context, not a complete native statistics
pass. Saved custom human-leader overrides are not loaded; traits come from Rules.

The separate leader-loader experiment executes original `0x41447d`, replacing
only section/row/string/numeric token readers. It verifies all 21 field-9 values
from each Rules file land in `0x606f4a +60*leader` (42 rows). The full scoring and
selection comparisons use no callee replacements. Original `0x490440` also
checks every encountered reference token without replacements: 90 Original,
52 FallRome. FallRome comments `*Ind` / `*E2` still denote slots 37 / 24, while
references omit the asterisk. The scoring adapter normalizes this annotation
and matches case-insensitively. General `TechTree`/AI v1 behavior is unchanged;
arbitrary comment relabelling versus native fixed identifiers remains outside
this bounded adapter comparison.

The native selector uses `0x4a2098` (zero leader-group flag) rather than the
broader `0x4a2111` player research gate. Flag 1 therefore blocks the native
selector but permits the player to request the advance. Both the asset-free
adapter test and the saved-input check retain that distinction. Missing or
malformed required scoring tables fail explicitly; a final text-only unit
annotation is accepted. No privileged inputs are added to `Observation`.

```sh
CIVTOT_HOME=/path/to/CIVTOT PYTHONPATH=/path/to/unicorn:tools \
  python3 tools/ai_research_adapter_check.py --vectors /tmp/ai-research-adapter.txt \
  > /tmp/ai-research-adapter.json
cmp /tmp/ai-research-adapter.json docs/evidence/ai-research-adapter.json
cmp /tmp/ai-research-adapter.txt crates/engine/tests/fixtures/ai-research-adapter.txt
CIVTOT_HOME=/path/to/CIVTOT cargo test -p engine --test ai_research_adapter_install
cargo test -p engine --test ai_research_adapter
```

### Built-in policy production comparison (#106, v0.10)

`tools/ai_production_compare.py` executes the complete original build selector
`0x47e26b` on prepared saved inputs, with **no replacement scoring callees**.
It restores the baseline memory for every live non-barbarian city and each of
six difficulties, seeds both original generators to 106 and records the
selected item and final generator states. Inputs include native-resolved
technology references, leader/group tables, full unit scalar fields
(including obsolescence, civ mask, hit-point scaling and Rules movement
multiplier), improvements/upkeep and the saved map/city/civ caches. City
accounting collection is off and presentation context is quiet. These are
prepared independent choices; they are not executions of complete native
turns or a reconstruction of all campaign caches.

The engine example collects both policies' production intentions through
the player interface, before the turn kernel's existing obsolete-build
replacement. A raw unit-ID mismatch can therefore still produce the same
upgraded unit; the percentages do not measure completed production. Its observation and private RNG differ from the
original's privileged context and shared RNG. `ai-production-comparison.json`
and `ai-production-choices.txt` retain **918 choices**:

| Input | Partition | Choices | AI v1 exact | Built-in exact |
|---|---|---:|---:|---:|
| Standard initial save | Calibration | 60 | 0 | 48 |
| FallRome | Holdout | 750 | 42 | 42 |
| std1 turn 01 | Holdout | 24 | 12 | 12 |
| std1 turn 10 | Holdout | 24 | 0 | 24 |
| std1 final | Holdout | 60 | 0 | 48 |

The standard archived holdout improves from 12/108 to 84/108 exact choices.
FallRome stays at **42/750 (5.6%)**: advanced worker/settler production,
development and scenario-specific planning remain substantial differences.
No acceptance budget was frozen and these percentages are descriptive. The
calibration changes were made using the standard initial position; the
remaining positions were retained for the reported holdout evaluation.
The retained std1 saves include the historical oracle/human workflow and are
not claimed to be controlled all-AI original campaigns.

```sh
cargo build -p engine --example ai_build_choices --release
CIVTOT_HOME=/path/to/CIVTOT PYTHONPATH=/path/to/unicorn:tools \
  python3 tools/ai_production_compare.py \
  --engine-example target/release/examples/ai_build_choices \
  --holdout-root /path/to/retained/std1 \
  --vectors /tmp/ai-production-choices.txt > /tmp/ai-production-comparison.json
```

The report and vectors reproduced byte-for-byte after integration. The
retained private saves are identified by hashes in the report; obtaining
those exact archived inputs is required for the full reproduction.

### Military capture subkernels (#106, v0.10)

`capture-kernels.json` and `capture-kernel-vectors.txt` retain **843 native
checks**: loot `0x53adc6` (75), building attrition
`0x53d9ac..0x53da34` (192), and military population loss for non-barbarian
conquerors `0x53da34..0x53dc1f` (576). The instruction ranges use authored
caller frames and stop before city destruction/ownership transfer. A control
hook stops the size-one recapture exit; it replaces no callee or decision.
Cases cover both alternating masks, recapture, capture modes, positive
signed-byte city sizes, low/high treasury arithmetic, population caches,
human masks, city knowledge and saved power ranks. Final CRT states are
checked alongside values. No original game records/assets are retained.

An additional eight native checks at `0x53f522..0x53f55c` prove that city
byte +12 records the previous owner after capture; it is not an immutable
founder identity. They are retained as `owner_history_checks` in the report.

The Rust kernel test consumes every vector. Player tests cover defended-city
rejection without state/RNG change, occupation and transferred economy,
rehoming a lone defender, size-one destruction with route/wonder cleanup,
and an authored CITYTAKEN effect with independent order-log replay. The
explicit order preserves earlier `move` replay semantics. The enclosing
engine occupation transition remains partial: no native capital relocation,
civil war, technology plunder, civilization respawn, full diplomacy or cargo
handling is claimed. See decision 0007.

```sh
CIVTOT_HOME=/path/to/CIVTOT PYTHONPATH=/path/to/unicorn:tools \
  python3 tools/capture_check.py --vectors /tmp/capture-kernel-vectors.txt \
  --report /tmp/capture-kernels.json
cargo test -p engine --lib capture::tests
cargo test -p engine --test builtin_campaign
```

### Built-in opponent in the browser (#106)

`builtin-observer.json` and `builtin-observer.png` retain a real Chromium
observer demonstration on the standard save with seed 7 and the built-in
policy: **100 rounds / 101 frames**, 88 battles, 41 founding results and 25
occupations (seven razed). The order log contains no rejected orders. The
final frame is turn 143, checksum `95887f80394d960d`, with 43 cities and 174
units. All map rendering is the observer's own geometric artwork.

The existing `tools/test_observer_browser.cjs` checks fog, step/back,
play/pause, restart during seek, archive download/import, malformed archive
rejection and identical final replay canvas pixels. Separately, the CLI
install test requires all 51 frames from a 50-round built-in run to equal
verified order-log replay byte for byte. This demonstrates the resulting
engine opponent; it is not an original-game campaign comparison.

```sh
cargo run --release -p totctl -- observe \
  "$CIVTOT_HOME/Original/Ph_Auto2.SAV" --builtin --turns 100 --seed 7 --port 0
# Against the printed loopback URL, with external Playwright/Chromium:
node tools/test_observer_browser.cjs http://127.0.0.1:PORT
```

### Built-in campaign cohort (#106)

`ai-campaign-audit.json` retains **64 runs** comparing AI v1 and `builtin-1`:
three generated-world seeds × six difficulties × two policies (100 rounds),
two standard-save seeds × six difficulties × two policies (100 rounds), and
one FallRome seed × two endpoint difficulties × two policies (25 rounds).
Every case reaches its requested round count, independently replays its
complete order log and has zero rejected orders. All 36 generated runs also
restore a halfway checkpoint and require byte-identical continuation. The
scheduled authored-rules suite independently covers 32 seeds for each policy.

The built-in cohort makes 90 generated-world occupations, 254 standard-save
occupations and 114 FallRome occupations. Final city ranges and baseline
comparisons are in `docs/v0.10.0-readiness.md`. These are engine sustainability
and replay checks; they execute no native original turns and establish no
original campaign or AI-strength parity. Source hashes and numeric metrics
are retained; original saves/Rules are read only. Difficulty changes use the
typed editor on an in-memory copy. The mechanics cohort disables events;
the separate browser run uses normal sibling-event loading. FallRome takes
minutes under both policies; the shorter scenario window is explicit.

```sh
cargo build -p engine --example ai_policy_audit --release
CIVTOT_HOME=/path/to/CIVTOT python3 tools/ai_campaign_audit.py \
  --engine-example target/release/examples/ai_policy_audit \
  --report /tmp/ai-campaign-audit.json
```

## Playable-client UI discovery (v0.11, second pass, #107)

`ui-discovery.json` records source identities, private screenshot hashes and
33 inspected route groups from the installed ToTPP reference UI. The manifest
contains 115 references to 112 private capture filenames with 111 unique content
hashes: the improvements panel and return after cancelling sale are identical.
All retained capture/source hashes were rechecked for the planning candidate.
Owner acceptance of the prototype and first scope is recorded alongside the
explicit native-coverage limits; this is not playable-client validation. Routes,
reinspection procedure and explicit gaps are in `docs/ui/discovery.md`.
Three sessions used copied install/Wine directories on an owned Xvfb display;
no RAM or presentation patches were applied. Original screenshots and the
generated human save remain outside Git. This is visual workflow evidence,
not unpatched-UI equivalence or automated pixel/next-turn parity.

The routes include map/city/production, distinct science-goal and required
project-choice screens, rates/revolution, FallRome foreign-report/refusal/
presentation entry, fresh standard setup, founding and scratch save/reload.
The reload retains the visible selected research project. Overwrite refusal
returns to the chooser; full persistence fidelity is not established.
The second pass adds worker/specialist edits, purchase and sale cancellation,
support/happiness panes, city navigation and a production change; advisor/world
reports and standard/scenario score forms; plain-dialog negotiation through
peace/advance exchange and intelligence; Fantasy unit transfer; classic custom
scenario labels/resources; Lua-extension and authored-invalid-file rejection.
An explicitly focused revolution question followed by Escape alone yields the
revolution notice and Anarchy in the installed reference. It is not proof of
unpatched equivalence or a general rule for every dialog. A later government
chooser leads to rate review and a newspaper notice in the small standard case.

`docs/ui/client-prototype.html` and its map/city/report/diplomacy PNG previews are authored
design fixtures, not reference screenshots. Their proposed interactions and
browser-only validation are recorded in `docs/ui/client-design.md`.
`docs/ui/screen-contracts.md` separates proposed interface/decision contracts
from reference observations; `docs/ui/client-milestones.md` records the owner-
accepted first scope and live decomposition (decision 0005, 2026-11-06). Later
release versions, dialog redesigns and sustained-play/native-parity acceptance
remain explicitly unassigned or unproven.


## Installed-art client prototype (#107, v0.11)

`client-artwork.json` records eight installed Original-profile source hashes,
browser-decoded dimensions and five numeric foreground/transparency samples.
The local tool reads selected BMP/GIF files from `CIVTOT_HOME`; no artwork,
extracted sprites, base64 data or installed-art captures are committed.
`docs/ui/asset-preview.md` documents the measured crop/matte recipes, explicit
lookup policy and limits. These are visual prototype examples, not a complete
original renderer, palette/adjacency decoder, scenario adapter or game client.

Re-derive with a local preview server and optional Playwright:

```sh
CIVTOT_HOME=/path/to/CIVTOT python3 tools/preview_client.py --profile Original
NODE_PATH=/path/to/external/node_modules node tools/test_client_artwork.cjs \
  http://127.0.0.1:8765 /tmp/client-artwork.json
```

The check verifies served bytes against the manifest, foreground/transparent
sample pixels, zoom, simulated move/redraw, citizen imagery and all main screens.
The JSON contains no original pixels. Install-free selection, fallback and
bounded-route tests run through `tools/test_preview_client.py` in make test/CI.

## Desktop toolkit feasibility (milestone 10, #126)

`desktop-feasibility.json` retains numeric reports/source hashes for the isolated
SDL2 application: native Linux and Windows, shared Original artwork loader,
scripted accented-name fixture, real OS input/focus/resize/close and packaged
launches. Windows ran on the actual Windows host, not Wine. All original pixels,
fonts and executable/DLL packages stay outside Git. Timing-dependent counters
are observations rather than golden byte-parity assertions. Commands, dependency
notices and explicit limitations: [desktop feasibility](../ui/desktop-feasibility.md)
and [spike README](../../tools/desktop-spike/README.md). This is not human gameplay
or a full native-renderer claim.
## Playable human clients (milestone 10, v0.12 released)

`client-atlas-v1.json` retains numeric identities/dimensions/opacity/fingerprints
for the 141 Original-profile crops used by both clients. Regenerate/check with
`CIVTOT_HOME=... cargo test -p totui installed_atlas_matches`; an explicit
`TOTENGINE_ATLAS_MANIFEST_OUT` writes a candidate numeric manifest. A different
owned installation is supported but does not claim to reproduce this corpus.
No sprite pixels are retained.

`client-input-route-v1.json` is an authored/recorded 879-input generated-game
route (seed 2, 32×24, two civs, human 1, difficulty 0), with a ground battle
and city occupation by round 67. It contains no original map/save/artwork.
`CIVTOT_HOME=... TOTENGINE_ENCOUNTER_OUT=/tmp/client-encounters cargo test -p
totui generated_land_encounter -- --ignored --nocapture` reconstructs private
before/after engine checkpoints. Existing built-in policy authored the route;
this is automation evidence, not human-play acceptance or a new AI port.

`playable-client-checks.json` retains cross-client/cross-platform checkpoint
hashes and exact scope/status for real browser and native-window input checks.
The browser/Linux extended routes match pending-choice continuation and
combat/capture results byte-for-byte; Windows ten-round checkpoints match.
Stopped Windows package attempts are explicitly recorded, not counted as passes.
Reproduction, packaging, owner acceptance and limits:
[`v0.12.0-readiness.md`](../v0.12.0-readiness.md). The original AI completion
program remains gated behind full playable-client completion.

## City citizens (v0.13 in progress)

`citizen-edits.json` and `citizen-edit-vectors.txt` retain 1,548 raw original
worker/specialist edit comparisons from unpatched civ2.bak: worker entry/stop
0x4dddc6/0x4ddec9, specialist entry/stop 0x4dd19f/0x4dd24d. UI current-city
access returns synthetic city 0 and the small-city message is recorded; native
branches and packed helpers run unchanged. Original arithmetic is compared at
a synthetic packed-field boundary, not a complete city/window transition.
The population-five role restriction executes in those original cases.
Re-derive with tools/citizens_check.py (Unicorn outside Git, CIVTOT_HOME); the
Rust regression test consumes numeric vectors without assets.

`client-atlas-v2.json` adds four measured People.bmp crops to v1's 141 sprites;
`city-citizens-v1.json` records generated turn-72 population-five fixture and
role/continuation/navigation checkpoint hashes, actual browser/Linux input
comparisons and explicit remaining platform/manual gates. All retained data is
numeric or authored input; original screenshots, sprites, maps and saves stay
private. Scope, exact commands and limits: [v0.13 readiness](../v0.13.0-readiness.md).

## Playable-client control corrections (v0.13 preparation)

`client-controls-v1.json` records the generated-warrior activation/movement
regression, original manual and installed-menu source identities, twelve matching
real-input checkpoints across browser/Linux/Windows, and presentation-only
blinking checks. `client-input-route-v2.json` translates the older UI shortcuts
without changing its generated engine trajectory; the v1 artifact stays intact.
Sources and limits: [original controls](../ui/original-controls.md). Reproduction:
[v0.13 readiness](../v0.13.0-readiness.md#reproduction).
Historical status of the rejected preview: owner retesting was pending; these
checks did not complete the human gate. The replacement build subsequently
received the separate owner acceptance recorded below.

## Original selection study after rejected preview

`ui-selection-study-v1.json` records nine directly inspected reference observations
from normal production/play in a private copied ToTPP installation. It includes
source/configuration identities, 22 key capture hashes and 48 temporal captures;
no images/assets/saves are committed. The one-setting indicator-patch comparison
has an explicit negative/uncertain result rather than an unpatched UI claim.
Read [the study and replacement contract](../ui/selection-and-movement-study.md).
The prior control-check evidence does not cover switching units by mouse while
Moving and does not satisfy the owner's selected-unit blink requirement. The
owner rejected that preview. No client code was changed during this study.

## Revised client selection contract implementation

`client-selection-v1.json` records the normal-play two-unit fixture and new
browser/Linux/Windows checks after the rejected preview. Selection-only actions,
stack choice and zoom/resize retain the whole checkpoint. Movement, skip and
next-turn results match across all adapters. Sprite-body pixels, rather than an
arbitrary marker, verify selected-unit blinking and hidden-phase activation.
`client-input-route-v3.json` adds explicit actor identities to the historical
879-input route and reproduces the same battle/capture/final game checkpoints.
Reproduction and 2026-11-07 both-client owner acceptance: [v0.13 readiness](../v0.13.0-readiness.md).
The reference-study facts and configured-ToTPP limits remain unchanged; the new
automated checks are not human acceptance or a claim of full original UI parity.

## Post-v0.13 asynchronous browser focus ordering

`client-focus-order-v1.json` records #146: delayed older focus refreshes may not
overwrite newer blur. Its controlled Chromium request/event route fails against
the unchanged v0.13 release and passes with focus-event epochs, followed by the
ten-round/pending-choice continuation route. It controls presentation HTTP and
window events only, not game state; physical OS focus transitions are outside
this synthetic-event regression. Released v0.13 evidence/tag stays unchanged.
