# Playable client: design proposal 01

**Status: awaiting owner review; not an accepted specification.** For #107,
v0.11, under [decision 0005](../decisions/0005-playable-client-planning.md).
[Reference findings](discovery.md) distinguish inspected behavior from unknowns.
The [clickable prototype](client-prototype.html) is a standalone local HTML file:
open it in a browser; no server or engine is needed. Static previews: [map](map-layout.png) and [city](city-layout.png). Its review navigation is
outside the proposed game window. All values and actions are authored fixtures.

## Layout proposal

Preserve the dense map/advisor/city structure seen in the reference. A thin
menu strip provides familiar command families and keyboard discoverability.
The empire status, active unit/terrain/mode and overview remain visible above
the map. Avoid making the map a collection of unrelated management cards.
The proposed dark, restrained palette and plain borders are our own styling,
not original stone textures or final art approval.

The city occupies the workspace: resource totals and production at left,
worked radius and citizens in the center, food/buildings/support at right.
The original puts city controls and a large information panel toward the upper
right; this proposal keeps the same information families but allocates them
more evenly. This is a design change for review, not a faithful pixel copy.
Original visual assets, if used later, must resolve from the user's install.
The primitive map and figures here are authored schematic drawings only.

At 1440 px the prototype presents three city columns. At 1024 px its third
column wraps below the worked radius. Desktop keyboard/mouse is the first
proposed input target; this does not establish a mobile or touch client.
A native window and browser may implement the same design; HTML is the review
medium and **does not choose a production toolkit**. Existing SDL2 story #94
is deferred until the design and deployment decision are settled.

## Interaction walkthroughs

| Screen           | Walkthrough to review                                                                                                                       | Result / cancellation rule                                                                                                                                                                                                          |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Map and units    | Inspect a tile in Viewing mode → switch to Moving → select adjacent destination → read preview → confirm. Zoom and center retain selection. | Proposed explicit confirmation prevents an inspection click from silently issuing an order. This adds a step versus the reference rig's observed click behavior; owner must judge its cost. Prototype movement only redraws a unit. |
| City             | Open Alder → inspect radius/output/storage/support → preview tile reassignment or cycle first citizen → inspect production.                 | Prototype markers and sample specialist numbers change, but do not calculate yields. Real client must show one consistent engine-derived preview/result.                                                                            |
| Production       | Select Settlers → inspect costs/support → Cancel, or Apply → return to city with new build.                                                 | Draft selection does not commit. Current item survives Cancel. Actual shield retention/penalties and legal choices must come from engine.                                                                                           |
| Research         | Open Science → inspect current progress and known advances → choose a legal project → Apply. Keep a distinct long-term goal view.           | Project-change availability and progress effects remain provisional pending discovery. Do not infer this workflow from the reference Goal dialog.                                                                                   |
| Rates/government | Adjust one rate, inspect total/cap and estimates → Apply. Start revolution → explicit consequence dialog.                                   | Prototype rebalances other rates to 100; this is proposed, not measured native behavior. Confirm performs a fixture transition; Escape/Cancel never commits. Government selection after anarchy needs further design.               |
| Diplomacy        | Select known civilization → inspect earned relationship/intelligence → send emissary → refusal or negotiation presentation → return.        | No hidden state exposed. Only the report, refusal and presentation entry were observed. Negotiation topics in prototype are placeholders, not a completed interaction spec.                                                         |
| Reports          | City rows show output/build → open a city → return. Additional advisors/world reports share clear drill-down and return paths.              | Prototype implements only the city drill-down; other report contents remain discovery work.                                                                                                                                         |
| Setup            | Choose standard/scenario → configure supported world/difficulty/leader settings → review roster/resources → start.                          | These settings consolidate only a subset of the observed standard wizard. Prototype creates no game. Scenario and multi-map restrictions need additional screens.                                                                   |
| Save/load        | Distinguish engine checkpoint from original import → show support/version → validate before replacing session → resume same human turn.     | Prototype downloads only a tiny labeled design-fixture JSON and previews an error. It does not load files or export native SAV.                                                                                                     |
| Turn progression | End turn with movable units → read explicit confirmation → cancel or confirm → results → next human decision.                               | Prototype changes a turn label only. A real client must preserve input ownership while opponents/events run and route resulting choices before accepting another move.                                                              |

Use the review strip to reach all nine layouts. The game menus expose a subset
of those same layouts. No production action, engine connection or disk mutation
is concealed behind an illustrative button; messages identify fixture behavior.

## State and engine boundaries the design exposes

`crates/engine/src/player.rs` currently declares exactly six orders: Move,
OccupyCity, SetProduction, SetResearch, FoundCity and EndTurn. It exposes own
city output/economy and partial visible map/unit information. This is enough
for some early client actions, not all screens above. Input focus, pan/zoom,
selection and draft dialog values are client state; accepted decisions, legal
choices, quotes and resulting gameplay mutations belong in engine APIs.

