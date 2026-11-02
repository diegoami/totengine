# Backlog

Working queue for ToTEngine. Mirrored on GitHub Issues — epics:
[#19 Epic T0](https://github.com/diegoami/totengine/issues/19) ·
[#20 Epic T1](https://github.com/diegoami/totengine/issues/20) ·
[#21 Epic T2](https://github.com/diegoami/totengine/issues/21) ·
[#22 Epic T3](https://github.com/diegoami/totengine/issues/22) ·
[#23 Epic RG](https://github.com/diegoami/totengine/issues/23).
Story `T0-4` is [issue #1](https://github.com/diegoami/totengine/issues/1) and so on
sequentially through `T1-9` = #18. IDs here are the source of truth; issues are the
working surface. Strategic context lives in
[docs/plan.md](docs/plan.md); tiers T0–T4 are defined there.

Legend — **Pri**: P0 = now, P1 = next, P2 = later · **Est**: S ≤ 1 day, M ≤ 1 week,
L ≤ 1 month (part-time) · **Diff**: E = easy, H = hard (see
[Assigning work](#assigning-work-difficulty-and-agents)) · **Status**: todo / doing / done.

Estimate is calendar size; difficulty is uncertainty. They are independent:
a large parser port can be easy (L/E), a small format puzzle can be hard (S/H).

## Assigning work: difficulty and agents

Two difficulty levels — a ToTEngine-local convention for picking the agent and its
effort level. It is deliberately independent of the review process being adopted;
when stories become harness tasks, the difficulty decides the assignment, the
harness decides the review.

| Diff | Meaning | Agent assignment |
|------|---------|------------------|
| **E** | Low uncertainty: mechanical, well-specified, tooling or docs. Format known, acceptance testable in one pass. | `general` agent, standard effort — or `explore` (medium) for pure-research stories. One review pass. |
| **H** | Reverse-engineering, parity-sensitive engine work, or architectural. Unknowns that only the oracle/experiments resolve. | `general` agent at **high effort**, design note agreed before code. Cross-model review; second opinion on milestone-critical stories. |

Rules of thumb: anything gated by *oracle parity* is H regardless of size; anything
whose acceptance is "it parses/renders and tests pass" is E unless the format itself
is unknown. An H story that grows two failed attempts gets split before a third.

Rules of thumb: anything gated by *oracle parity* is H regardless of size; anything
whose acceptance is "it parses/renders and tests pass" is E unless the format itself
is unknown. An H story that grows two failed attempts gets split before a third.

## Releases & milestone reviews

Releases are tags `vX.Y.Z` on `main`, gated by an independent milestone review before the
tag — one big review per release by a model family that implemented none of it, verdict
AGREE/BLOCK on a milestone issue, findings as ordinary issues, tag on exactly the reviewed
SHA. The process: [docs/milestone-review.md](docs/milestone-review.md). Suggested themes:
`v0.1.0` = Tier 0 complete, `v0.2.0` = first Tier-1 headless milestone (per plan.md §5).

## Epic T0 — Format toolchain & reconnaissance (current)

| ID | Story | Pri | Est | Diff | Status | Acceptance |
|----|-------|-----|-----|------|--------|------------|
| T0-1 | ~~Rules.txt parser → JSON summary~~ (done in first probe) | P0 | S | E | done | `totctl rules` output committed to `docs/evidence/` |
| T0-2 | ~~`.MP` decode + PNG render~~ (done, proven via biome-latitude check) | P0 | S | H | done | World.mp renders as Earth; findings in `docs/evidence/` |
| T0-3 | ~~SAV/SCN structural walker + autosave differ~~ (done for front sections) | P0 | M | H | done | ROMFALL walk + 40-byte unit-record delta demonstrated |
| T0-4 | Collect ToTPP Lua API reference (CHM/PDF from ToTPP release) + feature docs into `docs/refs/` | P0 | S | E | done | Lua `civ.*` API doc available offline; linked from plan |
| T0-5 | Language & stack decision (Rust vs C++), repo skeleton, CI on first build | P0 | M | H | done | Decision recorded in `docs/decisions/0001-language.md`; CI green |
| T0-6 | Extend SAV walker: unit records (40 B) & city records (92 B), count fields, `year` decoding | P0 | M | H | done | Walk prints full unit/city tables for ROMFALL.scn, verified vs in-game cheatscreen |
| T0-7 | Decode ToTPP `0x33` tail (diff `0x32` vs `0x33` of same scenario) | P0 | L | H | done | Warring States autosave walks fully; layout documented |
| T0-8 | `AnticChina.MP` variant decode (ToTPP-era MP flavor) | P1 | S | H | done | File parses + renders; layout note in evidence |
| T0-9 | Golden-file harness: parse-corpus tests over local scenarios (FallRome ×8, Midgard, china ×3) | P1 | M | E | done | `make test` re-generates & compares evidence corpus |
| T0-10 | Oracle automation: launch game under Xvfb via Wine, xdotool macro performs one action, capture save, structural diff | P1 | M | H | done | Scripted "save→act→save→diff" produces annotated deltas hands-free |
| T0-11 | `totctl` UX: `CIVTOT_HOME` env, rivers/improvements overlay on map render, `--scale` | P1 | S | E | done | Renders show rivers & railroads distinctly |
| T0-12 | License check of prior-art repos (axx0/Civ2-clone, richardpenman/civ2); record what may be referenced | P2 | S | E | done | Note in `docs/prior-art.md`; compliant referencing policy set |

## Epic T1 — Headless classic-scenario engine

| ID | Story | Pri | Est | Diff | Status | Acceptance |
|----|-------|-----|-----|------|--------|------------|
| T1-1 | Typed rules model: all 22 Rules.txt sections → engine structs incl. ToT tables (@TERRAIN1..3, transport relationships) | P1 | L | E | todo | Round-trips Rules.txt losslessly (text → structs → text) |
| T1-2 | World state model + deterministic turn kernel skeleton (seeded RNG abstraction) | P1 | L | H | todo | Headless boot of ROMFALL.scn into state; state checksum reproducible |
| T1-3 | Movement, terrain costs, zone of control, multi-map slot | P1 | M | H | todo | Move orders match original outcomes on oracle cases |
| T1-4 | Cities: food/shields/trade, growth, production queue, specialists, unrest | P1 | L | H | todo | 50-turn city math matches oracle saves exactly |
| T1-5 | Research tree, governments, wonders (incl. obsolete-by-tech) | P1 | M | E | todo | Tech progression parity on scripted runs |
| T1-6 | Combat: rounds, veterans, walls, terrain bonuses; identify original PRNG & seed | P1 | L | H | todo | Battle outcome sequences reproduced from same-seed oracle saves |
| T1-7 | Macro events interpreter (@IF/@THEN, ~50 commands, DELAY heap, JUSTONCE) | P1 | L | H | todo | FallRome Events.txt fires identically on scripted turns |
| T1-8 | Save writer: byte-compatible `0x32` .SAV | P1 | L | H | todo | Original (Wine) loads our save; re-save diff is field-explained |
| T1-9 | CI milestone job: 50-turn headless FallRome run vs golden saves | P1 | M | E | todo | Nightly job green; regressions caught |

## Epic RG — Regular game track (roadmap; enters after the T1 core)

Playing a *regular* game — new game from scratch, random or Earth world, AI
opponents, full campaign to a victory — needs more than the scenario engine:
nothing in T1–T3 produces a new-game state, generates a world, or fields
opponent AI. These stories reuse the T1 core; the new-game *UI* lands with T2.

| ID | Story | Pri | Est | Diff | Status | Acceptance |
|----|-------|-----|-----|------|--------|------------|
| RG-1 | Game-setup state model: new-game parameters (civ/leader/difficulty, world size/land%/climate, opponent count, rules variants) + its save encoding | P2 | M | H | todo | Setup section byte-matches oracle new-game saves across variants (rig-driven) |
| RG-2 | Starting placement: civ starts, goody huts, barbarian seeding from the game seed | P2 | M | H | todo | Same-seed oracle starts reproduce identical placement |
| RG-3 | Random world generator (seeded; `.MP`-shaped output; original's exact algorithm unknown → our own, flagged) | P2 | L | H | todo | Statistical parity vs original maps (land%/biome spread); renders sanity-checked |
| RG-4 | Opponent AI v1 headless: expansion, economy, research, unit handling; deterministic | P2 | L | H | todo | 50-turn autoplay stable + deterministic; plausible power curves vs oracle games |
| RG-5 | Diplomacy model: contacts, embassies, treaties, tribute, AI stances; save encoding | P2 | M | H | todo | Diplomacy state round-trips; oracle-save contacts match |
| RG-6 | Victory & scoring: spaceship chain, conquest, score/time; end-game state | P2 | M | H | todo | Victory conditions trigger identically on scripted oracle campaigns |
| RG-7 | Headless campaign loop: new game → autoplay N turns → outcome; CI golden run | P2 | M | H | todo | Nightly autoplay green; determinism (same seed+build ⇒ identical saves) |

ToT's four built-in campaigns (Original/Extended Original, Fantasy, Sci-Fi
quest modes) finish in T4 on top of RG + multi-map (see plan.md §5).

## Epic T2 — Graphical classic client (planned; stories drafted on entry)

 SDL2 client, city screen, orders/menus, original art & sound pipeline,
 plausible AI v1, human-playable FallRome end-to-end. Break down when T1 lands.

## Epic T3 — ToTPP layer (planned)

 Lua 5.3 host + `civ.*` API slices, `0x33` write support, Static.spr sprites,
 custom mod resources, *Warring States* boot & Lua conformance. Break down at T2
 completion.

## Standing tasks

- Keep `docs/evidence/` regenerated on every format-affecting change (T0-9 makes
  this automatic).
- Every format claim gets an evidence artifact or oracle test — no folklore.
- No original game assets in the repo, ever.
- Difficulty is re-assessed when a story starts; a wrong guess is a finding, not a
  failure — record it on the story.
