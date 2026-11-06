# Playable client: reference discovery

Status: second-pass reference inventory for #107, v0.11. Owner design approval
is pending. [Decision 0005](../decisions/0005-playable-client-planning.md) governs
this work. [Design proposal](client-design.md) and
[interactive layout](client-prototype.html) are separate from reference facts.

## Reference identity and method

On 2026-11-06 the main session inspected the reference UI with its own vision.
The executable is **ToTPP-patched `civ2.exe`**, not unpatched `civ2.bak`.
Record this as the installed reference UI; do not infer unpatched behavior or
patch equivalence from these observations. See the source hashes and capture
hashes in [ui-discovery.json](../evidence/ui-discovery.json).

Three private discovery sessions used the same copied fixture. The session copied both the install and Wine prefix into
`/tmp/totengine-ui-discovery/{install,prefix}`, redirected the copied prefix's
`d:` drive to the copied install, and ran on its own Xvfb `:99` at 1024×768×24.
No existing Wine/Xvfb process was present in the host check. The original install
and reference prefix were not run or edited. No RAM modifications or quiet
presentation patches were used. Initial clipping was a window/focus problem,
resolved by comparing Win32 and X backing-window geometry, setting the Win32
workspace and resizing its X backing window after focus restoration. Direct X
resize alone did not establish the internal layout; Win32 size alone left it
clipped. Thread-attached Win32 focus is needed before in-world input.

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

| ID  | Fixture and route                                                           | Observed layout / behavior                                                                                                                                                                                                                        | Limits                                                                                                                                                                                                                         |
| --- | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| U01 | Launch → single player                                                      | Launcher is a radio list with confirmation and exit.                                                                                                                                                                                              | Multiplayer, credits and configuration were not entered.                                                                                                                                                                       |
| U02 | Single player → game type                                                   | Separate list distinguishes original, extended, fantasy/science-fiction, scenario and saved-game entry.                                                                                                                                           | Fresh Original-game path is traced in U15; other branches remain untested.                                                                                                                                                     |
| U03 | Game type → load                                                            | Native file browser, remembered folder, SAV filter, filename, open/cancel. Loaded `Original/Ph_Auto2.SAV`.                                                                                                                                        | Scratch save/reload and overwrite refusal are traced in U18; corrupt files untested.                                                                                                                                           |
| U04 | Load summary → map                                                          | Menu strip; empire figures upper left, selected-unit details and mode across top, overview upper right, diamond map below. Larger window reveals the overview and more map.                                                                       | Initial window clipped at roughly 600×450. Map movement, scrolling and zoom behavior still need controlled measurements.                                                                                                       |
| U05 | Map → F1                                                                    | City report overlays the map. A row combines name, output symbols, build and progress. Double-clicking its city opens management.                                                                                                                 | One city in this fixture; sort and keyboard row behavior untested.                                                                                                                                                             |
| U06 | City report → double-click city                                             | Broad city workspace with worked terrain in center, resource totals left, build/progress/buy below, food storage lower right, citizens along bottom, support/improvements/happiness/info controls.                                                | First-pass limit; representative operations are now covered in U19–U22.                                                                                                                                                        |
| U07 | City → Build heading                                                        | Production chooser lists units, improvements and wonder together; turns and combat details appear. Separate military/domestic recommendation windows appear alongside it.                                                                         | No production change intentionally committed; Escape was followed by returning to the city. Shield-loss/obsolete-item behavior unmeasured.                                                                                     |
| U08 | Map → F6 → Goal                                                             | Advisor lists known advance(s); Goal opens a broad advance chooser with help and unit/city navigation controls.                                                                                                                                   | This is a **long-term goal chooser**, not proof of freely changing the current research project. Required initial project prompt is captured separately in U17; mid-project change is untested.                                |
| U09 | Map → Shift+T                                                               | Three rate sliders with locks, government rate cap, income/cost and discovery estimate. Fixture shows 40/60/0 and cap 60%.                                                                                                                        | Redistribution/locks/rounding not tested. No rate change attempted.                                                                                                                                                            |
| U10 | Map → Shift+R                                                               | Revolution question opened with Yes selected. Escape was sent; a later capture showed a revolution notification.                                                                                                                                  | Single sequence with focus transitions, not an isolated universal Escape rule. Do not describe Escape as cancellation; reload before other baseline-sensitive tests. The isolated follow-up and government chooser are in U31. |
| U11 | Standard map → F3                                                           | Explicit no-contact message.                                                                                                                                                                                                                      | No diplomatic information exists in this fixture.                                                                                                                                                                              |
| U12 | Load `FallRome/ROMFALL.scn` → F3                                            | Scenario unit/city names and December date appear in map panels. Foreign report shows contact name, mood, treaty, embassy information, power/reputation summary, intelligence and emissary actions.                                               | Loaded via file dialog as saved position, not scenario-start wizard. Contacts are fixture-dependent.                                                                                                                           |
| U13 | FallRome foreign report → first contact → emissary                          | Hostile contact refuses conversation in a modal message.                                                                                                                                                                                          | No treaty or negotiation state changed intentionally.                                                                                                                                                                          |
| U14 | FallRome foreign report → second contact → emissary                         | A separate full-screen diplomatic presentation opens, replacing the map workspace.                                                                                                                                                                | Visual presentation stalled in first pass. Plain-dialog negotiation and resulting Peace/Cordial report are covered in U25/U26.                                                                                                 |
| U15 | Original → new game → accept inspected selections                           | Complete standard path: size, difficulty, competition, barbarians, rules, gender, tribe, name, city style, introduction and first turn. Several choices retain earlier values.                                                                    | Custom/pre-made-world and customized-rules branches unentered. This generated world is unseeded and not claimed reproducible.                                                                                                  |
| U16 | Fresh game's selected settler → B                                           | City-name decision → founding notice → city workspace. Leaving that workspace was followed by turn advancement and the initial research prompt.                                                                                                   | Auto-end-turn option/trigger not isolated; naming cancellation and rejected names untested.                                                                                                                                    |
| U17 | Required first research prompt → choose Pottery                             | Eligible-project list, help, Goal and confirmation, with separate science recommendation. A saved/reloaded advisor shows Pottery with progress.                                                                                                   | Changing a project mid-research, skipping/canceling a required prompt and later discovery remain untested.                                                                                                                     |
| U18 | Ctrl+S → explicit scratch filename → repeat filename → No → Cancel → Ctrl+L | Save completion notice; overwrite Yes/No (Yes initially focused); No returns to save chooser. Reload summary retains the saved date; advisor retains selected project. Save folder initially remembered FallRome, even for the new standard game. | Only the copied install was written. Overwrite acceptance, corrupt files, decision-boundary persistence and matched next-turn execution untested.                                                                              |

