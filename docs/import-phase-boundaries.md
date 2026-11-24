# Saved-round and research-entry boundaries

Milestone 17 / #197, bounded native evidence for the human/phase predicate.
The [entry contract](scenario-entry-contract.md) is unchanged. No profile or
new session recipe is activated by this slice.

Execute three small regions of unpatched `civ2.bak` using copied/controlled
inputs, with no replaced callees:

- The single-player loader suffix `0x461d15..0x461d7b`, after its network
  initialization call. It selects the player as resume civ and clears the
  round-start flag. Record its transient writes without assigning unexplained
  UI words a pending-dialog meaning. Verify the civ/unit/city blocks stay intact.
- The civ-loop guard `0x474b62`, stopping before turn-start, forced-restart
  setup, human preparation or AI dispatch. This proves which branch skips
  the turn-start step, not that every later UI callback is understood.
- For the pinned source, continue through native human preparation (its advice
  bit is clear) to the unit-setup call `0x472ada`. The native human loop clears
  the saved active-unit slot to -1 before unit setup; civ/unit/city records are
  unchanged. No research selection call occurs in this prefix. This still stops
  before unit setup, redraw or the full input loop, not at an assumed modal.
- Research accumulation `0x4a1ebf`, running original code (including the
  original empty helper `0x40106b`) until return or the first cost, advice or
  project-selection boundary. Never execute or replace the chooser/dialog or
  cost routine in this probe. Check the exact signed-word progress update and
  preserve every other civ byte; detect unexpected RNG calls.

Retain numeric native vectors, binary/source hashes, stop addresses and scope.
The Rust transcriptions remain separate pure kernels. Add a conservative
saved-round context checker for the first stock surface context: a live human
player/active-civ match, one human bit, plain single-player classic Original,
known road multiplier, and a supported research-target range. Unsupported
contexts return a reason instead of silently selecting a fresh turn. This
checker identifies the resumed human-preparation boundary; it does not certify
the absence of all native pending decisions.
It selects a clean, unforced reconstruction policy rather than extracting a
serialized native restart flag. The native guard's forced branch is measured
separately, not silently ruled out for arbitrary files or external UI state.
It conservatively rejects an active/player mismatch even though the isolated
single-player loader suffix selects the player regardless of the active byte;
that restriction is engine preparation policy, not original-game legality.

The pinned save has research target -1 and progress 0. An empty target alone
does not determine the next UI decision. The native accumulation routine must
receive positive science; after its signed-word addition, a nonzero progress
with an empty target reaches selection (possibly preceded by advice). Zero or
negative science returns without changing progress. A running target reaches
the cost boundary. These are call boundaries, not observed modal timing.

The experimental schema-2 constructor still derives Research on load and
switches to engine observation after choosing it. Preserve that recorded recipe
and all schema-1/format-2 behavior. A future admitted initializer/driver must
use the measured boundaries rather than treating `target == -1` as proof of
a saved pending prompt. Full loader/UI/pending-choice behavior, interrupted
city processing, knowledge continuation and #198 remain separate gates.
Original AI/shared-RNG continuation is still outside this profile.

The [event-trailer trace](evidence/import-event-tail.json) continues from
`0x461d7b` on the pinned save. It observes the successful `EVNT` read set
the round-start flag back to one. The resumed-civ guard still skips
round-start event dispatch when the restart flag is zero. This explains why
the earlier loader suffix's flag reset alone is insufficient to characterize
the full load path.

The [post-event probe](evidence/import-loader-exit.json) runs the event reader
and continues in the same prepared memory from `0x4622ea` to `0x462e0d`.
With two display-helper calls substituted and nonnegative sign controls, it
preserves the civ, unit, city and loaded event records, the tiles, and all
seven civ caches. The round-start flag remains one, the resume civ remains
four, and research stays at target -1 with zero progress. A separate suffix
exercises mode restoration after the file-close boundary. It shows that
restoring a different mode can fill an empty human mask using the living
resume civ or the first living civ.

The pinned execution also continues through the final Rules refresh to
the successful same-mode return at `0x4630e3`. The probe supplies the parsed
`CIVILIZE`, `IMPROVE`, `ENDWONDER` and `UNITS` rows in place of the section and
file readers, plus a successful closing of the virtual save handle. The
native token conversion and table writes execute and count 89 enabled
advances. Civ, unit, city and event records, tiles, caches and the pinned
phase all compare unchanged through return.

