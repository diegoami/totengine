# Planned milestones

Owner requested milestone alignment with the reorganized backlog, 2026-11-08.
[Mission](mission.md) · [Delivery plan](plan.md) · [Backlog](../BACKLOG.md) ·
[Live GitHub milestones](https://github.com/diegoami/totengine/milestones).

The original thirteen historical milestones are unchanged. Milestone 14 is now
closed with released v0.16.0 after owner acceptance and independent AGREE.
Milestone 15 is closed with released v0.17.0 after third-round AGREE #224,
shipping inspection and a conditional entry contract with zero runtime admissions.
Milestone 16 is released as v0.18.0 after first-round AGREE #244 (stories stay open
pending owner acceptance); remaining packages (17–29) retain unassigned versions/dates. Exact release/review state
comes from the live queue.
This is a planning horizon, not a fixed count of remaining releases or a claim
of complete client delivery after the listed UI slices.

## Package map

| Milestone | Primary stories / PRs | Package prerequisites and gates |
| --- | --- | --- |
| [14 — v0.16.0: Client resources and recovery](https://github.com/diegoami/totengine/milestone/14) | [#176](https://github.com/diegoami/totengine/issues/176), [#177](https://github.com/diegoami/totengine/issues/177), [#178](https://github.com/diegoami/totengine/issues/178), [#179](https://github.com/diegoami/totengine/issues/179), [#181](https://github.com/diegoami/totengine/pull/181), [#182](https://github.com/diegoami/totengine/pull/182), [#191](https://github.com/diegoami/totengine/pull/191), [#210](https://github.com/diegoami/totengine/issues/210), [#211](https://github.com/diegoami/totengine/issues/211) | Existing engine/reference/design prerequisites. |
| [15 — v0.17.0: Scenario inspection and entry boundary](https://github.com/diegoami/totengine/milestone/15) | [#192](https://github.com/diegoami/totengine/issues/192), [#193](https://github.com/diegoami/totengine/issues/193) | Existing engine/reference/design prerequisites. |
| [16 — v0.18.0: Science, rates and government — both clients](https://github.com/diegoami/totengine/milestone/16) | [#194](https://github.com/diegoami/totengine/issues/194), [#195](https://github.com/diegoami/totengine/issues/195), [#196](https://github.com/diegoami/totengine/issues/196) | After [14](https://github.com/diegoami/totengine/milestone/14). |
| [17 — Classic save and scenario entry — both clients](https://github.com/diegoami/totengine/milestone/17) | [#197](https://github.com/diegoami/totengine/issues/197), [#198](https://github.com/diegoami/totengine/issues/198) | After [14](https://github.com/diegoami/totengine/milestone/14), [15](https://github.com/diegoami/totengine/milestone/15). |
| [18 — Diplomacy: contact and peace decisions — both clients](https://github.com/diegoami/totengine/milestone/18) | [#199](https://github.com/diegoami/totengine/issues/199), [#200](https://github.com/diegoami/totengine/issues/200) | After [14](https://github.com/diegoami/totengine/milestone/14), [16](https://github.com/diegoami/totengine/milestone/16). |
| [19 — Reports and reference help: own-civilization views](https://github.com/diegoami/totengine/milestone/19) | [#201](https://github.com/diegoami/totengine/issues/201), [#202](https://github.com/diegoami/totengine/issues/202) | After [14](https://github.com/diegoami/totengine/milestone/14), [16](https://github.com/diegoami/totengine/milestone/16). |
| [20 — Headless scenario experiments: reproducible batches](https://github.com/diegoami/totengine/milestone/20) | [#203](https://github.com/diegoami/totengine/issues/203), [#204](https://github.com/diegoami/totengine/issues/204) | After [15](https://github.com/diegoami/totengine/milestone/15), [17](https://github.com/diegoami/totengine/milestone/17). |
| [21 — Scenario observer: events, inspection and replay](https://github.com/diegoami/totengine/milestone/21) | [#205](https://github.com/diegoami/totengine/issues/205) | After [15](https://github.com/diegoami/totengine/milestone/15), [20](https://github.com/diegoami/totengine/milestone/20). |
| [22 — Session recording: play and observation bundles](https://github.com/diegoami/totengine/milestone/22) | [#206](https://github.com/diegoami/totengine/issues/206), [#207](https://github.com/diegoami/totengine/issues/207) | After [17](https://github.com/diegoami/totengine/milestone/17), [21](https://github.com/diegoami/totengine/milestone/21). |
| [23 — Optional city production queues — both clients](https://github.com/diegoami/totengine/milestone/23) | [#189](https://github.com/diegoami/totengine/issues/189) | After [14](https://github.com/diegoami/totengine/milestone/14). Optional; legal automation and new both-client acceptance required. |
| [24 — Optional rally points and land routing — both clients](https://github.com/diegoami/totengine/milestone/24) | [#190](https://github.com/diegoami/totengine/issues/190) | After [14](https://github.com/diegoami/totengine/milestone/14). Optional; legal automation and new both-client acceptance required. |
| [25 — Scenario author tools: lint and dry-run validation](https://github.com/diegoami/totengine/milestone/25) | [#208](https://github.com/diegoami/totengine/issues/208) | After [15](https://github.com/diegoami/totengine/milestone/15), [20](https://github.com/diegoami/totengine/milestone/20). |
| [26 — AI adoption: first bounded planner comparison](https://github.com/diegoami/totengine/milestone/26) | [#209](https://github.com/diegoami/totengine/issues/209) | After [20](https://github.com/diegoami/totengine/milestone/20). **Blocked on complete clients #21/#94/#125.** |
| [27 — Pluggable AI protocol and reference player](https://github.com/diegoami/totengine/milestone/27) | [#92](https://github.com/diegoami/totengine/issues/92) | Existing engine/reference/design prerequisites. **Blocked on complete clients #21/#94/#125.** |
| [28 — AI tournaments and scenario policy comparisons](https://github.com/diegoami/totengine/milestone/28) | [#93](https://github.com/diegoami/totengine/issues/93) | After [27](https://github.com/diegoami/totengine/milestone/27), [20](https://github.com/diegoami/totengine/milestone/20). **Blocked on complete clients #21/#94/#125.** |
| [29 — Original-AI fidelity: inventory and integration boundary](https://github.com/diegoami/totengine/milestone/29) | [#118](https://github.com/diegoami/totengine/issues/118) | Existing engine/reference/design prerequisites. **Blocked on complete clients #21/#94/#125.** |

## Selection and completion rules

1. Milestone 14 groups the resource/recovery code and planning PRs; it
   does not retroactively change v0.15 or establish a new human acceptance.
   Owner acceptance #210 and independent AGREE #216 passed; v0.16.0 is released
   at exactly the reviewed SHA and milestone 14 is closed.
   See [the release record](v0.16.0-readiness.md).
2. Inspection/entry boundary (15) is released; it grants no runtime admission.
   Science/government (16) is the next planned client package, scoped from current
   references. Classic entry (17) waits for an actually admitted profile; the
   headless/observer/recording chain then validates that same supported profile.
3. Diplomacy (18) and reports/help (19) are bounded first slices. Advanced orders,
   remaining native screens, sound, sea/air/cargo, endgame, Lua and multiple-map
   behavior still require separately scoped packages and mechanics evidence.
   The original feature/coverage matrix, not completing this list, determines
   whether #21/#94/#125 can close.
4. Queues (23) and rally points (24) are optional reviewed settings. They need
   actual production/routing actions and persistence; they do not replace
   original UI coverage. Author tools (25) start with lint/dry-run assistance,
   not a full graphical editor or original SAV/SCN export.
5. AI adoption (26), plugins (27), tournaments (28) and native-fidelity inventory
   (29) remain blocked until complete playable-client acceptance. Completing
   only science, entry or reports does not clear the gate. Source/API/license
   study and benchmark planning may proceed earlier. Original fidelity #117
   is a separate route; its A0 is not a prerequisite for every AI alternative.
6. MCP interface #250 (owner request, 2026-11-11) has no package milestone yet.
   Its headless/read-only slice and client-attached advisory slice can be scoped
   as separate packages; acting MCP agents share milestone 27's plugin gate.
7. Umbrella programs remain unassigned to these new release packages and link
   to their child milestones. Close a bounded story on its evidence; do not
   close the whole scenario/recording/client program after one slice. The older
   T0 umbrella keeps its pre-existing historical association.
8. Dependencies are explicit in milestone descriptions and story bodies. IDs
   do not imply a calendar or mandatory ordering between independent packages.
   Scope can be refined/split after reference and capability findings.
9. Before a release, freeze an admitted scope and version, collect relevant
   both-client human routes and checks, then use the owner-launched independent
   process in [milestone-review.md](milestone-review.md). No review, tag, full-AI
   port or new implementation is launched by milestone assignment.
