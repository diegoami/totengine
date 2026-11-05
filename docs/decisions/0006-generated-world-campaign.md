# v0.9.0 — generated worlds and headless campaigns

The next milestone after v0.8.0 is v0.9.0. The live scope is #90 (RG-3,
seeded world generator) and #91 (RG-7, new game → autoplay → outcome with
deterministic checkpoints and a CI golden run). Their GitHub issue bodies
remain the acceptance authority. Original-like built-in AI remains v0.10;
UI discovery/design remains v0.11, with client implementation across later
milestones and pluggable AI after the playable client.

## Implementation sequence

1. **Generator and measurement boundary.** `engine::worldgen` implements an
   explicitly engine-designed algorithm, rather than a claimed original port.
   Settings/seed determine output independently of gameplay RNG. Integer
   smoothed elevation/moisture/relief fields, ranked sea level, latitude
   biomes and sea-rooted basin drainage produce a dense surface map. `totctl
   worldgen` exports stock `.MP`-shaped bytes for existing decoders/renderers.
   Export round trips do not establish original-game map import.
2. **Statistical calibration.** Compare controlled original generated maps
   under matched world settings. Measure land fraction, biome spread, rivers
   and spatial coherence over multiple seeds; inspect renders. Retain only
   numeric summaries, hashes and derived renders. The first baseline has two
   default-setting RG-2 worlds plus one changed-setting control: this is too
   small for #90 acceptance. Broaden the original corpus before claiming it.
3. **Fresh-game initialization.** Feed generated terrain into the decoded
   placement boundary, compute required landmass/site/fertility inputs from
   evidence, create civ/unit/research state from Rules and explicit setup,
   and expose fresh-game construction through the player API. Do not use a
   stock save's cities or stale AI caches as a substitute for initialization.
4. **Campaign and checkpoints.** Drive the new world through AI v1, record
   setup identity, seed, order log and fingerprints; replay from the same
   generated initial state. Capture enough checkpoint state for continuation
   and deterministic comparisons, with an explicit versioned format. Native
   `.SAV` compatibility is a separate claim requiring the original load gate.
   Finite turn limits/outcome reporting must distinguish a run limit from
   a proven game victory; unknown victory rules are not to be invented.
5. **CI and release gate.** Add an asset-free, authored-rules campaign golden
   and scheduled autoplay alongside the install-backed checks. Verify #91's
   deterministic save/checkpoint acceptance and the decision-0002 order-log
   replay contract, then use the standard owner-run milestone review.

## First implementation slice (2026-11-05)

The generator, CLI export, decoder round-trip/determinism/overwrite checks,
metrics example and first comparison are implemented. The seed-7 render was
inspected. `docs/evidence/worldgen-v1.json` records source hashes and all
limits; `worldgen-v1-metrics.tsv` compares 16 engine seeds with the three
retained original worlds. No original assets were added.

Default land fraction is 47%, calibrated to the two default original samples
(mean 47.225%). The engine prototype still has excess polar terrain and too
few rivers/hills/mountains: original default river coverage is 4.025% of
real tiles, prototype mean 0.4188%. Neither #90 nor #91 is complete. The next
implementation task is biome/relief and drainage calibration plus a broader
matched-settings comparison, before campaign initialization.

Validation: all 116 engine unit tests, the asset-free CLI integration test,
workspace formatting and all-target Clippy with warnings denied passed.

## Biome/drainage correction (2026-11-05)

Algorithm v2 narrows the polar bands, adjusts moisture/relief thresholds and
replaces sparse strictly downhill river attempts with a flood outward from
the sea. Each land tile receives a previously visited downstream parent;
integer rain accumulation marks continuous rivers, including outlets across
inland sinks. Heap ties use tile index. These are explicit engine design
choices, not claims about original map generation.

The 16 tuning seeds are now separate from 64 fresh engine seeds (16–79).
`tools/worldgen_compare.py` checks the numeric corpus and produces means,
ranges, sample deviations and differences from the reference. V2 evidence
and three inspected renders are in `docs/evidence/worldgen-v2.json` and the
evidence README. Holdout biome means differ from the two original reference
maps by less than 0.9 percentage points each; river coverage is 3.4312%
versus 4.025%. This is a calibration result, not a statistical acceptance
gate: broader matched-settings original sampling and spatial comparison
remain required. The original samples are unchanged, with hashes rechecked;
no new live oracle run is claimed.

Regressions check sea-connected river components, a synthetic inland basin
with a flat spillway, and the smallest/largest supported dimensions and land
targets. Neither #90 nor #91 is complete; fresh campaign initialization is
still pending.

Validation: `CIVTOT_HOME=/mnt/d/games/CIVTOT make test` passed all 186 Rust
tests across 36 test binaries, the three Python report checks and the full
golden corpus (including the 50-turn fingerprint run and Rust/Python format
crosschecks). Workspace formatting and all-target Clippy with warnings
denied passed; the report re-derived byte-identically from its retained TSV.
