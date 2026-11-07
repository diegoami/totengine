# Original Test of Time controls reference

Owner direction: preserve the original keyboard conventions; verify them before
adding bindings. The corrected v0.13 preview uses this
reference for supported controls. This records declared conventions, implemented
corrections and remaining gaps; it is not full UI parity or owner acceptance.

The previous preview was owner rejected. The subsequent
[selection and movement study](selection-and-movement-study.md) identified its
selection precedence and blink/test gaps. The resumed implementation now selects
own units in both modes, blinks the selected sprite and handles ready/exhausted
selection transitions. `client-selection-v1.json` records the new checks; the owner
confirmed both clients and the city route pass on 2026-11-07. The reference study's ToTPP configuration limits
still apply, and full original UI parity is not claimed.

## Publisher manual

[English Test of Time manual, MicroProse/Hasbro, 1999](https://manuals.plus/m/0a903e975f22c7da36286b990be87bba1bd2c491a617c786f0280d99353c5fa8.pdf).
Use printed page numbers below; the PDF includes sixteen preliminary pages.
This is the Test of Time edition, including its Science Fiction and Fantasy
games. A classic Civilization II manual is not an interchangeable source.

| Printed page | Declared behavior |
| --- | --- |
| 5 | V switches between viewing and moving. Mouse movement is an option, initially disabled. |
| 9 | B builds a city with an eligible settler. |
| 67 | Eight surrounding numeric-keypad keys give compass directions; 5 does nothing. |
| 68 | Active-unit feedback; do not infer a tile-only blink. German pages 75/77 explicitly distinguish the active unit from the Viewing cursor. |
| 69 | W defers a unit until other active units have received orders. |
| 70 | Moving accepts keypad directions or cursor arrows. Viewing uses the keypad to move the inspection cursor. A activates units at that cursor, with a chooser for a stack. |
| 97–98 | M is mining, R roads/railroads, O transformation and E airbase construction; these letters must not trigger unrelated actions. |

The manual's special-key glyphs are imperfectly extracted: for example, the
Space symbol in the skip-turn paragraph appears as a letter. Read such glyphs
visually or corroborate them against the installed menu; do not implement the
extracted character literally. The manual describes intent, not proof that our
controller or either adapter implements it.

## Installed English menu and observed behavior

The owner's `Original/Menu.txt` independently declares:

| Action | Binding |
| --- | --- |
| View/move | V |
| Activate | A |
| Found city | B |
| Center view | C |
| Wait / skip turn | W / Space |
| End player turn | Ctrl+N; Enter activates the visibly focused End button when exhausted |
| Save / load | Ctrl+S / Ctrl+L |
| Zoom in / out | Z / X |
| Standard zoom | Shift+Z |
| Find city | Shift+C |
| Science advisor | F6 |

Menu SHA-256:
`b86fd8b92749b47efedbb63274970a925f3839813a635720af905388b0b4f3f3`.
This is a derived inventory; the copyrighted menu and manuals stay outside Git.
Bindings are contextual: F, for example, serves both fortification and settler
fortress construction. An unsupported action must not be reassigned to an
unrelated command merely because its original action is not implemented.

[Discovery U30](discovery.md) already observes V, keypad movement, A with stack
selection, entry into Moving, W/Space and Z against the original UI. This is
stronger evidence for those exercised transitions than menu declarations alone.
It does not establish every modifier, arrow-key dispatch or scheduling detail.

## Corrected preview and remaining acceptance

Implemented in the shared controller and both adapters:

- B/V/A/C, W/Space, Ctrl+N, Ctrl+S/L and Z/X/Shift+Z have their supported
  original meanings. The old unrelated single-letter shortcuts are removed.
  Research and city management remain explicit buttons. The partial research
  picker is not presented as a complete F6 science advisor.
- Activate unit enters Moving when the unit has moves; an exhausted selection
  reports its state and remains Viewing. A opens an explicit paged chooser for
  stacks. Viewing keypad directions move only the inspection cursor. Arrows
  move the active unit in Moving and retain viewport panning in Viewing.
- Render and hit testing share the same visible units, including sprite pixels
  above the ground diamond. Own-unit/stack selection takes precedence in both
  modes; selecting another own unit does not issue movement. One representative
  per stack is shown, with the chosen member visible after selection.
- The selected ready unit sprite blinks in Moving. Viewing has a distinct
  inspection cursor. Animation uses presentation time only, never input,
  revisions, game state or either RNG; hidden sprites remain hittable.
- Exhaustion advances to a remaining ready unit or Viewing/End of turn. New
  human turns and resolved required choices select a ready unit in Moving.
  End-turn advancement remains explicit. With all units exhausted, the existing
  End-turn button receives visible focus, so Enter activates it; Tab can still
  choose another button. This also applies when returning from city/dialog
  inspection with no ready units.
- Browser keypad decoding uses the physical code, including Num Lock-off
  navigation labels. SDL retains physical keypad keys. Text fields keep normal
  editor/clipboard shortcuts, and dialogs retain their own input priority.

Current regression evidence: `client-selection-v1.json`, the shared controller
tests and expanded real-input checkers. `client-controls-v1.json` is historical
coverage of the rejected preview and did not establish general mouse selection. Historical `client-input-route-v1.json` remains unchanged;
`client-input-route-v2.json` translates its old UI shortcuts to the corrected
keys and explicit Cities button. V3 additionally records actor identities for
orders, preserving the same engine trajectory across automatic UI selection.

Owner testing found the original preview blocker: click unit → Activate unit →
terrain only inspected because activation failed to enter Moving. Automated
regressions now cover that exact route with a generated warrior and byte-exact
checkpoint comparisons. The replacement contract and different-unit/sprite checks now supersede that
limited route. New owner acceptance in #137 passed on 2026-11-07; earlier passing
turn/city automation did not waive the then-reported blocker.

The mouse preference interface, native edge-arrow mouse movement, complete
unit scheduling, full advisor/menu layouts and unimplemented orders remain
outside these corrections. Adjacent terrain clicking retains the owner's
accepted mouse convention. The prototype is a starting point, not authority for
inventing replacement shortcuts or workflows.

## Continuing UI fidelity

Owner direction after finding the manual: use it to build a more faithful UI.
For each later client package, reconcile the relevant manual screen/workflow
with discovery captures and the installed original, then update the screen
contract before implementation. Keep both clients aligned. The established
original artwork, window/panel roles, command names, contextual availability and
immediate city-edit behavior are the baseline. Differences need explicit scope
and tracking, rather than silently becoming new conventions. This does not
claim that the current toolbar/panels already reproduce the original layout.

Viewing keypad input moves the inspection cursor across the visible map. The
map pans only when necessary to keep its diamond inside the workspace. This
visibility rule is client presentation policy, not a claim of exact native scroll
thresholds. Inspection remains inert and never selects a new movement actor.