## Second-pass routes

| ID  | Route                                                              | Inspected result / limit                                                                                                                                                                                                                                                                  |
| --- | ------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| U19 | FallRome Roma → worked tile → specialist → reassignment            | Removing a worked tile creates an entertainer and refreshes outputs/starvation feedback. Specialist clicks show tax collector then scientist; reassignment restores outputs. One context, not a complete yield formula.                                                                   |
| U20 | Roma → Buy → Never mind; reopen → Complete                         | Cancellation retains 5/100 shields. Quoted 641-gold purchase fills 100/100 with one turn remaining; pane refresh shows treasury 9359. No immediate unit creation observed.                                                                                                                |
| U21 | Improvements → granary → Never mind; Support / Happy               | Sale includes single/all-city scope; explicit cancellation retains building. Support has unit list/cost indicators; happiness panel separates stages/effects. Bulk-sale and unit activation from these lists remain untested.                                                             |
| U22 | City arrow → Rhegium → Build → Engineers → Apply                   | Chosen panel retained across city navigation. Build changes from Heavy Warship 5/150 to Engineers 5/40. One unit-category change only; category penalties not established.                                                                                                                |
| U23 | F2/F4/F5/F7/F8/F11 and defense Casualties                          | Defense counts/statistics, empty casualties, citizen rows, city revenue/maintenance, wonder status, ranked cities and demographics values/ranks. Report drill-downs/visibility not exhaustively tested.                                                                                   |
| U24 | F9 in standard/FallRome; F12 in early standard game                | Standard score components differ from scenario objectives/thresholds. Early standard space action gives an unavailable notice. Full spacecraft UI remains unobserved; FallRome F12 gave no visible report.                                                                                |
| U25 | Ctrl+O / Ctrl+P                                                    | Game preferences expose end-turn, mouse movement, advice and Enter-to-close-city settings. Unchecking optional Diplomacy Screen through UI enables readable plain dialogs. Copied game settings only.                                                                                     |
| U26 | Foreign report → neutral emissary, with visual diplomacy off       | Demands/refusals → peace offer/acceptance → topics → proposal → offered advance → back → finish. Foreign report shows Cordial/Peace. Intelligence report and return-to-contact path inspected. Gifts/tribute/third-party war and exact visibility eligibility remain open.                |
| U27 | Fantasy/Ya_Auto → Orders → Move To Another World                   | Destination chooser includes cancel; Undersea chosen and selected unit/map presentation changes. Terrain, game/date and order names differ. This is unit transfer, not only viewing a layer. Transfer rule completeness not claimed.                                                      |
| U28 | Load classic custom Constantine scenario → map/score               | Custom art, unit names, difficulty/currency/date labels and objective scoring. Scenario-start wizard/event decisions still need cases.                                                                                                                                                    |
| U29 | Attempt Warring States deiQin import                               | Specific missing Lua Scenario events extension diagnostic; scenario is not claimed loaded. Lua settings remain disabled; no patch profile change made.                                                                                                                                    |
| U30 | Constantine V → keypad 6; A → stack choice → keypad 6; W/Space/Z   | Viewing cursor steps (44,46)→(46,46). Activation chooser lists selected stack; chosen legion enters Moving mode. Directional order advances after exhaustion. Wait/skip change selection; zoom changes viewport scale. No general pixel→tile formula or complete scheduling/combat claim. |
| U31 | Reload small human save → Shift+R → focus Revolution → Escape only | Immediate Revolution! notice; rate view shows Anarchy. End-turn Enter reaches a government chooser (only Despotism legal), then rate editor and newspaper. Escape result is now isolated to this installed-reference input context; general vanilla equivalence/duration not inferred.    |
| U32 | Rate track and lock in U31 context                                 | Lock checkmark visible; unlocked tax-track input changes 40/60/0→50/50/0 and displayed estimates. One arrow input alone did not change displayed shares. Full slider/lock algorithm remains unproven.                                                                                     |
| U33 | Load authored short invalid text as SAV                            | Specific not-a-saved-game error; old map stays visible. Full prior-state preservation is not established merely by the picture.                                                                                                                                                           |

