# ToTEngine — agent orientation

Open-source native recreation of Civilization II: Test of Time in Rust, for
both desktop and playable browser clients using the user's original resources.
ToT scenario compatibility is the niche. Main targets: complete faithful UI
with optional QoL, headless all-AI scenario experiments, scenario observation,
different/pluggable AIs, scenario inspection/testing/creation assistance, and
recording of play/observation sessions. Ships no original assets. Current scope
and sequencing: `docs/mission.md`, `docs/plan.md`, `BACKLOG.md` and decision 0008.
Freeciv/C-evo AI/client/workflow reuse is an assessment candidate, not adopted
rules or a license change. Full-AI/plugin implementation stays behind complete
playable clients; source study and existing-policy scenario tools may proceed.
Exact original-AI #117 remains separate, not a prerequisite for every AI option.

## Layout

- `crates/` — Rust workspace: `tot-kit` (format decoders), `engine`,
  `lua-host`, `totctl` (CLI, successor of the Python probe), `totui`
- `tools/totctl.py` — Python format probe (rules/map/save/diff/scnmap);
  stdlib only
- `tools/oracle/` — headless-Wine rig that drives the original game as a
  behavioral oracle; **read its README before touching or running it**
- `BACKLOG.md` — root live-queue map and working conventions
- `docs/` — `mission.md`, `plan.md` (workstreams/dependencies), historical
  `feasibility.md`, `evidence/` (all proven format findings), `decisions/`,
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

Milestone 14 started, 2026-11-08, at the owner's instruction to begin the first
roadmap item after milestone alignment. #181 merged to main at
`d165aebb4d40c178168bfc2d733407a84af860b4`. #182 is testing the combined code
at `7ad1e9302566836ca643dd62c12a102a938b0f53`; CI/autoplay, 34 desktop/shared
tests, real-pack integration and pack-preparer tests pass. Actual packaged
Chromium/Linux/Windows routes are in progress. Planning PR #191 is retargeted
to main. No new release version, human acceptance or milestone review is claimed.
Preserve existing owner previews/saves. Current private packages:
`review-tmp/tot-client-foundation-{linux,windows}-7ad1e93`.

Owner mission/roadmap reorganization, 2026-11-08: `docs/mission.md` and decision
0008 define the seven product targets; `docs/plan.md` replaces the stale T0–T4
schedule with workstreams/dependency gates, and root `BACKLOG.md` maps the live
queue. New programs: #183 scenario compatibility/workbench, #184 headless
scenario experiments, #185 scenario observer, #186 session recording, #187
AI adoption/evaluation, #188 optional QoL, #189 city queues, #190 rally points.
Existing engine/client/AI umbrellas are reconciled, not closed by planning.
Planning branch `docs/scenario-platform-roadmap` includes the earlier source
study and remains stacked on the Isotrident feature commit; keep #181 and #182
separate when integrating it. Changes since that feature are documentation only.
Original resources remain the main product requirement; approved Isotrident
fallback stays optional. The follow-up backlog-to-milestone request created named packages 14–29 and
bounded stories #192–#209, recorded in `docs/milestones.md`; version numbers,
due dates and candidate SHAs remain unassigned. Historical milestones are
unchanged. Full-AI/plugin gates still depend on complete clients, including
additional original-feature slices beyond this planning list. No implementation,
public hosting or licensing change was launched by this reorganization.

