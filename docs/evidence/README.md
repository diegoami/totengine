# Tier-0 Evidence — first format probe results (2026-11-01)

Artifacts produced by `tools/totctl.py` (Python reference probe) against the local
install `/mnt/d/games/CIVTOT`. These validate the feasibility claims in
[../feasibility.md](../feasibility.md) with measured data.

## Files

| File | Produced by | What it shows |
|---|---|---|
| `rules-summary.json` | `totctl rules Rules.txt` | All 22 `@`-sections parsed; `@COSMIC` values, terrain/unit/government tables |
| `world-mp.png` / `world-mp.json` | `totctl map World.mp --rules Rules.txt` | ToT world map rendered to PNG (terrain-colored, voids black) |
| `world-mp-rivers.png` | `totctl map World.mp --scale 3` | River overlay (word bit `0x80`) — dendritic Earth river networks |
| `romfall-scnmap.png` | `totctl scnmap FallRome/ROMFALL.scn` | Embedded scenario map with river/road/railroad overlays |
| `anticchina-map.png` | `totctl map Scenario/china/AnticChina.MP` | ToTPP-era paired-slot `.MP` flavor: China with river networks + improvement flags |
| `romfall-walk.json` | `totctl save ROMFALL.scn` | Structural walk of a classic scenario (ToT 1.1, `0x0032`) |
| `warring-states-walk.json` | `totctl save "Warring States.scn"` | ToTPP-extended save (`0x0033`) walked: `ext033` units/cities/structural checks + `totpp_tail` Lua framing |
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
   findings. *Open:* `year` field at offset 670 reads 5159 — decoding TBD.

5. **ToTPP `0x33` saves keep the base layout at the front.** *Warring States.scn*
   walks successfully through settings + civ names ("Xiao", "Qin") at the same
   offsets as `0x32` — extension differences start later. (Its 6.4 MB autosaves
   will define the `0x33` tail layout in Phase 1/3.)

6. **The save-differ turns file sizes into semantics.** Two real autosaves 1.5 h
   apart: size delta exactly **−40 bytes = one ToT unit record** (a unit died or was
   disbanded); first changed region is offset 668–677 — precisely the `game` block
   where turn/year live. This is the oracle methodology of docs/plan.md working
   end-to-end on day one.

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
   move-one-unit save-diff pair. T0-6 stays open for this; see issue #3.
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
   in the Italy/Gaul core (128 tiles carry both; rails never set `0x40`).
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
   remaining 1,219; extra 0xF0 on 25,882, 0xFF on 1,219, plus low-bit
   variants of 0xF0 (0xF7 268, 0xFE 150, 0xFD 85, 0xFC 57, …). Dominant
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
| `[+0, +32+15×surface)` | extended per-tile arrays, mostly sparse; a dense **6 B/tile band** (168,960 B ≈ 28,000×6) and a **10,240-B blob** sit inside; sub-layout open |
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
  space); off-board units carry negative coords (−100/−500 observed);
- `@18 u8 type` — Rules.txt @UNITS index (190 rows in Warring States,
  values 0..188 observed);
- `@19 u8 owner` — 0=barbarians, 1..7 = civs (exactly 8 distinct values);
- `@28 u16` optional link, 0xffff = none (home city?);
- rest flags/counters (open).

**Decomposition** (walker `ext033`; the arithmetic is *descriptive* — the
post-city term is the leftover by construction — so validity rests on the
falsifiable structural checks in `ext033.checks`: unit-start validity,
block maximality, city-0 name printable, and no competing 92-B name grid
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
