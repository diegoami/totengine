# ToTEngine — agent orientation

Open-source recreation of Civilization II: Test of Time gameplay (especially
community scenarios), in Rust. Reads the user's original ToT install; ships
none of its assets.

## Layout

- `crates/` — Rust workspace: `tot-kit` (format decoders), `engine`,
  `lua-host`, `totctl` (CLI, successor of the Python probe), `totui`
- `tools/totctl.py` — Python format probe (rules/map/save/diff/scnmap);
  stdlib only
- `tools/oracle/` — headless-Wine rig that drives the original game as a
  behavioral oracle; **read its README before touching or running it**
- `docs/` — `feasibility.md`, `plan.md` (tiers/principles), `BACKLOG.md`
  (story queue), `evidence/` (all proven format findings), `decisions/`,
  `refs/` (vendored ToTPP Lua API), `milestone-review.md` (release process),
  `environment.md` (machine setup/quirks), `lessons.md` (distilled
  process/rig lessons — read before driving the oracle)
- `docs/evidence/` + `tools/golden.sh` — golden corpus; `make
  evidence-regen` after *intentional* format changes, then commit

## Commands

```sh
make test            # cargo test + py_compile + golden corpus (self-skips w/o CIVTOT_HOME)
make fmt clippy      # CI runs these with -D warnings
python3 tools/totctl.py save $CIVTOT_HOME/FallRome/ROMFALL.scn -o /tmp/walk.json
```

`CIVTOT_HOME` points at the original install (reference box:
`/mnt/d/games/CIVTOT` — provisioning on a new machine: `docs/environment.md`).
In its root, **`civ2.bak` is the original unpatched executable** (no ToTPP;
use it for "what does the original do/import") and `civ2.exe` is the
ToTPP-patched one the oracle runs (details: `docs/environment.md`).

## Hard rules

- **Never commit original game assets** (sprites, maps, saves, sounds).
  Evidence artifacts are derived data only (JSON summaries, renders we made),
  small.
- **Every format claim needs evidence** — an artifact in `docs/evidence/` or
  a check that re-derives it. No folklore from forums without verification
  against real data.
- The oracle's absolute paths stay in tools behind `CIVTOT_HOME`, never in
  library code.
- Releases are tags gated by independent review: read
  `docs/milestone-review.md` before proposing a version. **The main session
  never launches the milestone review** — the owner does. At a milestone,
  print the review-prompt template from that doc and stop; agent-launched
  reviews proved unreliable (owner decision, 2026-11-02).
- Prior art was license-audited (`docs/prior-art.md`); GPL projects are
  reference-only (no code copying). MIT vendors keep their notices.
- The checkout dir may be named `civtotre` while the remote is `totengine`;
  pass explicit working directories.

## Status shorthand

