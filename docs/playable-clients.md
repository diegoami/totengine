# Playable browser and desktop clients

This first playable slice supports generated Original-profile surface games,
one human civilization and the existing built-in opponents. Both clients use
the same validated human session, presentation, owned artwork and checkpoints.

Run **totplay** (Windows: **totplay.exe**) for the local browser client, or
**totdesktop** (**totdesktop.exe**) for a standalone game window. The browser
launcher starts a local loopback host and opens your default browser. Use the
in-game Close client button to stop that host; closing a browser tab keeps the
game paused and available for reconnection. Desktop needs no browser/server.
Linux desktop requires the system SDL2 runtime. Windows packages include it.

Enter your original Test of Time installation directory at setup, or set
`CIVTOT_HOME`. On Windows a typical path is `D:\games\CIVTOT`. Choose the seed,
map size, civilization count, human civilization and difficulty, then Start.
The installation must include the Original profile and its standard art.
Missing/incompatible resources produce an error without replacing your game.
No original artwork, Rules, saves or fonts are included. Desktop reads a system
DejaVu Sans font on Linux or Arial on Windows; `TOTENGINE_FONT` selects another
owned TTF if required.

## Controls

- Click to inspect a tile in Viewing mode. **M** toggles Moving. In Moving,
  click an adjacent tile to move the active unit; enemy attacks and supported
  city occupation require confirmation. Optional Confirm all moves also
  confirms empty-tile movement. Keyboard directions: **7 8 9 / 4 6 / 1 2 3**.
- **W** cycles movable units; **Space** skips the active unit's remaining moves.
  Inspect a stack and choose Activate unit to select a member.
- Arrow keys pan, **Home** centers, **+ / -** zoom. Center/+/- also have buttons.
- **F** opens founding for an eligible settler. Type the city name and accept;
  Escape cancels without consuming the settler. **C** cycles your cities.
- Choose production opens a picker: arrows or click select, Enter accepts,
  Escape cancels. Switching currently retains the shield box; native switching
  penalties are not implemented. Workers/specialists and buying/selling are
  reserved for later city-management work.
- **R** opens required research. Select explicitly and accept. Escape permits
  inspection while the choice still blocks gameplay; it never auto-selects.
- **E** ends the human turn, confirming if movable units remain. Opponents run
  in bounded steps and then return control. Pause/Continue controls that work.
  Loading a checkpoint during opponents requires explicit Continue.
- **S / L** save/load an engine checkpoint. Save pending choice preserves
  research and unfinished city naming. Tab/Shift-Tab and Enter navigate buttons;
  text fields support normal editing and clipboard shortcuts.

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
