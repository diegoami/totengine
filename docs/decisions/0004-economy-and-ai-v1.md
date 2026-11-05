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
  independently of trade routes (#85) and road waste (#84).
- `city_finance` obtains inputs from the current engine city output. The
  turn loop refreshes city science/tax before production completion; newly
  finished improvements affect the next evaluation. Overall turn-phase
  order remains engine-chosen, as documented in the existing turn loop.
- City taxes are not yet treasury income: maintenance and per-civ research
  accounting need their own implementation/evidence. Existing treasury
  effects and production remain intact. Research accounting subsequently
  landed in the research slice below; treasury/maintenance remains open.
- Preserve deterministic replay and include rates/specialists in world
  fingerprints. Regenerate the run50 fingerprints intentionally; neither
  asset files nor opaque executable dumps belong in evidence.
- Acceptance: every FallRome city's stored science/tax and every city in the
  current standard save match, using stored input trade/shields. Record
  this in derived evidence and install-gated tests; additionally test
  rounding, specialists, multipliers, wonder obsolescence, Fundamentalism,
  Capitalization and turn refresh without original assets.

## Research accounting (#82 dependency)

The original `0x4a1ae0` supplies costs; `0x4a1ebf` accumulates city science,
and `0x4a145e` resets progress and clears a completed target. Their saved
state joins the checksum. Cost vectors come from executing the original
bytes in Unicorn, independently of the Rust implementation. Existing std1
saves additionally check increments and discovery/reset boundaries.

`SetResearch` is a player order, validated against prerequisite-valid
choices, excluding `no` entries. Progress survives a target change. After a
discovery, the engine leaves target selection to the next player order
phase; subsequent science is banked. It does not silently pick an advance
on behalf of a player mid-turn. The original UI's rotating subset of
choices and ToTPP tech-group restrictions are not modelled.

`AiV1` starts as a wrapper around AI v0: it chooses the available advance
with highest Rules AI value (lowest ID wins ties), using only its own
observation. `totctl play/observe --ai-v1` opts into this increment. This is
an engine policy, not original-AI parity. Existing AI v0 remains selectable.

Costs preserve the saved comparison slot; updating its ranking is future
work. Unknown scenario tails yield no cost and no research choices, rather
than default costs. Diplomacy/technology rewards, event hooks and the late
spaceship-related science bonus remain separate work. See the evidence
section for the supported source fields and exact checks.

## Capital connections (#84)

`engine::connections` supplies the original's road/rail factor for city
waste. Decode the shared `0x471a13` routine once, preserving its shortest
land-path selection, tie-breaking, diplomatic checks, early termination at
an own city, and map-seam bounds. A road-only flood fill would give a
different result. Keep tile landmass IDs, city map IDs and saved diplomatic
words in the world fingerprint. Airports provide the decoded fallback
across landmasses; a Palace has its direct 1/2 factor with Railroad.

The current implementation covers surface-map land unit type 2 (the
original hard-codes that type for this search). Sea/air replacements and
other maps remain unsupported. The original-x86 probe and Rust tests
compare 123 saved city/capital pairs. FallRome shields now match 145/146
stored records; Aventicum's remaining one-shield difference is unresolved.
Trade-route values (#85) reuse this helper in the following slice.

## Recurring route income (#85 slice)

Decode the original route loop independently of supply/demand refresh and
delivery rewards. Preserve signed commodity IDs, partner indices, and the
partner city's cached pre-route trade. Recomputing partner totals recursively
would both lose the original cache semantics and loop on mutual routes.
`run_turn` refreshes each city's cache before its finance evaluation, in the
existing engine city order. The overall turn order is still engine-chosen.

The original loop and its building/connection callees execute without stubs
for 288 invented arithmetic cases and the saved FallRome route. That route
has commodity -1 and contributes zero: it does not validate positive income
in a live campaign. Rust additionally checks multiple slots, mutual partners,
cache changes and route-inclusive finance. Route creation, delivery rewards,
supply/demand refresh and non-surface routes remain open; #85 stays open.

## Remaining milestone work

The owner requested completion of the entire v0.8 queue and then a milestone
review candidate (2026-11-05). Treat the live issue acceptance as the gate;
do not substitute isolated formula tests for full campaign-parity claims.
The main session prepares the candidate and milestone issue, then prints
the owner-run review prompt; it does not launch the review.

Next production work decodes #89's row adjustment and #88's wonder lifecycle.
The original row fragment is executable independently of unit support. In
particular, cosmic rows = 10 bypass scaling **and even rounding**; the prior
port rounded unconditionally. The late adjustment starts at turn >= 200,
requires a human power leader, and depends on wonder slots 23/25 plus
knowledge masks 73/58/76. Preserve the original signed-mask arithmetic.

AI v1 still needs economy/unit/war refinements and the acceptance stated
in #82 (deterministic 50-turn autoplay plus justified oracle comparisons).
#83/#79 require oracle arrival/outcome evidence; #85/#86/#88/#89 require
finishing their decodes and checks. #84 retains the map/type limits above.
Do not close those on the strength of
#87. In particular, #89's late-game condition reads leader and wonder state,
not simply `(turn > 200 && difficulty > 1)`.
