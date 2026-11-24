# Versioned scenario initializer and checkpoint extension

Milestone 17 / #197, persistence slice. This is engine persistence policy,
not a new claim about ToT bytes. Native phase/movement evidence lives in
the evidence README's Saved round position section; the faithful admission
gate remains [classic-surface-032-v1](scenario-entry-contract.md).

## Boundary

New experimental scenario sessions use human checkpoint **format 3**.
Schema **1** (`experimental-scenario-v1`) preserves the existing engine-sight
recipe. Schema **2** (`experimental-saved-view-v1`) installs the saved initial
human map/entities for eligible saved-round contexts, then uses engine
observation after accepted gameplay begins. Neither profile admits an input
under classic-surface-032-v1. Generated games and legacy scenario checkpoints
keep format 2 and its canonical replay. Both clients retain **Open scenario
(experimental)**; no faithful Open ToT save / scenario route is activated.

The current client factory selects schema 2 only when saved-round eligibility
and the saved projection pass, all own live entities are shown, and no
SCENARIOLOADED trigger/AND half is present. Other contexts keep schema 1;
preflight discloses the chosen initial-view boundary and continued engine
visibility/contact behavior. The explicit `scenario_initialized` constructor
keeps schema 1; restore dispatches by the recorded schema/profile, never by
the current factory's defaults.

The startup guard scans parsed trigger/AND halves; it does not establish
support for unimplemented `@OR` startup branches. Schema-2 eligibility is an
experimental reconstruction choice, not evidence of complete event admission.
Unknown required logic still rejects under the frozen first-profile contract.

The extension binds:

- The install-relative source path, captured byte length and fingerprint;
  the same original snapshot must be supplied again on restore. Original
  bytes stay private and are never embedded in public evidence or recordings.
  Desktop and browser checkpoint saves preserve the captured bytes locally
  under `.scenario-inputs` in the checkpoint folder. Each copy is bound to
  the checkpoint text by its SHA-256 filename and verified on read. Existing
  checkpoints without a private copy still use the exact original at the
  recorded relative path. The installed rules, resources and events must still
  match. Copying the checkpoint alone requires supplying the original input
  again.
- The dependency manifest captured with the session: rules, map/unit/interface
  artwork and Events.txt identities, plus resolved Labels.txt and Describe.txt
  identities. Optional missing text is explicit through manifest absence.
  No later reread may supply an identity for bytes different from those parsed.
- The normalized start mode (saved round or fresh engine turn), saved round
  position, human owner, initial scheduling phase/cursor, policy and seed.
- Versioned movement/knowledge normalization policies, initial world and
  initial knowledge fingerprints, and event input/initial-state fingerprints.
  Both engine RNG states and draw counts are asserted initially and finally;
  their seed policy remains independent of native saved RNG continuation.
  These initial assertions precede replay of the existing ordered journal.

Fingerprints follow existing engine/resource consistency checks; they detect
accidental changes and are not cryptographic authentication. The captured
snapshot plus declared reconstruction policies is the initial-memory source;
the initializer verifies its reconstructed fingerprint instead of duplicating
all private map/city records in the checkpoint.

## Construction and restoration

The adapter first captures resources and snapshot into temporary owned values.
The engine constructs a temporary session and captures its initializer before
any accepted human request or opponent advance. Checkpointing cannot bind that
session to a different dependency manifest later.

Format 3 restore checks the checkpoint/version, snapshot, dependencies, rules,
human/policy/seed and the complete reconstructed initializer before replaying
any journal entry. It then validates the final world, knowledge and transcript
by canonical re-emission, as format 2 does. Missing/changed inputs, unknown
versions or profiles, altered phase/memory assertions and replay divergence
fail without replacing the active session. The same initializer is retained
when a checkpoint is saved after partial opponent work or ENDGAME.

No private copy is written during preflight or play. The client's explicit
Save checkpoint action stages the snapshot first and creates the checkpoint
slot only after the copy is available. Neither operation overwrites an
existing slot or original input. The private folder is outside the checkpoint
list and is not packaged with recordings. It may contain unused copies after
deleting a checkpoint or after an interrupted save. Back up the folder
privately when the original source bytes might no longer be available.
In the controlled ENDGAME route, Chromium restored after replacement of the
private install's source save and Linux SDL restored after removal. Both
restored checkpoints matched each other and the saved checkpoint byte for
byte. A shared adapter test also covers cache corruption, changed Rules and
failure to publish a checkpoint when its snapshot cannot be written.

