# Playable client: proposed delivery breakdown

Draft delivery plan for owner review, #107 / v0.11. The owner accepted design
revision 02 as a prototype on 2026-11-06; delivery scope is still to be settled. **No later versions or implementation
milestones are assigned.** These are dependent work packages; their release
boundaries must be chosen from the reviewed design. They are not a promise of
one release per row or a complete client in one milestone.

Inputs: [design proposal](client-design.md), [screen contracts](screen-contracts.md)
and [reference inventory/gaps](discovery.md). The complete original-AI program
#117 and pluggable AIs remain behind completion of the playable client. Neither
is a dependency of the UI work below. Existing built-in opponents are sufficient
for the human-play acceptance routes.

## First deliverable to approve

Proposed first deliverable: a **human-controlled standard surface-game turn**.
The player can start/import a supported game, inspect its known map, select a
unit/stack, move/attack, found a named city, end the human turn, see opponent
results and regain input. Keyboard and mouse paths must both work, with visible
rejections and explicit supported-format/resource errors. This is a narrower
claim than a playable full campaign: city/economic choice depth, government,
diplomacy, reports and scenario compatibility arrive through later packages.

Before assigning it a version, settle desktop delivery/toolkit, map/asset style,
movement confirmation and the minimum human-choice handling needed when the
first turn reaches founding, production or research. Do not ship a map that
silently auto-answers those required decisions or consumes the human's orders
through an AI action loop. Discovery proved these choices arise early.

## Dependencies and acceptance

| Package                                  | Deliverable and meaningful acceptance                                                                                                                                                                                                                                                                                           | Dependencies / boundary                                                                                                                                                                   |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Installation and presentation foundation | Resolve the user's install/scenario resources, support owned asset rendering or explicitly approved replacement art, show actionable missing-resource/unsupported-format errors, and render known map layers without exposing hidden tiles. Owner checks actual map/unit legibility at target window sizes.                     | Delivery/toolkit/style decisions; audit existing decoders/renderers and list missing formats. No bundled originals. An authored fixture render alone does not pass.                       |
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
implementation estimate is reliable until the asset/driver audit and first
human-turn integration are complete.

## Sustained-play validation to agree before client completion

Use a fresh standard game and at least one representative scenario, not only
generated autoplay. The owner must be able to reach the following outcomes
through the client without console commands or agent help:

1. Navigate unknown/known terrain, activate a stack, issue and reject orders,
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

This planning release needs owner-reviewed layouts/interactions and an accepted
first deliverable plus decomposition, followed by the normal independent
milestone review. The current proposal, screen contracts and prototype are ready
for that **design discussion**. The prototype is accepted as a planning basis; specific interaction and delivery
alternatives remain open. No v0.11 version bump, release candidate, implementation
milestone creation or review launch is justified yet.
