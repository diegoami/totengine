# Reference material

Offline copies of documentation the engine must be compatible with. **None of this
is original game data** — it is open, publicly published documentation and API
definitions, kept verbatim with attribution.

## `totpp-api-zerobrane.lua` — the ToTPP Lua API, machine-readable

- **What:** the complete `civ.*` / `totpp.*` API surface as a Lua table: 29 object
  classes (city, tile, unit, unitType, tribe, improvement, tech, leader, wonder,
  prodItem, spaceship, dialog, map, baseTerrain, terrain, image, commodity,
  traderoute, …), 112 documented functions, 97 value/property entries — with
  signatures, return types and behavioral notes (combat coroutine protocol,
  `onCalculateCityYield` pipeline, scenario-flags bitmask, turn trigger order).
- **Source:** [`packages/totpp.lua`](https://github.com/ProfGarfield/ZeroBraneForTOTPP/blob/main/packages/totpp.lua)
  from [ProfGarfield/ZeroBraneForTOTPP](https://github.com/ProfGarfield/ZeroBraneForTOTPP)
  (ZeroBrane Studio fork for ToTPP Lua scenario programming). Fetched 2026-11-01,
  unmodified below the prepended MIT notice (the fork inherits ZeroBrane Studio's
  MIT license; see `docs/prior-art.md`).
- **Why it matters:** this is the *contract* Tier 3 must implement. `civ.scen.*`
  triggers define where the engine must call into Lua; the object property lists
  define the game-state surface Lua must see. It doubles as autocomplete data for
  scenario authors.
- **Version note:** the API is versioned ("since 0.16/0.17" markers inside);
  the local install runs ToTPP 1.4 (Lua 5.3.6) — the engine targets that baseline
  and can consult `totpp.version` at runtime.

## Reading the local ToTPP install (not copied — read-only reference)

The install at `/mnt/d/games/CIVTOT` ships Lua helpers that scenarios `require`:

- `lua/civlua.lua` — the `civlua` compatibility library (serialize/unserialize,
  `justOnce`, findCapital, iterate helpers, terrain/domain shortcuts)
- `lua/functions.lua`, `lua/init.lua` — bootstrapping used by `events.lua`
- `TOTPP/desc/*.rtf` — feature descriptions (CitySprites, Mods, Playable,
  TransformTech)
- `TOTPP.ini` — patch flags (mirrors `totpp.patches` keys)

Scenario-side usage to test against: `Scenario/china/events.lua` + `Scenario/china/Lua/`
(the largest real-world consumer of this API).

## Online sources (for follow-up)

- ToTPP main thread: https://forums.civfanatics.com/threads/the-test-of-time-patch-project.517282/
- "[TOTPP] Get Started With Lua Events": https://forums.civfanatics.com/threads/totpp-get-started-with-lua-events.636192/
- The official ToTPP release package additionally ships a CHM manual with the same
  API in prose form.