Schema 2 suppresses construction-time sight refresh and seeds remembered
improvements, unit admission and city masks/sizes from the native-verified
saved projection. Repeated views, zero-work polling, goal edits and production
window closure retain this initial epoch. The first accepted gameplay order
switches to the declared engine continuation; failed orders do not. Its
knowledge fingerprint includes the initial unit projection/epoch, so both the
saved view and the later transition replay canonically. On the pinned Original
save, the initial 59 tiles remain under the existing engine-derived required
research prompt; choosing research starts engine observation with 60 tiles.
This measures the experimental normalization, not native prompt timing or
faithful continuing fog. The original's AI/RNG continuation is not claimed.

Format 2 restore deliberately reconstructs the legacy initialization path;
it must not silently upgrade an existing checkpoint's header or dependencies.
Labels and Civilopedia remain outside generated/legacy format 2 identity, as
before; the new format 3 dependency identity includes their captured bytes.
This preserves format-2 compatibility with the current initialization behavior;
it does not undo earlier semantic fixes such as saved-round resume or ENDGAME.
Historical scenario checkpoints that crossed those fixes may still reject
canonical replay, as documented by those changes.

## Checks and remaining gates

Meaningful regressions cover changed/missing snapshot and dependencies,
changed event input with identical initial world state, initializer mutation,
partial-opponent replay, terminal replay, and byte-identical legacy/generated
checkpoint restoration. Both browser and SDL input must save and reload format
3 scenarios through the existing experimental route.

Faithful admission still needs an admitted saved-human-view/continuation
recipe. Eligible experimental saved rounds now use the stored 59-tile initial
view; schema 1 and later engine play retain the known engine-sight difference.
Snapshot normalization must be declared and measured, and unknown native fog
memory must reject under the frozen contract. Independently admitted SCN
startup context is needed where applicable, along with full dependency checks
and #198's atomic entry/confirmation routes with owner acceptance. Original AI
mission/cache/RNG continuation is explicitly outside the first profile.
The additional native observation/city-memory kernels are implementation
building blocks, not amendments adding unrelated admission prerequisites.
See [the milestone progress map](m17-import-progress.md). This persistence
slice implements no video/ending screens or native SAV export.

The separate [Original dependency capture](import-stock-dependencies.md) is
available for future admission wiring. It retains exact raw/parsed/decoded
inputs and SHA-256 identities of the known first set, with revalidation before
future confirmation. Existing schema-1/schema-2/format-2 constructors and restore
do not call it; it is not an admitted initializer or a private snapshot cache.

The separate [native phase boundaries](import-phase-boundaries.md) reach unit
setup on the pinned source without a research-selector call in that prefix.
An empty saved target is not proof of a saved pending prompt. Existing recipes
retain their engine-derived Research choice; future admission must bind a
measured phase/choice recipe without silently changing historical checkpoints.


## Experimental staged round

Schema 3, `experimental-staged-round-v1`, is an explicit engine recipe
for checking the measured handoff. The installed Original inspector
exercises it with the captured dependencies and engine opponent policy.
Normal client setup continues selecting schema 1 or 2.

The resumed player's saved turn already has its city pass. After End Turn,
the staged recipe runs each subsequent living civilization's city pass and
movement allocation before its first order. The city pass is one replayable
work item. Round close runs the existing engine round effects once and
passes scheduling to the next round. The human's next city pass runs
before the next human order phase. Subsequent observation uses the engine
model.

An empty project with zero saved science enters the human order phase. The
recipe uses the existing chooser when there is banked science, an eligible
post-discovery choice, or a free advance. City passes are atomic, so this
recipe does not interrupt a pass at an individual native dialog.

The checkpoint descriptor binds the scheduling recipe. Replay reconstructs
the pending city pass, food-route accumulator and research bookkeeping from
the operation journal. Tests restore before and after civ 5's city pass
and continue both runs to the next human turn. Full classic admission
still requires verified continuing knowledge, native pending decisions and
the complete round contract.


## Experimental saved movement

Schema 4, `experimental-saved-movement-v1`, adds an explicit saved-view
movement phase to the staged scheduler. An ordinary awake unstacked land
unit can skip its remaining movement or move to a known empty surface tile
outside a city. The engine
stages the movement and validates the native-derived observation before
committing the order. A refused move leaves the world, journal, receipts
and saved view intact.

