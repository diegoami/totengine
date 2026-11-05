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

## Happiness and wonder work

Keep happiness arithmetic separate from its luxury, specialist and military
inputs. A native executable check on saved inputs must distinguish fresh
calculation from stored prior totals; do not call either a full-turn replay.
The same 15 FallRome happiness discrepancies occur under native original
recomputation. The engine matches those recomputations; #13 still needs its
separate sustained city-turn parity evidence.

Wonder registration uses city indices and discards overflow; destroyed
wonders remain unavailable. Pending Darwin choices are engine interaction
state, fingerprinted and replayed, not a newly invented save field. Original
completion calls discovery twice; grant an already selected target, then
leave subsequent selections to player orders. Other completion dispatch
effects must be checked before considering #88 complete.

## Treasury and maintenance

Use original `0x4ca5ae` for per-improvement fees and `0x4ca6e0` for the
payment/bankruptcy loop. Preserve the original city-turn caller's human-mask
gate: assigning an engine AI player does not alter the save's mechanics
mask. Income requires non-anarchy and nonnegative happy-minus-unhappy;
maintenance is skipped during anarchy and charged only to human-mask civs.
Bankruptcy must remove/refund buildings in ascending ID order, rather than
letting the treasury go negative or silently clamping the total upkeep.

The current engine credits the pre-production finance result and charges
upkeep after production; phase-order parity remains #13. Formula, saved-cost
and isolated bankruptcy checks are not a claim of whole-turn parity.

## Remaining milestone work

The owner requested completion of the entire v0.8 queue and then a milestone
review candidate (2026-11-05). Treat the live issue acceptance as the gate;
do not substitute isolated formula tests for full campaign-parity claims.
The main session prepares the candidate and milestone issue, then prints
the owner-run review prompt; it does not launch the review.

Production row arithmetic, power ranking refresh and wonder lifecycle now
have native comparison evidence. Leonardo, Statue, Manhattan and surface
Apollo completion effects are implemented, as are Darwin's pending choices.
Population/city caches and the surface barbarian arrival phase now run at
round close. See `docs/v0.8.0-readiness.md` for the remaining acceptance work;
these isolated comparisons do not establish full campaign parity.

AI v1 still needs economy/unit/war refinements and the acceptance stated in
#82 (deterministic 50-turn autoplay plus justified oracle comparisons).
#83 requires sustained live arrival evidence; #13 requires the complete city
phase comparison. Landmass strategy flags and remaining combat/wonder effect
limits still need audit. #84 retains its map/type limits and one saved shield
mismatch; #85 requires the combined corruption check. Do not close these
merely because their individual arithmetic fragments pass.


## Native surface city-phase integration (2026-11-05)

The initial finance-first turn ordering above has been superseded by the
original city phase, now executed by `run_city_phase`/`run_turn`: signed
starting-box growth/starvation, worker/totals refresh and food collection,
production, support/disband enforcement, post-production totals, mood,
science and treasury/upkeep. A newly completed Library affects post-production
science in that city; new unrest stops the following production phase.
The old `engine::cities` approximation is removed. City processing now follows
descending slots within a civ; headless civ scheduling after player orders,
initial transient inputs and turn events remain engine policy.

A controlled original-x86 sequence now checks the actual round loop for 50
turns/500 city economic transitions. The live-save gate, city-slot lifecycle,
landmass strategy refresh, pollution/settler-job state and sea/non-surface
limits remain explicit in `docs/v0.8.0-readiness.md` and the evidence README.
This does not alter the owner-approved v0.10 original-like AI target or the
later client/design milestones.
