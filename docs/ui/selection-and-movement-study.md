# Selection and movement: reference study after the rejected preview

Status: reference study and replacement interaction contract. The resumed
implementation now follows this contract in both clients; **new owner acceptance
is pending**. Verification is recorded in `client-selection-v1.json`. The owner rejected the v0.13 controls preview after reporting that
unit selection had become worse, and required blinking of the selected unit.
The owner then asked for more time studying the game/manual before more UI work.
Client changes were paused for this study and subsequently resumed at the
owner's instruction. Earlier automated passes did not resolve that report. #137 and the milestone remain open.

Evidence: [numeric study](../evidence/ui-selection-study-v1.json),
[earlier original UI discovery](discovery.md), and the publisher manuals linked
in [original controls](original-controls.md). Original screenshots, saves,
manuals, copied install and Wine prefix remain outside Git. The numeric artifact
retains source/configuration/capture hashes and nine observations S01–S09.

## What was actually inspected

An isolated copy of the previously inspected install and Wine prefix ran on a
private Xvfb. The owner's installation, reference prefix and running clients
were neither driven nor modified. The executable was the installed
**ToTPP-patched ToT 1.1**, not `civ2.bak`; no RAM writes or quiet presentation
patches were used. Normal production in the copied early Spanish standard save
produced the warrior, and native research decisions were resolved explicitly.

| Observation | Direct result | Boundary |
| --- | --- | --- |
| S01–S02 | Madrid's real city screen and first produced warrior were inspected. After required notices/research, the map entered Moving Units and showed Moves: 1. | One early standard city and unit, not a complete UI/gameplay comparison. |
| S05/S07 | A click on the warrior's occupied square, starting in Viewing, entered Moving and centered it. Repeated after recentering, with the same result. | Single ready unit outside a city; mouse movement preference was off. Sprite-head clicks overlap other logical tiles and must not be generalized. |
| S06 | One keypad-6 order moved the warrior from (54,30) to (56,30). With Always wait off, exhaustion also advanced 3800 BC to 3750 BC and refreshed its moves. | This is movement plus automatic round advancement, not two movement orders. |
| S08 | With Always wait checked and visually verified, Right moved the warrior to (58,30), kept the year/treasury unchanged and returned to Viewing. Inspecting it showed End of Turn; A at its actual location did not return to Moving. | The inspection cursor initially remained at the previous location. An earlier A there is not an exhausted-unit activation test. |
| S09 | Enter from the exhausted state began the next human turn in Moving with Moves: 1. Space returned to Viewing/End of Turn without another year/treasury change. | Only one-unit scheduling was measured anew. Multi-unit chooser behavior remains earlier U30 evidence. |

The city screen places resources and production/buy at left, terrain centrally,
Info/Support/Improvements/Happy/Close at right, citizens along the bottom and food
storage at lower right. The current client layout differs. Later city work must
reconcile the actual panel roles and workflows with its screen contract rather
than treating the prototype as an original-game capture.

## Blink: distinguish the requirement from the configured reference

The German Test of Time manual, printed pages 75/77, explicitly distinguishes
an active unit blinking in Moving from a square cursor in Viewing. The earlier
English summary collapsed this distinction into a tile-only marker. **The
owner's required client behavior is selected-unit blinking.** A blinking tile
alone is not acceptance of that requirement.

Temporal captures of the configured ToTPP reference show additional nuance:
with Animated Units off, the sampled warrior body stays constant while brackets
around its square vary. The install has an Active unit indicators patch. A
controlled second launch changed only `UnitIndicators.Enabled` from 1 to 0,
reloaded the same native-written baseline and verified that the patch disappeared
from the application log. Brackets still appeared, and the sampled static body
remained constant. Therefore this study **does not establish that this patch
causes the bracket behavior**, nor does it establish an unpatched-UI comparison.
Do not use that configured reference to dismiss the owner's requested unit blink.

The artifact records all 48 temporal-frame hashes, sampled RGB-region hashes,
settings and the comparison limit. A constant sampled region is not proof about
every pixel of a sprite, or every original-game graphics configuration.

## Why the passing client tests missed selection