The phase carries the saved tile visibility, remembered improvements and
entity knowledge into the next supported move. The movement budget still
comes from the authoritative engine unit. An explicit End Turn retains the
projection until the scheduler starts its first city pass or closes the
round. Other accepted gameplay orders and peace agreements end the
saved-view phase and use staged engine continuation. The checkpoint identifies which recipe reconstructs the
movement and rebuilds its private map state from the captured save and
the order journal.

The pinned move from (63, 5) to (62, 4) has zero remaining movement and
reveals exactly the native tile at (62, 2). The existing Original inspector
verifies the move through the human command and checkpoint restore routes.
An install-backed test continues with another supported move and confirms
that an occupied destination fails atomically. It also replays all four
native Skip slots with unchanged map knowledge, and restores the saved-view
handoff before the first opponent city work item. This recipe exercises
ordinary land movement. General action knowledge, opponent and city
knowledge through the round, and native pending decisions remain separate
admission work.


## Experimental saved view during city work

Schema 5, `experimental-saved-city-view-v1`, adds a guarded handoff to
the saved-movement recipe. It retains the projection during an opponent
city pass when the map, treaty inputs and existing observed identities
stay stable. It permits a new unit in an unseen city processed by that
pass. Surviving cities' remembered sizes refresh using the native-derived
city-memory kernel. A changed map or entity context, such as a birth at a
known destination, hands observation back to the staged engine. The
saved view continues through opponent production selection, Skip, End Turn,
and research without a free advance. Other opponent orders switch to
engine observation.

