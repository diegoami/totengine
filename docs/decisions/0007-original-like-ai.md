# v0.10.0 — original-like built-in AI

Issue #106 is the acceptance authority. The target is a recognizable original
opponent, backed by controlled comparisons, in generated campaigns and the
browser observer. AI v1 is the baseline, not the acceptance result. UI design
remains v0.11; pluggable AIs follow the playable client.

## Implementation and comparison sequence

1. **Research.** Decode selection and context-dependent advance values. Check
   controlled technology trees, leader settings and difficulties against the
   original executable, including random consumption. Integrate through
   `Observation`/`SetResearch`, with deterministic logs and replay.
2. **Expansion and development.** Compare founding sites, expansion pressure,
   settler support and city build choices in controlled early/midgame positions.
   Decode the production selector (0x434d81) and its valuation boundaries before
   describing engine heuristics as original behavior.
3. **Defense and war.** Compare garrison retention, target choice, movement and
   attacks in matched positions, including weaker/stronger opponents and multiple
   difficulties. Keep observation limits explicit. Identify transport and
   diplomacy dependencies as concrete gaps, rather than silently supplying
   hidden foreign state to the player API.
4. **Campaign acceptance.** Compare trajectories and behavior in regular games
   over several seeds and difficulty settings. Demonstrate the opponent in the
   observer. Retain numeric evidence, scenario descriptions and reproducible
   checks; enumerate remaining differences. A deterministic run alone is not
   evidence of original-like play.
5. **Owner-run independent milestone review.** Prepare readiness evidence and
   the prompt from `docs/milestone-review.md`, then stop for the owner to launch
   review. Do not tag a release before its candidate SHA has an AGREE verdict.

Each comparison must state whether it executes a complete native decision or
isolates a kernel using supplied callees. Original executable bytes and saves
stay outside Git. Keep v0.9's AI v1 replay/checkpoint contract stable; introduce
an explicit policy identity when the campaign opponent changes.

## Research selection kernel (2026-11-05)

`engine::ai_research` implements the selector at 0x49f98b. AI candidates receive
`CRT rand() % value` when value > 1, otherwise zero without a draw. Ascending
advance order and strict comparison retain the first tied score. The human
recommendation branch instead adds `rand() % 3 - 1` and above Chieftain filters
later advances congruent to acquired-count modulo three. This is substantially
different from AI v1's highest-static-value policy.

`tools/ai_research_choice_check.py` executes the original selection loop and CRT
in 512 controlled contexts, comparing winner, final CRT state and draw count.
It replaces **availability (0x49ecb6), group permission (0x4a2098) and advance
value (0x49c95d)** with supplied test inputs. The original's direct check for
two `no` prerequisites still executes. All six difficulties, human/AI branches,
empty/singleton/zero/tied candidates, mixed gates and signed integer boundaries
are covered; 128 native output vectors run without the install in Rust tests.
This establishes only the selection kernel, not original research decisions.

At this point 0x49c95d and policy integration were still open; the following
slice completes the valuation kernel. Campaigns still use AI v1. Production,
expansion, military, transport/diplomacy gap assessment and end-to-end acceptance
remain open.

Validation: install-backed `make test` passed all 195 Rust tests (one nightly
test intentionally ignored), the five Python report tests and full golden
corpus, including the 50-turn fingerprint and decoder crosschecks. Workspace
formatting and all-target Clippy with warnings denied passed. A second native
probe run re-derived both retained artifacts byte-identically.

## Complete research valuation kernel (2026-11-05)

`ai_research::value` now ports the complete 0x49c95d routine from bounded
classic Rules/cache inputs. It covers signed Rules value/modifier, effective
leader preference, all 21 leader-specific bonus cases, technology dependency
chains, military priorities, the naval-count bonus, city growth thresholds,
active wonder/expiry handling, human nuclear/space technology ownership,
discovery bonuses and known dependent advances. Final values clamp to one.
Dependency traversal guards malformed cycles; native parity is tested on
acyclic trees, not native stack-exhaustion behavior.

`tools/ai_research_value_check.py` executes **12,800 valuations and 128 complete
research selections with no replacement callees**. Availability, group checks,
dependency traversal, knowledge, wonder ownership/expiry, scoring and CRT all
execute original bytes. Retained authored numeric contexts/outputs exercise
all leader cases, human/AI masks, signed modifiers, active/expired wonders,
deleted cities, landmass boundary IDs and all six selection difficulties.
The Rust test checks every advance value, selected advance and final CRT state.
The full-choice test supplies the original eligibility outputs to the Rust
selector; the engine's future eligibility adapter is not established by this.

