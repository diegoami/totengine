# Backlog and delivery conventions

**GitHub issues are the live queue; GitHub milestones are scoped releases.**
This file maps programs and records working conventions, not a second story
status database. [Mission](docs/mission.md) · [Delivery plan](docs/plan.md) ·
[Planned milestones](docs/milestones.md) ·
[Decision 0008](docs/decisions/0008-scenario-platform.md).

## Program map

| Product target | Live program / stories | Planning position |
| --- | --- | --- |
| Native desktop and playable browser, complete original UI | [#21](https://github.com/diegoami/totengine/issues/21), desktop [#94](https://github.com/diegoami/totengine/issues/94), browser [#125](https://github.com/diegoami/totengine/issues/125) | Continue small both-client packages from reviewed contracts; retain human acceptance for each new route |
| Scenario opening, compatibility and creation assistance | [#183](https://github.com/diegoami/totengine/issues/183), classic engine [#20](https://github.com/diegoami/totengine/issues/20), macro events [#16](https://github.com/diegoami/totengine/issues/16), ToTPP/Lua [#22](https://github.com/diegoami/totengine/issues/22) | Capability matrix and bounded import first; diagnostics/test-run tools grow toward safe editing/export and authoring |
| Headless all-AI scenario experiments and parameter sweeps | [#184](https://github.com/diegoami/totengine/issues/184), standard campaigns [#23](https://github.com/diegoami/totengine/issues/23) | Build on existing policies; declare supported scenarios, changed parameters and reproducibility |
| Scenario-aware observation | [#185](https://github.com/diegoami/totengine/issues/185) | Extend released observer with scenario events/objectives/maps, comparison and replay navigation |
| Game and observation session recording | [#186](https://github.com/diegoami/totengine/issues/186) | Manifest/order/event/checkpoint foundation, presentation/notes, recovery; later audio/video/clips |
| Optional QoL and alternative UI settings | [#188](https://github.com/diegoami/totengine/issues/188), city queues [#189](https://github.com/diegoami/totengine/issues/189), rally points [#190](https://github.com/diegoami/totengine/issues/190) | Faithful baseline remains available; scope each workflow with actual engine dependencies and both-client acceptance |
| AI selection, reuse and plugins | [#187](https://github.com/diegoami/totengine/issues/187), protocol [#92](https://github.com/diegoami/totengine/issues/92), tournaments [#93](https://github.com/diegoami/totengine/issues/93) | Source/API/license assessment now; full ports/plugins after complete playable clients |
| Exact original-AI fidelity | [#117](https://github.com/diegoami/totengine/issues/117), A0 [#118](https://github.com/diegoami/totengine/issues/118), existing A1–A8 stories | Separate gated fidelity program; A0 is first within it, not a prerequisite for every alternative AI |

Shared mechanics remain real dependencies: diplomacy [#96](https://github.com/diegoami/totengine/issues/96),
endgame/scoring [#97](https://github.com/diegoami/totengine/issues/97), capture
[#113](https://github.com/diegoami/totengine/issues/113), and measured domain/map/
event work. An adopted AI or client cannot compensate for absent ToT mechanics.

## Near-term selection

1. v0.16.0 / milestone 14 is released after explicit both-client owner acceptance
   #210 and independent AGREE #216. Finish the separate post-tag follow-ups
   #217–#219 without moving the tag. [Release record](docs/v0.16.0-readiness.md).
2. Select the next bounded both-client package from
   [client milestones](docs/ui/client-milestones.md). Science/rates/government and
   original-save/scenario entry are scope candidates; finish reference/API gaps
   before assigning a release. Complete remaining original features in further
   coherent slices, not a single optimistic full-client milestone.
3. Scope the scenario capability corpus/import dependencies (#183), headless
   manifest/reporting (#184), observation (#185) and recording foundation (#186)
   as related packages. Existing policies permit useful scenario tests before
   external plugins. Parsing Roman Civil Wars is not a campaign pass.
4. Evaluate Freeciv/C-evo workflows and optional enhancements against the ToT
   baseline; retain #187 source assessment now. Do not launch full AI ports or
   the external plugin implementation before the client gate.

Named planning milestones 14–29 now carry bounded stories from this backlog;
[the package map](docs/milestones.md) records their dependencies and AI gates.
Milestone 14 is released as v0.16.0, after owner acceptance and independent review. No future versions, calendar schedule or fixed remaining release
count are assigned. Umbrella programs link to their child milestones and remain open
across slices; an unassigned program is not an abandoned target. Older T0–T4/RG identifiers remain useful history; they no
longer impose the old sequence or defer all authoring/QoL to T4.

## Story contract

Before selecting implementation work, an issue must name:

- The user route and both delivery surfaces where applicable.
- Its admitted fixture/profile and original-reference or source provenance.
- Engine/API, event, map, resource and other story dependencies.
- Confirm/cancel, invalid/stale input, unsupported feature and recovery behavior.
- Checkpoint/replay/recording effects and meaningful acceptance checks.
- Material exclusions and which claim it establishes: parse, simulate, observe,
  human play, export, AI strength or native fidelity.

A source study is not a port; a deterministic run is not proof of strong AI;
a checkpoint is not an original SAV export. Source availability does not change
our [reuse policy](docs/prior-art.md). Define optional automation/settings rather
than silently changing original rules, controls or scenario logic.

## Difficulty, priorities and delegation

Existing labels remain: `p0` urgent, `p1` next-candidate, `p2` later;
`diff-easy` / `diff-hard`; `t0`…`t3` and `epic` for historical grouping.
Dependencies and explicit milestone scope decide actual scheduling; labels are
not a promise that every p1 program runs at once.

| Difficulty | Meaning | Required preparation |
| --- | --- | --- |
| E | Low uncertainty: mechanical/tooling/docs, format known, acceptance testable | Focused validation and ordinary review |
| H | Reverse-engineering, parity-sensitive work or architecture with unresolved assumptions | Evidence/design note before code, high-effort investigation and independent milestone review |

Oracle parity is H regardless of line count. Reassess when work starts. Split
an H story after two failed approaches before trying a third; record unknowns.
This convention does not authorize spawning agents. Use delegation only when
the owner or applicable instructions request it.

## Releases and standing work

Latest released tag: **v0.16.0**, client resources and recovery.
[Changelog](CHANGELOG.md) and [release readiness](docs/v0.16.0-readiness.md)
record earlier slices; [GitHub milestones](https://github.com/diegoami/totengine/milestones)
and issues carry current state.

Release small, coherent themes, with the applicable human routes on both clients,
then independent owner-launched review. The main session prepares the candidate,
evidence and review prompt; it never launches milestone review. Tag exactly the
agreed SHA per [milestone-review.md](docs/milestone-review.md). Do not guarantee
one review round or close a story because its plan was rewritten.

Keep format evidence regenerated after intentional format changes. Never commit
original assets/scenarios/saves. Preserve owner previews and games. Downloaded
community scenarios are installed in individual folders in the real CIVTOT
Scenario directory, without overwriting existing resources.
