# Headless experiment manifests — format 1

Status: **generated-world slice of [#203](https://github.com/diegoami/totengine/issues/203);
not released, not owner-accepted.** Program: [#184](https://github.com/diegoami/totengine/issues/184).
Scenario starts wait for admitted scenario entry
([#197](https://github.com/diegoami/totengine/issues/197)). Batches with
interrupt/resume, first-divergence comparison and outcome reports are
[#204](https://github.com/diegoami/totengine/issues/204).

An experiment manifest says exactly what to run headlessly and how runs differ.
Running it produces one verifiable [recording](recording-bundles.md) per run and
deterministic summaries. The same manifest always reproduces byte-identical
output.

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
```

`check` prints the manifest and Rules identities, the engine, and each run's id,
seed, policy, setup descriptor and changed parameters.

`run` requires that `--out` **does not exist yet**. It never overwrites results
or author files. It writes:

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

Each run is exactly the corresponding `totctl campaign` invocation. For example,
`hard-s3` above equals
`totctl campaign Rules.txt --policy ai-v1 --seed 3 --difficulty 4 --width 40 --height 30 --turns 100`,
which the tests check.

## Not in this slice

- Scenario starts (#197).
- Per-civilization policies: they need a campaign format with one policy per
  civ.
- Resume of a partly completed batch, first-divergence comparison and outcome
  reports (#204).
- An MCP tool for experiments. The server never writes, so it would return
  summaries inline.
- Agent (#256) seats in experiments.
