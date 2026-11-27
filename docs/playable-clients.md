# Playable browser and desktop clients

A player-oriented guide to installing, starting and playing the game is in
the [user guide](manual.md); this page documents the same behavior for
contributors.

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

The clients support generated single-map games with the Original rules,
one human civilization and the existing built-in opponents. Both clients use
the same validated human session, presentation, owned artwork and checkpoints.

Run **totplay** (Windows: **totplay.exe**) for the local browser client, or
**totdesktop** (**totdesktop.exe**) for a standalone game window. The browser
launcher starts a local loopback host and opens your default browser. Use the
in-game **Game > Close client** menu entry to stop that host; closing a browser tab keeps the
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

The clients remember the last successfully started setup host-side (a small
`setup.json` beside the saves folder, shared by the browser and desktop
clients): the ToT directory, civilization count, human civilization,
difficulty and map size are prefilled on the next launch. An explicit
`CIVTOT_HOME` still wins over the remembered directory, and the seed is
deliberately not remembered — it regenerates per launch. Delete the file to
reset the prefill.

Both clients scale the 1024×768 logical stage to any window size. The browser
renders at native resolution (text stays vector-crisp at any scale and pixel
art stays nearest-neighbour); the desktop opens at the largest comfortable
default for the display — 2× on 4K, 1.5× on 1440p, the logical size on
1080p-and-below — and is freely resizable.

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

The map screen currently uses the provisional layout B frame ([design record](ui/layout-b.md));
the target is the original's top-band frame ([decision 0009](decisions/0009-client-design-direction.md)):

- A **menu bar** along the top holds every map command, each with its key:
  **Game** (save, load, new game, Settings, Keys, Close client), **Kingdom**
  (rates, revolution, Cities), **View** (Moving/Viewing, center, zoom),
  **Orders** (found city, wait, skip, activate, end turn), **Advisors** (F1, F2,
  F4, F5 reports, F6 science, research), **World** (F7, F8, F9, F11, Messages)
  and **Civilopedia**. Click a title, or press **F10** and use the arrows and
  Enter; Esc closes. An entry that cannot run right now is greyed and says why.
  Keys work exactly as before; the menus only make the commands visible.
