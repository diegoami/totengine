# Playable client: accepted first slice and delivery breakdown

Owner-reviewed delivery plan, #107 / v0.11. The owner accepted design
revision 02 as a prototype and the first-slice scope on 2026-11-06. **The first playable milestone is assigned; later release versions and the
remaining milestone count are not.** These are dependent work packages; their release
boundaries must be chosen from the reviewed design. They are not a promise of
one release per row or a complete client in one milestone.

Inputs: [design proposal](client-design.md), [screen contracts](screen-contracts.md)
and [reference inventory/gaps](discovery.md). The [implementation audit and
architecture recommendation](client-architecture.md) identifies the driver,
visibility, resource and persistence gaps and six concrete prerequisite stories
for both clients. The complete original-AI program
#117 and pluggable AIs remain behind completion of the playable client. Neither
is a dependency of the UI work below. Existing built-in opponents are sufficient
for the human-play acceptance routes.

## Required delivery targets

Owner clarification, 2026-11-06: **both a playable browser client and a normal
standalone desktop client are required**. Tracking: desktop [#94](https://github.com/diegoami/totengine/issues/94),
browser [#125](https://github.com/diegoami/totengine/issues/125), shared design
[#107](https://github.com/diegoami/totengine/issues/107).
The current observer and simulated
browser prototype are useful foundations, not either complete playable client.
The desktop application must have an ordinary launch and its own game window;
opening the browser preview is not desktop delivery. Toolkit selection follows the desktop feasibility spike. Both surfaces must
reach the first playable milestone together; internal implementation order may vary.

Share the engine, validated gameplay commands/views, human decision/session
protocol and persistence semantics. Evaluate asset decoding/rendering reuse as
part of the architecture audit. Browser install access needs an explicit file
selection or local-host arrangement; the desktop has its own installation and
window/input lifecycle. No surface ships the user's original assets.

Every selected gameplay slice must say which surfaces it delivers and retain
tracked work for the other. Overall client completion requires sustained human
play on **both**, including launch, input, decisions, original-resource loading
and save/resume. One implementation can inform the other; one cannot substitute
for the other.

## Accepted first deliverable

Accepted first deliverable: a **human-controlled standard surface-game turn**.
The player can start a supported fresh game, inspect its known map, select a
unit/stack, move/attack, found a named city, end the human turn, see opponent
results and regain input. Keyboard and mouse paths must both work, with visible
rejections and explicit supported-format/resource errors. This is a narrower
claim than a playable full campaign: city/economic choice depth, government,
diplomacy, reports and scenario compatibility arrive through later packages.

The installed original artwork source and first scope are accepted. The
[architecture audit](client-architecture.md) defines the prerequisite work;
actual toolkit adoption depends on the desktop feasibility spike.

## Proposed first slice for owner review

**Accepted first slice, 2026-11-06.** This heading is retained for existing review
links. The owner's answers were “1 yes 2 both 3 yes 4 option 1 5 like the
original”: local browser hosting is acceptable; both surfaces ship together;
the generated-standard-game scope is accepted; immediate movement confirms
attacks/capture; city edits follow the original's immediate behavior.
No later release version is assigned. Acceptance replaces the earlier city-draft
recommendation; it does not approve the proposed later revolution Escape change.

**Deliver both clients for the first human-turn slice.** Use a packaged local
Rust host plus browser Canvas/DOM, and a standalone native desktop application
using the same Rust human-session and resource APIs. Evaluate SDL2 in the
desktop spike; require Windows/Linux window, text input, modal focus and scaling
to pass before committing production UI to it. Browser hosting without a local
executable is not included in this first proposal. Browser progress must not
stand in for the native desktop application.

**Use the installed Original profile and a freshly generated standard surface
game**, with explicit human civilization selection and the existing built-in
opponents. Include owned-resource selection, fog-filtered map inspection,
pan/zoom, unit/stack activation, adjacent surface land movement/combat and
supported city occupation, wait/skip scheduling, founding with a typed name,
basic own-city inspection/production selection, required research selection,
EndTurn/opponent processing and return to human input. Limit combat/capture
claims to current supported engine contexts and show unsupported actions
explicitly. Do not promise sea/air/cargo, full original AI, original SAV import,
native SAV export, scenarios/multiple maps, full city economics, government or
diplomacy in this first slice. Track original-save/scenario import in the later
lifecycle package; engine checkpoint resume is included here.

**Use these owner-selected interaction defaults on both clients:**

- Viewing inspects without spending movement. In Moving mode, an adjacent
  legal empty-tile move commits immediately; attacks/city capture and ending a
  turn with movable units require confirmation. Keep an optional confirmation
  for every move. An input received before selection changes must never apply
  to the newly selected unit.
- Worker/specialist and other immediate city edits commit directly through
  validated commands and refresh outputs/mood, like the original. No global
  Apply/Cancel city draft; closing a city does not undo accepted changes.
  Production selection remains a picker with explicit acceptance/cancellation;
  buying/selling still require a quote and confirmation.
- Required research/government choices can return to inspection, but cannot
  be dismissed into unrestricted gameplay or auto-answered. The later proposed
  safe revolution Escape behavior has not been selected by these answers and
  stays deferred with government implementation.
- Human checkpoints preserve pending decisions. A failed load leaves the
  current session intact. Native import and engine checkpoints receive distinct
  labels; unsupported formats/capabilities get specific errors.

**Accept the slice through both a scripted parity route and actual human
input.** On each surface, start the same supported setup, select a stack,
reject an illegal move, found/name a city, choose production and initial
research, fight a supported ground battle, end turns and regain control. Save
and resume both at normal input and at a required choice; compare the next
accepted command, result and state/replay. Test duplicate input, wrong/stale
choices, cancel, focus loss and missing resources. The owner plays at least ten
human rounds in each client, with keyboard and mixed input and no console help;
fix observed blockers. This is a proposed minimum for this narrow slice, **not
the sustained campaign acceptance for completing the client track**.

If a route reaches an unimplemented required decision, stop with a supported-
scope diagnostic and retain a resumable checkpoint; never choose for the human.
The ten-round fixture must avoid those unimplemented systems through declared
setup, not silent defaults. If required government handling cannot be avoided
in a useful standard route, promote that phase into this slice's prerequisites.

With this scope accepted, schedule the shared foundation,
driver, browser, desktop and checkpoint integration stories with the acceptance
above. Choose release boundaries from those dependencies and the desktop spike
results; later full-city, science/government, diplomacy, report/lifecycle and
scenario work remains separately scoped. A fixed number of later milestones
would still be premature.

## First-slice implementation record

The accepted first slice is [milestone 10](https://github.com/diegoami/totengine/milestone/10),
“Playable human turn — browser and desktop”, released as v0.12.0 at
`4ccf2cd62f4f6022cd045cf0a467dea5f7e46fbd` after AGREE on #134.
Both actual clients and checkpoint parity are implemented. The owner explicitly
confirmed ten human rounds in each with no blockers. Both adapters passed, and
milestone 10/#126–#131 are closed. [Release readiness and limits](../v0.12.0-readiness.md). The broad desktop #94
and browser #125 continue to track the complete clients beyond this slice.

| Story | Deliverable | Dependencies |
| --- | --- | --- |
| [#126](https://github.com/diegoami/totengine/issues/126) | Native desktop feasibility: Windows/Linux window, owned art, text/modal input, scaling and packaging | Informs toolkit adoption and shared resource API |
| [#127](https://github.com/diegoami/totengine/issues/127) | Resumable human driver, validated choices and canonical recording | Existing engine boundary; preserves autonomous behavior |
| [#128](https://github.com/diegoami/totengine/issues/128) | Shared resources/atlas and filtered human views | Existing evidence; coordinates with #126/#127 |
| [#129](https://github.com/diegoami/totengine/issues/129) | Engine-backed browser adapter with packaged local host | #127/#128; integrates #131 |
| [#130](https://github.com/diegoami/totengine/issues/130) | Engine-backed standalone desktop adapter | #126/#127/#128; integrates #131 |
| [#131](https://github.com/diegoami/totengine/issues/131) | Human checkpoints and both-client acceptance | Persistence starts with #127; assembled acceptance requires #129/#130 |

Each issue specifies a human route, real state/replay checks, input/error cases
and explicit exclusions. All six stories shipped in the first-slice milestone
with both clients. This does not assign a fixed count of releases for the whole
client track or a version for the next package. Subsequent packages below retain their own discovery and scope gates.

## Released slice: v0.13 city workers and specialists

The owner requested the next milestone after the v0.12 release. Milestone 11
selects the citizen-management portion of the reviewed city design, delivering
both clients together. Stories #135–#137 cover validated commands/own-city
views, the shared workspace and assembled continuation/human acceptance.
[Scope and evidence plan](city-citizens-milestone.md). v0.13.0 is released after
both-client owner acceptance and round-two independent AGREE on #138.
The next scoped package is v0.14 below; further versions and the remaining
milestone count stay unassigned.
Buying/selling and production-switch penalties remain separate prerequisites to
complete city management. Existing v0.12 owner acceptance does not pass the new
citizen-management acceptance route. Full original AI remains gated.

## Released slice: v0.14 city production decisions

Milestone 12 / #148–#151 selects buying, single-city building sale and
category-switch shield budgets from the reviewed city contract. Both clients
are implemented through #152; native comparisons and actual Windows/Linux/
browser inputs pass. [Scope](city-production-milestone.md) and
[readiness](../v0.14.0-readiness.md) retain the evidence and limits. The owner accepted the new both-client production routes on 2026-11-07;
first-round independent AGREE on #155 is complete and v0.14.0 is released. Deeper city panels and the packages below remain separate work; full
original AI and pluggable AI stay behind complete playable-client delivery.

## Released slice: v0.15 city information panels

Milestone 13 / #157–#159 selects Info, Support, Improvements and Happy from the
reviewed city contract. New copied-reference observations and original-x86 stage
snapshots back the immutable views. Both-client panels, new owner acceptance and
third-round independent AGREE on #162 are complete; v0.15.0 is released at exactly
`948b0d2b94d22315190c664340d080062629c8ea`. Final package input passes
54 Chromium / 47 Linux / 47 Windows checkpoint comparisons. Non-blocking
#176–#179 remain open for subsequent work.
[Scope](city-information-milestone.md), [readiness](../v0.15.0-readiness.md).
Further versions/counts and the complete client/AI gates remain unchanged.

## Dependencies and acceptance

| Package                                  | Deliverable and meaningful acceptance                                                                                                                                                                                                                                                                                           | Dependencies / boundary                                                                                                                                                                   |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Installation and presentation foundation | Resolve the user's install/scenario resources, render the user's installed artwork, show actionable missing-resource/unsupported-format errors, and render known map layers without exposing hidden tiles. Owner checks actual map/unit legibility at target window sizes.                                                      | Delivery/toolkit/style decisions; audit existing decoders/renderers and list missing formats. No bundled originals. An authored fixture render alone does not pass.                       |
| Human-session and decision driver        | Suspend/resume for input; stable decision IDs, quote/response validation, phase ownership, event/notification ordering and refresh after commits. Demonstrate a real engine session paused at founding and research, reject stale/wrong-owner responses, and continue deterministically from accepted choices.                  | Validated Game/Order/view boundary; preserve decision 0003. Add focused engine tests for real mutation/continuation behavior, not widget snapshots.                                       |
| Map and unit turn                        | Pan/zoom, inspect versus move, stack activation, keyboard directions, legal feedback, wait/skip and scoped jobs/orders. Complete the proposed first standard-game turn with original owned resources and a built-in opponent; no CLI/RAM intervention. Wrong-order inputs do not mutate the game.                               | Presentation + decision driver, required early-choice subviews, legal unit/order model and scheduling. Attack/capture/transport/context limits stated. Likely splits into several slices. |
| City and production decisions            | Work a tile, cycle specialists, inspect support/happiness, change production with invested-shield feedback, buy/sell using exact quotes, navigate multiple cities and handle growth/disorder. Confirm/cancel and all resulting totals agree with engine state and replay.                                                       | New validated citizen/hurry/sale/name commands and own-city views; production legality and refresh semantics. Bulk sale separate if approved.                                             |
| Science and government decisions         | Resolve initial/later research, preserve goal/project distinction, adjust legal rates/locks, run revolution→anarchy→government choice→rate review→return. Required choices cannot be skipped or accidentally accepted through unrelated shortcuts.                                                                              | Engine choice phases, legal lists/caps and quoted effects; additional eligible reference fixtures for later governments/research.                                                         |
| Diplomatic conversations                 | Known-contact/intelligence views and supported treaty/proposal/advance/gift/tribute responses operate through a resumable conversation. Demonstrate refusal, peace, exchange and a nested back/cancel route without hidden-state leakage.                                                                                       | Diplomacy model #96, decision driver and visibility rules; inspect remaining topic branches before claiming them. Full native AI reconstruction is not required.                          |
| Reports and reference help               | All scoped advisor/world reports use engine-derived fields, correct standard/scenario scoring, visibility, sorting/scrolling and return navigation. At least one report drills into an own city/unit and restores its origin.                                                                                                   | Economics/diplomacy/score/endgame mechanics as relevant (#97 remains a dependency for complete endgame reports). Early unavailable spaceship message does not pass full space UI.         |
| Human checkpoint/import lifecycle        | Save and resume at normal unit input, required city/research choice and a nested conversation. Reproduce pending choice, next accepted command/results and randomness; incompatible/truncated data cannot replace the current session. Original SAV import and engine checkpoint have distinct labels and support declarations. | Serialize human/session/event/policy state and versioned formats. Existing autonomous campaign checkpoints are a foundation, not acceptance. Native SAV export not implied.               |
| Scenario and multiple-map compatibility  | Play a scoped classic custom scenario and a shipped multi-map game: custom names/currency/date/art, layer identity, legal unit transfer and scenario event choices. Unsupported Lua/extension cases give specific diagnostics.                                                                                                  | T4/multi-map and scenario/event engine capability; evaluate Lua scenarios separately. A layer selector without validated transfers does not pass.                                         |
| Sustained human play and usability       | Assemble the above into owner-played standard and scenario sessions, fix discovered blockers, exercise keyboard-only and mixed input, save/reload and interruption/loss/recovery paths, then independently review the claimed slice.                                                                                            | All claimed packages. Define fixtures, duration and campaign endpoints with owner from reviewed design; no twenty-turn autoplay substitute.                                               |

Each selected release must be independently coherent, and each underlying issue
must name its human route, supported fixture, engine/API dependency, cancel/error
case, persistence/replay check and material exclusions. Some packages can proceed
in parallel after their contracts are agreed; others must split further. No
implementation estimate is reliable until the desktop/decoder spikes and first
human-turn integration are complete. The initial code audit is now recorded in
the architecture recommendation; it does not replace those implementation checks.

## Sustained-play validation to agree before client completion

Use a fresh standard game and at least one representative scenario, not only
generated autoplay. The owner must be able to reach the following outcomes
through the client without console commands or agent help:

1. On each client surface, navigate unknown/known terrain, activate a stack, issue and reject orders,
   found/name cities, fight and recover from unit loss, and regain control after
   opponents/events. Check selection remains stable under fast/duplicate input.
2. Manage a growing and an unhappy/starving city, reassign workers/specialists,
   change a build with invested shields, buy/sell with insufficient-funds and
   cancel cases, and trace resulting totals through city and reports.
3. Discover advances, resolve a project choice, change legal rates, complete a
   government transition and a scoped diplomatic conversation including a
   rejected demand and nested proposal.
4. Save/reload before and during required decisions; compare the next command
   and resulting state/replay, not only a matching screenshot. Attempt an invalid
   import without losing the current session.
5. Repeat relevant routes with scenario labels/resources/events, multiple maps
   where claimed, long lists/names, keyboard-only navigation and smaller windows.
   Exercise victory/defeat and late-game systems before claiming full campaign
   completion, with fixtures/duration agreed up front.

Record the human sessions, problems found, resolutions and explicit untested
contexts in readiness evidence. Automated renderer/command/replay checks support
this; they cannot establish that a person can understand or finish a game.

## v0.11 readiness gate

Owner review of the prototype, first deliverable and interaction choices is
recorded above. The accepted prerequisite decomposition is in the live queue above.
The v0.11 planning release passed owner-launched independent review on #132
and is tagged at the reviewed SHA. Implementation proceeds in milestone 10;
future release candidates follow the same owner-launched review process. This planning release
does not pass any future playable-client or sustained-human-campaign gate.
