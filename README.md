# civtotre — Civilization II: Test of Time, recreated

Goal: an open, cross-platform program that **replicates the gameplay of Civ2: Test
of Time (ToT)**, with first-class support for the community's **scenarios** (both the
classic 1999-era macro-events kind and the modern ToTPP/Lua kind).

The engine is a *recreation*, not a bundle: it requires the user's original ToT
installation and reads its data files directly.

| Document | Purpose |
|---|---|
| [BACKLOG.md](BACKLOG.md) | Working queue: epics, stories, difficulty, acceptance criteria |
| [docs/milestone-review.md](docs/milestone-review.md) | Release process: independent milestone review, AGREE/BLOCK verdict, tag-on-reviewed-SHA |
| [docs/feasibility.md](docs/feasibility.md) | Evidence-based feasibility evaluation: formats, prior art, risks, effort, value assessment, verdict |
| [docs/plan.md](docs/plan.md) | Phased implementation plan: architecture, milestones, testing strategy |
| [docs/evidence/](docs/evidence/) | Tier-0 probe results — measured format findings, rendered map, save walks |

## Local reference environment (verified)

- Game: **Civ2 Test of Time 1.1** at `/mnt/d/games/CIVTOT` (all four built-in game
  modes: Original, Extended Original, Fantasy/Midgard, Sci-Fi)
- **ToTPP** (Test of Time Patch Project) installed — extended limits, Lua 5.3.6, mods
- Test scenarios: `FallRome` (classic `Events.txt` macro) and
  `Scenario/china` *Warring States* (2024, heavy ToTPP Lua, 6 MB saves)
- Runs under **Wine 10** with prefix `~/.wine_civ2tot` — usable as a behavioral
  *oracle* for differential testing:

  ```bash
  WINEPREFIX=~/.wine_civ2tot wine /mnt/d/games/CIVTOT/civ2.exe
  ```

## Status

**Tier 0 started — first tool works against real game data.**

```bash
# format probe: Rules.txt / .MP maps / .SAV+.SCN walks / save diffs
python3 tools/totctl.py rules /mnt/d/games/CIVTOT/Rules.txt -o /tmp/rules.json
python3 tools/totctl.py map   /mnt/d/games/CIVTOT/World.mp \
        --rules /mnt/d/games/CIVTOT/Rules.txt -o /tmp/world.png --json /tmp/world.json
python3 tools/totctl.py save  /mnt/d/games/CIVTOT/FallRome/ROMFALL.scn -o /tmp/walk.json
python3 tools/totctl.py diff  <a.sav> <b.sav> -o /tmp/diff.json
```

Highlights from the first session (details in [docs/evidence/](docs/evidence/)):
`.MP` format decoded and *proven* (World.mp renders as Earth with correct biome
latitudes); ROMFALL.scn walked — all 7 civ names decoded; ToTPP `0x33` saves share
the base front layout; the save-differ identified a "−1 unit record" delta between
two real autosaves.