v0.1.0 (Tier 0) and **v0.2.0 "headless game loop"** are released (v0.2.0:
2 review rounds, AGREE on #61) and **v0.3.0 "reproducible new game"** are
released (v0.3.0: AGREE on #70, 1 round; RG-2 done: `seeded_start.sh`,
`engine::placement`, goody huts; map tile framing and record counts
fixed) and **v0.4.0 "minimal production"** (AGREE on #72, 1 round: tile
yields, corruption, waste, unit support in `engine::yields`; production
queue in `engine::production`; decoded from `civ2.bak`). **v0.5.0 "player
interface"** is released too (AGREE on #98, 1 round; RG-8, decision 0003:
`engine::player` with Observation, Order, Game + order log + replay,
RandomPlayer; `totctl play/replay`). **v0.6.0 "AI v0"** is released too
(AGREE on #101, 2 rounds; #102 found create_city's `0x20000` is a
transient refresh bit): `engine::ai::AiV0`, `engine::founding` (FoundCity, create_city
partly decoded), obsolete units + veterans, goody-hut outcomes in part
(`engine::huts`; the rest is #79, moved to v0.8.0).
**v0.7.0 "browser observer"** is released too (AGREE on #104, 1 round):
`engine::observer` per-turn snapshots, `totctl observe`,
local browser UI in `totui`; #80/#81, usage in `docs/browser-observer.md`.
**v0.8.0 "AI v1 + economy depth"** is released (AGREE on #108,
GLM-5.3, 1 round; tag dc5427b): #87 science/tax split is
implemented in `engine::finance` (146/146 FallRome + 10/10 standard-city
records exact from stored trade/shields), city finance refreshes in
`run_turn`; civ rates and specialists are decoded and fingerprinted.
Per-civ research is implemented (`engine::research`, `SetResearch`,
`--ai-v1` first research-capable AI increment): 1,014 costs match original
x86 execution; a 50-turn standard-save run gains 14 advances and replays.
Capital road/rail connections now scale waste (`engine::connections`,
#84; 123 saved pairs and all 156 fixed-worker native shield totals match;
Aventicum's saved cache is stale).
Recurring route arrows are implemented (`engine::trade_routes`, #85 slice;
288 arithmetic cases match original x86). Partner trade caches refresh
before finance. Creation/delivery rewards and supply/demand refresh remain
open beyond #85's recurring-income scope. AI v1 now makes own-observation
economic build choices, bounds settler support, keeps stable garrisons and
handles visible threats; two 50-turn runs and replay agree. Archived original
power curves are compared in `docs/evidence/ai-v1-curves.json`. Treasury accounting now credits city
taxes and charges original human-mask upkeep, including exemptions and
bankruptcy sales (1,000 synthetic and 490 saved-input x86 cost checks).
Happiness/disorder stages now refresh after production (#86; 1,000
synthetic x86 cases and 156/156 original recomputations from saved inputs;
15 FallRome stored values differ under original recomputation too).
Wonder lifecycle and Darwin choices are implemented. Leonardo, Eiffel Tower,
Manhattan and surface Apollo one-offs now have native comparisons; ongoing effects now include native-checked Library, expiry, naval movement and
one-shot veteran handling; diplomacy/government/transport contexts are explicit limits. #89's late AI shield-row adjustment is implemented
and default AI row rounding corrected (1,108 original-x86 cases). Original
power score/rank refresh now runs at round close (256 synthetic + 2 saved-input
x86 comparisons); population and city-landmass caches refresh too (128 cases).
64 sustained live arrival phases now match 200 allocated units and all CRT
states. The live check corrected sea admission to use landmass word +4.
The decoded surface city phase now runs in `run_turn` (signed growth/debt,
workers, support/disbands, post-production mood/finance and forced human anarchy).
Actual round processing matches 500 controlled native city transitions over
50 rounds. A controlled live Wine sequence now also passes: 500 economic
transitions and civ treasuries from 50 original-written saves match run_turn. City-slot reuse, classic capacity and deletion extent now match native checks.
Surface hut-facing landmass flags now refresh (512 native contexts); the live
city fixture isolates economics rather than full original-AI round parity.
The owner-run v0.8 review passed all 181 tests and reproduced 85/85 native
artifacts; #109's non-blocking documentation follow-up is fixed after the tag.
`docs/v0.8.0-readiness.md` records acceptance gates and explicit context limits.
Plan: `docs/decisions/0004-economy-and-ai-v1.md`.
**v0.9.0 "generated-world campaigns"** is released (AGREE on #110,
GLM-5.3, one round; tag 17c1d25). `engine::worldgen` and `totctl worldgen`
provide deterministic engine-designed MP export. First measured baseline and
render are in `docs/evidence/worldgen-v1.json`. Algorithm v2 corrects polar
bands/relief and drains inland basins to the sea; 16 calibration and 64 fresh
engine seeds are measured in `worldgen-v2.json`. The expanded ten-original/
64-engine-holdout comparison passes declared land/biome/rivers coverage budgets
in `worldgen-v2-expanded.json`; coasts/biome patches remain smoother and spatial
parity is not claimed. `engine::newgame` now constructs fresh surface
games without a save; `engine::campaign` and `totctl campaign` provide AI v1
autoplay, order replay and versioned replay checkpoints (not native SAV).
The 100-round authored golden and 32-seed continuation/replay suite pass;
Original Rules seed 7 reaches 16 cities/53 units/35 advances and resumes.
The independent reviewer reproduced all retained comparisons/trajectories
byte-identically, passed 194 install-backed Rust tests, five Python evidence
tests, golden/fmt/clippy and the 32-seed nightly suite. No MUST-FIX findings;
#111's stale CLI status is corrected after the tag. `docs/v0.9.0-readiness.md`
records its gates and material geography limits.
**v0.10.0 "built-in opponent: research and land campaigns"** is released
(AGREE on #116, GLM-5.3, one round, no findings; tag 8f6b57f).
`engine::ai_research` implements original research
selection (512 isolated contexts) and full advance valuation (12,800 values
and 128 complete selections, unmodified native callees). Numeric regression
fixtures run without assets. Parsed Rules/world adapters and the native selector
gate now match 1,400 saved-input scores and 84 choices (Original + FallRome,
all seven civs/six difficulties). Leader field loading and 142 reference tokens
are separately native-checked. Context refreshes ownership/naval inputs but
does not claim a full native statistics pass or custom saved-leader overrides.
`BuiltInAi` now uses an observation-only research context and explicit
`builtin-1` identity (`play/observe --builtin`, `campaign --policy builtin-1`);
format-2 checkpoints restore private policy state. CLI campaign/observer defaults use the new policy; explicit legacy options
and the AI v1 golden remain available. Production compares 918 native prepared choices across six
difficulties: standard archived holdout 84/108 exact vs AI v1 12/108, but
FallRome remains 42/750. Known-land settlement/military routing and explicit
`OccupyCity` now support war/ownership transfer. Loot/building/population
kernels match 843 native vectors; capital/diplomacy/technology/cargo capture
handling remains partial. Six authored campaign/strategy tests plus event/replay
checks cover the integration. The reviewed v0.10 candidate passed 209 install-backed Rust tests, full golden/fmt/clippy,
32 seeds per policy in the nightly suite, 64 retained campaign comparisons
with exact replay (36 also continue through checkpoints), and the real-browser
100-round test. Release readiness/limits: `docs/v0.10.0-readiness.md`.
The release tag points exactly to the independently reviewed SHA.
v0.10 is original-derived, not the complete original AI. The owner's subsequent
full-port scope is `docs/original-ai-scope.md`, program #117: A0–A7 standard-game
completion, A8 full vanilla maps/scenarios. **The complete AI program is gated
behind the playable UI/client work** (owner decision, 2026-11-06). First AI task
after that gate: #118 (native decision inventory, explicit fidelity boundary
and revised work estimates). The proposed
trusted compatibility driver is not an approved amendment of decision 0003.
**v0.11.0 "UI discovery, reviewed design and client planning"** is released
(AGREE on #132, GLM-5.3, one round; tag ad605ec). The tag points exactly to
`ad605ec1eadd45939c6b3ee80611a559368393be`. Independent review reproduced all
209 install-backed Rust tests, Python/golden/fmt/clippy, checkpoint and UI/art
source evidence, plus the shipped browser check. No MUST-FIX; #133's stale
approval wording is corrected after the tag. This release is planning, not a
playable client. Next work is milestone 10 (both clients, version unassigned).
Second-pass reference inventory (33 route groups) and remaining coverage:
`docs/ui/discovery.md`;
derived capture/source hashes: `docs/evidence/ui-discovery.json` (original
screenshots stay outside Git). `docs/ui/client-prototype.html` is the local,
simulated nine-screen design for owner feedback, with nine report families,
both movement variants and nested diplomacy, plus own-art previews;
`docs/ui/client-design.md` records proposed interactions, API gaps and unversioned
delivery categories. `docs/ui/screen-contracts.md` specifies proposed decision/view/
command/persistence boundaries; `docs/ui/client-milestones.md` proposes the first
human-controlled standard-game turn and dependent delivery packages, without
assigning versions. Owner accepted revision 02 as a prototype on 2026-11-06
("ok for a prototype is ok"); use it as the planning basis without asking for
prototype acceptance again. The owner then explicitly answered five questions:
“1 yes 2 both 3 yes 4 option 1 5 like the original”. Local browser hosting is
accepted; browser and standalone desktop land together in the first playable
milestone; generated-standard-game scope is accepted, before import/deeper
management; immediate empty-tile moves confirm attacks/capture; city worker/
specialist edits commit immediately like the original, without a global draft.
Production pickers and buy/sell keep explicit decision confirmations. Both clients
remain required. Toolkit adoption follows feasibility; SDL2 is the first
candidate, not an already validated/selected toolkit. Do not
reinterpret this as browser-only, desktop-only or a desktop requirement replacing
the browser target. Live delivery stories: desktop #94, browser #125, shared
design/decomposition #107. The owner then explicitly directed an
installed-art visual pass: `CIVTOT_HOME=/path/to/CIVTOT python3 tools/preview_client.py`
serves the prototype with local profile BMP/GIF sheets (no original art committed).
Usage/recipe limits: `docs/ui/asset-preview.md`; numeric source/crop evidence:
`docs/evidence/client-artwork.json`. The browser renderer covers map/city art,
citizens/icons and interface textures; all game state/actions remain simulated.
Six install-free routing/selection tests and the optional real-browser artwork
check pass. This is not a production renderer or a human game-session driver.
`docs/v0.11.0-readiness.md` records acceptance, checks and explicit limits.
`docs/ui/client-architecture.md` audits the engine/observer/assets and defines
one Rust human-session layer with local-hosted browser and native desktop
adapters. Accepted decomposition is live: milestone 10 “Playable human turn —
browser and desktop” (version unassigned), #126 desktop feasibility, #127 human
driver, #128 resources/filtered views, #129 browser, #130 desktop, #131 checkpoints
and both-client acceptance. Complete-client umbrellas #94/#125 remain open;
full city/government/diplomacy/import/scenarios and sustained campaigns follow.
The owner scope gate under decision 0005 is satisfied for this first slice;
v0.11 is released and its review gate is complete. Start #126 desktop feasibility,
#127 shared human driver and #128 resources/filtered views; checkpoint design
starts with the driver. Browser #129 and desktop #130 must both pass #131
acceptance before the first playable milestone closes. Reference coverage includes representative city,
map/unit, government, negotiation, report, Fantasy transfer and classic custom
scenario routes are now inspected; remaining branches and delivery impacts are
explicit in the inventory. Lua-disabled and invalid-file rejection are covered.
The isolated installed-reference revolution Escape test starts a revolution;
the prototype deliberately proposes safe cancellation. No vanilla equivalence
or comprehensive native-UI parity is claimed. All owned Wine/Xvfb sessions stopped.
Milestone 10 implementation has started: `engine::human::HumanSession` owns
resumable human input and bounded builtin-1 scheduling, explicit research/named
founding, identity/revision/retry validation, filtered views/notices and replay-based
human checkpoints. See `docs/ui/human-session.md`; these are engine foundations,
not actual human acceptance in either client. `totui::resources` resolves profile
Rules/art and decodes BMP/GIF with shared measured masks; all five retained
Original crop opacity counts match. Decoder notices: `docs/third-party/README.md`.
Full atlas/adjacency/colors/font, desktop feasibility, both adapters and assembled
checkpoint acceptance remain. Next entry points: #126, #128, #129/#130 and #131.
Read `docs/decisions/0005-playable-client-planning.md` for settled owner choices; read
`tools/oracle/README.md` and `docs/lessons.md` before driving the reference UI.
v0.10 implementation record: `docs/decisions/0007-original-like-ai.md` (#106).
Roadmap: small releases, one theme each (GitHub milestones; BACKLOG.md
holds conventions only;
decision 0002; observing and playing are the two main goals): v0.3.0
reproducible new game → v0.4.0 minimal production → v0.5.0 player
interface (headless API) → v0.6.0 AI v0 (all-AI game runs) → v0.7.0
browser observer → v0.8.0 AI v1 + economy depth → v0.9.0 generated-world
campaign → v0.10.0 original-like built-in AI → v0.11.0 UI discovery/design
and client milestone planning → playable client across subsequent milestones
→ pluggable AIs after the playable client (version unassigned). The v0.10
target is recognizable original-game AI behavior, supported by oracle
comparisons; AI v1 in v0.8 is only the foundation. Client screen/workflow
specification and owner-reviewed design are still missing: decision 0005
requires them before substantial UI implementation; later versions and
milestone count are unassigned. Story state:
T1-1/2/3/5/8/9 and RG-1 are done; `engine::turn::run_turn` is the loop. T1-6's randomness half is **done**
(the original LCG and its `rand_int` are decoded, runtime-verified and
shipped in `engine::rng`). Its combat-round half is decoded and shipped
in `engine::combat`: rolls come from a *second* generator (the MSVC CRT
`rand()`); attack/defence strength (incl. ToTPP hooks) and the promotion
roll are decoded too, six oracle battles are bit-exact, and combat is
wired into `move_unit` with a UNITKILLED context for T1-7. Open: sea/air
and improvement-dependent special cases.
Units are driven in the oracle through RAM (`tools/oracle/unitram.py`),
not by map navigation. T1-4 (cities) and
T1-7 (events) are partially landed (T1-7 effects now change the world:
treasury, CREATEUNIT). Unit and city records are read in the game's own
framing since 2026-11-03 (evidence §Record framing FIXED). GitHub issues/milestones are the live queue,
`docs/evidence/README.md` carries every proven finding (and, for T1-6,
an explicit list of what has been ruled out), and `docs/lessons.md` the
rig/process traps.
