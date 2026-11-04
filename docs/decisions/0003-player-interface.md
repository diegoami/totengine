# Decision 0003 — the player interface (RG-8, v0.5.0)

*Status: accepted (owner: "and then start working on 0.5", 2026-11-04).
Story #74.*

## Context

Decision 0002 puts every civ behind a player. Several things will plug in:
- AI v0 (v0.6.0) and AI v1;
- the browser observer (v0.7.0);
- external AIs over JSON (v0.10.0);
- a human in the SDL2 client (v0.11.0).

The engine already has the state transitions: `move_unit` (moves and
attacks), `production_step`, and `run_turn`. What it lacks is a boundary:
- who may do what;
- what each civ may see;
- a record that makes a game reproducible.

## Decision

New module `engine::player`.

**1. Observation**: what civ `c` sees, rebuilt on demand from the world:
- its own: units, cities (with their output and build item), treasury,
  government, advances;
- the map tiles it has seen (`Observation::seen`: terrain, river, the
  special-suppression bit, road, rail; tile byte +4 bit `c`, the per-civ
  visibility decoded in §Map tile framing);
- foreign units and cities standing on tiles it currently sees.

Fog is engine-chosen for now (flagged; the original's reveal rules are
not decoded):
- a civ currently sees tiles within distance 1 of its units and within
  the 21-tile radius of its cities;
- seen tiles stay "seen" (their visibility bit is set when revealed).

**2. Orders**: `enum Order`, each validated against the world before it
applies.

| Order | Valid when | Effect |
|---|---|---|
| `Move { unit, to }` | the unit is the civ's, alive, has moves, `to` is a diamond neighbour | `move_unit` (a move, or an attack on foreign units) |
| `SetProduction { city, item }` | the city is the civ's; a unit type whose prerequisite advance the civ knows, or an improvement it can build and lacks | sets the build item (the box carries over) |
| `FoundCity { unit }` | added in v0.6.0: a settler of the civ, on land, no city within distance 1 | creates the city (`engine::founding`; evidence §City founding) |
| `EndTurn` | always | ends the civ's turn |

`SetResearch { advance }` is added during v0.8.0: accepts an available,
prerequisite-valid advance and sets the civ target without discarding
progress. Own research target/progress/cost/choices are in `Observation`;
accounting and current limits are documented in decision 0004.

A rejected order changes nothing and returns the reason. Players see
every result.

**3. Order log**: plain text, one line per submitted order, and a
header:
```text
totengine-orderlog 1
seed <u64>
save <file name> <size> <fnv64 of the bytes>
T <turn> C <civ> <order…> = <result>
```
**Replay** = boot the same save with the same seed, re-submit every
logged order in order, and require each result to match its log line.
The checksum after each turn must also match. Results are logged too, so
a divergence is caught at the first order, not at the end.

**4. Players**: `trait Player`:
- `fn act(&mut self, obs: &Observation) -> Order`. The driver calls it
  repeatedly, with a fresh observation and the previous order's result,
  until `EndTurn` or an action cap (an engine guard against loops).
- A player's randomness comes from its own generator, seeded from the
  game seed and the civ, never from the world's generators. So a player
  cannot change the world's dice.

**Driver:** `Game { world, rules, events, players, log }`:
- `play_turn()` runs civs 1–7 that are alive, in index order (an engine
  choice; the original's turn order is not decoded), then
  `world.run_turn`;
- `replay(log)` re-drives a game from its log.

**Test player:** `RandomPlayer` issues random orders, legal and not, so
that validation is exercised too. It moves a random own unit to a random
neighbour, sometimes changes a city's production, and ends its turn after
12 actions. It exists to exercise validation, logging and replay.

## Consequences

- **v0.5.0 acceptance** (#74): N turns of all-`RandomPlayer` play on a
  standard save and on FallRome replay bit-exact from seed + log. A
  tampered log line is detected at that line. It runs in CI on the
  synthetic save.
  - Tests: `engine::player::tests` and `crates/engine/tests/player_install.rs`.
  - CLI: `totctl play` writes a log; `totctl replay` checks one.
- **Observation fog:** the engine starts tracking per-civ visibility
  (tile byte +4); it joins the checksum.
- **AI v0 (#75):**
  - adds `FoundCity`; `SetResearch` followed with v0.8.0 research accounting;
  - plugs in as one more `Player`.
- **JSON protocol (#92):** serialises `Observation` and `Order`. The text
  log stays the canonical record.
