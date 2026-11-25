# ToTEngine user guide

ToTEngine is a work-in-progress recreation of **Civilization II: Test of
Time** that runs on your own original installation. It ships none of the
original game's files; the art, rules text and Civilopedia you see come
from your ToT directory at runtime. This guide explains how to install,
start and play the game with today's clients, and what is implemented and
what is not.

For the rules of the game itself, use the in-game **Civilopedia** (from
your installation) and the original printed manual — MicroProse/Hasbro,
1999 ([reference copy](https://manuals.plus/m/0a903e975f22c7da36286b990be87bba1bd2c491a617c786f0280d99353c5fa8.pdf)).
This guide describes ToTEngine's own clients and features; it does not
reproduce the original manual.

Developer and contributor documentation lives alongside this file
([playable clients](playable-clients.md), the [controls
reference](ui/original-controls.md) and the [evidence
index](evidence/README.md)).

This guide carries two kinds of screenshots: the **original game's**
screens (the look-and-feel target, captured from a real installation
during the interface study) and **ToTEngine's current implementation**
of the same screens (captured from the real browser client on Linux,
Chromium, by [tools/manual_screenshots.cjs](../tools/manual_screenshots.cjs);
the desktop client shows the same screens in its own window).

## The look and feel: the original game

ToTEngine recreates **Civilization II: Test of Time**, and its look and
feel target is the original game. These are screenshots of the original,
captured from a real installation during the November 2025 interface
study — this is the game you know, and the standard the clients aim at.

The original's main menu and its map screen after founding a city:

![The original Test of Time main menu with the comet artwork and the single-player, multiplayer, hall of fame, credits and ToTPP entries](screenshots/original/launch.png)

![The original map screen: stone-textured interface, the Game/Kingdom/View/Orders/Advisors/World/Cheat/Map/Civilopedia menu bar, the population/date/treasury panel, the End of Turn prompt and the circular minimap, with Madrid founded on the river](screenshots/original/map.png)

The original's city window and one of its Civilopedia research pages:

![The original city window for Madrid: City Resources, the Build box with Warriors at 4/10 and the Buy price, the citizen-occupied radius map, the Info/Support/Improvements/Happy buttons and the Food Storage box](screenshots/original/city.png)

![The original Civilopedia research page for Pottery: prerequisites, what it allows with Map Making, the Granary and Hanging Gardens, with Go Back, Tree, Description and Close](screenshots/original/research.png)

The engine screenshots that follow show **ToTEngine's current
implementation** of the same flow. Where a screen still looks different,
that difference is the remaining work, not the intent.

## Requirements

- Your original **Test of Time** installation with the Original profile
  and its standard art (a typical Windows path is `D:\games\CIVTOT`).
  The installation is never modified; missing or incompatible resources
  produce an error instead of touching your game.
- **Windows**: Windows 10 or later, x86-64. SDL2 is included in the
  package.
- **Linux**: x86-64, glibc 2.34 or later, plus your system's SDL2
  runtime for the desktop client.
- No original artwork, Rules, saves or fonts are included with the
  packages. The desktop client reads a system font (DejaVu Sans on
  Linux, Arial on Windows); `TOTENGINE_FONT` can point at another
  font you own.

Without a ToT installation you can still run the small, independently
authored **Open demo** profile, optionally with the separately licensed
[Isotrident artwork pack](ui/freeciv-assessment.md). The demo is a tiny
three-unit profile — it is not full Test of Time balance.

## Starting a game

Two programs are shipped:

- **totplay** — starts a local host and opens the game in your web
  browser. Close it from **Game > Close client** in the menu; closing a
  browser tab alone keeps the session available for reconnecting.
- **totdesktop** — a standalone game window; no browser or server is
  involved.

On the setup screen, enter your ToT installation directory (or set the
`CIVTOT_HOME` environment variable), then choose the seed, map size,
number of civilizations, your civilization and the difficulty, and
press **Start**.

- **Quick Start — random map** starts immediately with the current setup
  and a fresh random seed.
- The **seed** field is what makes a world reproducible: the same seed
  with the same setup generates the same map. The seed is deliberately
  not remembered between launches; type it back to replay a world.
- Setup remembers your last successful settings (in a `setup.json` next
  to your saves) and prefills them next time. Delete that file to reset.
- **Enhancements…** on setup opens the optional settings described
  below; everything there is off by default.

Both clients scale the 1024×768 picture to any window size. Full screen
is **Alt+Enter** or **View > Full screen**.

![ToTEngine today — the setup screen: installation directory, seed, civilizations, human seat, difficulty and map size, with Quick Start, checkpoint loading and the experimental scenario entry](screenshots/setup.png)

## The map screen

The map screen keeps the original arrangement: a **menu bar** along the
top, the map in the middle and a **right column** with the minimap, your
turn, government, treasury, rates, research, the selected unit and city,
and the end-of-turn prompt. On wide (16:9-and-wider) windows a second
column beside the right column shows the latest messages in full.

Every map command is in the menu bar with its key: **Game** (save, load,
new game, Settings, Keys, Close client), **Kingdom** (rates, revolution,
Cities), **View** (Moving/Viewing, center, zoom, full screen), **Orders**
(found city, wait, skip, activate, end turn), **Advisors** (reports and
science), **World** (world reports, Messages) and **Civilopedia**. An
entry that cannot run right now is greyed out and says why. **F10** opens
the menu bar from the keyboard.

![ToTEngine today — the map screen after founding a city: menu bar, the map with your settler and city, and the right column with minimap, turn, treasury, rates, research, the city panel and the end-of-turn prompt](screenshots/map.png)

On wide windows the messages join as their own column:

![ToTEngine today — the same map at a wide window size, with the messages column between the map and the right column](screenshots/map-wide.png)

### Units: selecting and moving

- **V** switches between **Moving** and **Viewing**, exactly as in the
  original. Clicking one of your ready units selects it and enters
  Moving in either mode; the selected unit's sprite blinks.
- Move with the numeric keypad (**7 8 9 / 4 6 / 1 2 3**, Num Lock on or
  off) or the cursor arrows. Clicking adjacent terrain also moves.
  Enemy attacks and city occupation ask for confirmation before they
  resolve.
- **A** activates the inspected unit or opens a chooser for a stack.
  **W** waits (cycles to your other ready units), **Space** skips the
  active unit's remaining moves, **C** centers the map on it.
- When your units have all moved, the prompt offers **End turn**
  (**Ctrl+N**). Ending the turn is always explicit; your opponents then
  play in bounded steps and control returns to you.
- **Z / X** zoom out/in, **Shift+Z** restores the standard zoom.

### Cities

- **Double-click** one of your cities (or its garrison) — or use
  **Manage city** in the right column — to open the city window in the
  classic arrangement: citizens strip, resources, food storage and
  production, the radius map, and the information panels.
- Click a radius tile to move a citizen there or off it; click a
  specialist to cycle entertainer → tax collector → scientist. Changes
  apply immediately. **PageUp/PageDown** move between your cities,
  **Escape** returns to the map with your edits kept.
- **Choose production** opens the item picker; **Buy production** quotes
  the gold cost and asks for confirmation; **Improvements** lists your
  buildings and sells one per city per turn with its refund shown first.
- **Info / Support / Improvements / Happy** panels show who is supported
  here, what is built, and the mood. The **Sort** button reorders these
  lists; it changes only the display.

![ToTEngine today — the city window: citizens strip, city resources, food storage and production on the left, the radius map in the middle, and the information panels on the right](screenshots/city-window.png)

### Your civilization

- **Research** (Advisors menu) opens the required research choice when a
  project completes — selecting explicitly is required, as in the
  original. **F6 Science** shows the running project, lets you set a
  long-term goal, and lists the advances you know.
- **Shift+T** opens the tax/science rates dialog with the government's
  limits; **Shift+R** starts a revolution. The government chooser and
  the rate review afterwards are required dialogs — they cannot be
  dismissed without an answer.
- **F1, F2, F4, F5 and F7** open your civilization's reports (city
  status, defense, attitude/mood, trade, wonders). They are read-only.
- **Civilopedia** shows your installation's own reference text by
  category. It needs a ToT installation; the Open demo has no
  Civilopedia text.

![ToTEngine today — the F6 science advisor: the running project, the goal setter and the available and known advances](screenshots/science.png)

![ToTEngine today — the Shift+T rates dialog with the government's per-share limit, locks, and the treasury and science effect preview](screenshots/rates.png)

![ToTEngine today — the F1 city status report with the report switcher, one row per city and sorting](screenshots/report-cities.png)

### Messages and help

- **?** (or **Game > Keys…**) shows the keyboard help with every
  supported original key and the ToTEngine additions.
- **Messages** (World menu) lists every notice of the session, newest
  first. Messages are display-only; they are not part of saved games.
- The **mouse wheel** scrolls any list under the pointer.

## Saving and loading

- **Ctrl+S / Ctrl+L** save and load engine checkpoints, in the city
  window too. Names are 1–64 letters, digits, underscores or hyphens;
  saving never overwrites an existing slot.
- Checkpoints are stored in `%LOCALAPPDATA%\totengine\saves` on Windows
  and `~/.local/share/totengine/saves` on Linux (override with
  `TOTENGINE_SAVES`). Both clients can share one folder.
- The `.cth` format is ToTEngine's own replay checkpoint, not an
  original SAV. It is version-bound: a checkpoint loads with the engine
  version that saved it, and it verifies your installation's resources
  on load. There is no autosave — save before closing.

![ToTEngine today — the save dialog with its name field and accept/cancel entries](screenshots/save-dialog.png)

## Optional enhancements

**Game > Settings** (or **Enhancements…** on setup) holds optional
conveniences that differ from the original. All are off by default and
shared by both clients (`enhancements.json` beside the saves; delete the
file to reset). Currently offered: a new-messages overlay on the map,
confirm-every-move, map tooltips, goal estimates in the science advisor,
an order strip under the map, a message log in the right column, and a
path preview with turn counts while moving. None of them change rules,
checkpoints or recordings.

## Experimental scenario play

**Open scenario (experimental)** on setup — or **Game > Open scenario…**
during a game — lists the `.scn` and `.sav` files in your installation
and shows a **preflight** before anything starts: the turn, map size,
playable civilizations, and every gap the engine knows about. Nothing
starts until you accept it.

![ToTEngine today — the experimental scenario listing over setup: the installation's .scn and .sav files, paged, with the gaps-disclosed-before-starting note](screenshots/scenario-preflight.png)

What works: the scenario's own Rules, Events and art; event text boxes,
flags and a growing set of event actions; dates as the original writes
them. A plain single-player save resumes its saved turn with spent
movement kept. What the preflight will tell you about: opponents are
played by ToTEngine's own AI rather than the original's; some event
actions are missing; Lua scripts are not run; in multi-map scenarios
only the first map is played. Starting a scenario replaces a running
game, and its checkpoints re-verify the scenario file on load.

This is an experimental route. Faithful scenario import remains future
work; the preflight always names the gaps before you commit.

## What is not implemented yet

The recreation is in progress, and several original systems are still
missing or partial:

- Sea, air and carrying units; map-edge wrapping moves.
- Faithful import of original saves and scenarios (the experimental
  entry above is not that), multiple maps, native SAV export.
- Capital relocation, civil war, technology plunder, civilization
  respawn; full diplomacy and negotiations.
- Some unit orders (fortify **F**, roads **R**, sleep) and fuller
  original workflows for research and city screens.
- Native victory handling; the original AI's plans for opponents.

A required but unsupported phase stops with a clear message and can
still be saved; it is never silently answered.

## Troubleshooting

- **Setup rejects my installation** — check that the directory contains
  the Original profile and its art; the error names what is missing.
- **The browser tab was closed** — start totplay again; it reconnects to
  the running session. Use **Game > Close client** to actually stop it.
- **A checkpoint will not load** — checkpoints are version-bound; load
  it with the engine version that saved it. Invalid loads leave your
  current game untouched.
- **Fonts look wrong on the desktop** — set `TOTENGINE_FONT` to a font
  file you own.

## Where this guide ends

This guide covers ToTEngine's clients. For the original game's rules,
strategies and background, use your installation's Civilopedia and the
original manual; for what ToTEngine implements and how it was verified,
the developer documentation linked at the top is the source of truth.
