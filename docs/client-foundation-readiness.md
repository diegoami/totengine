# Client resources and recovery — milestone 14 readiness

Status, 2026-11-08: **code integrated and automated routes pass; owner human
acceptance is pending**. The owner replied “I’ll test them later” on the new
check. [Acceptance #210](https://github.com/diegoami/totengine/issues/210) and
[readiness #211](https://github.com/diegoami/totengine/issues/211) remain open.
No numeric release version, frozen candidate or independent review is assigned.

## Owner feedback update: Quick Start and demo names

Follow-up [#213](https://github.com/diegoami/totengine/issues/213) adds the owner's
requested prominent Quick Start, random editable setup/new-game seeds and
readable demo technology labels/current topic. The current owner hosts and
Windows launchers use rebuilt `review-tmp/tot-client-quick-start-{linux,windows}`
packages. The active browser demo was saved and restored; refresh a tab retaining
a previous host revision. Existing original/demo saves remain compatible.
[Feedback validation](evidence/client-quick-start-v1.json) records the shared/
Chromium checks and seven unchanged fixed-seed Linux demo checkpoints. The
combined-package results below describe the earlier integration baseline; they
are not a fresh full three-surface run of these feedback changes. #210/#211
remain open and require the final candidate's actual acceptance/checks.

## Integrated scope

- Optional Isotrident artwork/resource delivery in both clients, retaining
  original-resource precedence and explicit invalid-path errors (#181).
- Browser queued-second-click handling, canceled-edit reconnect recovery,
  city-panel defaults on map/setup loads, and explicit second-tab continuation
  semantics (#182, #176–#179).
- Mission, backlog, named dependency-gated milestones and pinned C-evo/Freeciv
  source assessment (#191). No AI/client port or game-rules substitution.

All three PRs are merged. Integrated main at
`71f7d9fb4e20db250348ddac3b3da2021c5e1124` passed
[push CI](https://github.com/diegoami/totengine/actions/runs/42481973316).
The initial combined preview packages contained production code from
`7ad1e9302566836ca643dd62c12a102a938b0f53`. All crate/Cargo files are byte-identical
between those refs. Commit `89c7c1d0045420571ed980e00696dc58b56a9b6d` adds only
Windows checker coverage; it passed CI/autoplay before the explicit recovery
merge. These packages are previews, not final release artifacts.

## Combined evidence

[Machine-readable record](evidence/client-foundation-v1.json) retains source,
package and checkpoint hashes and private run locations.

| Check | Result and actual scope |
| --- | --- |
| Shared/desktop client tests | 34 passed, six fixture-author tests intentionally ignored; original installation available |
| Opt-in real-pack integration | Passed: actual pack load, Original rules/all unit sprites, normally produced unit selection/movement |
| Pack preparer tests | Two passed; both package inventories/hashes verified (98 Linux / 100 Windows payload files) |
| Original-resource packaged input | Chromium, Linux SDL/XTest and Windows SDL/SendInput pass; all 26 newly written common output checkpoints match in full, excluding copied input fixtures |
| Demo packaged input | All three surfaces complete ten rounds and required research/founding/normal save/reload; seven checkpoints match in full |
| Browser races | All seven cases pass, including held-first invalid second/third clicks, canceled queued edit with two failed ticks, and a fresh tab completing an authorized opponent round |
| Windows recovery parity | Actual guarded input passes map-load Info default and open-workspace Happy retention; no foreground guard was weakened |
| CI/autoplay | Combined production and checker refs passed; integrated-main push CI passed |

Full install-backed workspace/golden/strict Clippy and native findings from the
individual feature validations remain retained in
[artwork evidence](evidence/isotrident-fallback-v1.json) and
[recovery evidence](evidence/client-input-recovery-v1.json). They are historical
checks, not newly rederived native comparisons in this integration. Engine and
format code is unchanged. Any future frozen candidate must state its own exact
checks and applicable retained evidence.

## Owner previews and route

Fresh isolated owner saves are under
`/home/diego/review-tmp/totengine-owner-client-foundation`. Previous previews and
games remain untouched. QA hosts on 8817/8818 and native QA apps/displays are
stopped; the following two owner browser hosts remain running intentionally.

- Original browser: <http://127.0.0.1:8816>
- Open demo/artwork browser: <http://127.0.0.1:8820>
- Windows Original: `\\wsl.localhost\Ubuntu\home\diego\review-tmp\totengine-owner-client-foundation\launch-original.cmd`
- Windows demo: `\\wsl.localhost\Ubuntu\home\diego\review-tmp\totengine-owner-client-foundation\launch-demo.cmd`

Both client surfaces need the affected resource/input check: select/move an own
unit, inspect its blinking sprite and save/reload using a fresh slot, with
original resources and fallback art. This does not ask the owner to approve the
demo as a full alternative ruleset.

Original mode supplies `panelpractice`: load it, choose Happy, then reload inside
the open city (Happy retains). Escape to the map and reload the same unchanged
slot (Info defaults). Report blockers or confirm these routes pass. Automation
and earlier artwork approval cannot supply this new human result.

## Limits and remaining gates

Isotrident is a separately licensed GPL-2.0-or-later artwork pack with editable
PNG/spec source, credits and notices; engine code remains MIT. Its current
coast/style/animation limitations are documented in the artwork assessment.
Open demo has three units, seven enabled improvement slots and twelve research
choices; complete ToT rules or original scenario compatibility are not claimed.
Complete original resources continue to take precedence.

Public hosting, a complete redistributable standard rules profile, scenario
import, full client/native-AI reproduction and external plugins are separate
work. Original game assets are not in Git or these packages.

After actual owner acceptance, complete #211's candidate/version/release-note
work and applicable exact-candidate checks, then prepare the independent review
issue/prompt per [milestone-review.md](milestone-review.md). The owner launches
that review; no review or tag is created by this readiness record. Milestone 14
stays open while these gates remain.
