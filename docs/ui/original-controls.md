# Original Test of Time controls reference

Owner direction: preserve the original keyboard conventions; verify them before
adding bindings. The corrected v0.13 preview uses this
reference for supported controls. This records declared conventions, implemented
corrections and remaining gaps; it is not full UI parity or owner acceptance.

**Owner retest rejected this preview:** selection was reported worse, and the
owner requires the selected unit to blink rather than a tile alone. The client
was not changed during the subsequent reference study. Its findings, corrected
blink distinction, test gap and replacement contract are in
[selection and movement study](selection-and-movement-study.md), backed by
`ui-selection-study-v1.json`. Implementation claims below describe the rejected
preview's code, not accepted fidelity. In particular, sprite hit testing only
works in Viewing; selecting another unit in Moving remains a blocker.

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
| End player turn | Ctrl+N |
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
- Inspection recognizes visible unit pixels above the ground diamond. Moving
  onto an adjacent friendly stack submits a move instead of changing selection.
  The active unit is drawn above other units in its stack.
- A blinking diamond outlines the current unit square in Moving or inspection
  square in Viewing. Both renderers use presentation time only; animation does
  not generate input, change a revision, mutate game state or use either RNG.
- Browser keypad decoding uses the physical code, including Num Lock-off
  navigation labels. SDL retains physical keypad keys. Text fields keep normal
  editor/clipboard shortcuts, and dialogs retain their own input priority.

Regression evidence: `client-controls-v1.json`, the shared controller tests and
real-input checkers. Historical `client-input-route-v1.json` remains unchanged;
`client-input-route-v2.json` translates its old UI shortcuts to the corrected
keys and explicit Cities button, preserving the intended engine order sequence.

Owner testing found the original preview blocker: click unit → Activate unit →
terrain only inspected because activation failed to enter Moving. Automated
regressions now cover that exact route with a generated warrior and byte-exact
checkpoint comparisons. New owner acceptance in #137 remains pending; earlier
passing turn/city automation did not waive that blocker.

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
