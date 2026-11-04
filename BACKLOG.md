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

## Releases & milestone reviews

Releases are tags `vX.Y.Z` on `main`, gated by an independent milestone review before the
tag — one big review per release by a model family that implemented none of it, verdict
AGREE/BLOCK on a milestone issue, findings as ordinary issues, tag on exactly the reviewed
SHA. The process: [docs/milestone-review.md](docs/milestone-review.md). Themes:
`v0.1.0` = Tier 0 complete (released 2026-11-02 after 5 review rounds),
`v0.2.0` = headless game loop — T1 core + **RG-1** so scenarios *and*
standard games boot and run headless (**RG-2 moved to v0.3.0**, owner
decision 2026-11-03: the new-game entropy source is not found yet),
then the ladder below.

**Release ladder (owner retriage 2026-11-04):** one theme and 2–4 stories
per release, so that every review stays a single round and reviews come
regularly. Each release keeps everything before it green.

| Release | Theme | Stories |
|---|---|---|
| `v0.3.0` | Reproducible new game | RG-2 (start placement ✓, goody huts as objects); landed: map tile framing fix, Latin-1 names, headless demo |
| `v0.4.0` | Minimal production | T1-4 slice: tile yields, production queue (units, improvements) — cities build things headless |
| `v0.5.0` | Player interface | RG-8 (headless API: observation, orders, validation, order log), tested with a random-but-legal player — games replay exactly from seed + order log |
| `v0.6.0` | AI v0 | RG-10 AI v0 (explore, found cities, build settlers and defenders) — **a full all-AI game runs headless** |
| `v0.7.0` | Browser observer | OB-1 stream/replay; OB-2 browser observer (watch live, replay, fog toggle) — **watch the AIs play** |
| `v0.8.0` | AI v1 + economy depth | RG-4 AI v1 (research, attack, defence); RG-9 barbarians; T1-4 rest (unrest, specialists, trade); per-civ techs (GIVETECHNOLOGY applies) |
| `v0.9.0` | Generated-world campaign | RG-3 world generator; RG-7 new game → N all-AI turns → outcome, nightly golden |
| `v0.10.0` | Pluggable AIs | OB-3 JSON protocol + reference client; OB-4 tournament runner |
| `v0.11.0` | Playable client, minimal | T2-1 SDL2 native: map, units, move/attack/found city/end turn, simple city view; human = RG-8 player; observer mode; Windows build in CI |
| later | — | full T2 (city screen, menus, sound), RG-5 diplomacy, RG-6 victory, T1-10 parity, T3 Lua |

