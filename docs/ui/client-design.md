# Playable client: design proposal 02

**Status: prototype accepted by owner; detailed interaction and delivery decisions remain open.** For #107,
v0.11, under [decision 0005](../decisions/0005-playable-client-planning.md).
[Reference findings](discovery.md) distinguish inspected behavior from unknowns.
The [clickable prototype](client-prototype.html) is a standalone local HTML file:
open it in a browser; no server or engine is needed. Static previews: [map](map-layout.png), [city](city-layout.png), [reports](reports-layout.png) and [diplomacy](diplomacy-layout.png). Its review navigation is
outside the proposed game window. All values and actions are authored fixtures.
For the owner-directed visual pass using installed artwork, run the
[local asset preview](asset-preview.md). The committed PNGs remain schematic;
installed-art captures stay outside Git.
Detailed [screen/decision contracts](screen-contracts.md) and the proposed
[delivery breakdown](client-milestones.md) accompany this layout review.

## Layout proposal

Preserve the dense map/advisor/city structure seen in the reference. A thin
menu strip provides familiar command families and keyboard discoverability.
The empire status, active unit/terrain/mode and overview remain visible above
the map. Avoid making the map a collection of unrelated management cards.
The schematic revision uses our own dark palette and plain borders. The
owner-directed local artwork pass now uses installed city/dialog textures and
sprites; neither preview establishes final native window composition.

The city occupies the workspace: resource totals and production at left,
worked radius and citizens in the center, food/buildings/support at right.
The original puts city controls and a large information panel toward the upper
right; this proposal keeps the same information families but allocates them
more evenly. This is a design change for review, not a faithful pixel copy.
The local installed-art pass now resolves visual assets from the user's install.
The standalone file and committed previews retain authored schematic drawings;
neither mode computes native terrain connections or game mechanics.

At 1440 px the prototype presents three city columns. At 1024 px its third
column wraps below the worked radius. Desktop keyboard/mouse is the first
proposed input target; this does not establish a mobile or touch client.
A native window and browser may implement the same design; HTML is the review
medium and **does not choose a production toolkit**. Existing SDL2 story #94
is deferred until the design and deployment decision are settled.

## Interaction walkthroughs

