# Original AI: implementation scope and completion gates

Status: scoped proposal, 2026-11-06, requested by the owner after the v0.10
candidate. Tracking: [program #117](https://github.com/diegoami/totengine/issues/117).
Baseline: 8f6b57feb44a834a2a452a73e70664ee4895b81a.
This document does not change or approve that candidate, assign a release
number, or supersede the accepted v0.11 UI-discovery priority.

## What we are delivering

An independently written Rust port of the **unpatched ToT 1.1 opponent's
actual decision algorithms**, including their dependencies and quirks. The
reference is the user's `civ2.bak`, pinned by SHA-256
`f9d20fc09287e96ca6ee95118294ebbc4db857cc9da672608d89d10bb1a1fd14`.
Neither original executable bytes nor game assets are distributed.

There are two completion gates:

1. **Standard-game original AI:** complete single-map regular-game decision
   families, their mechanics, scheduling, saved/transient state and random
   consumption, with matched original decisions and integrated turns.
2. **Full vanilla ToT AI:** the first gate plus the original shipped multiple-map
   campaigns, map transitions and scenario-specific branches of the unpatched
   executable. Inventory these now; do not hide them in a later definition of
   “complete.” ToTPP extensions, custom Lua AIs and extended-limit compatibility
   remain the separate T3 track; excluding them does not exclude vanilla ToT.

A heuristic that produces similar results is useful, but does not close a
porting task. Likewise, exact kernels, matched engine replay, win rate and
matching city counts do not establish complete original AI.

## Baseline inventory: ported kernels versus missing decisions

| Family | Existing implementation/evidence | Remaining port/integration |
|---|---|---|
| Research | Native valuation/selector and parsed world adapters; `ai_research.rs`, `ai_research/adapter.rs`, `ai-research-*.json` | Actual cache refresh, privileged inputs, shared-RNG scheduling, custom saved-leader handling and complete automatic follow-on decisions. Current player uses a restricted approximation. |
| Production/economy | Complete native selector executes in `ai_production_compare.py` at 0x47e26b; native-checked city economy | Selector itself is unported; build priorities/budgets in `ai.rs` are heuristics. Include hurry/sell, worker/specialist choices, wonders, and spaceship decisions where the inventory finds them. |
| Settlement/workers | Native founding/placement and city citizen-allocation mechanics | Original site evaluation, expansion pressure, settler missions, terrain jobs/progress, development priorities and pollution response. Citizen allocation is not settler task planning. |
| Land military | Native-checked combat components; heuristic garrisons/known-land routing | Original unit/mission scheduler, threat maps, force allocation, goals, paths, attack selection, retreat/healing/fortification, barbarian unit decisions and seam handling. |
| Capture/conquest | Loot/building/population subkernels; partial explicit occupation; destruction | Whole attack/advance/capture sequence, capital relocation/civil war, plundered technology, treaties, elimination/respawn, caches and callback order; #113. |
| Government/rates | Forced-human-anarchy mechanics, finance/happiness | Voluntary revolution, government choice, science/tax/luxury adjustment and strategic budget decisions. |
| Diplomacy/special units | Saved diplomacy fields and some mechanics consumers | Contacts, stance, negotiation, war/peace/alliance, gifts/tribute/exchanges, embassies, diplomats/spies, bribery/incitement and sabotage; #96/#115. |
| Sea/air/trade/space | Some combat/movement and recurring trade-income kernels | Ports/coasts, transports/cargo, naval missions/invasions, air missions/fuel/basing, caravan/freight missions, route creation/delivery rewards and spaceship/launch/endgame policy; #114/#97. |
| Full turns/maps | Deterministic engine replay, partial surface city/round processing | Original civ/unit/city ordering, refreshed/transient AI state, shared generators, action-budget behavior, full state comparison and multiple-map execution. |

These are code/evidence observations, not newly verified claims about native
routine semantics. Native entry points other than the already-probed routines
remain unknown until A0. No planner is marked ported just because an offline
native call succeeds. The current 918 production cases compare raw intentions
before automatic obsolete-build replacement, not completed unit production;
42/750 FallRome intentions is neither a full-turn score nor a coverage measure.

## Required engine actions and state

The current `Order` enum exposes Move, OccupyCity, SetProduction, SetResearch,
FoundCity and EndTurn. Several native mission families therefore need mechanics
and validated execution before porting their planner can deliver working AI.
A0 must inventory these action families and their state, then define atomic
semantics and replay changes; these are not proposed API signatures:

| Action family | State/transition to account for |
|---|---|
| City management | Citizen/specialist assignments, hurry/buy, improvement sale and automatic production/obsolete-build dispatch |
| Civ management | Rates, revolution/government selection, negotiation/offers, war/treaties and exchanges |
| Unit orders/jobs | Retained mission/path, wait/sentry/fortify/heal/disband, terrain work progress, interruption/reservation and special land-unit actions |
| Domain/strategic missions | Cargo/loading/unloading, air range/fuel/bases, naval/coastal paths, trade delivery/wonder assistance, nuclear/space choices and consequences |
| Vanilla maps | Unit/city/map identity, inter-map travel, cross-map missions and required simulation |

Persistent state must enter save/checkpoint/fingerprint handling as appropriate;
transient native caches need correct derivation and phase timing. Assign each
mechanics prerequisite to a gate and trace it to an existing issue or a new
bounded implementation story. Shared client-facing actions remain reusable by
the later playable client.

## First dependency: fidelity and the player boundary (A0)

Decision 0003 gives `Player` only an `Observation` and a private RNG. The
existing research evidence shows original scoring reading foreign knowledge,
reputation/contact caches and a shared CRT generator. `turn.rs` and
`Game::play_turn` also explicitly use engine-selected scheduling. Exact native
campaign decisions cannot be promised while silently substituting those inputs.

Recommended architecture to specify, **not an approved change**:

- Keep the observation-only player path and `builtin-1` contract intact.
- Add an explicit, trusted original-compatibility driver inside the engine.
  It can read an audited mechanics context; this does not make hidden data
  available to human/external players or to the ordinary `Observation`.
- Retain validated actions wherever possible, but represent native decision
  RNG consumption, cache updates and scheduling in versioned replay/checkpoint
  state too. Logging orders alone must not omit state consumed by the driver.
- Pin mode/version/source identity and original generator seeds; no per-civ
  seed substitution when asserting global random-sequence parity.
- Require owner acceptance of the explicit decision-0003 amendment before
  implementing privileged execution. A0's read-only inventory/probes can
  proceed without that amendment. If the boundary is retained unchanged,
  label the result original-derived rather than full campaign parity.

This is internal compatibility machinery, not the deferred external AI protocol.

## Ordered work packages and exit criteria

Each package is milestone-sized planning scope, not a promise that one release
can contain it. Split a package when its inventory exceeds a small reviewable
slice. Every implementation slice still needs independent milestone review.

| Gate | Deliverable | Depends on | Tracking |
|---|---|---|---|
| A0 | Native decision inventory, reference driver/trace protocol, state/RNG boundary decision and large-scenario baseline | Current evidence | #118 |
| A1 | Native production/economic decisions and faithful research integration | A0 boundary and inputs | #119; existing research evidence |
| A2 | Original settlement and worker missions, with executable terrain jobs | A0; A1 resource/production context | #112, narrowed to this gate |
| A3 | Original land military missions, movement, defense and conquest | A0; required mechanics from #113 | #120; #113 capture-mechanics dependency |
| A4 | Government, revolution and rate/budget decisions | A0; A1 economy | #121, split from #115 |
| A5 | Diplomacy, war decisions, diplomats/spies and exchanges | A0; A3/A4 state/action prerequisites | #115 with #96 mechanics dependency |
| A6 | Naval/air/transport and trade/space missions | A0; A1/A3/A5 as each mission needs | #114; #97 where endgame affects choices |
| A7 | Matched complete single-map turns/campaigns; standard-game completion | A1–A6 and the required missing mechanics | #122; #95 retains its city-only scope |
| A8 | Vanilla multiple-map/scenario AI; full-ToT completion | A7 plus the multi-map engine dependencies discovered in A0 | #123; coordinate with the engine's T4 map work |

A1–A6 can share infrastructure and interleave dependency-ready slices; this
is not an instruction to wait for whole subsystems before testing integration.
A7 comparisons start during A0 and expand at every gate. A8's inventory starts
in A0 even though its final implementation depends on multi-map mechanics.

### A0: ready to start, before further heuristic tuning

Deliver a machine-readable catalogue of decision families: native entry/caller,
reads/writes, persistent versus transient state, direct/transitive callees,
random streams and calls, public versus hidden inputs, presentation boundaries,
engine action/state gaps, tested branches and unknown branches. Trace actual
callers, including automatic decisions and barbarian control, rather than
assuming every decision enters through a single planner.

The live rig normally drives ToTPP-patched `civ2.exe`; that is not automatically
the unpatched reference pinned above. A0 must establish an unpatched live target
or prove and record relevant hook equivalence. Do not mix executable identities
in an unpatched-AI parity claim.

Use offline original execution for isolated logic; capture live native order
and state where the offline fixture cannot reproduce scheduling or context.
No game-mutating callee may be stubbed in an integration-parity claim. Follow
the oracle README and lessons before any live session. A UI-only replacement
must have its boundary and reason recorded separately.

Measure standard and large-scenario observation/decision/round cost and action
budget exhaustion. Define supported scheduling from native traces; raising or
silently exhausting the current 200-order cap does not prove native behavior.

Exit: catalogue covers every known decision family above, unknown call paths
are named, the compatibility-interface proposal is concrete for owner review,
and every following gate has an entry-point/dependency inventory and a revised
work estimate. If two probing approaches fail, split and record the blocker
per BACKLOG conventions; do not guess a fallback planner and call it native.

### A1: economic planner, including research context

Port 0x47e26b and its gameplay callees from original evidence; resolve the
0x434d81 wrapper/dispatch separately. Preserve Rules/leader/difficulty branches,
random consumption, automatic obsolete-build switches, production side effects
and immediate chooser calls. Inventory citizen/specialist/hurry/sell/wonder/space
branches instead of presuming the build selector contains them all.

Complete research cache/leader/permission integration and automatic follow-on
choices. Exit: matched eligible candidates, raw chosen IDs, effective production,
research choices, gameplay mutations and RNG state in a frozen case matrix.
Use fresh validation inputs: the current standard/FallRome/std1 set has already
been inspected and is now a regression set, not a new blind holdout.

### A2: expansion and terrain development

Separate citizen allocation from settler/engineer jobs. Port site valuation,
site reservation, expansion pressure, worker-versus-founder assignment,
resource/support limits and job continuation. Add validated improvement orders,
work progress and interrupted-job semantics as needed by the measured behavior.
Cover roads/rail, irrigation/mines, terrain changes, fortresses/airbases and
pollution tasks to the extent the vanilla inventory actually exposes them.

Exit: same native mission/site/job, path/continuation, completed terrain changes,
worker release and random/state effects for conflicting jobs, unsafe sites,
upgraded workers and scenario restrictions. No forced settlement heuristic may
remain in a branch claimed as original.

### A3: land missions and complete conquest

Port original garrison selection, threat/goal assessment, mission assignment,
path choice, attacks, defensive orders, retreat/healing and barbarian control.
Represent retained missions and invalidation when the world changes. Finish
#113's atomic move/combat/advance/capture and associated mechanics; add those
actions to validated execution and the new versioned compatibility transcript.

Exit: matching mission, target, step, attack/decline decision, unit orders,
occupation consequences and RNG on matched tactical states. Cover peace and
hostility prerequisites; A5 supplies complete diplomacy decisions. Repeated
attacks and `OccupyCity` in the current engine are regression behavior, not a
substitute for proving the original sequence.

### A4/A5: government, rates and diplomacy

A4 adds voluntary revolution/government selection, rate adjustment, budget
constraints and the necessary state transitions. A5 ports contact/attitude/
negotiation and war decisions, exchanges and diplomatic-unit missions, backed
by #96's mechanics/state work. Neither can be replaced by treating every foreign
unit as hostile or retaining the starting government/rates forever.

Exit: identical native decisions and resulting state for legal alternatives,
rejected alternatives, changing threats/economy, leader settings and relevant
human/AI masks; match random consumption and diplomatic consequences. Identify
any UI choice boundary separately from AI selection.

### A6: domain and strategic missions

Implement and prove mission/action pairs for transport load/unload/cargo loss,
naval movement/invasions, air basing/range/fuel/attacks, trade creation/delivery
nuclear policy and strategic space/endgame choices. Preserve cross-unit plans and port/landing
reservations. Dependencies include missing sea/air combat contexts, delivery
rewards and spaceship state; scoring policy work is separate from claiming
that an original victory condition has been reproduced (#97).

Exit: matching native tasks, units/cargo, paths/bases, deliveries, targets and
outcomes in isolated and multi-unit fixtures. A planner cannot pass while its
selected action is unavailable or approximated by the engine.

### A7/A8: completion is an integration claim

A7 starts from matched native/engine state, including transient caches, original
RNG states, turn phase, Rules, leader settings and human/AI masks. Compare ordered
decisions, resulting supported gameplay fields and RNG after each phase, first
for one civ turn, then full rounds and sustained regular-game campaigns. Use
first-divergence reports; classify each mismatch as input loading, mechanics,
scheduling, planner, presentation-only or unsupported context. Do not allow
opposing errors to cancel in final city/population totals.

A8 repeats that discipline on each shipped vanilla map/campaign mode, including
inter-map paths, travel/colonization, target selection and scenario restrictions.
Missing map simulation is an engine dependency, not a reason to omit that mode
from a “full original AI” claim. Versioned save/checkpoint state must preserve
native persistent mission/job/cargo/diplomacy/space state identified by A0.

## Acceptance protocol to freeze before implementation

- Kernel gate: all selected outputs, mutations and RNG states agree exactly
  on an explicit branch/input matrix; guard cases fail explicitly when unsupported.
- Integration gate: no unclassified gameplay mismatch in the admitted suite,
  and no replacement gameplay callees. Any approved exception narrows the label
  and supported scope; it is not a full-parity pass.
- Minimum suite design: early/mid/late standard positions; all six difficulties;
  all leader-specific branches, varied governments/human masks; land war, peace,
  worker contention, coastal/island transport, air operations, trade, space,
  barbarians, capture/elimination and scenario constraints. Construct a coverage
  matrix, not the full Cartesian product of every setting.
- Start sustained testing with matched one-turn cases, then 50 consecutive
  matched rounds at three regular-game starts and difficulties 0/2/5. Trace
  the original actually making its decisions. Expand counts/modes from A0's
  branch catalogue and freeze them before evaluating held-out positions.
- Keep original files private; retain hashes, authored/numeric fixtures, derived
  traces and reproducible commands. State all UI shims and unavailable checks.
- Engine determinism, order replay, compatibility-mode checkpoint continuation
  and browser observation remain separate mandatory regressions.
- At each completion gate, publish the coverage matrix, exceptions/unknowns and
  independent owner-run verdict. Do not substitute a completion percentage or
  “original AI included” headline for that record.

## Size, sequencing and estimates

This is **eight delivery gates for standard-game AI (A0–A7), followed by the
full-vanilla map/campaign gate A8**. Several contain multiple reverse-engineering
and mechanics slices; nine gates is a dependency breakdown, not nine promised
releases or nine short coding tasks. Research is partially ported; the other
planners are not almost finished because their mechanics already exist.

The critical path is A0 → native decisions plus action/mechanics dependencies
→ matched full-turn execution → A7 → map dependencies → A8. More heuristic
calibration does not shorten that path. A0 (#118) is the first implementation priority
within this AI program. Its entry catalogue and missing-state inventory are the
basis for estimating person-days/ranges for the remaining gates; a calendar
date before that would be invented. Record that estimate and assumptions on the
program issue when A0 exits, and revise it after the first full planner port A1.

v0.11 UI discovery/design remains next in the accepted release roadmap. Later
client versions and this AI program's release slots must be reconciled with the
owner after UI scope is known; this document neither postpones UI until A8 nor
pretends full native AI is scheduled in v0.10/v0.11. External AI protocols and
tournaments remain after the playable client.
