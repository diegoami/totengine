# ToTEngine — Civilization II: Test of Time, recreated

Goal: an open, cross-platform program that **replicates the gameplay of Civ2: Test
of Time (ToT)**, with first-class support for the community's **scenarios** (both the
classic 1999-era macro-events kind and the modern ToTPP/Lua kind).

The engine is a *recreation*, not a bundle: it requires the user's original ToT
installation and reads its data files directly.

| Document | Purpose |
|---|---|
| [BACKLOG.md](BACKLOG.md) | Working conventions (difficulty, releases); the queue itself is GitHub [milestones](https://github.com/diegoami/totengine/milestones) and issues |
| [docs/milestone-review.md](docs/milestone-review.md) | Release process: independent milestone review, AGREE/BLOCK verdict, tag-on-reviewed-SHA |
| [docs/feasibility.md](docs/feasibility.md) | Evidence-based feasibility evaluation: formats, prior art, risks, effort, value assessment, verdict |
| [docs/plan.md](docs/plan.md) | Phased implementation plan: architecture, milestones, testing strategy |
| [docs/generated-campaign.md](docs/generated-campaign.md) | Seeded fresh games, AI autoplay and deterministic engine checkpoints |
| [docs/evidence/](docs/evidence/) | Tier-0 probe results — measured format findings, rendered map, save walks |

## Local reference environment (verified)

Civ2 Test of Time 1.1 + ToTPP at `/mnt/d/games/CIVTOT`, running under Wine 10
(prefix `~/.wine_civ2tot`) as a behavioral *oracle* for differential testing.
Full provisioning for a new machine — toolchain, scratch space, rebuild
steps for the gitignored oracle binaries, box quirks — lives in
[docs/environment.md](docs/environment.md); the oracle rig recipe in
[tools/oracle/README.md](tools/oracle/README.md).

## Status

The latest released tag is **v0.10.0 — built-in opponent: research and land
campaigns**, independently reviewed on
[#116](https://github.com/diegoami/totengine/issues/116). A version counts as
released once its
`vX.Y.Z` tag exists
([tags](https://github.com/diegoami/totengine/tags); see
[CHANGELOG.md](CHANGELOG.md); the queue is the GitHub
[milestones](https://github.com/diegoami/totengine/milestones)).

What runs today, headless, against your own ToT install:
- **Boot:** scenarios and standard-game saves boot into a world state.
- **Units and cities:** movement; combat replaying the original's battles
  exactly; macro events; city growth; tile yields, corruption, waste and
  the production queue, each decoded from the original executable and
  replayed against its saves.
- **New games:** seeded engine-designed worlds, decoded start-placement
  sampling, AI autoplay and deterministic checkpoints (`totctl campaign`;
  [scope and usage](docs/generated-campaign.md)).
- **Players:** every civ is driven by a player behind a headless
  interface, and games replay bit-exact from their order logs
  (`totctl play` / `totctl replay`; decision 0003).
- **AI:** the built-in opponent researches, expands, reserves garrisons,
  builds field armies, attacks and occupies cities (`totctl play --builtin`).
  Campaigns and browser observation use it by default; legacy AI options remain.
- **Browser observer:** watch live AI turns or
  replay a finished game, with full-map/per-civ fog views and turn controls
  (`totctl observe`; [usage and stream format](docs/browser-observer.md)).

AI v1 expands, invests in city improvements, researches and handles units
through the player interface (`totctl play --ai-v1` / `totctl observe --ai-v1`).
Economy work includes science/tax, recurring trade routes, maintenance,
happiness, wonder effects and capital road/rail waste. Controlled acceptance
checks match 50 original-written city saves and 64 live barbarian phases;
50-turn AI runs are deterministic and replay. The [completion record](docs/v0.8.0-readiness.md)
and [evidence](docs/evidence/README.md) describe the fixtures and limits.
v0.10 adds original-derived research scoring and controlled
production comparisons across six difficulties, with supported land campaigns
and explicit remaining differences. See [the readiness record](docs/v0.10.0-readiness.md)
and [decision 0007](docs/decisions/0007-original-like-ai.md). Full original-AI
parity is not claimed. The owner-requested [complete original-AI scope](docs/original-ai-scope.md)
now defines the remaining ports and completion gates ([program #117](https://github.com/diegoami/totengine/issues/117)).
Full AI reproduction is gated behind the playable UI/client work.
Owner-reviewed client discovery/design starts at v0.11.

```bash
# Rust CLI (successor of the Python probe)
cargo run -- mp /mnt/d/games/CIVTOT/World.mp

# Full local suite: Rust tests + Python probe + golden evidence corpus
export CIVTOT_HOME=/mnt/d/games/CIVTOT   # golden part self-skips when unset
make test
make evidence-regen                       # after an intentional format change

# Python format probe: Rules.txt / .MP maps / .SAV+.SCN walks / save diffs
python3 tools/totctl.py rules $CIVTOT_HOME/Rules.txt -o /tmp/rules.json
python3 tools/totctl.py map   World.mp -o /tmp/world.png --json /tmp/world.json
python3 tools/totctl.py scnmap FallRome/ROMFALL.scn -o /tmp/romfall.png
python3 tools/totctl.py save  FallRome/ROMFALL.scn -o /tmp/walk.json
python3 tools/totctl.py diff  <a.sav> <b.sav> -o /tmp/diff.json
```

Highlights from the first session (details in [docs/evidence/](docs/evidence/)):
`.MP` format decoded and *proven* (World.mp renders as Earth with correct biome
latitudes); ROMFALL.scn walked — all 7 civ names decoded; ToTPP `0x33` saves share
the base front layout; the save-differ identified a "−1 unit record" delta between
two real autosaves.
