# Tier-0 Evidence — first format probe results (2026-11-01)

Artifacts produced by `tools/totctl.py` (Python reference probe) against the local
install `/mnt/d/games/CIVTOT`. These validate the feasibility claims in
[../feasibility.md](../feasibility.md) with measured data.

## Files

| File | Produced by | What it shows |
|---|---|---|
| `rules-summary.json` | `totctl rules Rules.txt` | All 22 `@`-sections parsed; `@COSMIC` values, terrain/unit/government tables |
| `world-mp.png` / `world-mp.json` | `totctl map World.mp --rules Rules.txt` | ToT world map rendered to PNG (terrain-colored, voids black) |
| `romfall-walk.json` | `totctl save ROMFALL.scn` | Structural walk of a classic scenario (ToT 1.1, `0x0032`) |
| `warring-states-walk.json` | `totctl save "Warring States.scn"` | ToTPP-extended save (`0x0033`) walked with base-format offsets |
| `va-autosave-diff.json` | `totctl diff Va_Auto2.SAV Va_Auto.SAV` | Annotated diff of two real autosaves (oracle-methodology demo) |

## Findings (verified)

1. **`.MP` map format (ToT variant) — decoded.**
   8-byte header: `word width, word height, dword real_tile_count`; then
   `width×height` slots of **3 bytes**: `word` (little-endian) + 1 extra byte.
   - Terrain id = `word & 0x7F`; bit `0x80` = special-resource variant
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
