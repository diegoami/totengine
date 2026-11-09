# Milestone 16 — science, rates and government routes (#196)

Shared route design for both clients. Status: **owner accepted the recommendations on 2026-11-09 ("do as you advise, and
continue"); implemented on the branch `issue-196-engine-views`, pending owner
acceptance on both clients.** Accepted points: review-modal Escape resets and stays open; small F6 advisor with goal
list; engine rates quote (E2); toolbar unchanged, with *Rates*, *Revolt* and *Science* as right-panel buttons
(the toolbar and bottom row were full, and the second row had no room). It builds on the engine surface merged/open under #195
(`Decision::{Research, Government, Rates}`, `Command::{Rates, Goal, Revolution,
Government, ReviewRates}`, `HumanView::{rates, goal, revolution_available}`) and
on the owner decisions of 2026-11-09 recorded in
[screen contracts](screen-contracts.md) (revolution Escape = Cancel; government
chooser required without cancel; rate Cancel discards the draft; native research
list; goal persisted in checkpoints). Native evidence:
[evidence §Rates, revolution and government selection](../evidence/README.md).

Browser and standalone desktop share one controller (`totui::client::Client`).
Adapters only translate keys/pointer into `Input` and paint `Frame` draws, so
every route below is implemented once in `client/mod.rs` and each adapter needs
only key plumbing plus its own input/checkpoint acceptance run.

## 1. Routes

| Route | Entry | Engine command | Required? |
| --- | --- | --- | --- |
| Research prompt | `Decision::Research` opens it; toolbar *Research* reopens | `Research{decision,advance}` | yes, no cancel to the map without the pending status |
| Science advisor (read-only) + goal | **F6** / toolbar *Science* | `Goal{advance: Some/None}` | no |
| Rates | **Shift+T** / toolbar *Rates* | `Rates{science,tax}` | no |
| Revolution | **Shift+R** / toolbar *Revolution* | `Revolution` | no |
| Government chooser | `Decision::Government` | `Government{decision,government}` | **yes** |
| Rate review | `Decision::Rates` (after a government is set) | `ReviewRates{decision,science,tax}` | **yes** |

Key meanings follow the original (`original-controls.md`: F6 science advisor;
Shift+T/Shift+R from the decoded dispatch `0x40b783`/`0x40bde1`). An unimplemented
original key stays an explicit "not implemented" notice and is never reassigned.

## 2. Shared controller design

New `Modal` variants (presentation state only; nothing here is stored in
checkpoints except through engine commands):

- `Modal::Rates { mode, draft: [tax,lux,sci], opened: [tax,lux,sci], locks: [bool;3], row }`
  with `mode = Voluntary | Review{decision}`. `opened` is the split the dialog
  opened with: the committed shares for Voluntary, `Decision::Rates::opening` for
  Review. It is the remembered value `redistribute` needs.
- `Modal::Revolution` — consequence text, focused **Yes** (Enter), explicit
  **Cancel** button; Escape/close = Cancel (owner decision 1).
- `Modal::Government { decision }` — list from `Decision::Government::eligible`.
- `Modal::Science` — read-only advisor; `Modal::Goal` — goal list.
- Research modal unchanged except the list preselects `Decision::Research::default`
  (the goal-steered recommendation) and shows it as "suggested". The prompt still
  needs Accept; a default never answers for the player.

`required()` extends to `Government` → `Modal::Government`, `Rates` →
`Modal::Rates{Review}`. Load/restore already calls `required()`, so a reloaded
pending decision reopens the same modal.

**Cancel semantics (one table):**

| Modal | Escape / Cancel |
| --- | --- |
| Revolution question | closes, no command, no decision consumed |
| Rates (voluntary) | discards the draft, closes, no command |
| Rates (review) | resets the draft to `opened`, stays open, status says the review is required; Enter commits `ReviewRates` (may equal the opening) |
| Government chooser | stays open, status says a government must be chosen (owner decision 2) |
| Research prompt | existing behavior (closes; status says it is still required; toolbar reopens) |
| Science / Goal | closes (Goal returns to Science) |

The review-modal Escape (reset, stay open) is the one behavior the owner
decisions imply but do not spell out; flagged for confirmation in §6.

## 3. Rates dialog

Three rows (tax, luxury, science) in the original order, each showing the tenths
value as a bar with `-`/`+` cells and a lock toggle, plus the cap and government
name. Keyboard: Up/Down chooses the row, Left/Right moves the slider one tenth
through `engine::rates::redistribute(cap, draft, opened, locks, row, value)`,
`L` toggles the row's lock (modal context; the browser/desktop already forward
`l`), Enter = OK, Escape per §2, Tab cycles OK/Cancel and the lock/row buttons.
Pointer: click a bar cell sets that value through the same function.

- A refused move (everything else locked) keeps the draft and shows the reason;
  nothing is sent to the engine.
- OK sends exactly one `Rates` or `ReviewRates` carrying `science` and `tax`.
  Rejection (stale decision, illegal split — normally unreachable from the UI)
  shows the engine's reason and keeps the dialog open.
- Locks persist across openings in the original (globals); here they live for the
  client session only and reset on reload. Documented difference, no engine state.
- **Quote (new engine surface, see E2):** the dialog shows treasury, projected net
  income and science per turn for the *draft*. These must come from the engine,
  not client arithmetic.

## 4. Revolution and government flow