| Workflow          | Existing foundation                                                          | Missing work to schedule after design                                                                                                                                                   |
| ----------------- | ---------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Map/unit          | Move/attack, OccupyCity, FoundCity, EndTurn; own movement and visible tiles. | Unit scheduling, wait/skip/fortify/jobs, go-to, cargo/air/multi-map orders; legal-action/quote feedback; human turn/prompt driver.                                                      |
| City              | SetProduction; own output, finance, buildings, available improvements.       | Worker/specialist commands and detailed city-radius data; food/storage/support view; hurry/sale quote and commit; legal production list.                                                |
| Science           | SetResearch, ResearchView and own tech bits.                                 | Human choice prompt delivery, legal project/detail/goal model; validate active-project change semantics.                                                                                |
| Rates/government  | Government field and finance mechanics exist.                                | Rates view/commands/preview, revolution and government-choice phase; cancellation and persistence.                                                                                      |
| Diplomacy/reports | Partial capture/diplomacy state and power mechanics.                         | Earned contact/intelligence views, diplomatic commands/state machine (#96), report views; prevent disclosure through summaries.                                                         |
| Setup/save        | Newgame and campaign checkpoint/replay foundations.                          | Human session lifecycle, install resolver/errors, supported scenario setup, human-turn checkpoint/resume. Campaign policy checkpoints alone do not establish human-session persistence. |

Do not route the UI around `Game::apply` by handing out unrestricted mutable
WorldState. Extend the validated command/view interface where required and
preserve the observation boundary in decision 0003. The full native AI program
#117 remains gated behind the playable client; this design work does not
amend that decision or approve a trusted compatibility driver.

## Owner review requested

1. Is the reference-inspired menu/status/map arrangement and dense city
   workspace the right direction? Is matching the original arrangement more
   important than the proposed redistribution of the city panels?
2. Should Moving mode execute an adjacent click immediately, use the proposed
   confirmation, or offer confirmation only for attack/capture/destructive orders?
3. Which visual adjustments are needed before this becomes the basis for
   implementation? Review the supplied map/city images and interactive states.

These are concrete layout/interaction decisions, not permission to skip missing
discovery. Owner feedback should be recorded here with date and exact accepted
or revised choices. After closing coverage gaps, revise the design and then
freeze the first delivery slice. Do not label the initial prototype approved
because implementation work was generally authorized.

## Proposed implementation slices (unversioned, not yet scheduled)

These remain categories until discovery and owner feedback determine ordering
and release boundaries. Multiple categories may split into several releases.

| Candidate slice                    | Concrete completion check                                                                                                                                                                                                              | Dependencies / exclusions                                                                                                         |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Human turn and map workspace       | Human loads/starts a supported standard game; keyboard/mouse selection, movement/attack/capture/founding, visible rejection, turn handoff and return work through validated commands; fog respected; result/log replay checked.        | Define prompt driver, supported unit orders and rendering/assets. Not full city management or complete scenario parity.           |
| City/economic decisions            | A human assigns workers/specialists, changes production, buys/sells with correct quotes, handles growth/disorder; city and report values agree with engine; Cancel has no mutation.                                                    | Engine commands/views and measured reference interactions; persistence across city transitions.                                   |
| Science, government and diplomacy  | Human resolves research discoveries, rate/government transitions and supported contact/treaty choices; modal priority, earned intelligence and interrupted turns persist.                                                              | Later research-choice discovery, government state machine and diplomacy mechanics #96; likely several slices.                     |
| Campaign lifecycle and reports     | Supported setup/scenario import, all scoped reports and errors, save/reload at decision boundaries, missing-assets guidance; multi-map requirements explicitly scoped.                                                                 | Asset resolution and scenario support; native SAV export is not implicitly promised.                                              |
| Assembled sustained play/usability | Owner completes a new-game campaign segment and a scenario segment covering movement/combat, cities/economy/research, notifications and save/reload; no console edits or agent-only commands; session issues fixed and replay checked. | Duration/fixtures fixed with owner after design; include conquest, loss and recovery paths. Twenty-turn autoplay is insufficient. |

Each eventual issue needs a human-play route, keyboard route, supported fixture,
engine/API dependency, failure/cancel checks and its explicit limits. No client
release versions, milestone count, dates or complete-playability claim are
assigned by this proposal. v0.11 cannot close until discovery, owner-reviewed
design and accepted implementation decomposition satisfy #107.

## First-pass validation

The standalone prototype was checked in real headless Chromium (Playwright,
2026-11-06). All nine views opened without JavaScript errors. Checks exercised
production Cancel/Apply, specialist preview, research selection, rate totals,
revolution Escape/default Cancel, refusal presentation, report-to-city navigation,
setup preview, fixture download and error display, end-turn Cancel/Confirm and
no horizontal overflow at 1440 px / city layout at 1024 px. Unit-order controls
are disabled while a management screen is open. These checks validate the
prototype interaction states only; they are not engine, accessibility or human
usability acceptance. Captured previews contain only our authored drawings.

Owner review remains pending. The fresh setup/founding/initial-research and
native scratch-save sequence added in U15–U18 narrows discovery gaps; it does
not complete the coverage table in the discovery record.
