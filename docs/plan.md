# ToTEngine delivery plan

Reorganized from the owner's seven product targets, 2026-11-08.
[Mission](mission.md) → this plan → [live backlog](../BACKLOG.md) → [planned milestones](milestones.md).
[Decision 0008](decisions/0008-scenario-platform.md) supersedes the old sequential
T0–T4 schedule; historical evidence and released tags remain unchanged.

## Principles

1. **Scenario compatibility defines the product.** Preserve ToT rules, event
   timing, resource overrides, objectives and multiple-map semantics. Standard
   games remain a product target and an essential controlled test environment.
2. **One authoritative native engine.** Desktop play, browser play, headless
   experiments, AI policies, observers and author tools use the same validated
   simulation and versioned persistence. The original executable is an oracle,
   not a shipped runtime requirement.
3. **Faithful baseline, explicit enhancements.** Original controls and complete
   UI feature coverage come from the manual/reference inventory and reviewed
   design. Queues, rally points and alternative settings have their own command,
   persistence and interaction contracts.
4. **Evaluate reuse.** C-evo/Freeciv can inform or supply eligible algorithms,
   clients and workflows. They do not define ToT mechanics, solve scenario import
   automatically or establish stronger AI without comparison.
5. **Measure compatibility.** Format/mechanics claims need retained evidence.
   Parsing, simulation, observation, human play and original-format export have
   separate acceptance gates. Unknown scenario features are visible.
6. **Keep original resources private.** Use configurable resource loading, never
   original paths in engine libraries. Commit only small derived evidence.
   Preserve existing owner games and installations.

## Baseline and remaining work

Latest release: **v0.15.0**, independently reviewed at
`948b0d2b94d22315190c664340d080062629c8ea`. See
[readiness](v0.15.0-readiness.md) and [release evidence](evidence/client-v015-release.json).
This plan does not designate a new release.

| Capability | Available foundation | Remaining target |
| --- | --- | --- |
| Native/browser clients | Shared human sessions, generated surface games, movement/founding/research, checkpoints, workers/specialists, production transactions and city panels | Complete UI coverage: science/government, diplomacy, reports/help/audio, advanced orders, original imports, scenarios/multiple maps and sustained campaigns on both clients |
| Headless | Save/scenario boot, modeled macro events, original-derived mechanics, built-in policies, generated campaigns and deterministic order replay | Admitted scenario campaigns, parameter sweeps, objective-aware reports and author regressions |
| Observer | Live/replay browser observer, full/per-civ views, turn controls and archives | Scenario presentation/objectives/events, supported map layers, comparison and recording integration |
| AI | Versioned built-in/legacy policies and partial original-derived decisions | Per-faction policy selection, bounded candidate comparisons, external plugins and independently measured native fidelity |
| Scenario tools | Format inspection/diffs and an installed private corpus | Capability reports, client import, event/extension coverage, run diagnostics, safe editing/export and creation assistance |
| Recordings | Canonical order logs, observer NDJSON and replay checkpoints | Unified session manifests, view/annotation streams, recording controls/recovery, then audio/video capture |