The preview's map handler performs sprite hit testing only while `!moving`.
Once Moving is set, a click on a different location reaches `move_to` before
own-unit selection. That includes clicks intended to select another own unit.
Depending on the hit location, the old active unit receives a move or a rejection
instead of the clicked unit becoming selected. The previous friendly-stack
change made this precedence worse.

The real-input movement checkers first used W to reach their intended warrior,
then switched to Viewing and clicked **that already-selected warrior**. That
route establishes movement from that setup; it does not establish general mouse
selection. Checking only that a diamond's pixels blink likewise does not test
that the selected unit blinks. Those are test-coverage errors, not evidence that
the owner's report is inconsistent with the implementation.

## Replacement contract for both clients

This contract separates reference facts, owner choices and unsupported later
work. The subsequent implementation is in the shared selection module and both
renderers; it does not change the reference facts or imply full native UI parity.

| Input/state | Required result |
| --- | --- |
| Click a visible own ready unit, in either mode | Select/activate that unit and enter Moving. Show its identity and moves. Selection must precede movement interpretation and must not spend any unit's moves. |
| Click an own stack | Make its members explicitly selectable through stable identities. A at the inspection location remains a keyboard route. Rendering and hit testing must agree about which member is shown. |
| Click an exhausted own unit | Inspect/select with an unmistakable exhausted state. Do not advertise it as ready to move or blink it as the active ready unit. |
| Moving: keypad/arrows or an adjacent empty-terrain click | Send the order for the displayed active ready unit. Retain the owner's accepted immediate empty-terrain mouse convention and attack/capture confirmations. |
| Click a different own unit while Moving | Select that unit; never move the previously active unit as a side effect. A keyboard move can still target a friendly occupied square. Do not introduce an ambiguous click that means both selection and movement. |
| Viewing: keypad or inspection click | Inspect without spending movement. A ready own-unit click can explicitly enter Moving as above. Empty terrain inspection is distinct from activating a unit. |
| Accepted order exhausts/kills the active unit | Advance selection to an eligible remaining unit, or show Viewing/end-of-turn when none remains. Preserve stale-input rejection so a queued click/key cannot move the new selection. |
| New human turn / required decision resolved | Present an eligible ready unit in Moving when one exists, with clear movement feedback; otherwise show inspection/end-turn state. |
| Blink and pointer hit testing | Blink the selected ready unit sprite in Moving using presentation time only. The unit remains selectable during the hidden blink phase. Inspection-cursor feedback is a separate visual state. |
| End of turn | Keep the already accepted explicit human end-turn default. Native automatic advancement is preference-dependent; do not silently add it while correcting selection. |

Required regression routes include selecting **a different own unit** while in
Moving and Viewing, stacked and exhausted units, clicking sprite head/body/feet,
selection during both blink phases, modal/focus priority, and queued input across
automatic selection changes. Real mouse routes must use ordinary window/canvas
coordinates at normal and resized/zoomed views. They must assert selected
identity and movement budget before issuing an order, not preselect with W and
infer that the later mouse click selected the actor.

The unit/sprite blink check must sample the actual selected sprite while keeping
other map pixels stable; a changing arbitrary marker pixel is insufficient.
Native/UI states and owner-selected deviations must remain explicit. Full
waiting queues, fortification/sleep, go-to, native mouse-edge arrows and complete
multi-unit scheduling are not recovered merely by these representative routes.

## Implementation and acceptance status

The earlier controls preview was owner rejected. The resumed implementation now
uses one shared render/hit order and one visible member per stack. Own-unit
selection takes priority in either mode, chooser entries retain entity IDs,
exhaustion advances to another ready unit or Viewing, and new human input selects
a ready unit. The selected sprite blinks through rendering time alone and remains
hittable while hidden. End-turn advancement remains an explicit human action.

New shared regressions and actual browser/Linux/Windows routes test a different
own unit, stack selection, sprite-body blink versus a stable other unit, hidden-
phase activation, zoom/resize, exhaustion and next-turn selection. Numeric
comparisons and scope are in `client-selection-v1.json`; these automated results
are not the pending new owner play result. No review candidate/tag is ready.

The owner ended the earlier session and later authorized continuation. The
reference Wine/Xvfb study was stopped cleanly. Existing owner games must be
preserved when switching preview builds. Full waiting queues, native mouse-edge
arrows and other later client packages remain outside this correction.