- **Wide and full-screen windows** (#321): the picture keeps 768 logical
  pixels of height and grows as wide as the window allows, up to 21:9. On the
  map screen the extra width goes to the map, and the right column stays at the
  right edge. Dialogs, the city window and setup keep their layout, centred. **Alt+Enter** or
  **View > Full screen** toggles full screen (the browser asks for its own full
  screen; Esc there returns to a window). A 4:3 window looks as before.
- **Side column on wide screens** (#337): from 16:9, a second column sits
  left of the right column. It shows the latest turn's messages in full, then
  earlier ones newest first, with **All messages** at the bottom. It replaces
  the message dock and the new-messages overlay there; the map gives up that
  width. It is on by default when the window is that wide. At 4:3 and 16:10
  nothing changes. It is presentation only and never changes the game.
- The **right column** shows a **minimap** of what your civilization has seen
  (a click centres the map there), your turn, government, treasury, rates and
  research, the selected unit, the selected city with **Manage city** and
  **Choose production**, and the end-of-turn prompt with **End turn**.

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
  standard zoom. The **+ / -** keys and the **View** menu remain available as conveniences.
- **B** opens founding for an eligible settler. Type the name and accept;
  Escape cancels without consuming the settler. **Cities** opens the next own city's window (a notice explains how to found one when you have none).
- Press **?** (or **Game > Keys…**) for the keyboard help: original keys plus ToTEngine additions.
- **Messages** (**World** menu) keeps every turn's notices for the session, newest first and paged. Messages are presentation-only: checkpoints do not store them.
- **Mouse wheel** pages the list under the pointer: reports, Civilopedia, Messages, choice lists, the checkpoint list and the city panels. It does nothing over the map. In the city window, the **Sort** button reorders the Info, Support and Improvements lists (listed order, name, shields, food or upkeep, largest numbers first); it only changes the display.
- **Settings** (**Game** menu), or **Enhancements…** on the setup screen, holds optional improvements that differ from the original. All are off by default and saved in `enhancements.json` beside the saves folder, shared by both clients; delete the file to reset. *New-messages overlay on the map* lists a turn's notices over the map (with **Open messages** / **Dismiss**). *Confirm every move* asks before each unit move; attacks and captures always ask. *Map tooltips* describes a known tile when the pointer rests on it: terrain, river/road, city and units, from your civilization's view only. *Goal estimates in the science advisor* adds the goal path and turn estimate to F6 (see below). *Order buttons under the map* adds a strip with the selected unit's orders (Move/View, Wait, Skip, Found city, Activate), each doing exactly what its key does; the map gets a little shorter. *Message log in the right column* keeps the latest messages beside the map, newest first, with **All messages** opening the full log; the minimap gets smaller to make room. *Path preview with turn counts* works the same way while a unit is selected to move: the tip shows the cheapest route over tiles your civilization knows (gold dots, a number where each turn's moves run out) and how many turns it takes; it follows the engine's move costs and only the foreign units you can see, so it is an estimate, and it gives no goto order. None of them affect checkpoints, recordings or rules.
- **Double-click an own city on the map** (including its garrison) or use
  **Manage city** to open its workspace. The first city click only selects or
  inspects; use movement keys to move into an own city. Click a radius tile
  to remove/assign its worker; arrows select a tile and **Space** changes it.
  Removing a worker creates an entertainer. Click a specialist to cycle
  entertainer → tax collector → scientist; tax/scientist roles need population
  five. Outputs/mood refresh immediately, without collecting income or ending
  the turn. **PageUp/PageDown** navigate your cities; **Escape** returns to the
  map and keeps accepted edits. **Ctrl+S / Ctrl+L** save/load inside the city too.
  The city window follows the classic arrangement (layout B, #314): a title bar
  with Previous/Next city, the citizens strip (workers, then specialists to
  click), City resources, Food storage and production with Choose/Buy/Improvements
  on the left, the radius map in the middle, the panels on the right and
  Save, Load, Research and Return to map along the bottom.
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
- **Research** (**Advisors** menu) opens the required research choice. Select explicitly and accept. Escape permits
  inspection while the choice still blocks gameplay; it never auto-selects.
- **Rates (Shift+T)** and **Revolution (Shift+R)** are in the **Kingdom** menu,
  **Science (F6)** in **Advisors**. Rates shows the three shares with per-share limits for the government;
  Up/Down pick a row, Left/Right (or −/+) move it through the original slider
  rule, **L** locks a row, Enter applies one validated command and Esc discards
  the draft. The dialog shows an engine quote of treasury, taxes, upkeep, science and
  disorder for the displayed split. Revolt asks for confirmation (Esc cancels;
  Enter confirms) and names the turn at which the government chooser opens. That
  chooser and the rate review after it are **required**: Esc keeps them open
  (Esc in the review resets the split), and Save pending / Load checkpoint stay available.
  Science (F6) is a window with the running project and its progress bar, the
  goal with **Set goal… / Clear goal**, the advances available next and the
  advances you know (#315); the goal only suggests the next research choice.
  With *Goal estimates in the science advisor* on (Settings), it also shows the
  path to the goal, a turn estimate (a lower bound at today's science rate) and
  a sort button for the available list. Locks last for the client session.
- **Reports (F1, F2, F4, F5, F7)**, also in the **Advisors** and **World** menus, open own-civilization
  reports: City status, Defense, Attitude (the *Mood* tab), Trade and Wonders. Up/Down select a row, PageUp/PageDown page, **S** (or a header
  button) sorts, Enter opens the selected city and Esc there returns to the report; Esc closes.
  Reports are read-only views of your own data and never change the game. See
  [reports milestone](ui/reports-milestone.md) for what is and is not covered.
- **Civilopedia** (its own menu; also a button inside the research prompt) shows the
  selected ruleset's own `Describe.txt` text by category. Left/Right change category,
  Up/Down select or scroll, Enter reads or goes back, Esc steps back and closes. In the research
  prompt it describes the highlighted advance and returns to the same choice. It needs a ToT
  installation; the open demo has no Civilopedia text.
- **Ctrl+N** ends the human turn, confirming if movable units remain. Opponents run
  in bounded steps and then return control. Pause/Continue controls that work.
  Loading a checkpoint during opponents requires explicit Continue.
- **Ctrl+S / Ctrl+L** save/load an engine checkpoint. Save pending choice preserves
  research and unfinished city naming. Tab/Shift-Tab and Enter navigate buttons;
  text fields support normal editing and clipboard shortcuts. Original orders
  not yet supported (such as F fortification, R roads and S sleep) are not
  reassigned to unrelated actions. Research and city controls remain explicit
  buttons until their fuller original workflows are implemented.

## Experimental scenario entry

Owner decision 2026-11-18 ([decision 0008](decisions/0008-scenario-platform.md#experimental-scenario-entry--owner-2026-11-18)).
**Open scenario (experimental)** on the setup screen, or **Game > Open
scenario…** during a game, lists the `.scn` and `.sav` files in your
installation's folders (up to three levels deep; files in the install root
itself are not listed). Choosing one opens a **preflight**; nothing starts until
you accept it. The preflight shows the turn, map size, the civilizations you
can play (any with units or cities; the one the file marks as human is
preselected), the opponents' engine policy, and every gap the engine knows
about:

- the game starts a fresh engine turn, so every unit gets its full movement,
  unless the file is a plain single-player save played by its own player:
  that resumes the saved turn as the original does (#197), with spent
  movement kept and the human moving first;
- opponents are played by the engine AI (`builtin-1`, or `ai-v1` when the
  built-in AI cannot read the scenario's Rules), not the original AI's plans;
- event triggers that never fire, event actions that do nothing yet and
  CREATEUNIT (approximate placement);
- Lua scripts beside the scenario, which are not run;
- for a file whose scenario calendar is not decoded, dates shown as turn
  numbers;
- in multi-map scenarios (Midgard, Warring States), everything beyond the
  first map: only map 0 is played, and the preflight counts the units and
  cities left out.

Dates read as the original writes them, with the scenario folder's
`Labels.txt` for era and month names (#330): the status panel shows
`date · Turn N`, and the title, reports and Science advisor show the date.

Event TEXT boxes (#326) open as pop-up dialogs, one at a time, when the human's
turn is in progress: boxes fired by opponents' moves or the round wait for the
human's turn, boxes fired by the human's own moves open at once. Enter or Esc
closes each; every box is also kept in the message log. A box marked
`No Broadcast` (Macro.txt) appears only when the human civilization triggered
it (the attacker of UNITKILLED/CITYTAKEN); one whose trigger names no
civilization reaches nobody (the original's choice there is not verified).

Event flags (#328) work as in the original. FLAG sets or clears per-civilization
flags, and CHECKFLAG tests them once at the start of every turn. Flags set
without `Continuous` are cleared right after that test. An `@AND` event needs
both of its triggers since the last turn start. A file without `@INITFLAG`
ignores flags, as the original does.

Other event support (#328) also follows the original:
- **SCENARIOLOADED** fires once when the scenario starts.
- **RECEIVEDTECHNOLOGY** is checked at every turn start.
- **DELAY** postpones an event's actions by that many turns.
- **TAKETECHNOLOGY** removes an advance (with `Collapse`, also the advances
  that depend on it).
- **CHANGETERRAIN** changes a rectangle of tiles and destroys cities and
  units there.
- **MAKEAGGRESSION** declares war.
- **CREATEUNIT** honours `Randomize`, `InCapital` and `homecity`.

The preflight still lists what is missing: bribery, negotiation rules, sounds,
and the few partial cases.

The scenario's folder supplies its Rules, Events and art; interface sheets the
folder lacks come from `Original`, then the install root. Files the decoders
cannot frame are rejected with a reason. Starting replaces a running game.

Engine checkpoints of a scenario game record the scenario's install-relative
path, size and fingerprint, and the seed. Loading one re-reads that file from
the chosen installation and refuses if it changed. This is **not** native
scenario import: the faithful target stays `classic-surface-032-v1`
([scenario-entry-contract.md](scenario-entry-contract.md), #197/#198).

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
map-edge wrapping moves, faithful SAV/scenario import (an experimental entry
exists, see above), multiple maps, native
SAV export, government, diplomacy, full city management and native victory
handling remain later work. A required unsupported government phase stops with
a diagnostic and can be saved; it is never silently answered. This release
does not complete the playable-client track or unlock the full original AI port.

The art is read from your install: first-slice terrain/units, generic city style
and dialog/panel textures. Coast joins, native civilization recoloring,
city-style selection and full native window/layout parity are not claimed.
Retain LICENSE and notices when redistributing these executables.