One unusual behavior is preserved: 0x49cc67 rereads the current civilization's
landmass presence inside its loop over other civilizations. The enclosing
own-presence guard therefore prevents its "isolated" branch from becoming
true. This makes that military-value increment much simpler than an intended
foreign-presence interpretation would suggest. The stronger-human adjustment
earlier in the routine really does compare both civilizations' landmass counts.

This API is a **mechanics context**, not a new player observation. It contains
foreign knowledge masks, human acquired counts, reputation, treaties and shared
landmasses. Decision 0003 forbids silently exposing hidden foreign state and
requires players to use private RNGs. Native research uses the same global CRT
as other mechanics; the engine cannot promise global draw-for-draw campaign
parity merely by seeding a private player CRT with the same number.

Next implementation steps:

1. Add strict parsed Rules and world-cache adapters, with install-backed
   comparisons; verify leader trait inputs rather than guessing defaults.
2. Define the observation-safe research context and document any approximation
   to original privileged information. Preserve decision 0003's boundary.
3. Introduce an explicit new AI policy identity and checkpoint handling, wire
   research through `SetResearch`, and verify continuation/order replay and
   browser observation before changing the campaign default.

Validation: all 196 install-backed Rust tests passed (one nightly test
intentionally ignored), along with the five Python report tests, full golden
corpus including run50 and decoder crosschecks, workspace formatting and
all-target Clippy with warnings denied. Both native research regressions pass;
a second valuation probe reproduced its report and numeric fixture byte-for-byte.

## Parsed Rules and saved-world adapters (2026-11-06)

`ValueRules::from_rules` now resolves the classic scoring tables and rejects
missing sections, unresolved references, malformed definitions, out-of-range
signed values and unsupported capacities. Unused unit/advance slots are
disabled. A trailing text-only unit annotation is accepted, as are unused
trailing rows beyond the native 28 wonder/21 leader tables. The adapter needs
the classic 68 improvement rows and 28 expiry rows; it does not invent these
for a smaller Rules file.

`ValueContext::from_world` reads the scoring flags from **save @660**
(`ResearchSettings::flags`), rather than @652 (`WorldState::game_flags`). It
uses Rules leader field 9, the world's cached contact/landmass inputs, current
knowledge masks and a freshly counted live domain-2 roster. It does not mutate
the world or consume world RNG. This remains a privileged mechanics context,
not an observation. Custom human-leader overrides stored in saves remain
outside this adapter's contract.

The native leader loader at 0x41447d verifies field-9 storage for 42 rows from
two Rules files. Only section/row/token readers are replaced in this separate
loader check. The valuation/selection routines have no replacement callees.
The native reference resolver at 0x490440 verifies all 142 encountered tokens;
FallRome's `*Ind`/`*E2` comments must resolve plain `Ind`/`E2` references, and
matching is case-insensitive. This adapter normalizes the annotation; it does
not rewrite the general TechTree resolver or old AI v1 checkpoint behavior.
Arbitrarily relabelled comments/native fixed identifiers still need separate
compatibility work; this is not a port of the original text loader.

`WorldState::native_research_candidates` combines valuation with original
ownership/prerequisite checks and **0x4a2098's zero-group-flag gate**. Group
flag 1 remains permitted by the broader player research interface but is
excluded from the native AI selector; this distinction is tested explicitly.
The new context does not change what `SetResearch` permits a player to request.

`ai-research-adapter.json` and its numeric fixture record 1,400 native scores,
1,400 eligibility results and 84 complete choices: both retained Original and
FallRome positions, all seven civs, all six selection difficulties. The Rust
install test boots each source through the actual decoder and checks every
value/gate, winner, final CRT state and unchanged world checksum. Ownership
and naval inputs are reconstructed before native execution; the other saved
caches are held fixed. In FallRome the saved naval word differs from the live
roster, as recorded in the artifact. No full native statistics refresh is
claimed by this comparison.

Observation-safe policy integration and explicit campaign/checkpoint policy
identity remain next. Campaigns still use AI v1; no milestone completion or
live original-AI round parity is claimed.

Validation: install-backed `make test` passed all 199 Rust tests (one nightly
test intentionally ignored), five Python report tests and the full golden
corpus including run50 and decoder crosschecks. Workspace formatting and
all-target Clippy with warnings denied passed. The native adapter probe's
report and numeric fixture reproduced byte-for-byte.

## Observation policy, campaigns and occupation (2026-11-06)

