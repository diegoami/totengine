# Versioned scenario initializer and checkpoint extension

Milestone 17 / #197, persistence slice. This is engine persistence policy,
not a new claim about ToT bytes. Native phase/movement evidence lives in
the evidence README's Saved round position section; the faithful admission
gate remains [classic-surface-032-v1](scenario-entry-contract.md).

## Boundary

New experimental scenario sessions will use human checkpoint **format 3**
with initializer schema **1**. Generated games and legacy scenario checkpoints
keep format 2 and its canonical replay. Both clients retain **Open scenario
(experimental)**; no faithful Open ToT save / scenario route is activated.
The initializer explicitly identifies its profile as `experimental-scenario-v1`.
This identifier does not admit any corpus input under classic-surface-032-v1.

The extension binds:

- The install-relative source path, captured byte length and fingerprint;
  the same original snapshot must be supplied again on restore. Original
  bytes stay private and are never embedded in public evidence or recordings.
  This slice uses an external input reference: retain the exact selected
  bytes at the recorded relative path (a byte-identical private installation
  copy also works). A private snapshot-cache lifecycle belongs with #198.
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

Faithful admission still needs an admitted saved-human-view recipe: the current
experimental runtime adds the known extra engine-sight tile on load, and the
native saved readers are not yet the session's human observation source.
Snapshot normalization must be declared and measured, and unknown native fog
memory must reject under the frozen contract. Independently admitted SCN
startup context is needed where applicable, along with full dependency checks
and #198's atomic entry/confirmation routes with owner acceptance. Original AI
mission/cache/RNG continuation is explicitly outside the first profile.
The additional native observation/city-memory kernels are implementation
building blocks, not amendments adding unrelated admission prerequisites.
See [the milestone progress map](m17-import-progress.md). This persistence
slice implements no video/ending screens or native SAV export.