The pinned [native probe](evidence/import-city-view.json) follows four
Skip orders to civ 5's first city. It stops at entry to production
selection after a unit has appeared in an unseen city. The human view
contains 59 tiles throughout, and its remembered improvements and
previous unit knowledge stay intact. The engine recipe preserves the
same human view across the first city work item and replays at its
checkpoint. The native trace stops inside that city phase, while the
engine check completes its atomic pass and the first engine production
order. Pending native decisions and changes during spatial opponent
orders need separate verification. A prepared native movement control
supplies the first engine spatial order after the city probe. It moves
slot 2 into the unseen city, links it after slots 34 and 19, and
retains 59 explored tiles and all remembered improvements. This isolates
one move without running the native production selector or opponent policy.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_city_view_check.py --out /tmp/city-view.json
cmp docs/evidence/import-city-view.json /tmp/city-view.json
```


## Experimental opponent stack movement

Schema 6, `experimental-saved-opponent-stack-v1`, extends the city-view
recipe to guarded opponent land steps between unseen tiles. The mover
can go from a singleton to empty land or a friendly city stack. It can
leave a friendly city stack with clear unit masks for empty land. The step uses ordinary engine
movement, appends the mover to the existing stack, and runs the
native-derived unit observation on a temporary snapshot. The recipe
commits the result when the saved player's projection and remembered
improvements still match. Other movement kinds and an observed change
continue through the staged engine policy.

The private observation snapshot records births already guarded by the
unseen-city pass. The pinned native birth had clear visibility and no
standing orders. The new engine records start unobserved, with stack links
reconstructed from the existing engine allocation order.

The measured [native stack move](evidence/import-city-view.json) joins
slot 2 after slots 34 and 19 at (43, 37). The engine command now produces
the same stack order and the same 59-tile view. The inspector and
install-backed tests verify replay immediately before and after the move.
The following move of slot 10 from (41, 39) to empty land at (41, 37)
also keeps 59 tiles and replays. Slot 19 then leaves the middle of the city
stack for (42, 36), while the two units it leaves behind link 34 → 2.
The three ordered moves retain the same viewer memory. A known
destination declines the guarded path. Broad opponent movement,
changing spatial knowledge and native pending choices still need
admission work.


## Experimental unseen opponent combat

Schema 7, `experimental-saved-opponent-combat-v1`, extends the staged
recipe to the checked class of a winning attack between two unseen
opponent land singletons outside cities. It runs the existing engine
combat and event entry points on temporary state. On a checked outcome
the defender is removed from the private snapshot, the survivor's
observation runs through the native-derived kernel, and the engine
commits the result when the saved human view and remembered improvements
still match. Other outcomes and immediate macro effects use the existing
engine path. The event fallback leaves the initial RNG and event state
intact so the prior path can apply them once. This recipe also permits a
checked unseen move into a friendly stack outside a city.

The [prepared combat trace](evidence/import-opponent-combat-view.json)
runs a native Rules refresh before repeating the three measured opponent
moves and directing slot 20 against slot 22.
The seed-7 control wins in place, removes the defender and keeps all
59 explored tiles and remembered improvements. Seed 1 also wins and
leaves the attacker with five damage. The inspector and install-backed
replay match the seed-7 outcome and final combat CRT state and restore
the checkpoint on either side. The seed-1 engine control also matches the
native survivor damage and final CRT state. The following field-stack
move goes from (11, 45) to (9, 45) and appends slot 4 after slot 7.
It also keeps 59 tiles and replays from its checkpoint. Nine controls
vary the routes on the two tiles. Native movement uses the terrain cost
when either tile lacks a route, charges one third when both have roads,
and charges zero when both have rail. The actual forest destination only
has a road on the arrival tile. The native move spends six thirds against
an allowance of three and refuses the next step. The staged field-stack
recipe uses the tested route rule and exhausts the Settler's movement.
The next checked move leaves that two-unit field stack for a friendly
city with one resident unit. Slot 7 spends one third and links ahead of
slot 24. An attack-zero control places the mover behind the resident.
The entry controls confirm that the city connects to the route on the
origin tile. The staged recipe mirrors this arrival, keeps all 59 tiles
and restores the checkpoint. The subsequent move takes slot 24 out of
the city and back to the field stack. It spends one third, leaves slot 7
alone in the city, and inserts slot 24 before slot 4. Three controls
verify terrain, road and rail costs and the ordering of an unarmed mover
on departure. This step also preserves the 59-tile view and replays.

With two thirds left, slot 24 then attempts to move from the field
stack into an empty forest costing six thirds. In this checked context,
a unit that has spent movement rolls the native CRT against the cost
when the cost exceeds the remaining allowance. A roll below two moves
the unit and the native record adds all six thirds. A roll of two or
more exhausts the unit in place. The pinned sequence draws five,
leaves the stack links intact, and replays the updated combat CRT state.
Four prepared seeds also exercise the cutoff and the two possible
positions without changing the saved view. The engine uses its
checkpointed CRT for this guarded operation. The same rule applies to
the next Warrior's unstacked move from the road at (13, 43) to the
mountain at (15, 43). The cost is nine thirds and the pinned sequence
rolls six with two thirds left, exhausting the Warrior on the road.
A seed-zero control confirms the cutoff. A seed-five control crosses
into the mountain and records ten spent thirds. Both preserve the saved
view and the pinned attempt replays through its RNG advance. The next
Phalanx enters a road from a tile without one. It spends all three
movement thirds, with no CRT draw, and the original refuses its next
attempt. The staged recipe uses the checked unstacked land movement
when a road is present at exactly one end of the step.

The [prepared annual update](evidence/import-round-close.json) runs
the native update at `0x4702fc` after the last checked move. It advances
from round 43 to 44, consumes two CRT draws, and leaves the selected
human's explored bits and remembered improvements byte-identical.
Rectangle and directory calls run in the emulator with controlled
directory failures. The staged engine round retains this saved view
when map, unit, city, wonder and diplomacy state are stable and the
round applies no events or barbarian arrivals. A synthetic round-start
event exercises the fallback and fires once. The inspector restores
and verifies the checkpoint across the checked annual update. The same
probe then runs the turn-44 human turn start with optional autosave
disabled, executes native observation, and reaches the first input poll
and one empty-queue iteration. Native selection reaches slot 18 with 59
tiles and no research choice. The engine retains the same projection
through the checked human city pass, resets its matching private unit
records at the per-civ movement reset, and accepts a Skip for slot 18
with checkpoint replay. An independent northwest move from the same
handoff advances slot 18 to (50, 8) and adds one explored tile. The
private snapshot records native expenditure and direction for the
checked full-budget step. The engine matches the four raw tile changes,
two remembered-improvement changes, and all five changed unit bytes.
The inspector checks that slot 18 is ready and that both orders replay.
A separate native selector control compares every valid anchor with the
engine's guarded initial-focus function. All 2,000 results agree using
the wrapped diamond distance and the first ready slot on ties. Clients
still need to apply the anchor on entry and retain it for later input.
The [saved viewport probe](evidence/import-saved-viewport.json) binds
the initial (61, 5) to signed words at post-city offsets 63 and 65.
Native resume selects slot 9 from those bytes. The original prepared
control supplies (0, 0) and selects slot 6. Both retain 59 tiles.
The inspector checks the captured-byte anchor and both selections.

Native opponent policy, the outer-loop UI, wider macro behavior and
broader combat contexts remain outside these controlled actions.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_opponent_combat_view_check.py --out /tmp/opponent-combat-view.json
cmp docs/evidence/import-opponent-combat-view.json /tmp/opponent-combat-view.json
```

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_round_close_check.py --out /tmp/import-round-close.json
cmp docs/evidence/import-round-close.json /tmp/import-round-close.json
```

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_saved_viewport_check.py --out /tmp/import-saved-viewport.json
cmp docs/evidence/import-saved-viewport.json /tmp/import-saved-viewport.json
```

