# Oracle rig (T0-10)

Headless Wine + Xvfb harness that drives the original game and harvests saves.

## Pieces

| File | What |
|---|---|
| `xtool.c` | X11 automation: XTest key/button injection, `focus`/`focusptr` (no WM on Xvfb — set input focus explicitly or make it follow the pointer), `shot` (PPM, palette-aware for 8-bit screens), `stats` (state fingerprint), `windows`. Build: `gcc -O2 -o xtool xtool.c -lX11 /usr/lib/x86_64-linux-gnu/libXtst.so.6` |
| `winject.c` | Win32-level injector (SendInput/SetCursorPos + `post` WM_LBUTTONDOWN). Build with llvm-mingw: `clang --target=x86_64-pc-windows-gnu -O2 -o winject.exe winject.c -luser32`. Run inside the game prefix: `wine winject.exe key 13`. **X-level keys never reach the game's DirectDraw surface; SendInput does.** |
| `ppm_bands.py` / `ppm_text.py` | Read the screen without eyes: bright-row detection and ASCII rendering of PPM regions. |
| `build_cd.py` | Builds a fake-CD ISO of the install (needs `PYTHONPATH` to a pip-less pycdlib install). |
| `turnloop.sh` | The acceptance loop: ends N turns (3× SPACE + RETURN via winject) and snapshots the autosave each turn. |

## Launch recipe (what finally worked)

1. `Xvfb :99 -screen 0 1024x768x24` (ToTPP needs high color; classic civ2.bak wanted 8-bit but see below).
2. `WINEPREFIX=~/.wine_civ2tot wine civ2.exe` (the **ToTPP exe** — its NoCD patch matters, see findings).
3. Launcher: it's a **listbox — double-click items** (single clicks only select). Row 2 = scenario browser → pick scenario → leader/difficulty/gender/name dialogs accept plain RETURN (xtool focus + Return).
4. In-world: **winject** keys only (`key 32`=skip unit ×3, `key 13`=end turn). Autosave lands in `<scenario dir>/<XX>_Auto.SAV`; each turn rewrites it.
5. `turnloop.sh N <scen-dir> <Auto.SAV> <outdir>` → consecutive same-session saves.

## Findings (2026-11-02)

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

## Proven run

`turnloop.sh 6 Midgard Oc_Auto.SAV …` → turns 4→9, six consecutive saves;
`totctl diff turn01 turn02` → "size delta +92 bytes = +1 city record (ToT)"
(Midgard's events founded a city between turns), 64,202 differing bytes in
1,873 regions — the fog-of-war map block dominates, as expected.
