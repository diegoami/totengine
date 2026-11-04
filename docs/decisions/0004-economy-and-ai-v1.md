# v0.8.0 implementation plan — economy before AI v1

The owner requested the next release after v0.7.0 (2026-11-04).
The live scope is milestone v0.8.0: #13, #15, #79, #82–#89.
Decisions 0002/0003 continue to govern players, observations and replay.
This note records implementation boundaries; it does not claim the milestone
is complete or authorize a release without independent review.

Start with city science/tax output (#87), then research and AI choices (#82).
Keep original-game arithmetic supported by binary/record evidence, and mark
engine-selected policies as such. Research costs, progress, discovery and
barbarian arrivals must not be invented to make a campaign appear complete.

## First slice: science/tax (#87)

- Decode civ rates and city specialist slots from existing record framing.
- A pure `WorldState::split_city_trade` accepts trade after corruption/routes
  and net shields for Capitalization, and returns luxuries/science/taxes.
  This boundary lets finance be checked against the game's stored trade
  independently of the still-open trade routes (#85) and road waste (#84).
- `city_finance` obtains inputs from the current engine city output. The
  turn loop refreshes city science/tax before production completion; newly
  finished improvements affect the next evaluation. Overall turn-phase
  order remains engine-chosen, as documented in the existing turn loop.
- City taxes are not yet treasury income: maintenance and per-civ research
  accounting need their own implementation/evidence. Existing treasury
  effects and production remain intact.
- Preserve deterministic replay and include rates/specialists in world
  fingerprints. Regenerate the run50 fingerprints intentionally; neither
  asset files nor opaque executable dumps belong in evidence.
- Acceptance: every FallRome city's stored science/tax and every city in the
  current standard save match, using stored input trade/shields. Record
  this in derived evidence and install-gated tests; additionally test
  rounding, specialists, multipliers, wonder obsolescence, Fundamentalism,
  Capitalization and turn refresh without original assets.

## Remaining milestone work

AI v1 still needs its research and economy inputs and the acceptance stated
in #82 (deterministic 50-turn autoplay plus justified oracle comparisons).
#83/#79 require oracle arrival/outcome evidence; #84–#86/#88/#89 require
finishing their decodes and checks. Do not close those on the strength of
#87. In particular, #89's late-game condition reads leader and wonder state,
not simply `(turn > 200 && difficulty > 1)`.