Unmerged work is separate: [#181](https://github.com/diegoami/totengine/pull/181)
adds Isotrident/Open demo; [#182](https://github.com/diegoami/totengine/pull/182)
fixes client follow-ups #176–#179. Both passed CI at this planning snapshot.
Neither is a release or full-client/scenario completion. Roman Civil Wars is
installed and statically parsed; no playable/native acceptance is claimed.

## Workstreams and live programs

| Stream | Deliverables and acceptance | Tracking |
| --- | --- | --- |
| Playable clients and original UI | Maintain a screen/feature coverage matrix. Complete reviewed workflows in both clients; demonstrate keyboard/mouse, cancel/error, save/resume and human routes. Include reference help, sound, scenario dialogs and advanced unit/city behavior before claiming the full original interface. | [#21](https://github.com/diegoami/totengine/issues/21), desktop [#94](https://github.com/diegoami/totengine/issues/94), browser [#125](https://github.com/diegoami/totengine/issues/125); [client packages](ui/client-milestones.md) |
| Scenario compatibility/workbench | Admit bounded SAV/SCN profiles with correct resource/rules/event identity. Extend macro, Lua and multiple-map support from a capability matrix. Add lint/diff/test-run assistance, then editing/export. Open/play/observe/run the same admitted scenario; reject unsupported input without losing the current game. | [#183](https://github.com/diegoami/totengine/issues/183), macro [#16](https://github.com/diegoami/totengine/issues/16)/[#20](https://github.com/diegoami/totengine/issues/20), ToTPP [#22](https://github.com/diegoami/totengine/issues/22) |
| Headless scenario experiments | Version run manifests for starts, policy/config/seeds, difficulty and declared overrides. Emit objective/event outcomes, stalls, limits, traces and comparable summaries. Reproduce and resume admitted runs. Existing policies suffice for the first package. | [#184](https://github.com/diegoami/totengine/issues/184); standard campaigns [#23](https://github.com/diegoami/totengine/issues/23) |
| Scenario observation | Add custom labels, event/objective inspection, map identity, live/replay comparison and recording navigation. Define viewing versus simulation pauses. Recorded seeking never executes another turn. | [#185](https://github.com/diegoami/totengine/issues/185) |
| Session recording | Unify game/run identities, orders/events/checkpoints and stop reasons; keep camera/perspective/annotations separate. Record/open/recover play and observation sessions, then scope video/audio/clips. | [#186](https://github.com/diegoami/totengine/issues/186) |
| Optional UI/QoL | Study Freeciv/C-evo workflows against ToT's baseline. Scope queues and rally points, disclose automation, preserve legal commands and checkpoints, and keep classic behavior available. Whole-client adoption remains a measured candidate. | [#188](https://github.com/diegoami/totengine/issues/188), queues [#189](https://github.com/diegoami/totengine/issues/189), rally points [#190](https://github.com/diegoami/totengine/issues/190) |
| AI choice and evaluation | Compare eligible C-evo/Freeciv-derived and built-in policies with matched starts, knowledge, budgets and held-out scenarios. Version per-faction policy/config/private state. Add plugins/fault handling and tournaments after the client gate. Keep native reproduction separately measured. | [#187](https://github.com/diegoami/totengine/issues/187), plugins [#92](https://github.com/diegoami/totengine/issues/92), tournaments [#93](https://github.com/diegoami/totengine/issues/93), native fidelity [#117](https://github.com/diegoami/totengine/issues/117) |

## Delivery order and gates

These are dependency stages, not one release per stage or a fixed milestone count.
Named planning packages 14–29 and their concrete child stories are now mapped in
[milestones.md](milestones.md), following the owner's backlog-to-milestone request.
Their scopes retain reference/engine/human-acceptance gates; versions/dates and
release candidates remain unassigned. Refine/split each package from evidence.

1. **Reconcile the baseline.** Integrate/review the separate client-fix and
   optional-art candidates through the normal process. Keep published tags,
   owner previews and saves intact. Distinguish released from pending work.
2. **Complete client packages while establishing scenario tooling.** Next client
   scope candidates are science/rates/government and bounded original save/
   scenario entry. Finish missing engine decisions and reference contracts
   before promising UI behavior. Continue diplomacy, reports/help, advanced
   orders, sound and campaign/endgame dependencies in further slices. As
   separately scoped tasks, establish the scenario capability matrix, recording
   manifest and headless-run specification; no AI port is needed for this work.
3. **Prove an admitted scenario across modes.** Select a bounded classic profile
   from the corpus; complete its mechanics/events/import dependencies, then
   validate desktop play, browser play, headless all-AI runs, observation and
   recording on that same profile. Use parameter sweeps and author diagnostics
   to find gaps. This does not imply full FallRome/Lua/multiple-map parity.
4. **Extend compatibility and usability in coherent slices.** Add ToTPP/Lua,
   multi-map transfers/campaigns and deeper creation assistance according to
   measured dependencies. QoL packages can ship when their legal actions are
   ready; they need not wait for an unbounded final polish phase. They do not
   substitute for missing original features. Full client acceptance requires
   sustained human standard/scenario play on both surfaces and an explicit
   coverage/limitations record.
5. **Implement AI alternatives and plugins after the complete client gate.**
   Source study, license/API assessment and benchmark design may happen earlier.
   Then choose bounded adoption experiments and plugin slices. Original
   reproduction #117 is a distinct fidelity route, not a predecessor to every
   other AI. A0 #118 is first only within that program. External tournaments #93
   depend on #92; built-in-policy scenario experiments #184 do not.

Scenario authors may supply goals/behavior not expressible by a general policy.
Define explicit capabilities/constraints and unknowns; do not assume an AI can
infer arbitrary Lua/prose. Capture (#113), diplomacy (#96), endgame/scoring
(#97), sea/air/cargo/trade and map transfers are engine dependencies for all
relevant streams, regardless of which AI is used.

## Reuse and architecture

```text
desktop + playable browser | headless runner | observer | scenario workbench
                 session / recordings / filtered views
          validated commands, decisions and policy interfaces
        native Rust simulation: rules, events, RNG, maps, phases
             ToT formats / user resources / scenario identity
```

Human views and ordinary policies remain observation-limited. Spectator archives
are explicitly full-state products; selecting fog in a full archive is a viewing
control. Playback/recording does not grant player mutation authority. Preserve
request/entity/revision validation across any adopted client workflow.

Evaluate bounded C-evo planner/workflow experiments and whole clients against
both delivery surfaces, ToT dialogs, resources, visibility and lifecycle. Another
engine's import screen does not supply ToT event/rule compatibility. The
[source assessment](ai-client-reuse-assessment.md) records coupling and the lack
of a playing-strength benchmark.

Reference work can help the next client packages immediately: study the runnable
Freeciv client and C-evo source for production/list navigation, routing, reports,
focus and confirmation flows; compare each proposed interaction with the ToT
manual and accepted design. For AI, inventory planner/action dependencies,
scenario-role assumptions and private-state formats before choosing an adapter.
These are bounded research inputs to #188/#187, not a commitment to copy or port
an entire client. Record which workflow was inspected and what remains unknown.

Code policy remains MIT with audited provenance. Freeciv game-code/client reuse
needs an explicit integration/licensing decision; a process boundary is not an
automatic exception. C-evo source and media have different reuse status. The
approved Isotrident pack retains its separate license/source/credits. The main
ToT product still uses original resources; neither fallback art nor Open demo
commits us to another game's rules or public hosting.

## Acceptance and evidence

| Claim | Required check |
| --- | --- |
| ToT format/mechanic compatibility | Re-derivable original-data/native evidence and declared coverage; originals outside Git |
| Scenario support | Versioned profile/capability matrix: parse → load → simulated behavior → human campaign, with macro/Lua/map/objective gaps explicit |
| Both-client UI delivery | Actual keyboard/mouse/focus, invalid/cancel/loss cases, checkpoint continuation and owner human routes on both clients |
| Headless experiments | Same manifest reproduces orders/events/results; variants report exactly which parameters changed |
| Observation/recording | Live/replay event agreement; seeking/annotations do not execute gameplay; interrupted/incompatible recording recovery |
| AI adoption | Matched knowledge/budgets/starts, scenario objectives/failure metrics and deterministic private state; strength and native fidelity measured separately |
| Original save export | Original loads/re-saves our output with retained differential evidence; engine checkpoints never stand in for this gate |
| Release | Coherent scope, checks, applicable human acceptance and independent owner-launched review; tag exactly the agreed SHA |

No calendar estimates or later version numbers are assigned here. The original
2026-11-01 [feasibility study](feasibility.md) is historical reconnaissance;
this plan and the mission are the current scheduling/scope authority.
