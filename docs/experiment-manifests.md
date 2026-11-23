# Headless experiment manifests — format 1

The generated-world manifest shipped with v0.19.0
([#203](https://github.com/diegoami/totengine/issues/203)). The generated campaign
batch and replay slice advances [#204](https://github.com/diegoami/totengine/issues/204).
Scenario starts require the admitted initializer tracked in
[#197](https://github.com/diegoami/totengine/issues/197).
Program [#184](https://github.com/diegoami/totengine/issues/184).

An experiment manifest says exactly what to run headlessly and how runs differ.
Running it produces one verifiable [recording](recording-bundles.md) per run and
deterministic summaries. Successful runs with the same manifest, engine, Rules
and round limit produce byte-identical output.

## Manifest

```json
{
  "format": "totengine-experiment 1",
  "name": "difficulty-sweep",
  "engine": "0.18.0",
  "start": {
    "kind": "generated",
    "rules": "Rules.txt",
    "rules_identity": "9535 1ccad4603a4ff062",
    "width": 40, "height": 30, "land_percent": 47,
    "civilizations": 5, "difficulty": 2
  },
  "policies": {"default": "builtin-1"},
  "rounds": 100,
  "seeds": [3, 5, 7],
  "variants": [
    {"name": "easy", "difficulty": 0},
    {"name": "hard", "difficulty": 4, "policy": "ai-v1"}
  ]
}
```

- **`start`** is the base setup. `rules` is relative to the manifest's folder.
  Omitted world fields use the engine defaults (80×50, 47% land, 5
  civilizations, difficulty 2). The optional `rules_identity` (`<len> <fnv>` of
  the canonical Rules, as `totctl record info` prints it) pins the exact Rules.
- **`seeds`** are random variation: each seed sets both the world and the
  gameplay seed.
- **`variants`** are deliberate changes from the base: `width`, `height`,
  `land_percent`, `civilizations`, `difficulty` or `policy`. Each run records
  which parameters its variant changed. Without variants there is one `base`
  variant.
- Runs are *variants × seeds*, named `<variant>-s<seed>`, at most 256.
- **`engine`** is optional; when given, it must equal this build.
- Unknown fields are rejected everywhere.

### Rejected rather than approximated

- `"kind": "scenario"` starts: they need admitted scenario entry (#197).
- **Mixed per-civilization policies** (`policies.per_civ` differing from
  `default`): campaigns run one policy for every civilization. The manifest is
  refused instead of silently running something else.
- Repeated seeds, duplicate or invalid variant names, 0 rounds or more than
  30,000, unknown policies, a Rules-identity mismatch and an engine pin
  mismatch.
- **Invalid worlds:** `check` builds every run's start once, so for example a
  width below the generator's minimum fails validation instead of mid-batch.

## Commands

```sh
totctl experiment check difficulty.json             # validate and list the runs
totctl experiment run difficulty.json --out results # execute into a new folder
totctl record verify results/hard-s3.totrec Rules.txt
totctl experiment run difficulty.json --out pause --round-limit 40
totctl experiment resume difficulty.json --from pause --out resumed
totctl experiment compare difficulty.json results/easy-s3.totrec resumed/easy-s3.totrec
totctl experiment compare difficulty.json results/easy-s3.totrec results/hard-s3.totrec
```

`check` prints the manifest and Rules identities, the engine, and each run's id,
seed, policy, setup descriptor and changed parameters.

`run` requires that `--out` **does not exist yet**. It never overwrites results
or author files. It writes:

- `batch.identity`: engine version, manifest text identity and canonical Rules identity;
- `<run>.totrec`: a reproducible campaign recording per run (`stop` is the
  replayed outcome; producer `totctl experiment <name>: run <id>`);
- `summary.json` (`totengine-experiment-summary 1`): the manifest and Rules
  identities, the engine, and per run its variant, seed, changed parameters,
  policy, setup, rounds, stop reason, fingerprint, living civilizations,
  cities, units, recording file and recording identity;
- `summary.txt`: the same as a table, also printed to stdout.

A run that fails at runtime is listed as `failed` with its error, never as a
result. The summaries contain no timestamps or paths, so rerunning a manifest
gives byte-identical files.

Without a round limit override, each run is exactly the corresponding `totctl campaign` invocation. For example,
`hard-s3` above equals
`totctl campaign Rules.txt --policy ai-v1 --seed 3 --difficulty 4 --width 40 --height 30 --turns 100`,
which the tests check.

## Resume and compare

Use `--round-limit N` to save a checkpoint for each run after at most N
rounds. N must be between 1 and the manifest's target. Runs with further
rounds pending show `paused` and record the `Stopped` stop reason.
Runs that reach a terminal outcome retain that outcome.

`resume` requires the same engine, manifest bytes and canonical Rules.
It replays each checkpoint to verify its orders, progress, policy and
outcome, then continues to the original target. An interrupted command
may leave only `batch.identity` and some completed recordings. Resume can
use these without a summary, and starts any missing run from round zero.
A damaged recording marks its run failed while other runs continue.
Artifacts written before this feature require a fresh batch because they
lack `batch.identity`. The finished output is byte-identical to an
uninterrupted run when all recordings verify.

`compare` verifies two recordings against the expanded manifest. Keep
each original run filename so the command can identify its setup and
policy. It prints the first differing canonical log line, including the
number of matching completed rounds and both values. If one verified
log ends before the other and every overlapping line matches, it reports
the matching prefix. Lines cover setup, the starting fingerprint, order
results and round fingerprints.

## Not in this slice

- Scenario starts (#197).
- Per-civilization policies: they need a campaign format with one policy per
  civ.
- Scenario event and objective outcome reports (#204). An abrupt stop can
  only recover recordings already published at run boundaries.
- An MCP tool for experiments. The server never writes, so it would return
  summaries inline.
- Agent (#256) seats in experiments.