**Observing and playing are the two main goals** (decision 0002). The
ladder reaches a watchable all-AI game first (v0.7.0), because it needs
less UI than playing; the playable client (v0.11.0) can move earlier on
the owner's call.

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
| T1-1 | Typed rules model: all 22 Rules.txt sections → engine structs incl. ToT tables (@TERRAIN1..3, transport relationships) | P1 | L | E | done | Round-trips Rules.txt losslessly (text → structs → text) |
| T1-2 | ~~World state model + deterministic turn kernel skeleton (seeded RNG abstraction)~~ | P1 | L | H | done | Headless boot of ROMFALL.scn into state (710 units / 146 cities / 9000 tiles); checksum reproducible — `totctl boot`, tests in `make test` |
| T1-3 | ~~Movement, terrain costs, zone of control, multi-map slot~~ | P1 | M | H | done | Rules columns oracle-proven (move field 3: panel 1→4 test); kernel implements thirds/road/rail/min-1/ZOC/domain; ROMFALL 641/710 movable. **Caveat (T1-6 oracle):** the original's move unit is the byte @0x5f5eb8 = 15 in FallRome (not 3), so 'thirds' match only rulesets where that byte is 3 |
| T1-4 | Cities: food/shields/trade, growth, production queue, specialists, unrest | P1 | L | H | in progress | Fields + growth math proven on a peaceful std-game lab (**record framing fixed 2026-11-03**: owner u8@10, size u8@11, boxes @28/@30; settlers visible via unit home; the std1 series replays — 125 of 126 checked transitions exact (9 of 10 growths; Ur listed), growth restarts the box at the turn's surplus) (cap=(rows−1)×(size+1), >-threshold; engine::cities); open: production items, tile yields, specialists/unrest |
| T1-5 | ~~Research tree, governments, wonders (incl. obsolete-by-tech)~~ | P1 | M | E | done | @CIVILIZE tree (100 advances, classic prereqs, all reachable in the scripted run); 28 wonders w/ @ENDWONDER obsolescence; `totctl tech` |
| T1-6 | Combat: rounds, veterans, walls, terrain bonuses; identify original PRNG & seed | P1 | L | H | in progress | **Round formula decoded + shipped (`engine::combat`).** Combat rolls use the MSVC CRT `rand()` (0x5b11d0, state 0x5e2590), *not* `rand_int`: `rand()%A > rand()%D` ⇒ defender takes attacker FP, else attacker takes defender FP; 2 draws/round + 1 trailing. Attack strength + context, land defence strength (incl. ToTPP hooks) and the promotion roll decoded; 6 battles bit-exact (3 predicted from our seeds, incl. fortified D 54→81 and a promotion). **Wired into the kernel**: `move_unit` onto an enemy tile → `WorldState::attack` (decoded defender choice via the save's stack links, damage persistence flag @652, one-point attack cost, promotion) → `Battle::unitkilled_ctx` for T1-7; ROMFALL acceptance test replays battle 2 bit-exact. Remaining: sea/air/city-improvement special cases, fortress/walls/palace (need improvements in state), wonder obsolescence (needs per-civ techs). Unit/city record framing **fixed** in both walkers (units were 6 B late, cities 34 B late — evidence §Record framing FIXED; engine unit index = game index; evidence regenerated). Unblocks T1-7 UNITKILLED |
| T1-7 | Macro events interpreter (@IF/@THEN, ~50 commands, DELAY heap, JUSTONCE) | P1 | L | H | in progress | Parser + interpreter shipped (all 64 FallRome events parse; SCENARIOLOADED/UNITKILLED/CITYTAKEN/NEGOTIATION/RANDOMTURN triggers, TEXT/CREATEUNIT/CHANGEMONEY/GIVETECHNOLOGY/PLAYCDTRACK effects, ANYBODY + exact name matching per Macro.txt, JUSTONCE latch; CREATEUNIT = count at first valid location); `totctl events`; **effects applied** (`engine::effects`: treasury oracle-proven at civ-block i32@10, CHANGEMONEY clamp, CREATEUNIT first-valid placement — validity rule unverified, `move_unit_with_events` fires UNITKILLED from combat); fixed GIVETECHNOLOGY key/receiver, CHANGEMONEY receiver, CITYTAKEN `city=` matching, TRIGGERATTACKER/DEFENDER resolution; remaining: per-civ techs for GIVETECHNOLOGY, an oracle replay of one event, NEGOTIATION/RANDOMTURN driving from the turn loop. UNITKILLED context now comes from `combat::Battle::unitkilled_ctx` (Macro.txt semantics); `Defender Only` not modelled. Fixed vs Macro.txt: exact matching, `locations` attached to CREATEUNIT, one create per action (the two env-gated tests had asserted content not in the install's Events.txt) |
| T1-8 | ~~Save writer: byte-compatible `0x32` .SAV~~ | P1 | L | H | done | `SavEditor` surgical writer (round-trip byte-identical; typed edits); oracle loaded an edited save and preserved the rename; re-save diff field-explained (974 B, unit-bookkeeping stride) |
| T1-9 | ~~CI milestone job: 50-turn headless run, deterministic~~ | P1 | M | E | done | `engine::turn::WorldState::run_turn` (cities grow/starve on the replayed T1-4 model, TURN/TURNINTERVAL/RANDOMTURN events fire + apply, movement reset). CI (asset-free): `turn::tests::fifty_turns_are_deterministic`. Local (install): `totctl run` FallRome 50 turns with its Rules + Events vs `docs/evidence/fallrome-50turn.txt` (golden.sh run50), and `standard_game_install.rs` (Original/Ph_Auto.SAV, 50 turns). Scope note: the golden is the *engine's* fingerprint; parity with the original's saves is T1-10 |
| T1-10 | Oracle-parity turn replay: engine turn N→N+1 vs the original's autosave pair | P2 | L | H | todo | One harvested std-game turn pair field-explained by the engine (cities exact; units/AI out of scope until RG-4) |

## Epic RG — Regular game track (roadmap; enters after the T1 core)

Playing a *regular* game — new game from scratch, random or Earth world, AI
opponents, full campaign to a victory — needs more than the scenario engine:
nothing in T1–T3 produces a new-game state, generates a world, or fields
opponent AI. These stories reuse the T1 core; the new-game *UI* lands with T2.

| ID | Story | Pri | Est | Diff | Status | Acceptance |
|----|-------|-----|-----|------|--------|------------|
| RG-1 | ~~Game-setup state model: new-game parameters + its save encoding~~ | P1 | M | H | done | Byte-map on a vision-verified variant family: 684=difficulty, 689=rivals, dims@29900, RNG mirror@685-688; wizard reference documented (evidence §RG-1) |
| RG-2 | Starting placement: civ starts and goody huts from the game seed (barbarians → RG-9) | P1 | L | H | in progress | Same-seed oracle starts reproduce identical placement. **Done (2026-11-04):** oracle side (`seeded_start.sh`: NR, CRT, world params, map seed) and civ start placement (`engine::placement`, 3 oracle worlds replay draw for draw, evidence §RG-2 start placement). **Open:** goody huts as objects (the hut formula is decoded, not oracle-checked) |
| RG-3 | Random world generator (seeded; `.MP`-shaped output; original's exact algorithm unknown → our own, flagged) | P2 | L | H | todo | Statistical parity vs original maps (land%/biome spread); renders sanity-checked |
| RG-8 | Player interface: per-civ observation (fog-limited), order set, validation, order log; all-AI default (decision 0002) | P1 | M | H | todo | A game replays bit-exact from seed + order log; every RG-4 decision goes through it |
| RG-10 | AI v0, as an RG-8 player: explore, found cities, build settlers and defenders; deterministic | P1 | M | M | todo | 100-turn all-AI game on a standard save: every civ founds cities and grows; replays from seed + order log |
| RG-9 | Barbarians: when and where they appear (turn-time activity per barbarian level), decoded against the oracle | P2 | M | H | todo | Same-seed oracle game shows the same barbarian arrivals for N turns |
| RG-4 | Opponent AI v1 headless, as an RG-8 player: expansion, economy, research, unit handling; deterministic | P2 | L | H | todo | 50-turn autoplay stable + deterministic; plausible power curves vs oracle games |
| RG-5 | Diplomacy model: contacts, embassies, treaties, tribute, AI stances; save encoding | P2 | M | H | todo | Diplomacy state round-trips; oracle-save contacts match |
| RG-6 | Victory & scoring: spaceship chain, conquest, score/time; end-game state | P2 | M | H | todo | Victory conditions trigger identically on scripted oracle campaigns |
| RG-7 | Headless campaign loop: new game → autoplay N turns → outcome; CI golden run | P2 | M | H | todo | Nightly autoplay green; determinism (same seed+build ⇒ identical saves) |

ToT's four built-in campaigns (Original/Extended Original, Fantasy, Sci-Fi
quest modes) finish in T4 on top of RG + multi-map (see plan.md §5).

## Epic OB — Observer UI + pluggable AIs (v0.4.0; decision 0002)

| ID | Story | Pri | Est | Diff | Status | Acceptance |
|---|---|---|---|---|---|---|
| OB-1 | Observation/replay stream: per-turn snapshots + events, full-map and per-civ fog views | P1 | M | M | todo | A finished RG-7 game streams and replays identically |
| OB-2 | Browser observer: map, units, cities, turn stepping, play/pause, fog toggle, replay | P1 | L | M | todo | A 100-turn all-AI game can be watched live and replayed |
| OB-3 | Out-of-process AI protocol (JSON over stdio/socket) + a reference client | P1 | M | M | todo | An external AI plays a full game; protocol documented and versioned |
| OB-4 | Tournament runner: N AIs × M seeds headless, results table | P2 | M | M | todo | Deterministic results; same inputs ⇒ same table |

## Epic T2 — Graphical classic client (after OB; stories drafted on entry)

| ID | Story | Pri | Est | Diff | Status | Acceptance |
|---|---|---|---|---|---|---|
| T2-1 | Minimal playable SDL2 client: map from the install's art, units, move/attack/found city/end turn, simple city view, observer mode; human as an RG-8 player | P1 | L | M | todo | A human plays 20 turns of a standard game vs AIs on Windows 10/11 and Linux |

 SDL2 native client (Windows 10/11, Linux, macOS), city screen, orders/menus,
 original art & sound read from the user's install, human player as an RG-8
 player implementation, human-playable FallRome end-to-end. Reuses OB's map model.

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
