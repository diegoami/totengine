# Playable client: reference discovery

Status: first pass for #107, v0.11; discovery incomplete. Owner design approval
is pending. [Decision 0005](../decisions/0005-playable-client-planning.md) governs
this work. [Design proposal](client-design.md) and
[interactive layout](client-prototype.html) are separate from reference facts.

## Reference identity and method

On 2026-11-06 the main session inspected the reference UI with its own vision.
The executable is **ToTPP-patched `civ2.exe`**, not unpatched `civ2.bak`.
Record this as the installed reference UI; do not infer unpatched behavior or
patch equivalence from these observations. See the source hashes and capture
hashes in [ui-discovery.json](../evidence/ui-discovery.json).

Two private discovery sessions used the same copied fixture. The session copied both the install and Wine prefix into
`/tmp/totengine-ui-discovery/{install,prefix}`, redirected the copied prefix's
`d:` drive to the copied install, and ran on its own Xvfb `:99` at 1024×768×24.
No existing Wine/Xvfb process was present in the host check. The original install
and reference prefix were not run or edited. No RAM modifications or quiet
presentation patches were used. Initial clipping was a window/focus problem,
resolved with `ShowWindow(SW_MAXIMIZE)` and thread-attached Win32 focus. A direct
X resize did not establish a usable DirectDraw viewport.

Captures used `tools/oracle/xtool shot` and `ppm2png.py`, followed by visual
inspection. Native dialogs require explicit focus; returning to the main window
while another modal exists can create misleading overlapping workflows. Keys
sent without verified focus are not evidence that an action succeeded.

**Original screenshots remain outside Git**, under the private capture directory.
The repository contains only our descriptions, source/capture hashes and our
own fixture drawings. Screenshots are not standalone durable reproductions;
the following route table is the reinspection procedure. No original artwork,
menu asset, scenario or save is redistributed.

## Inspected routes

| ID  | Fixture and route                                                           | Observed layout / behavior                                                                                                                                                                                                                        | Limits                                                                                                                                                                                                          |
| --- | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| U01 | Launch → single player                                                      | Launcher is a radio list with confirmation and exit.                                                                                                                                                                                              | Multiplayer, credits and configuration were not entered.                                                                                                                                                        |
| U02 | Single player → game type                                                   | Separate list distinguishes original, extended, fantasy/science-fiction, scenario and saved-game entry.                                                                                                                                           | Fresh Original-game path is traced in U15; other branches remain untested.                                                                                                                                      |
| U03 | Game type → load                                                            | Native file browser, remembered folder, SAV filter, filename, open/cancel. Loaded `Original/Ph_Auto2.SAV`.                                                                                                                                        | Scratch save/reload and overwrite refusal are traced in U18; corrupt files untested.                                                                                                                            |
| U04 | Load summary → map                                                          | Menu strip; empire figures upper left, selected-unit details and mode across top, overview upper right, diamond map below. Larger window reveals the overview and more map.                                                                       | Initial window clipped at roughly 600×450. Map movement, scrolling and zoom behavior still need controlled measurements.                                                                                        |
| U05 | Map → F1                                                                    | City report overlays the map. A row combines name, output symbols, build and progress. Double-clicking its city opens management.                                                                                                                 | One city in this fixture; sort and keyboard row behavior untested.                                                                                                                                              |
| U06 | City report → double-click city                                             | Broad city workspace with worked terrain in center, resource totals left, build/progress/buy below, food storage lower right, citizens along bottom, support/improvements/happiness/info controls.                                                | Worker reassignment, specialist cycling, buying, selling and city-to-city navigation not exercised.                                                                                                             |
| U07 | City → Build heading                                                        | Production chooser lists units, improvements and wonder together; turns and combat details appear. Separate military/domestic recommendation windows appear alongside it.                                                                         | No production change intentionally committed; Escape was followed by returning to the city. Shield-loss/obsolete-item behavior unmeasured.                                                                      |
| U08 | Map → F6 → Goal                                                             | Advisor lists known advance(s); Goal opens a broad advance chooser with help and unit/city navigation controls.                                                                                                                                   | This is a **long-term goal chooser**, not proof of freely changing the current research project. Required initial project prompt is captured separately in U17; mid-project change is untested.                 |
| U09 | Map → Shift+T                                                               | Three rate sliders with locks, government rate cap, income/cost and discovery estimate. Fixture shows 40/60/0 and cap 60%.                                                                                                                        | Redistribution/locks/rounding not tested. No rate change attempted.                                                                                                                                             |
| U10 | Map → Shift+R                                                               | Revolution question opened with Yes selected. Escape was sent; a later capture showed a revolution notification.                                                                                                                                  | Single sequence with focus transitions, not an isolated universal Escape rule. Do not describe Escape as cancellation; reload before other baseline-sensitive tests. Government chooser after anarchy untested. |
| U11 | Standard map → F3                                                           | Explicit no-contact message.                                                                                                                                                                                                                      | No diplomatic information exists in this fixture.                                                                                                                                                               |
| U12 | Load `FallRome/ROMFALL.scn` → F3                                            | Scenario unit/city names and December date appear in map panels. Foreign report shows contact name, mood, treaty, embassy information, power/reputation summary, intelligence and emissary actions.                                               | Loaded via file dialog as saved position, not scenario-start wizard. Contacts are fixture-dependent.                                                                                                            |
| U13 | FallRome foreign report → first contact → emissary                          | Hostile contact refuses conversation in a modal message.                                                                                                                                                                                          | No treaty or negotiation state changed intentionally.                                                                                                                                                           |
| U14 | FallRome foreign report → second contact → emissary                         | A separate full-screen diplomatic presentation opens, replacing the map workspace.                                                                                                                                                                | The presentation did not advance to readable negotiation choices during this pass. Neither options nor successful treaties verified.                                                                            |
| U15 | Original → new game → accept inspected selections                           | Complete standard path: size, difficulty, competition, barbarians, rules, gender, tribe, name, city style, introduction and first turn. Several choices retain earlier values.                                                                    | Custom/pre-made-world and customized-rules branches unentered. This generated world is unseeded and not claimed reproducible.                                                                                   |
| U16 | Fresh game's selected settler → B                                           | City-name decision → founding notice → city workspace. Leaving that workspace was followed by turn advancement and the initial research prompt.                                                                                                   | Auto-end-turn option/trigger not isolated; naming cancellation and rejected names untested.                                                                                                                     |
| U17 | Required first research prompt → choose Pottery                             | Eligible-project list, help, Goal and confirmation, with separate science recommendation. A saved/reloaded advisor shows Pottery with progress.                                                                                                   | Changing a project mid-research, skipping/canceling a required prompt and later discovery remain untested.                                                                                                      |
| U18 | Ctrl+S → explicit scratch filename → repeat filename → No → Cancel → Ctrl+L | Save completion notice; overwrite Yes/No (Yes initially focused); No returns to save chooser. Reload summary retains the saved date; advisor retains selected project. Save folder initially remembered FallRome, even for the new standard game. | Only the copied install was written. Overwrite acceptance, corrupt files, decision-boundary persistence and matched next-turn execution untested.                                                               |

