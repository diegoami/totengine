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
| `romfall-walk.json` | `totctl save ROMFALL.scn` | Structural walk of a classic scenario (ToT 1.1, `0x0032`) |
| `warring-states-walk.json` | `totctl save "Warring States.scn"` | ToTPP-extended save (`0x0033`) walked with base-format offsets |
| `va-autosave-diff.json` | `totctl diff Va_Auto2.SAV Va_Auto.SAV` | Annotated diff of two real autosaves (oracle-methodology demo) |

## Findings (verified)

1. **`.MP` map format (ToT variant) — decoded.**
   8-byte header: `word width, word height, dword real_tile_count`; then
   `width×height` slots of **3 bytes**: `word` (little-endian) + 1 extra byte.
   - Terrain id = `word & 0x7F`; bit `0x80` = **river** *(corrected 2026-11-01,
     third session — initially mis-attributed to special resources; see
     finding 7 below for the proof)*
   - Void/non-existent tile = `extra & 0xF0` (marks Civ2's round-world voids)
   - `World.mp`: 150×120 = 18,000 slots, 9,000 real tiles (matches dword), 90-byte tail

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
   turn 0; 7 civ name blocks decode perfectly (Valentianus III/West Romans, Hengist/
   Anglo-Saxons, Chlodwig/Francs, Genseric/Vandals, Theodoric I/Visigoths,
   Theodosius II/Byzantines, Rugila/Huns); 56 techs pre-discovered; 10 wonders
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
   ToC the per-map block is Visible Improvements (7 B/tile) + Map (6 B/tile) + 2;
   empirically the **2 bytes sit before the tile blocks** (decoding from the block
   start yields garbage terrain; from +2 yields ids 0–10). The stored grid is
   **dense**: `width = hdr0/2 × hdr1 = surface` tiles (ROMFALL 75×120 = 9,000;
   `hdr0` = 150 is the *display* width — ToT tiles span 2 display columns).
3. **Map tile (6 bytes): byte 4 = terrain | river(`0x80`)** — identical encoding to
   the `.MP` word, cross-format consistency. ROMFALL terrain histogram: ocean
   2,768 / plains 1,756 / forest 1,448 / grass 1,050 / hills 870 / desert 800 …
4. **Improvements: map byte 2, bit `0x40` = road, bit `0x80` = railroad.** Verified
   spatially on ROMFALL: 3,497 road tiles form a connected empire-wide network;
   413 railroad tiles cluster densely in the Italy/Gaul core (128 tiles carry both;
   rails never set `0x40`). ASCII renders show the Mediterranean, Italy's boot and
   the road/rail networks in historically sensible places. Remaining bits
   (`0x02` base, `0x08/0x10/0x20` variants — irrigation/mine/farmland/fortress
   candidates) are open; oracle save-diffs will pin them.
5. **The 7 visibility bytes per tile show near-identical per-byte distributions**
   (~530 nonzero each on ROMFALL) — consistent with per-civ fog-of-war snapshots
   (7 civs); unproven, needs a fog-changing oracle pair.
6. **`totctl` UX (T0-11):** `CIVTOT_HOME` env resolves relative input paths and
   defaults `--rules`; `--scale` on renders; new `scnmap` subcommand renders the
   embedded scenario map with river/road/railroad overlays (artifacts:
   `romfall-scnmap.png`, `warring-states-scnmap.png` where applicable).


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