Resumed 2026-11-08: [PR #182](https://github.com/diegoami/totengine/pull/182)
implements the post-v0.15 follow-ups #176–#179 on the separate
`fix/client-input-recovery` branch, head
`194b955335560593d58d0a988fc4c272b7401140`, based directly on main.
Worktree: `/home/diego/review-tmp/totengine-client-input-recovery`.
It preserves the first queued gesture attempt, clears obsolete pending edits,
resets city panels on map/setup loads while retaining open-workspace state,
and documents/tests second tabs completing already advancing opponents.
Local validation passed: 259 install-backed Rust tests including desktop,
11 Python tests, full goldens/fmt/strict default+desktop Clippy, actual Chromium
and Linux SDL routes with 26 newly written checkpoints identical across both.
Windows native input and human acceptance were not rerun. Evidence:
`docs/evidence/client-input-recovery-v1.json` on the fix branch. PR #182 is open;
GitHub CI and generated campaign autoplay both passed on its exact head.
#176–#179 stay open until it lands. Owned QA host 8815 and isolated native app/display are stopped; owner
previews/saves are untouched. PR #181 and the source-assessment branch below
remain separate; no release, milestone review or AI port was launched.

Session close, 2026-11-08: [PR #181](https://github.com/diegoami/totengine/pull/181)
(Isotrident fallback, head `de0bca17a4a1fa1df963af79203d130a67ebed9a`)
is **open; all four GitHub checks passed** (Rust, Python, autoplay, GitGuardian).
It is not merged/released or independently milestone-reviewed. Current branch
`docs/c-evo-source-assessment` is pushed and **stacked on that feature commit**;
its later commits are documentation/source-study only. Keep the Isotrident PR
separate when integrating these notes. Next session: read
`docs/ai-client-reuse-assessment.md` and `docs/ui/freeciv-assessment.md`, then
inspect current review/PR status before choosing the next scoped package.
The Open demo was an assistant-authored validation baseline; the owner approved
Isotrident artwork, not a complete alternative gameplay ruleset. Public hosting,
full redistributable rules, AI adoption benchmarks and whole-client adoption
are unfinished. #176–#179 remain separate follow-ups. No new milestone/version
was assigned, and no AI/client port or public deployment was started.

2026-11-08 owner direction: ToT **scenario compatibility is the niche**; evaluate
stronger Freeciv/C-evo AI, whole clients and UI workflows as real adoption
candidates. Do not assume exact original-AI reproduction is the sole acceptable
AI route. Source/adapter assessment: `docs/ai-client-reuse-assessment.md`; pinned
C-evo-x checkout is `/home/diego/review-tmp/c-evo-x-reference`. Its code has
public-domain declarations, but bundled help excludes graphics/sounds. No code
or media was adopted. Full-AI/plugin implementation gates remain; source study
does not launch a port or cancel #117. Owner authorized community scenario
downloads **installed in the real CIVTOT Scenario directory**, in individual
folders, not merely left in temporary reference storage. Roman Civil Wars plus
audio is installed under `Scenario/Roman Civil Wars`; static parse and hashes
passed, no native play acceptance claimed. Preserve existing games/root assets.

Owner approved **Isotrident as fallback artwork** on 2026-11-08 for a future web
application without a ToT installation. Assessment/usage/licensing and remaining
hosting/rules scope: `docs/ui/freeciv-assessment.md`. Freeciv 3.2.7 source and GTK
binary are installed under `~/projects/freeciv`; its Civ2 rules are approximate,
not an oracle for original ToT behavior. This approval permits a separately
licensed GPL-2.0-or-later artwork pack, with PNG/spec source and credits; no
Freeciv game code/rules are copied into our MIT engine. The unreleased candidate
adds one shared fallback adapter for both clients, an external pack preparer and
optional package inclusion. Blank setup runs a clearly labeled, independently
authored **Open demo**, not full original standard rules. Complete ToT resources
still take precedence. Public hosting and a complete redistributable standard
rules profile remain separate work; no new version/tag/milestone is assigned.

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
behind the playable UI/client work** (owner decision, 2026-11-06). First task within original-fidelity program #117
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
browser and desktop” (v0.12.0 released), #126 desktop feasibility, #127 human
driver, #128 resources/filtered views, #129 browser, #130 desktop, #131 checkpoints
and both-client acceptance. Complete-client umbrellas #94/#125 remain open;
full city/government/diplomacy/import/scenarios and sustained campaigns follow.
The owner scope gate under decision 0005 is satisfied for this first slice;
v0.11 is released and its review gate is complete. #126 desktop feasibility and
#127 shared human driver are completed; #128–#131 finish the first slice. Browser #129 and desktop #130 must both pass #131
acceptance before the first playable milestone closes. Reference coverage includes representative city,
map/unit, government, negotiation, report, Fantasy transfer and classic custom
scenario routes are now inspected; remaining branches and delivery impacts are
explicit in the inventory. Lua-disabled and invalid-file rejection are covered.
The isolated installed-reference revolution Escape test starts a revolution;
the prototype deliberately proposes safe cancellation. No vanilla equivalence
or comprehensive native-UI parity is claimed. All owned Wine/Xvfb sessions stopped.
Milestone 10 implementation now supplies both actual clients: `totplay` (loopback
browser host) and `totdesktop` (standalone SDL2 window, optional `desktop` feature).
Shared `totui::client` owns atlas/presentation/input and named checkpoint slots;
`engine::human` owns validated input, bounded builtin-1 scheduling, required
research/founding, notices and last-seen fog memory. Human checkpoints format 2
restore that memory and exact engine/resource/policy/session state through replay.
Windows and Linux packages contain notices/runtime DLLs, never game assets/fonts.
Usage: `docs/playable-clients.md`; validation/current gates:
`docs/v0.12.0-readiness.md`. **v0.12.0 is released** at exactly the reviewed SHA
`4ccf2cd62f4f6022cd045cf0a467dea5f7e46fbd` (AGREE on #134, GLM-5.3,
one round, no findings). The independent reviewer reproduced 233 Rust tests,
11 Python tests, golden/fmt/clippy, the 32-seed-per-policy campaign suite and
real browser/Linux desktop input with all checkpoint hashes matching. The owner
confirmed ten human rounds in both clients with no blockers. Windows/Linux
packages and checksums are attached to the GitHub release; no game assets/fonts.
#126–#131, review #134 and milestone 10 are closed. Full-client #94/#125 and
full-AI gate #117 remain open beyond this narrow slice. Next work is a separately
scoped playable-client package from `docs/ui/client-milestones.md`; deeper city
management and input polish are natural next candidates. No next release version
or fixed remaining milestone count is implied. Full original-AI work remains
gated behind complete playable-client delivery.
**v0.13.0 “city workers and specialists” is released**, exactly at reviewed SHA
`3be920940e87d7b647f9153f21f83b30eca1e495` (Kimi K3.1 Pro, AGREE on #138,
round two). Browser and standalone desktop both ship; owner confirmed both new
city routes pass on 2026-11-07. Milestone 11 and #135–#138 are closed. Windows/
Linux packages and checksums are attached to the GitHub release, with no game
assets/fonts. Native packed citizen edits retain 1,548 original-x86 comparisons.
Shared city workspace supplies immediate worker/role edits, outputs/mood,
explicit current production, navigation and checkpoints; it is not the full
original city UI. Selection follows the reference study (own-unit hits in either
mode, stable stack identities, ready selected unit sprite blinking and hit
handling while hidden, exhaustion/new-turn transitions). Enter visibly defaults
to End turn when exhausted; Viewing keypad moves its visible cursor and scrolls
only to keep it visible. Original control conventions and explicit context
limits remain in docs/ui/original-controls.md and the selection study.
Round-one findings #139–#144 were fixed through #145: excess entertainers enter
finance/mood; SDL batches preserve same-city keys without retargeting; browser
busy blur, visible inspection movement and stale docs were corrected. The
reviewer reproduced 241 Rust tests, eleven Python, goldens/fmt/default+desktop
Clippy, 23 desktop-feature tests, seven 32-seed campaign tests, all 1,548 native
vectors and real browser/Linux input with 27 identical checkpoints. Main also
verified final Windows input and package hashes. Evidence/readiness:
docs/evidence/client-review-r2.json, client-selection-v1.json and
docs/v0.13.0-readiness.md. #146 is a non-blocking asynchronous focus→blur ordering
follow-up discovered in round two; retain the released SHA and fix afterwards.
The owner explicitly directed “after that merge and continue as long as you can”.
Continue with #146, then the next scoped city/production client package from
reviewed screen contracts and docs/ui/client-milestones.md. Both clients remain
required. Buying/selling, shield-switch penalties, deeper panels and remaining
full-client packages are not shipped by v0.13. Complete original-AI and pluggable
AI work remain gated behind full playable-client delivery. New milestone review
is owner-launched only; no automatic review or tag.
**v0.14.0 “city production decisions” is released**, exactly at reviewed SHA
`5e3be851782a5b4b9bc253d127f1b27212dad50a` (Kimi K3.1 Pro AGREE on #155,
one round, no findings). Milestone 12 and #148–#151/#155 are closed. #146's focus
ordering follow-up shipped through #147. #152 implements immutable own-city
quotes, validated buy/sale and authoritative captured switch budgets with replay;
#153/#154 record validation and new owner acceptance. Both clients show production
progress, shield consequences and confirmed transactions. Native corpus: 4,091
comparisons. Owner accepted both-client purchase/sale/switch/reload routes.
The reviewer reproduced 248 Rust, 11 Python, golden/fmt/Clippy, 25 desktop tests,
seven 32-seed campaign tests, all native vectors/fixture authors and actual
Chromium/Linux routes. Retained Windows input also passed before review.
Final reviewed-SHA Windows/Linux packages and checksums are on the release.
Final Chromium/Linux package input passes; Windows SDL opens but two release-time
input reruns were blocked by the owned-window initial focus guard. This is
explicit in release notes and docs/evidence/client-v014-release.json; do not
claim those final Windows input reruns passed. Preserve all owner previews/games:
current accepted browser is port 8797; Windows launcher is under
review-tmp/totengine-owner-v014. Readiness: docs/v0.14.0-readiness.md.
**v0.15.0 “city information panels” is released**, exactly at independently
reviewed SHA `948b0d2b94d22315190c664340d080062629c8ea` (AGREE #162,
Kimi K3.1 Pro, round three). Milestone 13 and #157–#159 are complete. Both
clients have Info/Support/Improvements/Happy with immutable own-city filtered
support/upkeep/wonder views and five native-checked mood snapshots. Original
study retains eleven private capture identities; 1,280 native stage snapshots
reproduce byte-identically, with existing garrison-input limits explicit.
City double click binds first ID/entity/session/pointer/focus; shared, SDL-batch
and browser timing regressions protect selection and stale inputs. City lists
page through explicit sale confirmation; Info alone owns tile keys.
Independent review passed 255 Rust, eleven Python, full goldens/fmt/strict
Clippy, 30 desktop tests, seven campaign tests including 32 seeds per policy,
six fixture authors and actual Chromium/Linux input. Exact-reviewed-SHA final
Linux/Windows archives and checksums are on the GitHub release. Final packaged
Chromium/Linux/Windows input passes 54/47/47 full checkpoint comparisons.
The final Windows run acquired foreground focus; earlier guard-blocked runs
remain historical and were not relabeled. Release record:
docs/evidence/client-v015-release.json; readiness: docs/v0.15.0-readiness.md.
First/second review findings #163–#169/#171–#174 are fixed. Third review
opened non-blocking #176–#179: queued invalid/third browser click, stranded edit
recovery, new-session Info default, and reconnect/second-tab contract/testing.
They remain open for subsequent work; no next version is assigned here.
Owner approved the new panel route in both clients on 2026-11-07. No manual
acceptance of later entry/review fixes is claimed. Preserve owner previews/games:
browser ports 8797/8799 and launchers under review-tmp/totengine-owner-v014 and
review-tmp/totengine-owner-v015. Those previews are older builds; do not overwrite
live saves. Full-client #94/#125 and original-AI gate #117 remain open. Complete
original AI/pluggable AI stay behind complete playable-client delivery.
Read `docs/decisions/0005-playable-client-planning.md` for settled owner choices; read
`tools/oracle/README.md` and `docs/lessons.md` before driving the reference UI.
v0.10 implementation record: `docs/decisions/0007-original-like-ai.md` (#106).
Historical release ladder follows; current future scope/order is decision 0008 and `docs/plan.md`.
Roadmap: small releases, one theme each (GitHub milestones; BACKLOG.md
holds conventions only;
decision 0002; observing and playing are the two main goals): v0.3.0
reproducible new game → v0.4.0 minimal production → v0.5.0 player
interface (headless API) → v0.6.0 AI v0 (all-AI game runs) → v0.7.0
browser observer → v0.8.0 AI v1 + economy depth → v0.9.0 generated-world
campaign → v0.10.0 original-like built-in AI → v0.11.0 UI discovery/design
and client milestone planning → v0.12.0 first playable turn on both clients
→ full playable client across subsequent milestones
→ pluggable AIs after the playable client (version unassigned). The v0.10
target is recognizable original-game AI behavior, supported by oracle
comparisons; AI v1 in v0.8 is only the foundation. Decision 0005 required original UI discovery and owner-reviewed design;
that first design/first-slice gate is complete. Remaining native UI branches
still require reference/contracts; later versions and milestone count are
unassigned. Decision 0008 organizes scenario, recording, QoL and AI-choice work. Story state:
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
