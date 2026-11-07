# Playable browser and desktop clients

The playable clients support generated Original-profile surface games,
one human civilization and the existing built-in opponents. Both clients use
the same validated human session, presentation, owned artwork and checkpoints.

Run **totplay** (Windows: **totplay.exe**) for the local browser client, or
**totdesktop** (**totdesktop.exe**) for a standalone game window. The browser
launcher starts a local loopback host and opens your default browser. Use the
in-game Close client button to stop that host; closing a browser tab keeps the
game paused and available for reconnection. Desktop needs no browser/server.
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

## Controls

The supported map commands follow the original Test of Time manual and installed
menu; [reference and remaining differences](ui/original-controls.md).

- Click to inspect in Viewing mode. **V** toggles Moving/Viewing. Choose
  **Activate unit**, or press **A** at the inspection cursor, to enter Moving
  with that unit. A offers a chooser for a stack. The selected square blinks;
  the selected unit appears above other units in its stack.
- In Moving, click adjacent terrain to move, or use **7 8 9 / 4 6 / 1 2 3**
  on the keypad. The keypad works with Num Lock on or off. Cursor arrows also
  move in Moving; in Viewing they pan. Viewing keypad keys move only the
  inspection cursor. Enemy attacks and supported city occupation require
  confirmation; Confirm all moves also confirms empty-tile movement.
- **W** cycles movable units; **Space** skips the active unit's remaining moves.
  A selected exhausted unit reports that it has no moves left.
- **C** centers on the active unit. **Z / X** zoom in/out; **Shift+Z** restores
  standard zoom. The **+ / -** buttons and keys remain available as conveniences.
- **B** opens founding for an eligible settler. Type the name and accept;
  Escape cancels without consuming the settler. **Cities** cycles own cities.
- **Manage city** opens the selected own city's workspace. Click a radius tile
  to remove/assign its worker; arrows select a tile and **Space** changes it.
  Removing a worker creates an entertainer. Click a specialist to cycle
  entertainer → tax collector → scientist; tax/scientist roles need population
  five. Outputs/mood refresh immediately, without collecting income or ending
  the turn. **PageUp/PageDown** navigate your cities; **Escape** returns to the
  map and keeps accepted edits. **Ctrl+S / Ctrl+L** save/load inside the city too.
- **Auto workers**, or the centre tile, requests automatic allocation. An empty
  tile clicked without a spare specialist also requests automatic allocation,
  following the original transition. First remove a worker when you want to
  assign that citizen to a particular empty tile. Blocked tiles reject the edit.
- Choose production opens a picker: arrows or click select, Enter accepts,
  Escape cancels. Switching currently retains the shield box; native switching
  penalties are not implemented. Buying/selling and deeper city panels remain
  later city-management work.
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