## Load a checked experimental replay

Create a named checkpoint in the save directory used by either client.

```sh
totctl prepare-original-replay "$CIVTOT_HOME" Original/Ph_Auto2.SAV "$TOTENGINE_SAVES" checked-original
```

Use **Load engine checkpoint** and select `checked-original`. The command
verifies the pinned save and reference dependencies, uses Built-in AI with
seed 7, and preserves a private copy of the SAV beside the checkpoint.
The original install supplies its reference resources at restore. A
pristine checkpoint selects slot 9 at the saved viewport. During play the
shared client uses the last inspected tile when the checked saved-view
session reaches the next human turn. For a checked Skip it selects the
nearest ready unit from the exhausted unit's tile. The native saved-anchor
sequence is 9, 6, 26, 18 and enters the first opponent city phase with
anchor (51, 9). Engine inspection replays that sequence. A later
checkpoint load uses the ordinary client selection policy. This load path is experimental. Faithful
SAV entry remains open under #198.

### Checked Wait

In the checked saved-view context, **W** defers an awake unstacked own land
unit without spending movement. The native selector first searches for
another ready unit that has yet to wait. If every remaining candidate has
already waited, the selector clears those flags and searches again. A
solitary ready unit stays selected. The native annual update clears
deferrals at round close.

The private checkpoint journals Wait, seals outstanding deferrals and
restores focus after Wait or a checked exhausted Skip or Move. The
shared client uses the ordinary presentation behavior outside the checked
context. The inspector replays six native Wait commands and checks the
unchanged movement and view. The witness executes the native writer and
reader boundaries with a virtual unit table and tests a mixed Skip
sequence. In a separate control, slot 9 waits, slot 6 exhausts its
movement going northwest, and the native selector picks slot 26.
The inspector verifies the same move and restores slot 26 from the
checkpoint.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_wait_check.py --out /tmp/import-wait.json
cmp docs/evidence/import-wait.json /tmp/import-wait.json
```

### Rate changes in a checked saved view

The [native rate commit](evidence/import-saved-rates.json) runs the
prepared dialog's commit fragment through city refresh for three legal
rate pairs. All three retain 59 explored tiles and the same remembered
improvements. The checked engine stages the rate order, verifies that
map and observation inputs are stable, and preserves the saved view.
The inspector verifies each case and restores the resulting checkpoint.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_saved_rates_check.py --out /tmp/import-saved-rates.json
cmp docs/evidence/import-saved-rates.json /tmp/import-saved-rates.json
```

### Quoted production in a checked saved view

The [prepared native commit](evidence/import-saved-production.json)
captures the switch budget, executes the build and shield writes, and
stops before further presentation. City 5 keeps 12 shields when its
item stays at 3 and retains 6 when switching to item -42. Both controls
keep the 59-tile saved view and its remembered improvements. The
checked replay uses the validated quote and retains the same view. The
inspector compares the selected item and shields and restores each
checkpoint.

### Worker editing in a checked saved view

