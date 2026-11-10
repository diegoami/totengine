# Playable-client screen and decision contracts

Mission alignment, 2026-11-08: [decision 0008](../decisions/0008-scenario-platform.md)
keeps these original-UI contracts as the baseline and adds optional QoL and
recording. City queues (#189) require explicit automation/completion/legality/
shield-budget and persistence semantics. Rally points (#190) require known-map
routing, manual-order priority and interruption/transfer semantics. Neither is
implemented or implicitly covered by the existing production/movement routes.
Session recording (#186) separates canonical orders/events/checkpoints from
viewpoint, camera and notes. Define new controls/settings in a concrete reviewed
contract before implementation; preserve both client surfaces and original
scenario behavior. Scenario entry/author assistance is tracked by #183.

Owner-reviewed planning contracts under #107 / decision 0005 (2026-11-06). This specifies proposed
behavior and required information, not implemented engine APIs. Route IDs refer
to [reference discovery](discovery.md) and its capture manifest. The
[prototype](client-prototype.html) is an authored layout and interaction aid.
Native and proposed differences are explicit below.

## Required browser and desktop surfaces

The owner requires both a playable browser client and an ordinary standalone
desktop client. These contracts apply to both. Share gameplay and decision
semantics; validate launch, input/focus, resource access, save/resume and human
play separately. Browser file access/hosting and desktop packaging/windowing
need explicit implementation plans. The existing observer and simulated design
prototype are not the playable browser client, and do not replace desktop work.

## Original interface baseline

The owner explicitly requires a more faithful UI after locating the publisher's
manual. [Original controls](original-controls.md) records the manual/page references,
installed menu declarations, observed behavior, corrected bindings and remaining
UI differences. Use these sources alongside discovery captures when refining
screen contracts; the authored prototype does not authorize conflicting key
meanings or replacement native workflows. Both client surfaces share this basis.

## Input ownership and pending decisions

A human session needs more states than the current synchronous `Player::act`
loop. Proposed session states are: awaiting a human order, processing accepted
orders/opponents, awaiting a required choice, presenting a notice, and finished.
An inspectable snapshot does not imply that commands are currently accepted.
Only the current decision may commit while a required choice is pending.
Reports opened from a decision return to that decision, not an unrestricted map.
Waiting for input is a session state, not `EndTurn`, a default answer or a busy
`Player::act` loop. UI inspections and time spent thinking must not consume an
AI action budget or cause an automatic human turn advance. Interrupt/resume
must preserve the pending decision and reject duplicate submissions.

Each required choice should have a stable decision ID, owning civilization,
kind, allowed responses, relevant quoted consequences, and continuation target.
Responses must reject stale IDs, replaced targets and no-longer-legal choices.
The session must return a fresh view after each accepted response; retaining
pre-commit cost or treasury labels is not acceptable client behavior. Quotes
are tied to their source state, and revalidation belongs to the engine.

Persist the current phase/decision and its context, not UI object addresses.
On reload, reconstruct the same required choice and input owner before
accepting another order. Existing campaign checkpoints reproduce autonomous
policy state; they do not establish a resumable human decision protocol.

## Map, units and multiple worlds

Evidence: U04, U16, U27–U30. U30 distinguishes cursor movement, stack activation,
unit movement/exhaustion, wait/skip and zoom. Mouse-directed movement depends
on preferences (U25); it is not one unconditional native click rule.

Proposed state: current map/layer, viewport/zoom, inspection cursor, selected
unit or stack, Viewing/Moving mode, current input owner and any pending order.
Inspection selection, activation and gameplay movement are distinct operations.
A one-key cursor step must not spend unit movement. Pan, zoom and opening a
report must not change the selected unit's orders.

Required view: known tiles for the selected layer, own/visible units with
stable IDs, civilization/name/type/home/health/veterancy/orders/moves, visible
cities and stacks, legal destinations/actions with rejection reasons. Keep
unknown tiles and foreign private details absent. Terrain, resource and job
labels resolve through the current rules/scenario, not standard English names.

Entry: standard start/import or return from another screen. Viewing mode
inspects. Activating a multi-unit tile opens a chooser; selected unit becomes
the order target. Moving mode offers legal orders for that type and context.
The owner selected immediate empty-tile movement with attack/capture
confirmation; optional confirmation for every move remains available. Viewing
never moves a unit. Other destructive actions retain their own explicit quotes
and confirmations. An accepted order returns movement/combat/capture/founding or a
specific rejection. Advance selection after exhaustion only after the result is
presented; never send another queued click to a newly selected unit.

Founding needs a name decision, validation, confirmation/result and city entry
(U16); the current `FoundCity` order has no human name parameter. Wait, skip,
activate, fortify, sleep, disband, home/job and path orders require explicit
engine/session contracts; they cannot all be represented by `EndTurn`.

Multiple maps: distinguish viewing a layer from **moving a unit** to another
world. Fantasy's observed order opens a destination list (U27). Use legal
transfers and explicit current-layer feedback, honor unit/terrain/scenario
restrictions, and preserve the unit's identity across the transition. Engine
surface support does not authorize exposing a decorative layer switch as
working multi-map gameplay. Cargo, air, teleport and off-map arrivals remain
separate required contracts, with detailed reference cases still open.

Keyboard: provide discoverable bindings for modes, activation, directions,
wait/skip, zoom/center, founding, city/report access and ending the turn. No map
shortcut fires while editing text or answering a dialog. Remapping and final
shortcut collision rules remain a design choice.

## City management and production

Evidence: U05–U07, U19–U22. Reference worker/specialist edits immediately refresh
resource totals. A removed worker becomes a specialist; clicking that specialist
cycles roles. U19 captures entertainer, tax collector and scientist states.
U21 separates Info, Support, Improvements and staged Happiness panels.

Required own-city view: identity/owner/location/layer/name, population and
citizen roles, worked/available radius tiles, raw and net outputs, food storage
and growth/starvation status, shield box, build/remaining cost/turn estimate,
finance/upkeep, buildings/wonders and effects, supported versus present units,
commodity supply/demand and routes where available. Do not force a player to
infer this from totals or expose an unrestricted world snapshot to supply it.

Tile reassignment and specialist changes are validated commands with resulting
resource and mood views. The owner selected immediate worker/specialist edits
like the original, without a global city draft or Apply/Cancel layer. Refresh
outputs and mood after each accepted edit; rejection leaves gameplay unchanged.
Closing the city does not undo accepted edits. Production pickers and buy/sell
confirmations remain explicit decisions, rather than committing a purchase or
selection merely because its row was inspected.
Closing a city does not implicitly end a human turn in the proposed session;
end-turn preferences and unit scheduling need their own visible rules.

Production choice separates the current item from the draft selection and
recommendations. Item details include type/cost/support, legal availability,
obsolescence/upgrade and invested-shield effects. Apply commits through the
engine; Cancel preserves the current item. U22 proves one ship→engineer choice
with invested shields in one scenario context, not all category penalties.

Buying: request a state-specific quote; show item, completion cost, treasury and
when the item actually appears. Reference cancellation and completion are
observed in U20; filling the shield box is not immediate unit creation. Selling:
show building, income, lost effect and scope. The installed reference offers
single-city and all-city sale alternatives (U21); vanilla equivalence is not
proved. A bulk sale must have its own explicit scope/quote and must not be
silently added to an ordinary single-city command.

City navigation retains the chosen information panel, updates every city-bound
field and never applies a pending draft to a different city. Observe more cases
before fixing exact sort/next-city ordering. Opening a unit from support/present
lists must identify whether it changes selection, wakes a unit or closes the
city; these are not interchangeable actions.

## Science and goals

Evidence: U08, U17. A required project chooser and an optional long-term goal
chooser are different screens. The advisor displays known advances, active
project/progress and discovery estimate. A required project choice has an
engine-provided legal list; response commits the chosen project and resumes the
suspended phase. Help/goal/unlock detail returns to that same choice.

The long-term goal does not grant technology or independently replace the
active project. The prototype's project selection is a required-choice layout
preview, not proof that arbitrary advisor changes should be allowed.

**Native rules (#194; evidence §Research selection, goal and mid-project
changes).** There is no human path that changes a *running* project: the target
changes only at discovery (reset, progress to 0), when the civ gains the advance it
is researching by other means (target cleared, **progress kept**), in the
chooser, and through scenario/event actions. The chooser opens when science is
added with no target and non-zero progress, and after a discovery with no
target. Its list applies enabled/unknown/prerequisite filters, the leader
tech-group gate (flag must be 0) and, for a human at non-zero difficulty, a
thinning rule that hides every later candidate with `(id − acquired) % 3 == 0`.
The goal is one global value, not saved with the game; accepted candidates on
its path are pre-selected but the prompt still appears. Building the list
consumes the CRT generator.

**Contract (implemented in v0.18.0, #195/#196).** Keep the required `Research`
decision as the only way to set a project; do not add an advisor "change
project" command. The decision's list comes from one engine function carrying the
native filters. Goal is client/session state with its own explicit checkpoint
field (the original does not persist it), cleared on discovery of the goal.
The suggestion shown with the list is `Decision::Research::default`, the first
candidate on the path to the goal; it is part of the decision payload so replay stays
deterministic. The original's recommender (its CRT consumption) is not implemented.

**Owner decisions, 2026-11-09 (accepted and implemented in v0.18.0).**

1. *Candidate list fidelity:* **adopt the native rules exactly** (enabled,
   unknown, prerequisites, group gate with flag 0, both-`nil` exclusion and the
   human thinning rule). #195 ports them into one engine function; accepted human
   checkpoints are regenerated and the change is called out in release notes,
   since players lose some previously offered choices.
2. *Goal persistence:* **persist the goal in checkpoints** as explicit session
   state outside the original format, cleared when the goal advance is
   discovered. Native reload would forget it; this is a documented difference.

## Rates and government

Evidence: U09/U10, U31/U32. The isolated installed-reference Escape test started
a revolution when Yes was selected. The proposal deliberately makes Escape
cancel a destructive confirmation; this is an owner-review difference.

Rate view: current government/caps, three shares totaling 100%, locks, treasury,
net income/upkeep and research estimates from a consistent quote. Client lock
state may constrain the draft, but the engine validates committed shares and
caps. An impossible adjustment must retain the last valid values with a reason.
The prototype's redistribution is authored; U32 proves only the displayed
40/60/0→50/50/0 example and lock indication, not an entire native slider rule.

Revolution: show consequences → explicit Confirm/Cancel → anarchy state →
required government chooser → rate review → resulting notice → resume input.
U31 reaches the chooser with only Despotism legal, then the rate screen and
newspaper notice. The number of anarchy rounds is fixture-specific, not an
inferred general duration. Additional government choices/effects need eligible
fixtures and mechanics coverage before implementation acceptance.

### Native rules and proposed contract for milestone 16 (#194)

Native facts, with their evidence and static/executed status, are in
[evidence §Rates, revolution and government selection](../evidence/README.md).
Summary used below: the per-slider cap is 6/6/7/8/8/8/10 tenths for Anarchy,
Despotism, Monarchy, Communism, Fundamentalism, Republic, Democracy; the
redistribution rule and locks are the executed `0x409704` algorithm; governments
2–6 need their advance or an active Statue of Liberty, Fundamentalism also needs
its advance enabled in the loaded rules; Anarchy ends at the first per-civ turn
step whose turn counter is a multiple of four (every step with Statue of
Liberty); the chooser lists governments 1..6 filtered by eligibility.

**Engine surface (#195).** *Implemented:* the rates slice, then (slice 2) `Command::Revolution`, `Decision::Government`, `Decision::Rates` (resolved by `Command::ReviewRates`) and anarchy resolution at the start of the human turn, all replayed through checkpoints; `Phase::UnsupportedGovernment` is gone. Research slice: the human prompt/`SetResearch` list uses the native filter and thinning (`engine::research::human_research_list`), `Command::Goal`/`HumanView::goal` persist the goal in checkpoints and `Decision::Research::default` follows it. *Client routes (#196):* shared by both clients, see [science-government-milestone.md](science-government-milestone.md). *Not implemented:* the recommender's CRT consumption, native prompt timing, the revolution scenario restriction. Rates: `Command::Rates {science, tax}` / `Order::SetRates` (`engine::rates`), legal iff `science`, `tax` and `10 - science - tax` are all
in `[0, cap]` for the civ's current government, rejected atomically with a reason
string (no typed reason enum yet); `HumanView::rates` supplies the cap and current
shares, and `HumanSession::preview_rates` returns an immutable `RatesQuote` (treasury,
taxes, upkeep, science, luxuries, disorder cities) for a candidate split. Lock state
and the native redistribution (`engine::rates::redistribute`, tested against all
621 native vectors) stay client presentation: the engine validates only committed
shares.
Revolution: a `Revolution` command is legal for a human civ that is not already in
Anarchy and sets Anarchy at once (native). The native scenario restriction
(`0x607640 & 0x80` with `0x5f5e50 & 0x10`) is not decoded and is **not enforced**
(`engine::government::can_start_revolution`). A pending government choice is an
authoritative decision (`Decision::Government { id, eligible }`) persisted in checkpoints, opened
at the first qualifying turn step, rejecting stale ids and ineligible
governments. Accepting it sets the government through the existing setter effects
and opens a `Decision::Rates { id, view, opening }` (resolved by `Command::ReviewRates`) before input returns (native: the setter opens
the rate dialog for a human civ). `Phase::UnsupportedGovernment` was replaced by
these, not bypassed (implemented in #195 slice 2).

**Owner decisions, 2026-11-09 (accepted and implemented in v0.18.0).**
Native evidence for each is in the evidence README section above.

1. *Revolution question and Escape:* Escape/close means **Cancel**. Enter on the
   focused default (Yes) still confirms, and an explicit Cancel button is shown.
   This deliberately differs from the original, where Escape alone started the
   revolution (U31); document it in client help. Cancel returns to the map with
   no engine mutation and no decision consumed.
2. *Government chooser:* a **required decision with no cancel**. Escape shows the
   reason and keeps the chooser open; the pending decision persists in checkpoints.
   This differs from the native "negative selection leaves Anarchy running until
   the next qualifying turn step" path, whose UI exposure is untested.
3. *Rate dialog:* Esc/Cancel **discards the draft**; OK sends one `Command::Rates`.
   After a government change the review (`Command::ReviewRates`) is required, may accept the
   normalized current shares unchanged, and Esc there resets the split and keeps it open. Native Cancel behavior remains untested, so #196
   acceptance must show the chosen behavior rather than claim original parity.

**Route to freeze for validation (#194 → #196).** New standard game → Shift+T
(legal change, illegal change rejected with cap reason, locks) → Shift+R → question
(cancel path, then confirm path) → Anarchy → end turns until a multiple-of-four
turn → chooser (only legal governments, stale/ineligible rejected) → rate review →
map; save/reload before and after each decision. The second-government fixture is authored by `tools/government_fixture.py` (evidence
§Second-government fixture): the human civ knows Monarchy and is in Despotism, so the
chooser must list exactly Despotism and Monarchy, caps are 6 then 7, and the retained
vectors give the expected rate results; the generated standard start offers only
Despotism, as U31 did. An engine/client version of this fixture is part of #195/#196.

The shared client route design for the science, rates and government screens (modals, key plumbing, cancel table, engine additions, acceptance) was accepted on 2026-11-09 and is implemented in v0.18.0: [science-government-milestone.md](science-government-milestone.md).

## Diplomacy and intelligence

Evidence: U11–U14, U25/U26. With the optional visual diplomacy presentation
turned off through the installed preferences, the plain dialog sequence is
readable. Refusal, incoming alliance/tech demands, peace offer, player topics,
proposal choices, an offered-advance chooser and return paths were inspected.
The foreign report then displayed Cordial/Peace; this is a native UI outcome,
not a port of those algorithms or a complete diplomacy mechanic specification.

Use a conversation/decision stack, with current contact, relationship, demand
or offer, legal replies, quoted transfers and pending subchoices. Incoming
messages and replies are part of the same controlled turn/decision context.
A knowledge-exchange chooser must return to the proposal; leaving proposals
must return to topics, and finishing must restore the foreign report. Do not
convert generic Escape into accepting a selected demand.

The contact report and intelligence report expose different detail levels.
The inspected intelligence view includes government, treasury, capital,
research, treaties and known advances, plus city navigation. Exact eligibility
for every field is still unverified; the engine must explicitly supply what
this player may know. Do not fill the view from hidden foreign world state.

Tribute, gifts, alliance formation/breaking, map exchange, third-party war,
diplomatic unit missions and rejected/counteroffers beyond captured branches
remain follow-up discovery/mechanics cases. Prototype text for uninspected
branches must say so rather than imply complete behavior.

## Advisors, world reports and reference content

Evidence: U05/U08/U12/U23/U24/U26. Report families and fields:

| Family                 | Inspected content                                           | Required return / limits                                               |
| ---------------------- | ----------------------------------------------------------- | ---------------------------------------------------------------------- |
| City status            | City identity, outputs, item/progress                       | Own city drill-down and return.                                        |
| Defense                | Unit statistics, active/in-production counts; casualty view | Preserve selected view; casualty timeline separate and uninspected.    |
| Attitude               | City citizen rows                                           | Proposed drill-down; exact native drill-down untested.                 |
| Trade                  | City tax/science and improvement counts/maintenance         | Supply/demand is a separate observed entry, not yet fully traced.      |
| Science                | Known advances, project/progress, goal                      | Return to originating required choice when applicable.                 |
| Foreign / intelligence | Contacts and relationship; expanded intelligence            | Distinct visibility contracts, nested return path.                     |
| Wonders                | Location/owner/status and era sections                      | Unknown/lost/expired states explicit; exact visibility unverified.     |
| Ranked cities          | Five ranked city/citizen/wonder rows                        | Public information rule distinct from own-city permission.             |
| Score                  | Standard components versus scenario objectives/thresholds   | Use the correct mode; do not substitute standard score for objectives. |
| Demographics           | Values, units, ranks, selected competitor comparisons       | Derive public/earned values explicitly.                                |
| Spaceships             | Early standard game reports unavailable                     | Late assembly/launch/arrival layouts unobserved.                       |

**First slice (unreleased, #201/#202):** City status, Defense and Trade are implemented as own-civilization read-only views; see [reports-milestone.md](reports-milestone.md). The other families below remain unscoped.

The Civilopedia needs scenario-provided names/descriptions and entity links,
but its detailed navigation/media behavior is still uninspected. Preferences,
city-report notifications and help are first-class routes, not hidden developer
settings. Report/UI font, scale, scrolling and keyboard selection need tests
with long custom names, large city lists and foreign text.

## Setup, import, save/resume and diagnostics

Evidence: U01–U03, U15/U18, U27–U29/U33. Standard setup is a sequence of world
mode/size, difficulty, civilization count, barbarians, rules, leader identity,
tribe/name/style, generation and introduction. The prototype consolidates some
of these choices; it does not reproduce all wizard branches or configure a game.
Scenario entry may constrain these fields and introduce its own decisions.

Import must validate format, rules/assets, required extensions and scenario
entry context before replacing the current session. U29 shows a useful missing-
Lua-extension diagnostic; U33 rejects an authored invalid file. Merely retaining
the old picture is not proof that the native importer preserves all prior state.
Proposed client replacement is atomic, with the prior session available after
failure. Missing/unsupported capability messages name the requirement, supported
alternatives and corrective action, without silently losing scenario behavior.

Use the chosen scenario's currency, date/era, difficulty, terrain, unit and
improvement labels. Fantasy/Constantine demonstrate that these are not cosmetic
optional extras. Resolve owned resources through an installation profile;
ship none of the originals. Unsupported ToTPP/Lua or multiple-map behavior
needs a clear boundary rather than partial silent play.

Human-session checkpoints and original SAV/SCN import must be visibly distinct.
Native SAV export is not implicitly promised. A checkpoint includes pending
phase/decision, commands/results, randomness, policy state, event/Lua state and
supported map/context, with format/version validation. Validate resume at a
normal order, required choice and nested conversation. The native save/reload
route in U18 proves only visible date/project retention, not full engine parity.

## Owner decisions and deferred implementation choices

2026-11-06, answers “1 yes 2 both 3 yes 4 option 1 5 like the original”:

- Local Rust hosting is acceptable for the first playable browser version.
- Both clients reach the first playable milestone together.
- The generated-standard-game first slice is accepted; original-save/scenario
  import and deeper management follow later.
- Immediate movement, with confirmation for attacks/capture, is selected.
- City edits commit immediately like the original; no global city draft.

The accepted prototype remains the layout basis. Toolkit choice is an
implementation feasibility decision; SDL2 must pass the desktop spike before
adoption. Detailed later government, diplomacy, late-game and scenario branches
remain scoped discovery/implementation work, not silently approved native
parity. The revolution Escape redesign remains a later unaccepted proposal.
No substantial playable-client implementation is delivered by this planning
package. The live decomposition is in the delivery plan.