The original loader caller retains options bits `0x0000c000` from the save
and takes the rest from process preferences. Two controlled preferences on
the pinned save reach different human preparation branches. Enabling advice
reaches the native advice path, while the preference matching the save
returns from its gate. The probe stops before processing advisory state or
displaying a dialog. The save's own advice bit therefore cannot determine the
entry path by itself. Admission still needs an explicit verified preference
policy and evidence from the ensuing decision and input paths.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_loader_exit_check.py --out /tmp/loader-exit.json
cmp docs/evidence/import-loader-exit.json /tmp/loader-exit.json
```

The [unit-selection trace](evidence/import-unit-setup.json) extends the
resumed branch through the native active-unit search. It prepares the
unit-selection display mode and movement inputs from the pinned Rules.
The pinned state selects slot 6 at `0x47297f`. Exhausting all four own
units reaches the no-unit branch at `0x472a8e`. Leaving only slot 9 ready
selects slot 9. All three executions preserve the prepared civ, unit and
city records and research remains target -1, progress 0. The stops precede
presentation and order processing. Other display modes, pending prompts and
the full native load path still need evidence.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_unit_setup_check.py --out /tmp/unit-setup.json
cmp docs/evidence/import-unit-setup.json /tmp/unit-setup.json
```

The [input handoff probe](evidence/import-input-handoff.json) follows the
selected-unit path through `0x41f704` to `0x4205a7`, before notification
processing. It substitutes three preceding display calls. The original
observation loops keep 59 explored tiles and leave tile bytes, the player's
remembered improvements, unit visibility masks and city knowledge unchanged
on the pinned input.

Separate selected and exhausted controls reach `0x45abc1` before the
input loop's event pump. The selected control substitutes the full
observation call as well as display helpers. It arrives in unit mode at
slot 6. The exhausted control arrives in viewing mode with no active unit.
The controls then execute the input-loop prologue and stop at the first
indirect call to `PeekMessageA` at `0x576ee4`. The probe checks the import
with `objdump -p`. The pinned selected-unit trace also reaches that
instruction with both native observation calls intact and the redraw wrapper
substituted. It keeps 59 explored tiles and leaves the captured knowledge
bytes unchanged. Research still has target -1 and progress 0 in all three
cases. These prepared traces stop before Windows input and cannot establish
live prompt timing or the full loader path.

An empty-queue control returns zero from the first `PeekMessageA`, runs the
native idle branch, and supplies an empty result for its second,
message-filtered call at `0x577011` (filter `0x3bd`). With the host
background callback disabled, all three paths reach the next general poll at
`0x576ee4`. The idle iteration preserves the selected slot, display mode,
and every byte of the civ, unit, and city records. The traced pinned path
also preserves all captured human knowledge. Real queued messages and their
handlers remain open.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_input_handoff_check.py --out /tmp/input-handoff.json
cmp docs/evidence/import-input-handoff.json /tmp/input-handoff.json
```


### First orders after the prepared input loop

[The first-order probe](evidence/import-first-order.json) re-executes the
pinned observation and idle prefix, then calls the original game callbacks
with three independent inputs. It prepares the running-window guards,
supplies an empty filtered queue and substitutes the listed display, audio,
network and key-state calls. The keyboard move uses an internal direction
code and returns after the native movement and observation logic. The probe
does not drive a live Windows queue or show modal timing.

- **W** selects slot 9 and sets attribute bit `0x4000` on slot 6. Sprite
  selection in the current clients is an engine presentation choice without
  that native record write.
- **Space** spends slot 6's full movement allowance, 3 in the saved units, and
  writes 255 to its last-direction byte. Slot 6 remains active at the return of
  the character callback.
- **Northwest** moves slot 6 from (63, 5) to (62, 4). The native terrain cost
  records 6 spent against an allowance of 3. Occupancy and destination
  ownership change, and the player's saved knowledge grows from 59 to 60 tiles.
  The other six remembered-improvement arrays and the civ and city records
  stay intact.

Research remains target -1 with progress 0 after all three accepted inputs.
This proves that an empty research target need not prevent initial unit
commands in the prepared context. It supplies no assertion about when a live
session opens the research chooser later in the round.

The install-backed CLI test applies the measured occupancy and ownership
changes before running the saved-knowledge observation kernel. Its tile and
cache deltas exactly match the native move. A separate check executes the
current engine movement order and reaches the same position with zero moves
remaining. Its generic reveal includes an extra tile near Madrid. The two
knowledge models therefore still diverge after a real first order, which
prevents the current runtime path from satisfying the continuation predicate.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_first_order_check.py --out /tmp/first-orders.json
cmp docs/evidence/import-first-order.json /tmp/first-orders.json
CIVTOT_HOME=... cargo test -p totctl --test original_import
```


