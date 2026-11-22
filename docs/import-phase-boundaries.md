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
starts after the final refresh and file close. It shows that the same-mode
successful return preserves that phase and that restoring a different mode can
fill an empty human mask using the living resume civ or the first living civ.
The interval between the two measurements still needs evidence.

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
