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
