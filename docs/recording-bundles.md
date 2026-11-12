# Session recording bundles — format 1

Status: **first slice of [#206](https://github.com/diegoami/totengine/issues/206),
implemented for generated campaigns (CLI) and human sessions (library); not
released, not owner-accepted.** Program: [#186](https://github.com/diegoami/totengine/issues/186).
Code: `crates/tot-record`. Client start/stop controls, markers UI and both-client
acceptance are [#207](https://github.com/diegoami/totengine/issues/207).

## What a recording is

A recording is one UTF-8 file, conventionally `*.totrec`. It separates three
kinds of content, so a reader always knows what can be reproduced and what is
only shown:

| Section | Content | Class |
| --- | --- | --- |
| `gameplay` | The canonical engine checkpoint: `totengine-campaign` for all-AI campaigns, `totengine-human 2` for human sessions. It carries setup, policy identity and the canonical order log. | **Reproducible.** Replaying it reconstructs the game, including private policy state. |
| `observer` | Observer stream v1 frames, labelled with a perspective (`all-views`, `spectator`, `civ-N`) and the round of frame 0. | **Derived.** Verification regenerates the frames from `gameplay` and compares them. Full-state perspectives are labelled as such. |
| `presentation` | NDJSON notes, markers and camera/view changes: one object per line with `round` and `kind`. | **Inert.** It is validated but never executed, and adding or editing it cannot change gameplay. |
| `resources` | The resource identity text a human session bound (profile, relative names, sizes, fingerprints). | **Identity only.** No paths and no asset bytes. |

A bundle with `gameplay` is `content reproducible`. A bundle without it, for
example a single civ's observer export, is `content view-only`: verification
checks identities and labels and does not claim the game can be rebuilt.

## Layout

```text
totengine-recording 1
engine 0.18.0
kind campaign|human
setup <NewGameConfig descriptor>
policy builtin-1|ai-v1
human none|<civ>
rules <len> <fnv64>
producer <free text, e.g. totctl campaign>
content reproducible|view-only
progress <rounds completed (campaign) / world turn (human)>
stop turn-limit|one-civilization <c>|no-civilizations|stopped|interrupted
section <name> <label> <len> <fnv64>
<len bytes>
...
end <section count>
```

Sections appear at most once, in the order `resources`, `gameplay`, `observer`,
`presentation`. Every section is length-framed and FNV-1a-64 fingerprinted, the
same fingerprint engine checkpoints use. The `end` trailer must match the number
of sections and be the last line, so truncation and appended data are detected.
Unknown format versions, kinds, sections or labels are rejected with a message,
not skipped. Files are limited to 64 MiB.

The fingerprints detect accidental change. They are not cryptographic signatures
or provenance proofs.

## Identities and diagnostics

- **Engine.** Replay needs the recording's engine version, as the checkpoints
  already require. A different build is a diagnostic ("recorded with engine X").
- **Rules.** Bound by the length and fingerprint of the canonical emitted Rules.
  Campaign checkpoints alone do not bind Rules; the bundle adds that, so
  verifying against other Rules reports "Rules identity differs" instead of
  replaying a different game. `totctl record info` prints the identity a
  recording needs without any Rules at hand.
- **Resources (human).** The identity text is stored, and the human checkpoint
  binds it. A changed identity is a diagnostic, not a silent substitution. Matching
  the stored identity against a locally loaded resource profile is the client's
  job (#207).
- **Claims.** Verification replays `gameplay` and checks that the declared setup,
  policy, progress, human civ and stop reason agree with the replay. `stopped`
  and `interrupted` are caller-declared; the others must equal the replayed
  outcome. Observer frames must equal regenerated frames byte for byte.

## Writing and interrupted-write recovery

- **Atomic, no-clobber saves.** A bundle is written to a `.totrec-partial-*`
  temporary file in the target directory, synced, and then linked into place
  only if the target does not exist (`tempfile` `persist_noclobber`). An existing
  bundle is never modified, and a failed save leaves no partial target.
- **Journal while recording.** `totctl campaign --record OUT` keeps
  `OUT.journal` while it runs: a header with the manifest identities, then
  `record <progress> <len> <fnv>` frames, each a complete checkpoint, synced to
  disk. It journals the starting state, every `--record-every K` rounds
  (default 10) and the end. After the bundle is saved, the journal is removed.
- **Recovery.** If a run is interrupted, the journal remains, and a new
  recording to the same name is refused until it is dealt with.
  `totctl record recover OUT.journal NEW.totrec` scans the frames, keeps the last
  complete and fingerprint-valid record, discards any torn tail (reporting how
  many bytes), and writes a reproducible bundle with stop reason `interrupted`.
  It never edits or deletes the journal. The recovered checkpoint continues
  exactly like the uninterrupted run, which tests check.

Each journal record is a full checkpoint, so journal size grows with rounds ×
log length. That is acceptable for this slice; incremental records are a
possible later refinement.

## Commands

```sh
totctl campaign Rules.txt --seed 3 --turns 100 --record run.totrec [--record-every 10] [--stream run.ndjson]
totctl record info run.totrec                # identities, labels, no replay
totctl record verify run.totrec Rules.txt    # replay + check every claim
totctl record recover run.totrec.journal recovered.totrec
```

`--record` does not change the run's stdout, checkpoint or log. With `--stream`
the bundle also embeds the all-views observer frames.

## Not in this slice

- Start/stop/save/open controls, markers and annotation UI in the browser and
  desktop clients, and their human acceptance (#207). The library already builds
  and verifies human bundles from a session checkpoint and journals them.
- Scenario/save-based recordings (`.SAV`/`.SCN`): these depend on admitted
  scenario entry and #203 experiment manifests. The manifest carries generated
  setup only.
- Observer frames for human sessions, audio/video/clip capture, sharing and
  hosting.
- An MCP tool for recordings. It would be a natural read-only addition to #250
  stage 1.
