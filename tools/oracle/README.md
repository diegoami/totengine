# Oracle rig (T0-10)

Headless Wine + Xvfb harness that drives the original game and harvests saves.

## Pieces

| File | What |
|---|---|
| `xtool.c` | X11 automation: XTest key/button injection, `focus`/`focusptr` (no WM on Xvfb — set input focus explicitly or make it follow the pointer), `shot` (PPM, palette-aware for 8-bit screens), `stats` (state fingerprint), `windows`. Build: `gcc -O2 -o xtool xtool.c -lX11 /usr/lib/x86_64-linux-gnu/libXtst.so.6` |
| `winject.c` | Win32-level injector (SendInput/SetCursorPos + `post` WM_LBUTTONDOWN + **`focus`** + `seq` key-repeat for unit-skip loops). Build with llvm-mingw: `clang --target=x86_64-pc-windows-gnu -O2 -o winject.exe winject.c -luser32`. Run inside the game prefix: `wine winject.exe key 13`. **X-level keys never reach the game's DirectDraw surface; SendInput does.** `focus <hwnd-hex>` (thread-attach + SetForegroundWindow/SetActiveWindow/SetFocus) is REQUIRED before in-world keys when the badge shows no foreground — without it SendInput is silently discarded (post-reboot rig, 2026-11-02 night). |
| `ppm_bands.py` / `ppm_text.py` | Read the screen without eyes: bright-row detection and ASCII rendering of PPM regions. |
| `ppm2png.py` | PPM→PNG so **vision-capable models can read the screen** (the session model hands `/tmp/*.png` to a vision subagent — dialog items, selection state and coordinates come back literally; this replaced ASCII squinting entirely). |
| `memspool.py` | **Live memory access**: daemon that spawns the game as its own descendant (yama ptrace_scope=1 compliant), then serves `pids/maps/read/write/scan/snapshot/diffsnap` over a unix socket (`write` = process_vm_writev, for patch-and-prove seed experiments). Cheat-Engine-grade RE with zero installs. |
| `memspool_ctrl.sh` | start/stop/restart for the memspool daemon itself (the pkill trap applies to daemon management too — keep it in a script). |
| `newgame.sh` | Drive a fresh STANDARD game through the wizard (params: world-size/difficulty/competition Downs). |
| `restart_game.sh` / `stop_game.sh` | All game process management lives in scripts — never put a pkill pattern in an interactive command line (the self-kill trap struck four times; `restart_game.sh` also forces DISPLAY=:99 — a caller's :0 once leaked in and the game opened windows on the wrong screen). |
| `build_cd.py` | Builds a fake-CD ISO of the install (needs `PYTHONPATH` to a pip-less pycdlib install). |
| `turnloop.sh` | The acceptance loop: ends N turns and snapshots the autosave each turn. |
| `stdloop.sh` | Standard-game turn loop: arrow-moves settlers to exhaust them, Return×2 ends the turn, drains dialogs, harvests the **rotating autosave** (Ph_Auto/Ph_Auto2 — pick by mtime). The battle-tested way to bank per-turn saves. |
| `drain_dialogs.sh` | Dismiss ANY visible titled window not on the allowlist (Defense/Domestic advisors, Civ Tutorial, Game-saved toasts, emissary spam all block turn end AND Ctrl+S). |
| `saveturn.sh` / `cityloop.sh` | Ctrl+S save + turn-advance with turn-counter verification. CAUTION: the save dialog remembers the last-used folder (may be FallRome!) and name collisions stack overwrite prompts that wedge the game — prefer autosave harvesting. |
| `lab_std.sh` | Fresh standard game under the memspool daemon (wizard flow from newgame.sh, RAM access for snapshots). |
| `rngslot.py` | `get` / `set <v>` the live RNG state at VA `0x5dd53c` (memspool-backed). Seeding it to 1 and reading `0x0438e694` later is the T1-6 runtime proof. **`--crt`** addresses the MSVC `rand()` state at `0x5e2590` instead. **Combat rolls come from that one**, so seed `--crt` to force a known battle. |
| `unitram.py` | RAM-level unit driver: `bases`, `unit <idx>`, `near x y [r]`, `state` (cursor/active/both RNGs), `ready`/`select <idx>` (make a unit active without clicking), `frame ax ay dx dy` (poll live combat A/D/rolls). Bases are read from the live `.text` immediates (ToTPP relocates the tables). |
| `seeded_start.sh` | **Reproducible new game** (RG-2): runs the wizard, then while City Style is open pins NR, CRT, the 5 world parameters (`0x5ceaa8`) and the map seed (patch at `0x56fa6c`). The same `NR`/`CRT`/`PARAMS`/`MAPSEED` give the same turn-1 autosave. Harvests every save newer than a run-start marker, plus `placement.txt`. It leaves the install as found: backs up every `*.SAV` first, restores them on exit, and removes saves the run created. |
| `randhook.py` | Call-site logger for CRT `rand` (cave in the dead startup function `0x41562b`, buffer above `0x607c50`). Two runs diffed give the first divergent draw. |
| `launcher_pick.py` | Select a launcher game-type item by reading the red radio button (the list remembers the last choice and wraps). Used by `seeded_start.sh`, `newgame.sh` and `lab_std.sh` via `GAME` (default 0 = Original). |
| `load_save.sh` | Start the game under memspool and load a save by its install-relative path (`load_save.sh original/ph_auto2.sav`): picks "Load a Saved Game", clicks the filename field, types `d:\…`. |
| `hut_entry.py` | Controlled hut entry: pins CRT at entry, separates the three identified presentation callers, captures outcome/RNG/mutations and restores hooks. Fresh load per case; command examples below. |
| `corrhook.py` | `on`/`off`/`dump`: log calls of the corruption function `0x4c35a2` (city, amount, waste flag, capital distance). Same cave as randhook — use one at a time. |

## Navigation: what works and what does not (2026-11-03)

**Update (late night): units are driven through RAM, no map navigation
needed.** `unitram.py` (table above) reads the live unit array, cursor,
active unit and both RNG slots, and `select <idx>` makes any unit active.
It clears its orders, the fortified attr bit `0x0100` and moves spent, then
presses `W` until the game's active-unit field shows it. After that a
numpad key (winject VK `0x61..0x69`) moves or attacks in a known direction
(NE=9, SE=3, N=8, ...). `unitram.py frame` polls the combat function's stack
frame during the attack animation (A, D, firepower, rolls). Writing the
active-unit field yourself does **not** select a unit; the game keeps its
own copy and only writes this field out. The view struct is at `0x61fb34`
(evidence §T1-6 combat) if you ever do need pixels.

The 2026-11-03 notes below still describe the click/keyboard behaviour
correctly:

Getting the cursor to a *known map tile* is the rig's weakest point. Measured facts:

- **Clicks are not a usable coordinate input.** Four calibration clicks
  (viewport centre ±192/±120 px, reading the resulting `Loc:` each time) gave
  no consistent linear pixel→tile map, so the assumption "click recentres the
  view and `tile = f(pixel − centre)`" is **false** in this build. Do not build
  a click-arithmetic navigator until the view origin is read from RAM instead.
- **Arrow keys do not give a clean basis either.** 32×Up + 10×Right from
  `(44,70)` landed on `(62,12)`, i.e. not the expected `(22,28)` offset —
  sign/step per key still unmeasured. If you need this, measure ONE key with a
  single press and a single `Loc:` read.
- **`Loc:` only renders in Viewing Pieces mode.** In Moving Units mode the
  panel shows the active unit instead, so you cannot read the cursor.
- **Find City does not recentre the map.** The row is selected correctly
  (click the ROW, then OK — a highlight alone is not a selection) and the view
  stays where it was. It is a lookup, not a jump.
- **The Find City listbox only does SINGLE-LETTER prefix jumps.** Typing
  "MEDIOLANUM" lands on the first `M` entry (the last key's prefix), never on
  the full string. Multi-character type-ahead does not exist in this control —
  use one letter, then Up/Down, then verify by vision. (This — not fog and not
  key timing — was the long-running "Find City is broken" mystery.)
- **`W` (wait) and `Enter` do nothing when no unit needs orders.** Every map
  click while a unit is active issues that unit a goto order, so a session of
  blind clicking quietly consumes the whole turn's order budget and leaves the
  game in Viewing Pieces with no activatable unit. Symptom: `W`/`Enter` do
  nothing and arrow keys consume no randomness. Recovery is a fresh save or a
  scenario restart, not more clicking.

## Launch recipe (what finally worked)

1. `Xvfb :99 -screen 0 1024x768x24` (ToTPP needs high color; classic civ2.bak wanted 8-bit but see below).
2. `WINEPREFIX=~/.wine_civ2tot wine civ2.exe` (the **ToTPP exe** — its NoCD patch matters, see findings).
3. Launcher: it's a **listbox — double-click items** (single clicks only select). Row 2 = scenario browser → pick scenario → leader/difficulty/gender/name dialogs accept plain RETURN (xtool focus + Return).
4. In-world: **winject** keys only (`key 32`=skip unit ×3, `key 13`=end turn). Autosave lands in `<scenario dir>/<XX>_Auto.SAV`; each turn rewrites it.
5. `turnloop.sh N <scen-dir> <Auto.SAV> <outdir>` → consecutive same-session saves.

## Findings (2026-11-02)

### Standard-game navigation attempt (RG-1, evening session)

Goal: reach a *new game without scenario* for setup-section diffs. Findings:

- The **launcher overlay** ("The Test of Time", 287×245 at +369+484) sits ON TOP
  of the main window; the main title screen's own art occupies x≈56–330 —
  don't confuse the two (cost an hour). The launcher's items are the bands at
  x≈372–654, y≈495–712 (~30 px apart, ~7 rows).
- **Keyboard works on the launcher; injected clicks do not** (neither winject
  SendInput `dblclickat` nor XTest `xtool clickat` activated an item, with or
  without focus/PointerRoot). `xtool focus <hwnd>` + `xtool key Return`
  reliably activates the focused/default item.
- **Return on the launcher default opens a 291×344 game-type dialog**
  (at +367+385) whose first item is highlighted — and whose default led to the
  **Midgard Scenario browser** (ToTPP remembers the last-used scenario; NOT in
  TOTPP.ini — feature flags only there). Down+Return from the launcher default
  selected the scenario flow; the standard game needs a different item in the
  game-type dialog (arrow keys + Return; unread as of this session).
- Wizard chain once a game type is chosen (Midgard path): scenario browser →
  (load) → **Select Difficulty Level** (352×278 at +336+451) → **Select Gender**
  (352×146 at +336+583; neither Return nor Escape advanced it — needs a click
  or arrow+Return). Escape backs out one dialog per press, all the way to the
  title screen.
- Wine window coordinates == X coordinates (desktop 1024×768); `winject wins`
  lists HWNDs with rects — use it to locate dialogs instead of guessing from
  screenshots.
- **The pkill trap claimed another victim**: `pkill -f '[c]iv2'` in a command
  that also contains the literal `civ2` (e.g. a later `wine civ2.exe` or even
  `pgrep civ2`) kills the own shell. The kill must live in a command line that
  contains the bracketed form ONLY.

### Older findings (2026-11-02 morning)

- **Classic `civ2.bak` cannot run headless**: after its "Please Note" dialog it
  exits — the ToT CD check wants the mixed-mode CD's *audio tracks* (MCI
  cdaudio: the exe contains "Failed to open CDAUDIO device"; no GetVolumeInformation
  calls at all in a +relay trace). A full data ISO + CD-ROM-typed drive does not
  satisfy it. The ToTPP exe's NoCD patch is the practical path (and it writes
  plain **0x32** saves for non-Lua scenarios — verified on Midgard).
