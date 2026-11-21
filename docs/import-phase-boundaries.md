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
