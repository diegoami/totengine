# Oracle rig (T0-10)

Headless Wine + Xvfb harness that drives the original game and harvests saves.

## Pieces

| File | What |
|---|---|
| `xtool.c` | X11 automation: XTest key/button injection, `focus`/`focusptr` (no WM on Xvfb — set input focus explicitly or make it follow the pointer), `shot` (PPM, palette-aware for 8-bit screens), `stats` (state fingerprint), `windows`. Build: `gcc -O2 -o xtool xtool.c -lX11 /usr/lib/x86_64-linux-gnu/libXtst.so.6` |
| `winject.c` | Win32-level injector (SendInput/SetCursorPos + `post` WM_LBUTTONDOWN + **`focus`**). Build with llvm-mingw: `clang --target=x86_64-pc-windows-gnu -O2 -o winject.exe winject.c -luser32`. Run inside the game prefix: `wine winject.exe key 13`. **X-level keys never reach the game's DirectDraw surface; SendInput does.** `focus <hwnd-hex>` (thread-attach + SetForegroundWindow/SetActiveWindow/SetFocus) is REQUIRED before in-world keys when the badge shows no foreground — without it SendInput is silently discarded (post-reboot rig, 2026-11-02 night). |
| `ppm_bands.py` / `ppm_text.py` | Read the screen without eyes: bright-row detection and ASCII rendering of PPM regions. |
| `ppm2png.py` | PPM→PNG so **vision-capable models can read the screen** (the session model hands `/tmp/*.png` to a vision subagent — dialog items, selection state and coordinates come back literally; this replaced ASCII squinting entirely). |
| `memspool.py` | **Live memory access**: daemon that spawns the game as its own descendant (yama ptrace_scope=1 compliant), then serves `pids/maps/read/write/scan/snapshot/diffsnap` over a unix socket (`write` = process_vm_writev, for patch-and-prove seed experiments). Cheat-Engine-grade RE with zero installs. |
| `memspool_ctrl.sh` | start/stop/restart for the memspool daemon itself (the pkill trap applies to daemon management too — keep it in a script). |
| `newgame.sh` | Drive a fresh STANDARD game through the wizard (params: world-size/difficulty/competition Downs). |
| `restart_game.sh` / `stop_game.sh` | All game process management lives in scripts — never put a pkill pattern in an interactive command line (the self-kill trap struck four times; `restart_game.sh` also forces DISPLAY=:99 — a caller's :0 once leaked in and the game opened windows on the wrong screen). |
| `build_cd.py` | Builds a fake-CD ISO of the install (needs `PYTHONPATH` to a pip-less pycdlib install). |
| `turnloop.sh` | The acceptance loop: ends N turns (3× SPACE + RETURN via winject) and snapshots the autosave each turn. |

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