- ToTPP launcher input quirks: single-click = select (no window change — this
  cost hours), double-click = activate; RETURN on the launcher opens the
  TOTPP Configuration dialog (its default), not Launch.
- No WM ⇒ `XSetInputFocus` before any X-level key; game DirectDraw windows
  reject direct focus — use `focusptr` (PointerRoot) or winject.
- Never put `pkill -f <pattern>` and a literal matching string in the same
  shell command — it kills its own shell (use `[p]attern` brackets and keep
  the pattern out of the same command line).

## Rules-column oracle channel (no in-world input needed)

To prove which Rules.txt column drives an in-game value: back up the
install's `Rules.txt` (+ `Original/Rules.txt`) with a sha256 record, change
only the candidate field, start a fresh standard game through the launcher
(XTest works on dialogs), read the unit/status panel ("Movers N" gives the
movement rate), then restore both files and `sha256sum -c` the record. This
is how T1-3 proved `@UNITS` field 3 = movement rate without touching the
map surface. Fresh-game setup also re-reads Rules.txt, so no restart tricks
are needed.

## Proven run

`turnloop.sh 6 Midgard Oc_Auto.SAV …` → turns 4→9, six consecutive saves;
`totctl diff turn01 turn02` → "size delta +92 bytes = +1 city record (ToT)"
(Midgard's events founded a city between turns), 64,202 merged-region span
/ 52,073 exactly-changed bytes in
1,873 regions — the fog-of-war map block dominates, as expected.


## Controlled hut-entry parity (#79)

`hut_entry.py` operates on the live game owned by the current session.
Back up install saves/configuration before starting the session, confirm
there is no other Wine/Xvfb rig, and use a fresh reference load per entry.
It changes RAM and temporarily patches the dead startup cave plus hut/RNG
instructions; `finally` restores every hook. No save/map/sprite bytes are
emitted. Stop the game before restoring install backups.

```sh
./load_save.sh Original/Ph_Auto2.SAV
python3 hut_entry.py --seed 1 --output /tmp/mercenary-final.json
./load_save.sh Original/Ph_Auto2.SAV
python3 hut_entry.py --seed 2 --far --fertility 12 --year 1000 \
  --output /tmp/tribe-final.json
```

`--far` moves the selected unstacked land unit (default index 9) to an
empty source south of which a distant hut is selected by the original
hash. `--home-settlers` assigns existing homeless settlers to an owned
city to exercise the nomad branch; `--near-own --level 0 --turn 10`
exercises an empty encounter on a landmass with an own city. These are
explicit controlled inputs, not natural-play observations. Derived cases
and the complete input values are in `docs/evidence/huts-live.json`.

The original reward/dialog code consumes CRT draws too. Call sites
`55852f`/`581d96` (rendering) and `582b9b` (UI object token) use a separate
CRT state in this fixture; the original rand routine still executes.
All other calls keep the pinned gameplay stream, and are recorded by
call site/pre-draw state. Without separation, even a gold dialog adds
176 presentation draws after the three gameplay draws. Do not claim that
these fixtures reproduce presentation-dependent CRT progression.

ToTPP relocates the global tech-owner masks and updates them on grants;
its old per-civ packed compatibility bytes can remain stale. The probe
reads the mask base from the patched immediate at `54bb88`, then derives
classic knowledge for the civ. Hut selection bounds at `54b357` and
`54b3dc` remain 100 in the reference patched build.

`tools/hut_fixture.py` combines captures into small JSON evidence and
plain engine-test vectors. The install-backed `huts_install.rs` runs the
actual WorldState dispatcher against those inputs and compares every
outcome, gameplay draw count/state, treasury/advance mutation, created
unit slot/type/home/movement and tribe size/improvements.