1. Shift+R/Revolution button. If `!revolution_available` the button is inert and
   the status gives the reason (already in Anarchy / not a human civ).
2. `Modal::Revolution`: states only evidence-backed consequences — Anarchy starts
   immediately, rates are capped at 60% each, and the chooser opens at the first
   turn step whose turn is a multiple of four (every step with the Statue of
   Liberty). It does not describe unmeasured Anarchy effects.
3. Yes → `Command::Revolution`, notice "Revolution", map input resumes.
4. Turns are ended until the engine opens `Decision::Government`; the chooser
   shows eligible governments with cap summaries; an ineligible/stale choice is an
   inert rejection with the engine's reason.
5. Accepting opens `Decision::Rates`; the review modal shows the normalized
   opening split (e.g. Monarchy cap 7 → Despotism cap 6) and requires OK.
6. Map input resumes; a notice reports the new government.

Civ-unrest anarchy (forced, flag `0x1`) uses the same chooser, one aligned turn
later (engine behavior, already vector-tested).

## 5. Science advisor and goal

F6 opens a deliberately small advisor: known-advance count, current project with
progress/cost, science per turn, the goal, buttons *Set goal…* and *Clear goal*.
It cannot change a running project (native: no such path). *Set goal…* opens
`Modal::Goal`, a paged list from the engine; choosing sends `Command::Goal` at
once (no required decision). The research prompt's suggestion follows the goal.
The advisor is not the full original F6 screen; the contract wording stays
"partial".

## 6. Engine additions needed before or with the client

| ID | Change | Why |
| --- | --- | --- |
| E1 | `HumanView::goal_choices: Vec<usize>` filtered by `goal_is_valid` | client must not rebuild rules to list goals |
| E2 | `HumanSession::preview_rates(science, tax) -> RatesQuote {rates, treasury, net_income, science}` as an immutable preview (no journal entry, no RNG) | consistent finance/research quote for the dialog; cost of scratch-copying city finance to be measured |
| E3 | `HumanView::anarchy: Option<{next_choice_turn}>` | revolution/anarchy status text without client re-derivation |
| E4 | stable typed reasons for rejected rates/government (currently strings) | testable inert-rejection text; optional, strings are acceptable for the first slice |

E1–E3 are read-only view additions (E3 adds the revolution chooser turn too); they change `human.rs` and therefore
re-trigger the scenario-audit baseline refresh (see `AGENTS.md`).

## 7. Adapter work

- Browser (`play.js`): allow `Shift+t`, `Shift+r` and `F6` (currently Shift+letters
  other than `z` are dropped) in the keydown filter, with `preventDefault`.
- Desktop (`desktop.rs`): map Shift+T/Shift+R and `Keycode::F6` in `key()`; extend
  its key unit test.
- Toolbar is already full; *Rates*, *Revolt* and *Science* are right-panel buttons (see §9);
  the economy text below *Choose production* moved down by 22 px for them. A rendered check on
  both adapters must show no clipped labels.
- Pending-decision status text in `Phase::Human` and the placeholder
  "client route is not implemented yet (#196)" notice are replaced.

## 8. Acceptance for #196

Recorded-play fixtures with actual input on browser/Chromium and desktop/SDL (and
Windows where available), full checkpoint comparison at each step:

1. generated start: Shift+T legal change, illegal change refused with the cap
   reason, locks respected; Cancel leaves shares unchanged; reload keeps shares.
2. Shift+R: Cancel/Escape leaves Despotism; Yes starts Anarchy; save/reload during
   Anarchy; end turns to the multiple-of-four step; chooser appears.
3. Chooser: Escape keeps it open; save/reload reopens the same decision; the
   generated start lists only Despotism, the second-government fixture
   (`tools/government_fixture.py`) lists Despotism and Monarchy; choose Monarchy.
4. Rate review: opening split shown, Escape resets and stays open, OK resumes the map.
5. Research prompt with and without a goal (default marked, still requires Accept);
   F6 goal set/clear; save/reload preserves the goal.
6. Stale/duplicate submission and focus-loss cases per the existing both-client suites.

The second-government fixture needs an engine/client load path (the Monarchy
start must be loadable as a checkpoint or session); that remains part of #195/#196.
Owner acceptance on both clients and an owner-launched independent review gate
closure; none of this is claimed here.

## 9. Decisions

1. Review-modal Escape resets to the opening split and stays open — accepted.
2. Small F6 advisor with goal setting (§5) — accepted.
3. Engine rates quote (E2) — accepted; it scratch-copies the world per quote. Measured on a
   generated 80×50 game with one city: 8 µs (release) / 75 µs (debug) per quote. Cost grows
   with the civ's city count; re-measure with a mature empire before release.
4. Toolbar: not widened; three right-panel buttons below *Choose production*.
   The advisor shows no turns-to-discovery estimate (the native formula is not decoded).

## 10. Implementation notes

- E1 `HumanView::goal_choices`, E2 `HumanSession::preview_rates`, E3
  `HumanView::{anarchy_choice_turn, revolution_choice_turn}` are implemented; E4
  (typed reasons) is not.
- A required decision (chooser, review) also shows *Load checkpoint*, because
  Esc cannot dismiss it and a saved game must stay reachable.
- Route code: `crates/totui/src/client/routes.rs`; tests `rates_dialog_*`,
  `revolution_cancel_confirm_chooser_review_and_reload`, `science_advisor_*`.