The [prepared native worker editor](evidence/import-saved-citizens.json)
uses the city-radius tile and current-city accessor, then runs the native
edit and city totals before redraw. Removing and restoring the worker on
tile 4 and resetting allocation from the center all keep 59 explored
tiles and the complete remembered-improvement array. The checked replay
retains that view and synchronizes the private worked-tile and packed
specialist fields. The inspector compares all three transitions and
restores the checkpoints.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_saved_citizens_check.py --out /tmp/import-saved-citizens.json
cmp docs/evidence/import-saved-citizens.json /tmp/import-saved-citizens.json
```

### Production purchases in a checked saved view

The [prepared native purchase](evidence/import-saved-purchase.json)
runs pricing, affordability and the successful purchase before redraw.
City 5's current item costs 19 gold. The purchase leaves 6 gold in the
treasury and fills the shield box from 12 to 20. A second control starts
with 18 gold and leaves the treasury and shield box unchanged. Both
retain the complete saved map and remembered improvements. The checked
engine accepts the quoted purchase, preserves the saved view, and
rebases the switch budget at 20 shields. The inspector verifies these
values and restores the resulting checkpoint.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_saved_purchase_check.py --out /tmp/import-saved-purchase.json
cmp docs/evidence/import-saved-purchase.json /tmp/import-saved-purchase.json
```

### Turn-44 human production after the worker edit and purchase

The [prepared native turn-44 witness](evidence/import-human-city-birth.json)
uses the same opponent and annual setup as the quiet handoff. It supplies
the measured worker edit, treasury and paid shield box before executing
native human city work and input preparation. The new Phalanx occupies
slot 22 at (53, 9), with zero movement spent and zero shields left in the
city box. The map and remembered improvements remain intact through the
first input poll, with 59 explored tiles and slot 18 active.

Initializer schema 8 (`experimental-saved-human-production-v1`) stages
a single ordinary land unit built in a known human city. It runs the
bounded native observation kernel with the same active-contact argument
as the witnessed production call. The path retains saved knowledge only
when that operation leaves the staged snapshot intact. The checked replay
keeps the original 59 tiles and shows the new Phalanx. Native first
credits two shields, subtracts the unit cost, then clears the surplus
at `0x4c748e`. Starting with five extra shields reaches the same reset.
Schema 8 clears the box when the guarded birth succeeds. Schema 7
replays its original 60-tile engine observation and two-shield surplus
for this sequence.

The checked engine places the Phalanx in slot 38. The native prepared
case uses slot 22. The inspector reports both slots and checks the
map, production result and checkpoint replay. The selector also covers
a coherent pair of own land units in a city. At the five witnessed
inspection anchors, native selection follows slot order for a distance
tie, including when the two stack links are reversed. The checked
engine follows the same rule using its own slots. The live turn handoff
selects slot 18 from (51, 9) even with the new stack in the city.
Faithful import admission remains open.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_human_city_birth_check.py --out /tmp/import-human-city-birth.json
cmp docs/evidence/import-human-city-birth.json /tmp/import-human-city-birth.json
```

### Orders from the new human city stack

The [native input probe](evidence/import-human-city-stack-orders.json)
restarts the paid-production handoff for each of six cases. Wait on
either member sets its deferred bit and selects the other. Unit
observation runs with the active-contact argument and leaves map
knowledge intact. A northwest order on either member detaches it
for (52, 8), spends six thirds from a full three-third allowance,
and leaves the other unit in the city. The move keeps 59 explored
tiles and changes the destination's remembered improvements. Skip
spends the actor's full three-third allowance without changing the
pair's links, positions, or saved map knowledge. The controls supply
each actor's prior direction from the checked engine state and
confirm that Skip resets it to 255.

Checked schema-8 replay accepts these orders in the measured
two-unit surface city context. Wait requires a ready partner and
unchanged staged observation. The northwest step requires a full
three-third budget, an empty known destination, and a six-third
movement cost. It leaves the partner in the city and applies saved
observation after unlinking the pair. Skip keeps the pair linked and
verifies that the staged observation is unchanged before committing
the actor's spent movement. The inspector checks all six cases and
restores their checkpoints.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_human_city_birth_check.py --orders --out /tmp/import-human-city-stack-orders.json
cmp docs/evidence/import-human-city-stack-orders.json /tmp/import-human-city-stack-orders.json
```

### Re-entry into the occupied human city

