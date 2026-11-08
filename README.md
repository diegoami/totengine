# ToTEngine — Civilization II: Test of Time, recreated

An open, native recreation of **Civilization II: Test of Time** for desktop and
browser play, headless all-AI experiments, observation and scenario work.
**ToT scenario compatibility is the project's distinguishing purpose.**

The main game experience reads the user's original ToT resources, including
scenario overrides. The original executable is not needed to run our engine;
no proprietary artwork, maps, saves, sounds or scenarios are distributed.

## Mission

- Play standard games and scenarios in both an ordinary desktop application and
  a playable browser client, backed by the same native Rust simulation.
- Test scenarios headlessly with all civilizations under AI, varying declared
  parameters and comparing reproducible outcomes.
- Observe live games and replays, including scenario events, objectives and maps.
- Recreate ToT's complete UI and controls, with optional QoL and alternative
  settings such as city production queues and rally points.
- Support different, pluggable AIs and evaluate C-evo/Freeciv AI/client/workflow
  reuse without replacing ToT mechanics or scenario semantics.
- Help open, diagnose, test and eventually create/edit scenarios.
- Help record playing and observation sessions, with replay, annotations and
  later audiovisual capture.

These are product targets, not a statement that all are implemented.
[Mission and boundaries](docs/mission.md) · [Delivery plan](docs/plan.md) ·
[Live backlog](BACKLOG.md) · [Planned milestones](docs/milestones.md).

## Status

Latest released tag: **v0.15.0 — city information panels**, independently
reviewed on [#162](https://github.com/diegoami/totengine/issues/162).
[Windows/Linux downloads](https://github.com/diegoami/totengine/releases/tag/v0.15.0)
include `totplay` and `totdesktop`, runtime/dependency notices and no original
resources. See [launch/controls](docs/playable-clients.md),
[readiness](docs/v0.15.0-readiness.md) and [changelog](CHANGELOG.md).

| Available foundation | Current boundary |
| --- | --- |
| Both playable clients: generated surface games, unit input, founding/research, workers/specialists, quoted production/buy/sell, city panels and checkpoints | Complete original UI, original-save/scenario import, diplomacy/government/report depth and sustained scenario campaigns remain unfinished |
| Headless save/scenario boot and modeled turns/events, deterministic order replay, generated campaigns | A successful parse/boot is not full scenario compatibility; general scenario parameter sweeps and objective-aware reports remain planned |
| Browser observer: live turns, cached replay, full/per-civ views, turn controls and archives | Scenario-specific presentation, complete event/objective inspection and recording integration need further work |
| Versioned built-in opponent and legacy policies | Original-derived research and supported land campaigns are partial; full native AI and external plugins are not shipped |
| Format inspection/diff, order logs, observer archives and replay checkpoints | Full scenario authoring and unified play/observation recording are future programs |

Pending candidates remain separate from releases:
[Isotrident/Open demo #181](https://github.com/diegoami/totengine/pull/181) and
[client follow-ups #182](https://github.com/diegoami/totengine/pull/182).
The approved Isotrident artwork fallback is optional and separately licensed;
Open demo is a small authored demonstration, not full redistributable ToT rules.
Public hosting and a complete independent rules profile remain separate scope.

Freeciv and C-evo are concrete sources for AI, client and UI-workflow assessment,
not ToT rules oracles. Their playing strength and adaptation cost have not been
benchmarked here. [Source assessment](docs/ai-client-reuse-assessment.md) ·
[Artwork assessment](docs/ui/freeciv-assessment.md) ·
[Reuse policy](docs/prior-art.md).

## Documentation

| Document | Purpose |
| --- | --- |
| [Mission](docs/mission.md) | Product targets, compatibility, resources and reuse boundaries |
| [Delivery plan](docs/plan.md) | Workstreams, dependencies, sequencing and acceptance |
| [Backlog](BACKLOG.md) | Map to live GitHub programs and story/release conventions |
| [Client packages](docs/ui/client-milestones.md) | Both-client delivery, original UI contracts and human acceptance |
| [Headless campaigns](docs/generated-campaign.md) | Seeded fresh games, AI autoplay and deterministic checkpoints |
| [Browser observer](docs/browser-observer.md) | Live/replay usage, fog modes and stream semantics |
| [Evidence](docs/evidence/README.md) | Measured format/mechanics findings and reproducible checks |
| [Milestone review](docs/milestone-review.md) | Owner-launched independent review and exact-reviewed-SHA releases |
| [Environment](docs/environment.md) | Local toolchain, installation and reference/oracle setup |
| [Initial feasibility study](docs/feasibility.md) | Historical reconnaissance; the current plan supersedes its schedule |

## Development

```bash
# Rust CLI (successor of the Python format probe)
cargo run -- mp /mnt/d/games/CIVTOT/World.mp

# Full local suite; install-backed checks self-skip without CIVTOT_HOME
export CIVTOT_HOME=/mnt/d/games/CIVTOT
make test
make fmt clippy
make evidence-regen  # after an intentional format change

# Python format inspection
python3 tools/totctl.py rules "$CIVTOT_HOME/Rules.txt" -o /tmp/rules.json
python3 tools/totctl.py map "$CIVTOT_HOME/World.mp" -o /tmp/world.png --json /tmp/world.json
python3 tools/totctl.py save "$CIVTOT_HOME/FallRome/ROMFALL.scn" -o /tmp/walk.json
```

The Rust workspace contains `tot-kit`, `engine`, `lua-host`, `totctl` and `totui`.
Original behavior is measured using the owner's installation and an isolated
Wine oracle. Read [its README](tools/oracle/README.md) and
[lessons](docs/lessons.md) before running it. The engine and clients run natively;
Wine is a development reference, not their runtime architecture.
