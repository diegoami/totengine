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
with checkpoint replay. The inspector checks that slot 18 is ready.
Native automatic client selection remains open.

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