| Screen           | Walkthrough to review                                                                                                                                                                             | Result / cancellation rule                                                                                                                                                                                                            |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Map and units    | Inspect in Viewing mode → switch to Moving → compare per-move confirmation and immediate-movement variants using the selector on the map → test an adjacent move/cancel.                          | Both are unapproved proposals. Viewing never orders. Native keyboard directions and activation are observed in U30; mouse behavior depends on preferences. No combat/fog/costs run in this fixture.                                   |
| City             | Open Alder → inspect radius/output/storage/support → preview a tile assignment or citizen role → inspect production.                                                                              | Entertainer/tax collector/scientist states are now observed (U19); prototype role/output values remain authored. Immediate native edits versus proposed previews require owner choice.                                                |
| Production       | Select Settlers → inspect costs/support → Cancel, or Apply → return to city with new build.                                                                                                       | Draft selection does not commit. Current item survives Cancel. Actual shield retention/penalties and legal choices must come from engine.                                                                                             |
| Research         | Open Science → inspect current progress and known advances → choose a legal project → Apply. Keep a distinct long-term goal view.                                                                 | Project-change availability and progress effects remain provisional pending discovery. Do not infer this workflow from the reference Goal dialog.                                                                                     |
| Rates/government | Adjust an unlocked rate, inspect total/cap → lock other rates and try an impossible change → Apply. Revolution requires confirmation; preview the post-anarchy government choice and rate review. | Native early-government sequence is now observed (U31/U32). Prototype Escape defaults to cancellation, intentionally differing from the installed revolution dialog. Redistribution is authored; extra governments remain unobserved. |
| Diplomacy        | Select a known civilization → send emissary → refuse incoming demands → accept a peace offer → open proposals → select an offered advance → return through proposals/topics/report.               | Branch sequence is based on U26. Only the authored treaty label changes; no real technology or diplomacy runs. Gift/tribute controls explicitly remain unobserved placeholders.                                                       |
| Reports          | Switch among nine report families, compare standard/scenario score forms, inspect early unavailable spaceships, and drill from Cities into Alder.                                                 | Layouts now have reference evidence (U23/U24). Values, public-information rules and additional drill-downs are not implemented mechanics; late space screens remain unknown.                                                          |
| Setup            | Review world size, six difficulties, civilization count, barbarians, rules/style and leader fields → preview a start.                                                                             | A proposed consolidation of the observed standard wizard, not a game constructor. Custom rules, scenario-start and identity/style details need further design.                                                                        |
| Save/load        | Distinguish engine checkpoint from original import → preview invalid-format and missing-Lua-capability diagnostics → download the labeled fixture selections.                                     | Diagnostic types are observed (U29/U33), but no file is loaded. Fixture JSON is neither a human checkpoint nor native SAV.                                                                                                            |
| Turn progression | End turn with movable units → read explicit confirmation → cancel or confirm → results → next human decision.                                                                                     | Prototype changes a turn label only. A real client must preserve input ownership while opponents/events run and route resulting choices before accepting another move.                                                                |

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
| City              | SetProduction; own output, finance, buildings and available improvements.    | Worker/specialist commands and radius/storage/support views; hurry/sale quotes and commit; legal production choices; human city naming.                                                 |
| Science           | SetResearch, ResearchView and own tech bits.                                 | Human choice prompt delivery, legal project/detail/goal model; validate active-project change semantics.                                                                                |
| Rates/government  | Government field and finance mechanics.                                      | Rates view/commands/quote, revolution/anarchy/government-choice phase and human-session persistence.                                                                                    |
| Diplomacy/reports | Partial capture/diplomacy state and power mechanics.                         | Earned contact/intelligence views, diplomatic commands/state machine (#96), report views; prevent disclosure through summaries.                                                         |
| Setup/save        | Newgame and campaign checkpoint/replay foundations.                          | Human session lifecycle, install resolver/errors, supported scenario setup, human-turn checkpoint/resume. Campaign policy checkpoints alone do not establish human-session persistence. |

Do not bypass `WorldState::apply_order` and the Game order-log boundary by
handing the UI unrestricted mutable WorldState. Extend the validated command/view interface where required and
preserve the observation boundary in decision 0003. The full native AI program
#117 remains gated behind the playable client; this design work does not
amend that decision or approve a trusted compatibility driver.

## Owner feedback and remaining decisions

2026-11-06: after receiving the GitHub design/plan links, the owner said:
“ok for a prototype is ok”. Revision 02 is accepted as a prototype and a basis
for continued planning. No specific redesign was requested.

The response does not select the movement variant, city-edit commit behavior,
production toolkit or first implementation deliverable/decomposition. Those
choices remain open; the prototype acceptance should not be requested again.
Further visual refinement can proceed from this prototype as the interaction
and delivery scope are settled.

These are concrete layout/interaction decisions, not permission to skip missing
discovery. Owner feedback should be recorded here with date and exact accepted
or revised choices. Review the explicit coverage gaps and scope them into the delivery plan,
revise the design and then freeze the first delivery slice. Do not label the initial prototype approved
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

## Prototype validation (revision 02)

The revised standalone prototype was checked in real headless Chromium
(Playwright, 2026-11-06). All nine main screens and nine report families opened
without JavaScript errors. Checks exercised both movement variants with
confirmation cancellation, three specialist roles, production Cancel/Apply,
research selection, 100% rate conservation and impossible locked-rate rejection,
revolution Escape/default Cancel, government-choice preview, nested diplomatic
reply/proposal/advance/back/end routes, treaty label refresh, report-to-city
navigation, six setup difficulties, fixture download and two diagnostic types,
end-turn Cancel/Confirm and 1440/1024 layout overflow checks.

These validate authored interaction states only; no engine runtime or original
AI port changed. They do not replace accessibility/usability or sustained human
play. All four previews contain only our authored drawings/text/fixture values.

The reference inventory now has 33 representative route groups across standard,
FallRome, Fantasy and a classic custom scenario, plus explicit Lua/invalid-file
rejection. The detailed gaps remain in the discovery record and screen contracts.
Prototype acceptance is recorded above. Detailed interaction decisions and
acceptance of the first deliverable/decomposition are still pending. No milestone candidate or substantial client implementation is
claimed by this package.