`BuiltInAi` (`builtin-1`) now drives `totctl play/observe --builtin` and
`totctl campaign --policy builtin-1`. Its research scorer receives own
knowledge, own city sizes/wonder hosts/naval count, Rules leader identity and
public game/scenario settings. Foreign knowledge, contact/reputation and
landmass caches remain unknown. The CRT selector uses a private per-player
stream, never the world's combat stream. Its choices consequently need not
match the privileged native-context comparisons above. Tests alter foreign
knowledge/caches and require unchanged observations and choices.

Generated checkpoints explicitly record this policy in format 2. Format 1
retains AI v1 and its existing golden transcript. Restore reconstructs private
policy state by playing the prefix, rejects unknown policy identities and
checks the complete canonical checkpoint. The CLI defaults to the built-in policy for generated campaigns and browser
observation. Explicit legacy options remain available. `Campaign::new` retains
AI v1 for existing callers; the CLI uses `with_policy`. Saved-game `play`
retains its random-player default and selects the new opponent with `--builtin`.

The new policy also changes production and movement. It reserves a city
garrison, prioritizes urgent defense/happiness/growth, budgets expansion and
field armies, and selects affordable land attack strength. Settlers seek
known food-rich sites away from known cities and visible enemies. Field
units route over known land toward missing garrisons or visible targets;
attacks use a conservative visible strength estimate. These are engine
policies calibrated with original production choices, **not** a port of the
native unit-AI or complete build-scoring dispatcher. They retain the player
observation boundary and cannot cross the map seam.

War testing exposed missing city ownership transfer. The explicit
`OccupyCity` order now validates a land combat unit, an adjacent live foreign
surface city, available movement and absence of foreign defenders. It
charges ordinary movement, transfers treasury, applies building/population
loss, removes or rehomes displaced units, transfers ownership and refreshes
city statistics. A surviving city fires CITYTAKEN through the existing event
engine; Leonardo ownership is evaluated after transfer. A zero-population
city uses the already-decoded destruction path. Existing `move` transcripts
retain their pre-v0.10 semantics; this explicit order is required for capture.

The capture loot kernel has 75 native vectors, alternating-bit building
attrition has 192, and non-barbarian military population loss has 576,
including final CRT states. The building phase uses **one** draw to choose a
shared alternating mask for all five bytes, rather than independent building
rolls. Recapture skips attrition and protects size-one cities. Population
handling retains the original saved-rank/human-knowledge exceptions for
size-one AI cities. The retained evidence executes unmodified instruction
ranges with authored caller frames and no replacement callees. It does not
assert parity of the enclosing capture dispatcher.

Capital relocation/civil war, technology plunder, treaty consequences,
civilization respawn/elimination, sea cargo, worker improvements and naval
transport remain concrete gaps. The engine transition currently chooses a
fresh production item through its normal player policy instead of executing
the native capture dispatcher's immediate chooser. Its visibility and event
ordering remain engine-defined. No whole-capture or whole-campaign native
parity is claimed.

Concrete follow-ups are tracked in #112 (worker/scenario production), #113
(remaining capture consequences), #114 (transport and large-scenario turn
scaling), and #115/#96 (government/rates/diplomacy-aware planning). They have
no assigned release and do not change v0.11's owner-reviewed UI discovery
priority. The inherited 200-order per-civ turn budget is a material limit on
large scenarios, alongside expensive repeated observations; the standard-game
demonstration does not erase those limitations.

Candidate validation: install-backed `make test fmt clippy` passes 209 Rust
tests (one nightly test intentionally ignored), five Python report tests and
the complete golden corpus, including run50 and decoder crosschecks. The
separate autoplay suite passes 32 seeds for each policy (64 campaigns). Both production artifacts
reproduce byte-for-byte on v0.10. The capture probe additionally verifies all
eight previous-owner encodings; an authored occupation test starts with a
different saved history owner and verifies its replacement. The legacy
checkpoint golden changes only its package-version header to 0.10.0; its
order transcript, fingerprint and log identity are unchanged.


## Subsequent full-original-AI scope (2026-11-06)

After the candidate, the owner asked when the complete original AI would be
included and requested a concrete scope. [The full-port plan](../original-ai-scope.md)
and program #117 now separate standard-game completion (A0–A7) from full
vanilla ToT maps/scenarios (A8). The existing opponent is original-derived;
this candidate does not contain the complete original decision algorithms.

A0 (#118) starts with native decision/callee/state inventory, scheduler/RNG
traces, missing action/mechanics dependencies and a concrete fidelity-boundary
proposal. Decision 0003 is not silently amended by this scoping work. The
existing v0.10 candidate and #116 review remain pinned to their recorded SHA.
v0.11 UI discovery/design retains its accepted position. Full-AI release slots
and calendar estimates depend on A0's inventory and later roadmap reconciliation.
