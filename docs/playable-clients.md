# Playable browser and desktop clients

The current [mission](mission.md) and [delivery plan](plan.md) target the full
ToT interface and scenario play on both clients using original resources. The
routes below describe shipped behavior and explicitly labeled candidates.
Optional queues/rally points are planned under #188–#190; session recording is
#186. Neither feature set is implied by the existing controls/checkpoints.

An optional [Isotrident artwork pack](ui/freeciv-assessment.md)
supports both clients when ToT artwork is unavailable. Leave setup's ToT path
blank for the independently authored **Open demo**, or provide a ToT directory/
Rules.txt to keep your own rules with fallback graphics. The demo is a small
three-unit profile, not full original ToT balance. Prepare the pack with
`tools/prepare_isotrident.py`; set `TOTENGINE_ISOTRIDENT` to its directory, or
place it in `isotrident/` beside the executables. It retains GPL-2.0-or-later and
artist credits separately from the MIT engine. No original ToT resources are
included. The published v0.15 packages predate this candidate.

The clients support generated Original-profile surface games,
one human civilization and the existing built-in opponents. Both clients use
the same validated human session, presentation, owned artwork and checkpoints.

Run **totplay** (Windows: **totplay.exe**) for the local browser client, or
**totdesktop** (**totdesktop.exe**) for a standalone game window. The browser
launcher starts a local loopback host and opens your default browser. Use the
in-game Close client button to stop that host; closing a browser tab keeps the
session available for reconnection. In v0.15 a refreshed/reconnected tab can
resume an already-running opponent phase; #179 tracks the contract follow-up.
Desktop needs no browser/server.
The reference Linux x86-64 package requires glibc 2.34 or later; desktop also
requires the system SDL2 runtime. Windows x86-64 packages include SDL2 and have
been tested on Windows 10.

Enter your original Test of Time installation directory at setup, or set
`CIVTOT_HOME`. On Windows a typical path is `D:\games\CIVTOT`. Choose the seed,
map size, civilization count, human civilization and difficulty, then Start.
The installation must include the Original profile and its standard art.
Missing/incompatible resources produce an error without replacing your game.
No original artwork, Rules, saves or fonts are included. Desktop reads a system
DejaVu Sans font on Linux or Arial on Windows; `TOTENGINE_FONT` selects another
owned TTF if required.

## Checkpoint versions

Engine checkpoints are version-bound. The v0.16 candidate loads v0.16 engine
checkpoints; v0.15 preview checkpoints require their preserved preview binaries.
Demo display-name changes keep Rules/art identity unchanged within an engine
version. Original SAV/SCN import remains a separate, unimplemented client route.

## Quick Start and reproducible seeds

The prominent **Quick Start — random map** button starts with the current
setup and a fresh random seed. Setup also generates a random seed when the
client opens or a new game is confirmed. **Generated seed** remains visible and
editable; use the ordinary Start button with a chosen seed to reproduce a world.
Required research/founding choices still need an explicit human selection.

Open demo research now has readable presentation labels, used consistently in
the chooser, current-topic display and discovery notices. Its underlying rules
and resource identity stay unchanged so existing demo checkpoints still load;
the labels do not add new unlocks or change the demo into full ToT rules.

## Controls

