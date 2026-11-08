# Generated-world campaigns

This command's generated-world scope remains as documented below. The broader
[mission](mission.md) adds headless scenario parameter experiments under
[#184](https://github.com/diegoami/totengine/issues/184), dependent on admitted
scenario support [#183](https://github.com/diegoami/totengine/issues/183), with
observation/recording through #185/#186. A campaign checkpoint is an existing
foundation, not a claim that these scenario/recording packages are implemented.

`totctl campaign` boots an engine-designed surface world directly from a
seed and Rules. It uses no reference save. The v0.10 `builtin-1` opponent
drives every civilization; `--policy ai-v1` selects the legacy policy.

```sh
cargo run -p totctl -- campaign "$CIVTOT_HOME/Original/Rules.txt" \
  --seed 7 --turns 100 --checkpoint /tmp/campaign.ctc --log /tmp/campaign.log
# Resume for another 50 rounds, with the same Rules:
cargo run -p totctl -- campaign "$CIVTOT_HOME/Original/Rules.txt" \
  --resume /tmp/campaign.ctc --turns 50 --checkpoint /tmp/campaign-150.ctc
# Asset-free authored test rules also work:
cargo run -p totctl -- campaign crates/engine/tests/fixtures/campaign-rules.txt \
  --policy ai-v1 --seed 7 --turns 100
```

Options: `--seed` (sets world and gameplay seeds), `--width`, `--height`,
`--land-percent`, `--civs` (2–7), `--difficulty` (0–5), `--turns`,
`--checkpoint`, `--log`, `--policy ai-v1|builtin-1`, and `--resume`. Resume uses its recorded setup and
rejects setup overrides. Outputs refuse to overwrite an existing file.
The default is an 80×50 logical world, 47% land, five AI civs, difficulty 2,
seed 1, and 100 rounds. Generator bounds are documented by `totctl worldgen`;
campaign setup can additionally reject excessive land/sea regions or a
placement that fails to yield distinct land starts. At most 30,000 rounds
are supported per campaign.

## Initialization policy v1

Terrain and rivers come from generator v2. Eight-neighbor connected land and
water components receive distinct region IDs (1–254); the original's region
assignment, which can associate coastal water with land, is not reproduced.
Water regions retain their tile count for sea-arrival admission. The start
sampler receives each land region's plains/grassland count as its +2 input.
Fertility is an engine estimate: four times Rules base food plus a quarter
of neighboring base food, clamped to 1–15 on land; site-cache bits start zero.
These choices are explicitly engine policy rather than original-generation
or original-initialization parity.

The decoded placement sampler chooses one unhomed, initially buildable
land settler per civ. Each civ starts in Despotism with zero treasury and
advances, 60% science, 40% tax and no cities or improvements. Names are
engine labels. Huts and hashed special resources are enabled; recurring
barbarian arrivals default to the zero activity level. RNG streams for
terrain, placement, gameplay and AI are separate. Power and landmass caches
are derived from the fresh world before play.

Only the standard surface terrain IDs and classic unit/advance capacities
are supported. Scenario events, authored scenario starts and multiplayer
setup are outside this command's scope. The built-in policy requires the classic
research/leader tables; the smaller authored campaign fixture supports AI v1.
See [decision 0007](decisions/0007-original-like-ai.md) for behavioral scope.

## Checkpoint contract

`.ctc` is a versioned **engine replay checkpoint**, not a native `.SAV`.
Format 1 retains AI v1. Format 2 records the explicit `builtin-1` policy;
unknown policies and policy overrides during resume are rejected.
It records initialization/generator versions, setup, engine package version,
Rules identity, the full order log and the final world fingerprint. Restore
rebuilds the initial world and AI players, replays the deterministic AI
prefix, and requires identical log/checkpoint bytes. This restores private
AI random state as well as the world's state, at a cost proportional to the
number of recorded rounds. Rules must be supplied again. Changed Rules,
unsupported versions, truncated/tampered transcripts and divergent state
are rejected. Checkpoints are reproducible for the same supported build;
cross-version compatibility is not promised.

`TurnLimit` means the requested rounds were consumed with multiple civs
remaining. The other outcomes report one or zero civilizations remaining.
These are observed conditions, not claims that the original's victory
rules have fired.

## Verification

The authored-rules 100-round golden contains the complete order transcript
and final fingerprint in `docs/evidence/generated-campaign-v1.ctc`.
Regular tests compare it byte for byte. The scheduled workflow runs both policies across 32 seeds
for 100 rounds each, comparing uninterrupted play against checkpoint restore
at round 37 plus 63 more rounds and independently replaying each order log.

```sh
cargo test -p engine --test generated_campaign
cargo test -p engine --test generated_campaign --release -- --include-ignored
cargo test -p totctl --test campaign
# Install-backed fresh-game test (uses Rules, never a save):
CIVTOT_HOME=/path/to/CIVTOT cargo test -p engine --test generated_campaign_install
```

The test Rules are authored by this project and have synthetic balance.
They are not a copy of original game data. Numeric trajectories using both
those Rules and the user's Original Rules are retained in `docs/evidence`.