The [native city return trace](evidence/import-human-city-stack-returns.json)
moves each member northwest to (52, 8), resets its spent movement in the
prepared input, and drives it southeast to (53, 9). The native arrival
spends six thirds, links the returning Phalanx ahead of the resident,
and removes the field unit marker from the saved map and memory. The
view keeps 59 explored tiles. The budget reset is a controlled input.
The checked engine's re-entry path remains open.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_human_city_birth_check.py --returns --out /tmp/import-human-city-stack-returns.json
cmp docs/evidence/import-human-city-stack-returns.json /tmp/import-human-city-stack-returns.json
```


### Opponent first contact before turn 45

The [prepared encounter trace](evidence/import-opponent-contact.json)
runs the native contact entry for civs 5 and 6 at (48, 28). The first
encounter on turn 44 changes both treaty words from zero to `0x4401`.
It reaches advance grant entry with advance 39 going from civ 6 to civ
5. Unit and city records, the human's 59 explored tiles and remembered
improvements all remain intact up to the grant. Two full controls run
the grant and remaining encounter through native return. They supply
an exception-chain page and substitute SetRect with its memory write.
Neither full control changes the prepared human map or memory.

With symmetric known treaty words of 1, the turn-44 control returns
after updating both words to `0x401`. The same pair on turn 45 reaches
the trade grant. Full first contact leaves both treaty words at
`0x4405`. With the prepared loader CRT seed of 1, the exchange grants
39 to civ 5 and 46 to civ 6. With CRT state `0xd766ce46` captured
before the engine encounter, native selects the same advances. Both
cases make nine CRT draws. The probe loads the complete stock
valuation tables. All nine native base scores match the checked engine
context before the encounter. The report records the candidate scores,
grant arguments and changed technology masks. The live checked replay
currently uses engine sight at the first-contact boundary. Applying
these encounter effects during continued play remains open.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_opponent_contact_check.py --out /tmp/import-opponent-contact.json
cmp docs/evidence/import-opponent-contact.json /tmp/import-opponent-contact.json
```


### Checked opponent encounter replay

Initializer schema 9 stages the measured turn-44 encounter between
opponents 5 and 6. It checks the contact site, treaty inputs, stock
context, CRT state, and all nine candidate values before committing.
The staged movement runs the encounter inside observation. It applies
the two checked technology grants, uses nine CRT draws, and records the
reciprocal treaty. The human map projection must survive the whole
movement unchanged. The inspector verifies the encounter and restores
its resulting checkpoint. Schema 8 preserves its original replay.

This path covers the witnessed stock encounter. The next action on the
checked route attacks an occupied city and still triggers engine sight.
Full turn-45 saved observation and faithful entry remain open.


### Opponent attack on the occupied city

The [prepared native attack](evidence/import-opponent-city-combat.json)
uses the post-contact CRT and treaty words. Unit 35 attacks east from
(46, 28) and remains there, with six thirds spent and eight damage.
Unit 36 is deleted. City slot 9 loses its final citizen and becomes
inactive, with its tile occupancy flags cleared. The human view and
remembered improvements stay intact through the first city-panel
font import. Closing the native panel allows the callback to return.
It keeps all 59 explored tiles and clears the city occupancy bit in
viewer-memory byte 1144.

Initializer schema 10 stages the witnessed attack in checked replay.
It verifies the combat result and CRT state, applies the existing
surface-city destruction mechanic, and clears the native occupancy
bits in the remembered tile. The saved view stays at 59 tiles. The
inspector replays a checkpoint from immediately after the attack.
Earlier initializers keep their original combat behavior.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_opponent_city_combat_check.py --out /tmp/import-opponent-city-combat.json
cmp docs/evidence/import-opponent-city-combat.json /tmp/import-opponent-city-combat.json
```


### Checked opponent hut replay

The [prepared native hut entry](evidence/import-opponent-hut.json)
executes the southwest move from (28, 12) into (27, 13) through native
return. The move spends nine thirds, grants advance 1 to civ 6, clears
the project target, and leaves its accumulated science at 12. It keeps
all 59 human explored tiles and remembered improvements.

Initializer schema 11 stages this move and reward when the unit,
research and CRT inputs agree with the witness. It checks the outcome
before committing the update. The inspector checks and restores the
post-hut checkpoint. Earlier recipes keep their fallback behavior.
The next fallback in this route is an armed arrival into a friendly
field stack.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_opponent_hut_check.py --out /tmp/import-opponent-hut.json
cmp docs/evidence/import-opponent-hut.json /tmp/import-opponent-hut.json
```


### Saved observation through turn 45

The [prepared armed arrival](evidence/import-opponent-armed-stack.json)
moves unit 24 from (9, 45) into the friendly stack at (9, 47). It takes
the head, spends three thirds, and leaves the CRT and human knowledge
unchanged.