The supported map commands follow the original Test of Time manual and installed
menu; [reference and remaining differences](https://github.com/diegoami/totengine/blob/main/docs/ui/original-controls.md).

- Click an own ready unit to select it and enter Moving, in either mode.
  Selection takes priority over movement: clicking another own unit never moves
  the previous unit. **V** toggles Moving/Viewing; **A** and **Activate unit**
  also activate the inspected unit/stack. A stack opens an explicit chooser.
  The selected ready **unit sprite blinks** in Moving and stays clickable during
  its hidden phase. Viewing has a separate inspection cursor.
- In Moving, click adjacent terrain to move, or use **7 8 9 / 4 6 / 1 2 3**
  on the keypad. The keypad works with Num Lock on or off. Cursor arrows also
  move in Moving; in Viewing they pan. Viewing keypad keys move only the
  inspection cursor. Enemy attacks and supported city occupation require
  confirmation; Confirm all moves also confirms empty-tile movement.
- **W** cycles movable units; **Space** skips the active unit's remaining moves.
  A selected exhausted unit reports that it has no moves left. When an order
  exhausts the active unit, another ready unit is selected; with none remaining,
  Viewing shows End of turn. A new human turn selects a ready unit in Moving.
  Ending the turn remains explicit; no automatic human turn advancement.
- **C** centers on the active unit. **Z / X** zoom in/out; **Shift+Z** restores
  standard zoom. The **+ / -** buttons and keys remain available as conveniences.
- **B** opens founding for an eligible settler. Type the name and accept;
  Escape cancels without consuming the settler. **Cities** cycles own cities.
- **Double-click an own city on the map** (including its garrison) or use
  **Manage city** to open its workspace. The first city click only selects or
  inspects; use movement keys to move into an own city. Click a radius tile
  to remove/assign its worker; arrows select a tile and **Space** changes it.
  Removing a worker creates an entertainer. Click a specialist to cycle
  entertainer → tax collector → scientist; tax/scientist roles need population
  five. Outputs/mood refresh immediately, without collecting income or ending
  the turn. **PageUp/PageDown** navigate your cities; **Escape** returns to the
  map and keeps accepted edits. **Ctrl+S / Ctrl+L** save/load inside the city too.
- v0.15 adds **Info / Support / Improvements / Happy** panels.
  Support distinguishes units supported here from units present in Info. Building
  sale rows retain explicit confirmation. Happy omits unchanged stages. The
  selected panel stays across city navigation; viewing a list never orders a
  unit. Tile arrow/Space keys apply only in Info; radius mouse edits remain
  immediate in every panel. The owner approved these panels in both clients;
  independent review agreed in round three (#162), and v0.15.0 is released.
- **Auto workers**, or the centre tile, requests automatic allocation. An empty
  tile clicked without a spare specialist also requests automatic allocation,
  following the original transition. First remove a worker when you want to
  assign that citizen to a particular empty tile. Blocked tiles reject the edit.
- **Choose production** opens a picker: arrows or click select, Enter accepts,
  Escape cancels. The city shows invested/required/remaining shields and an
  estimate at current output. A draft shows shields lost or restored when
  changing category. While the city stays open, switches use its initial budget;
  returning to the original category can restore that budget. Closing/navigating
  to another city ends it; buying rebases it. The map's quick picker ends its
  budget on acceptance. Saving an active city budget restores that city on load.
- **Buy production** shows a treasury/shield quote and requires confirmation.
  Insufficient funds leave only Cancel. Payment fills the shield box; the item
  completes during the next production phase. **Improvements / sell** selects
  an owned building, then shows its refund and lost effects before confirmation.
  One sale per city per turn is allowed; Palace/wonders are excluded. Cancel
  leaves gold, shields and accepted orders unchanged.
- **Research** opens the required research choice. Select explicitly and accept. Escape permits
  inspection while the choice still blocks gameplay; it never auto-selects.
- **Ctrl+N** ends the human turn, confirming if movable units remain. Opponents run
  in bounded steps and then return control. Pause/Continue controls that work.
  Loading a checkpoint during opponents requires explicit Continue.
- **Ctrl+S / Ctrl+L** save/load an engine checkpoint. Save pending choice preserves
  research and unfinished city naming. Tab/Shift-Tab and Enter navigate buttons;
  text fields support normal editing and clipboard shortcuts. Original orders
  not yet supported (such as F fortification, R roads and S sleep) are not
  reassigned to unrelated actions. Research and city controls remain explicit
  buttons until their fuller original workflows are implemented.

## Saving and scope

Checkpoint names contain 1–64 letters, digits, underscores or hyphens. Saving
requires a new name and never overwrites an existing slot. Slots appear in the
load dialog; setup also offers Load engine checkpoint. Default storage is
`%LOCALAPPDATA%\totengine\saves` on Windows and
`$XDG_DATA_HOME/totengine/saves` (or `~/.local/share/totengine/saves`) on Linux.
`TOTENGINE_SAVES` overrides it. Both programs can use the same folder.

The `.cth` format is an engine replay checkpoint, **not an original SAV**. It
requires the exact engine version and matching resource contents, and stores
pending decisions and private opponent state locally. Format 2 first ships in
v0.12; private development format 1 is unsupported. Invalid loads preserve the
current game. Long sessions take longer to replay on load. Save before closing;
there is no autosave or cross-version migration in this slice.

Supported action contexts are adjacent surface land movement/combat and the
existing limited city-occupation model. Capital relocation, civil war,
technology plunder and civilization respawn are incomplete. Sea/air/cargo,
map-edge wrapping moves, original SAV/scenario import, multiple maps, native
SAV export, government, diplomacy, full city management and native victory
handling remain later work. A required unsupported government phase stops with
a diagnostic and can be saved; it is never silently answered. This release
does not complete the playable-client track or unlock the full original AI port.

The art is read from your install: first-slice terrain/units, generic city style
and dialog/panel textures. Coast joins, native civilization recoloring,
city-style selection and full native window/layout parity are not claimed.
Retain LICENSE and notices when redistributing these executables.