The corresponding [screen contracts](screen-contracts.md) describe proposed
input/decision ownership, fields, commands, return routes, persistence and known
limits. The [delivery breakdown](client-milestones.md) is a draft for owner
review, with no assigned implementation versions.

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

## Known gaps and delivery impact

The inventory now covers representative routes across every family requested
by #107. It is not exhaustive behavior recovery. Unknown branches must be
accepted/scoped in design review and investigated before their implementation
claims; this inventory does not silently waive them.

| Coverage               | Still unknown / next check                                                                                                                                                                                                                          |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Map/unit               | Mouse preference variants, edge/drag scrolling, path/go-to cancellation, combat/capture prompts, transport/air/teleport, exhausted/blocked orders and precise scheduling. One-key Viewing/Moving/activation/wait/skip/zoom routes are now observed. |
| City                   | Cross-category shield penalties, rejected/obsolete builds, bulk sale, unit activation/disband from city, disorder/growth event ordering, city sort semantics. Representative worker/specialist/buy/sale-cancel/navigation and panes are observed.   |
| Science                | Mid-project changes, later discovery/prompt cancellation, unavailable/goal prerequisites and retained progress. Initial project/goal distinction is observed.                                                                                       |
| Government             | Additional eligible governments, failed transitions, exact duration/caps/rounding and full lock redistribution. Early revolution/anarchy/choice/rate/notice sequence is observed.                                                                   |
| Diplomacy              | Gift/tribute/alliance/third-party-war/map-exchange branches, diplomatic-unit missions, exact intelligence visibility and interruptions/checkpoints. Representative demands, peace, knowledge exchange and nested returns are observed.              |
| Reports/help           | Supply/demand details, report drill-downs, casualty timeline, Civilopedia/media and late space construction/launch. Representative advisor/world report layouts and both score modes are observed.                                                  |
| Setup/persistence      | Custom/pre-made world and scenario-start wizard branches; missing assets; native overwrite acceptance and exact next-turn/required-choice resume. Standard setup, save/reload, overwrite refusal and invalid/extension diagnostics are observed.    |
| Scenario/multiple maps | Scenario event choices, cargo/teleport/restricted transfers and successful Lua import remain untested. Shipped Fantasy transfer and classic custom Constantine layouts are observed; Lua-disabled rejection is explicit.                            |
| Usability              | Keyboard-only full routes, text scaling/long names/localization, accessibility, focus/modal priority and sustained human play. Prototype browser tests do not establish these.                                                                      |

Owner review of layout, movement/city edit behavior, first delivery scope and
decomposition remains required by decision 0005. No release candidate or later
client version is assigned. The scope of remaining reference work must be
recorded in that reviewed plan rather than treated as implemented compatibility.