Capture filenames for each route and observations are in the JSON artifact.
The U10 notification was explicitly focused/acknowledged before the successful
FallRome load. Earlier failed text-entry/load attempts are not counted as
successful loads. The successful file load used a scratch Win32 helper to set
the filename edit and activate Open; it did not patch game state. The private
Wine server was stopped by its copied prefix, and the recorded Xvfb PID was
stopped after discovery. Copies/captures remain private scratch.

## Menu evidence is an inventory, not an interaction test

The installed `Menu.txt` was inspected as a local source. It declares these
workflow families, in addition to those entered above:

- Game preferences, graphics and city-report options; save/load; session exit.
- View/move mode, zoom presets, grid, view centering and window layouts.
- Settler jobs, fortification/sleep, waiting/skipping, disband/home, cargo,
  go-to, airlift/paradrop, teleport and ending the player turn.
- Defense, foreign, attitude, trade and science advisors; casualty timeline.
- Wonder, city ranking, score, demographic and spaceship reports.
- Civilopedia categories and scenario/map editing tools.

Declared shortcuts include V for view/move, B for founding, W for wait, Space
for skip, Ctrl+N for end player turn, F1/F3/F6 for city/foreign/science,
Shift+T for rates, Shift+R for revolution and Ctrl+S/Ctrl+L for save/load.
A declaration is not proof of availability, mode-specific behavior or dispatch.
Some keys are overloaded by unit role (for example F and P). Do not implement
one unconditional shortcut table from the menu text.

## Discovery still required before closing #107

| Coverage                 | Next experiment / acceptance                                                                                                                                                                                                                                                                                                  |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Map and unit control     | Record one input at a time in both modes: mouse selection, stack selection, drag/edge/keyboard scrolling, zoom, go-to, directional move, attack/capture, wait/skip, exhausted/fortified units, blocked/rejected orders, end-turn prompts. Read cursor/view state if needed; do not infer map coordinates from blind clicking. |
| City decisions           | Worker/specialist reassignment, support/unit activation, buy and sell confirmation/cancel, change production with invested shields, legal/obsolete items, disorder/growth and multiple-city navigation.                                                                                                                       |
| Science                  | Initial project choice is now observed (U17). Next trigger a later discovery, test mid-project changes/cancellation, unavailable advances and progress retention.                                                                                                                                                             |
| Government/economy       | Test rate locks/cap/redistribution and real cancellation; isolate the revolution key sequence, then observe government selection and anarchy status. Use disposable fixture reloads.                                                                                                                                          |
| Diplomacy                | Reach readable negotiation choices in a known-contact fixture; inventory treaty, tribute, tech, refusal, embassy/intelligence, declaration of war and return-to-map paths. Record presentation/asset failures separately.                                                                                                     |
| Reports                  | Enter every report family in menu inventory, not just city/science/foreign. Trace report-to-city/unit/detail navigation and hidden-information limits.                                                                                                                                                                        |
| Setup and persistence    | Fresh standard setup and scratch save/reload are observed (U15/U18). Next inspect custom/pre-made and scenario-start wizards, overwrite acceptance, unsupported/corrupt-file errors, missing assets and exact next-turn resume.                                                                                               |
| Scenario / multiple maps | At least one additional multi-map shipped game plus a custom scenario. Inspect map switching, cargo/teleport, custom names/art/text and event decisions. FallRome alone does not cover this.                                                                                                                                  |
| Input/usability          | Keyboard-only traversal, focus restoration, screen-size scaling, text legibility, modal priority and interrupt/notification handling.                                                                                                                                                                                         |

This is a useful first pass, not screen-by-screen completeness or a release
candidate. No implementation milestone/version is fixed from it yet.