### City science and the empty research project

The [native city science probe](evidence/import-city-science-entry.json) runs
the pinned player's city slot 5 from the same prepared resume image. It
executes the city-phase routine `0x4cb01d` through actual science computation
and accumulation. The computed value is zero, the routine returns, and the
player still has target -1 and progress 0. The only substitutions are the
city-window query returning -1 and the observation redraw callback.

Separate controls supply one point at the call from city processing into the
science stage. With the pinned advice setting the accumulator reaches
`0x4a1436`. With advice enabled it instead reaches `0x477030` first. Both
controls increment progress to 1 and stop before executing the chooser or
adviser. The Rust phase checker reproduces the three boundaries. This traces
the saved city in isolation, before opponents or the rest of a round can
alter it.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_city_science_entry_check.py --out /tmp/city-science-entry.json
cmp docs/evidence/import-city-science-entry.json /tmp/city-science-entry.json
```


### Handoff from viewing mode

The [round handoff probe](evidence/import-round-handoff.json) supplies
characters through a host message-dispatch substitute. The original
character handler and civ scheduler execute in the same prepared stack.
Two controls exhaust the player's units before reaching the viewing-mode
poll, then dispatch one Enter with a closed city window.

With the saved options word `0x01cd3258`, the original handler takes the
city-window command path and returns to the next input poll for civ 4.
Clearing the process preference bit `0x40` produces `0x01cd3218`. That
control leaves viewing mode, clears the resume civ and reaches civ 5's
native turn-start routine `0x4725c8`. Execution stops on entry to city
slot 6's phase at `0x4cb01d`, on the same numbered turn. The player's
civ and unit records, the cities, the tile bytes and all seven memory
arrays stay intact through that boundary.

The selected-unit route keeps the saved preference word and movement
budgets. Four Space orders spend the full allowance of slots 6, 9, 26 and
18, in that order. The fourth order hands off automatically to the same
civ and city phase. Each unit's direction byte becomes 255 and the viewer's
saved map knowledge stays intact. This route supplies no Enter command.

The shared engine now has a per-civ city-phase call used by its existing
round-close loop. Playable classic continuation still has to schedule it
before each subsequent civ's orders and persist that scheduling.
This trace establishes the entry boundary in the prepared context. It
does not run the next city, an AI order or a full live round.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_round_handoff_check.py --out /tmp/round-handoff.json
cmp docs/evidence/import-round-handoff.json /tmp/round-handoff.json
```

## Checked resume decisions

The [composed probe](evidence/import-pending-choice.json) carries one
prepared memory image through the event reader, stock Rules refresh,
caller preference overlay, and first human input poll at the saved
viewport. Its resumed-civ control skips round-start dispatch. Forced
restart and fresh-round controls cross into dispatch. The probe supplies
display context and host queue operations.

The pinned seat reaches input with government 1, an empty research
project, and zero science progress. Separate controls leave input
available with a stored progress of one or with Anarchy. Those controls
establish the resume boundary. Fresh-turn science and government
resolution remain distinct. The inspector compares the exact pinned
capture and the engine's initial decision state. An experimental
checkpoint preparation requires that comparison to pass. The same probe
checks the two victory records. The pinned marks are clear. Controls that
set an active or spent mark retain it at the first poll. Experimental
checkpoint preparation accepts the pristine initial event state and
rejects these marked controls.

```sh
CIVTOT_HOME=... PYTHONPATH=/path/to/unicorn:tools python3 tools/import_pending_choice_check.py --out /tmp/import-pending-choice.json
cmp docs/evidence/import-pending-choice.json /tmp/import-pending-choice.json
```