Initializer schema 12 stages the measured stack arrival. The checked
Original route then continues through the rest of turn 44 and opens
turn 45 with the full original 59-tile view. The returned movement
budget permits the earlier Phalanx to re-enter its city, where it
takes the head of the pair. The inspector checks the actual turn
transition and compares the restored post-return checkpoint. Earlier
recipes retain their original behavior at the relevant boundaries.
This remains an experimental replay profile.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_opponent_armed_stack_check.py --out /tmp/import-opponent-armed-stack.json
cmp docs/evidence/import-opponent-armed-stack.json /tmp/import-opponent-armed-stack.json
```


### Saved observation through turn 46

The second [armed field arrival](evidence/import-opponent-armed-stack.json)
uses the prepared turn-45 move into (9, 49). Native spends six thirds
from a full three-third budget, puts unit 24 at the head, and preserves
human memory and the CRT state. The [known encounter
control](evidence/import-opponent-contact.json) reaches native return for
civs 2 and 5 on turn 46 with treaty words `0x405` in both directions.
It makes no trade call, advance grant, or CRT draw.

Initializer schema 13 stages both boundaries. For known contacts between
opponents in the checked single-player stock context, the between-trade
turn path updates the reciprocal encounter bits. Human contacts, trade
turns, and other first contacts retain their existing guards. The
checked replay reaches turn 46 with all 59 tiles, and the player's
Phalanx can leave its city again on the refreshed budget. The inspector
replays the resulting checkpoint. Schema 12 preserves its earlier
fallback and checkpoint behavior.


### Saved observation through turn 47

The [prepared zone-of-control controls](evidence/import-opponent-zone.json)
cover two opposing approaches to (38, 34). The native callback refuses
both while the rival occupies the adjacent square. Moving that rival
away allows the first movement and charges six thirds. At refusal
native increments opaque byte 16 of the moving unit record. The first
19 attempts leave the movement budget intact. Attempt 20 resets the
byte and spends the six remaining thirds. The unit stays on its
original square, and the CRT, map and human knowledge stay intact.

Initializer schema 14 reproduces the witnessed refusals, mirrors the
private byte, and feeds the rejection into the checked opponent policy.
The witnessed route reaches turn 47 with the original 59 tiles. The
player can re-enter the occupied city on the refreshed budget and
restore the resulting checkpoint. Schema 13 retains the earlier
fallback. Faithful direct entry remains open.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_opponent_zone_check.py --out /tmp/import-opponent-zone.json
cmp docs/evidence/import-opponent-zone.json /tmp/import-opponent-zone.json
```

### Hidden opponent founding on turn 47

The [prepared founding witness](evidence/import-opponent-founding.json)
executes two prepared calls to the native founding callback through
return. The harness supplies host directory and
rectangle operations and reads the city-name row from the installed
Original pack. Civ 5 consumes unit 2 and reuses city slot 9. Civ 6
consumes unit 37 and appends city slot 10. Both cities start at size one
and the new occupancy byte reaches all seven civilizations. The human keeps all 59 explored tiles. The report records
the installed city-name file and the selected name by hash.

Schema 15 stages both foundings in the checked replay. It applies measured
site, fertility and occupancy changes and keeps the human city list and
map intact. The hidden cities use the engine's generated labels, identities,
workers and initial production. Native private civ counter writes and
the prepared loader's transient city identity remain outside this recipe.
Schemas through 14 retain their prior behavior. Faithful direct entry
remains open.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_opponent_founding_check.py --out /tmp/import-opponent-founding.json
cmp docs/evidence/import-opponent-founding.json /tmp/import-opponent-founding.json
```

### Westward opponent stack arrival on turn 47

The [prepared arrival](evidence/import-opponent-armed-stack.json) compares
two movement inputs for civ 7's warrior. With nine spent thirds left in
its record, native refuses the move and writes three private unit bytes.
With refreshed movement, the warrior moves west to (10, 44), spends six
thirds and becomes the stack head. The native callback uses no CRT draw
and preserves all 59 human explored tiles and remembered improvements.

Schema 16 mirrors the movement refresh for civ 7's checked turn-47 city
pass, then stages the arrival and replays across it. Earlier schemas
retain their existing behavior. The saved view still hands off to engine
sight at the next annual update in this route.
