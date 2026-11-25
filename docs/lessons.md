# Lessons learned (process & rig)

Distilled operational lessons — each one was paid for. The *format findings*
live in `docs/evidence/README.md` (decoded fields, structure claims); this
file is about how to run experiments and work the repo without repeating
mistakes. See also `tools/oracle/README.md` for rig mechanics.

## Process discipline

- **Keep going between steps.** The owner wants implementation, validation,
  fixes, and draft PRs handled continuously across a milestone. Batch progress
  and pause for a required review or an owner-only blocker.

- **Never inline process-killing.** The pkill self-kill trap struck FIVE
  times across two strings: any command line containing a `pkill -f
  '[p]attern'` AND the literal string matching that pattern ANYWHERE else
  in the same command line (a later `wine civ2.exe`, a `grep`, an `echo`,
  even a `setsid ... memspool.py serve` after a `pkill -f
  '[m]emspool.py serve'`) kills its own shell mid-run. The bracket trick
  only protects the pattern itself, not other occurrences. ALL process
  management lives in scripts (`tools/oracle/stop_game.sh`,
  `restart_game.sh`, `memspool_ctrl.sh`); interactive commands only ever
  call those scripts.
- **Verify client input by the coordinates delivered, not by one lucky
  screenshot.** #289 "fixed" desktop clicks by mapping window coordinates
  onto the logical stage, and one Xvfb screenshot seemed to confirm it. But
  SDL's renderer already performs that mapping whenever `set_logical_size` is
  in effect, so every click at 1.5×/2× was scaled twice and missed (#292). Log the raw
  event coordinates once (`eprintln!` in a scratch build) and drive a
  complete route (start, choose, toolbar) at a non-1× window before claiming
  an input fix. Run the client yourself; don't stop at the unit test.
- **A chrome change must re-run the real-input scripts to completion.** The
  desktop acceptance checks (`tools/check_client_linux.py`,
  `tools/check_client_windows.ps1`) find a focused button by its exact draw
  rects. #286's gold focus hairline moved the select fill one pixel inward,
  so neither script could focus any button; nobody noticed because the
  release ran only its own scratch rig (v0.20 review, #307). After touching
  `style::button` or other draw geometry, run
  `python3 tools/check_client_linux.py` to the end, not just the unit tests.
- **Point scripted client runs at scratch saves.** Clients write host-side
  files beside the saves folder (`setup.json` on Start, `enhancements.json`
  on a settings toggle). A scripted Xvfb run without `TOTENGINE_SAVES` writes
  into the owner's real `~/.local/share/totengine/`. Export
  `TOTENGINE_SAVES=<scratch>/saves` in every rig script.
- **Force the rig display.** A caller's inherited `DISPLAY=:0` leaked through
  `${DISPLAY:-:99}` and the game silently opened windows on the wrong screen
  (zero windows on :99, looked like a hung game). Rig scripts
  `export DISPLAY=:99` unconditionally.
- **Harvest-time staleness checks.** Autosaves rotate slots
  (`Ca_Auto`/`Ca_Auto2`): after a ride, list mtimes of ALL slots and take the
  newest, then verify the copy is NOT byte-identical to the previous run's
  baseline before trusting it. (The E variant silently copied a stale save
  and produced a bogus "D≡E" result — caught only by the byte-compare.)
- **Never press keys blind.** Returns pressed while waiting for a dialog that
  never appears land in the game world (advisor popups like the Toynbee
  ranking, unintended actions). Wait for window titles; if the chain stalls,
  diagnose with a screenshot, don't keep pressing.
- **The main session never launches milestone reviews** (owner decision
  2026-11-02, see `docs/milestone-review.md` + AGENTS.md hard rules) — print
  the template and stop.

- **Rig regression, 2026-11-15 (open): in-world input dead, world window
  unenumerable.** Symptom: after any save loads to the world (RAM reads fine,
  world renders), no input lands — winject SendInput keys, XTest keys and
  X-level focus all leave turn/active-unit/RNG frozen — and `winject wins`
  (64- and 32-bit builds) lists only `Default IME` while dialog-phase windows
  in the same run enumerated fine. The game also died silently twice ~4–5 min
  after loading derived #194 fixtures (pristine baseline survived 7+ min
  idle); Xvfb died once mid-session when a wrapping `timeout` SIGTERM'd a
  restart script — start Xvfb with `setsid nohup` so it outlives shells.
  Afternoon triage added: the enumeration loss happens **exactly at world
  entry** (dialogs list fine seconds earlier); a frozen PPM surface does NOT
  prove a hang — the main thread sits in a wineserver `select` waiting for
  input, which is normal idle (check `/proc/<pid>/task/*/wchan`, expect
  `anon_pipe_read`, before calling a wedge); walking child windows
  (`EnumChildWindows`, now in `winject wins`) finds nothing either, the game's
  windows are simply not in any external process's Win32 window set; a 16-bit
  Xvfb breaks the wizard (launcher_pick cannot read the radio) — stay on x24.
  Not the cause: wine version (unchanged since Nov 1), the prefix, bitness,
  missing CD/audio (none configured, same as working sessions). Next steps
  when picking this up: WINEDEBUG=+relay,+event tail around world entry for
  the window-destroy/recreate sequence, a fresh test prefix, or a winedbg
  thread backtrace at the blocked select. Machine rebooted 08:37 the same
  morning; the previous post-reboot rig note (2026-11-02) needed only
  explicit `winject focus`. Diagnostic shortcuts that paid off: `/proc/<pid>/
  status` State Z exposes a dead game while memspool still lists it; compare
  the pristine save side by side before blaming edited bytes; `pgrep -af
  '[x]yz'` self-matches when the literal appears anywhere else in the command
  line; never open `/proc/<pid>/fd/*` entries with a reader (grep hung on a
  pipe endpoint) — `ls` only.

- **A static decode of "initial values" is a claim about saved records
  too — check it against saves before shipping.** v0.6.0 claimed
  `create_city` sets flags `0x20000` (true at `0x432b2f`), and the engine
  wrote it permanently; the reviewer found it in 0 of 1,480 saved cities
  (#102). The bit is a transient refresh marker cleared by the next totals
  pass. Before claiming any record value, grep the saves (the std1 series
  has freshly founded cities) and grep the disassembly for every
  `and`/`or`/`mov` on the same field, not just the writer you read.

## Wizard & navigation (oracle)

- **Gate inspection before RNG pinning.** A manual screenshot check can take
  minutes. `seeded_start.sh` checks City Style first, then pins the generators
  immediately before Return. The intro title has a suffix (`In the Beginning …`);
  matching it as an exact caption falsely rejects a valid generated map.
  Stone textures also vary between launches: radio-list gates compare the
  selected button center in a known title/geometry, rather than background pixels.

- **Vision-verify every selection before Return.** Screenshots →
  `ppm2png.py` → a vision-capable subagent reads dialog items, selection
  state, and coordinates literally. This replaced ASCII squinting entirely
  and caught two wrong-landings in one session.
- **Arrow keys are silently swallowed** when racing a dialog's opening
  transition. Reliable pattern: wait for the window TITLE → focus the NEW
  dialog HWND → keys ≥0.7 s apart → vision-verify the highlighted item →
  Return. Both knob-variant runs that skipped this silently produced the
  baseline settings.
- **Dialogs remember previous answers** across restarts (the difficulty
  dialog re-opened pre-selected at the previous run's Emperor and the next
  two runs inherited it). Never assume defaults: the remembered value IS the
  experiment's baseline — vision-check the starting selection, and restore
  knobs changed by earlier variants before isolating a new one.
- **The ToTPP launcher ignores injected clicks entirely** (SendInput and XTest
  both). Keyboard (xtool focus + key) is the only reliable activation.
- **Owner-drawn title-screen menus are a dead end** for clicks (no dropdown
  ever appears) — the launcher list is the only entry point.
- Focus each NEW dialog HWND explicitly (no WM on Xvfb; re-focusing the
  launcher behind an open dialog sends the keys to the wrong window).
- Wine window coordinates == X coordinates; `xwininfo -tree -root` /
  `winject wins` locate dialogs by title/rect — don't guess from screenshots.

## Experiment method (RG-1 style)

- **World-gen is NON-deterministic** (CORRECTED 2026-11-02 evening): three
  runs with identical, vision-verified answers produced three different
  worlds (same structure — length/dims/684/689 — different terrain and
  placement, ~3.5k bytes). An earlier "deterministic given settings" claim
  was an artifact of the stale-harvest bug below (C's autosave copied three
  times). Diff experiments remain valid — just diff verified specimens
  directly; don't promise reproducibility from settings alone.
- **Stale-harvest once manufactured a false law.** The bogus C≡D≡E
  byte-identity ("determinism") came from copying a stale autosave slot
  while the fresh save rotated into the other `Ca_Auto*` slot. The two
  checks that catch it: compare mtimes of ALL rotating slots, and
  byte-compare the harvest against the previous baseline — a "new world"
  that is byte-identical to the previous run is a RED FLAG, not a law.
- **Validate wizard variants mechanically at harvest**, before analysis:
  map dims at 29900 (2×height, width, area), `save[684]` difficulty,
  `save[689]` rivals. If a value that should have moved didn't, the Downs
  were swallowed — redo the run rather than analyze garbage.
- Classify diff bytes three-way across verified variants (e.g. C2/D2/E2):
  bytes differing only in the competition pair are competition fields, only
  in the size pair are size fields, in both pairs are world-noise. Then
  CONFIRM candidates against a third specimen (a candidate that wobbles
  across same-answer worlds is noise, not a field).
- Every format claim gets an evidence artifact and a commit that re-derives
  it (repo hard rule); rig findings go to `tools/oracle/README.md` +
  `docs/evidence/README.md` in the same commit as the tooling that produced
  them.

## Rig quirks

- **yama ptrace_scope=1**: reading the game's memory requires the game to be
  a DESCENDANT of the reader. `memspool.py` spawns wine as its direct child —
  a backgrounding script orphans wine (reparented to init) and it becomes
  unreadable again. A game started outside the daemon is brought under with
  `memspool.py restart`.
- **Orphaned wine session** presents as: game process sleeping with threads,
  zero windows on the display, no wineserver around. Fix is `stop_game.sh`
  (kill + `wineserver -k`) then a fresh start — retrying navigation just
  wastes time.
- The memspool client's `makefile().readline()` can return an empty line and
  crash JSON parsing — use the raw-socket recv-until-newline pattern with a
  timeout (documented in the oracle README).
- ToTPP needs high color: Xvfb must be `-screen 0 1024x768x24`.
- ToT classic (`civ2.bak`) cannot run headless (CD-audio check); the ToTPP
  exe's NoCD patch is the only practical oracle exe.
- **Drive units through RAM, not the map** (`tools/oracle/unitram.py`). A
  single key press with a `.data` diff before and after finds any UI global
  in minutes (cursor, view struct, active unit). Navigation was never the
  real blocker for T1-6.
- **When a known generator slot does not move, hunt the generator, not the
  formula.** Snapshot `.data` + heap around the action and test every u32
  for `after = step^n(before)` under each candidate LCG (NR, MSVC
  214013/2531011, Borland 22695477). Static call-site surveys of one RNG
  function miss a second generator entirely; that cost T1-6 a full
  session.
- **ToTPP patches `.text` immediates at load time.** For relocated tables
  (units, unit types), disassemble the *live* `.text` read from the
  process, not the exe file. Over 35k bytes differ.
- **Long-running functions can be read live.** The combat function's
  stack frame persists through the attack animation (over 1 s). Polling
  the stack for a signature of known locals (x/y of both units) recovers
  intermediate values (strengths, rolls) that no save or post-hoc
  snapshot holds.
- **Do not build a vision-click navigator for the map.** Four calibration
  clicks plus arrow-key runs proved there is no usable linear pixel→tile map
  in this build (the view origin is not where the click re-centres). The
  cheap fix is to read the view origin from RAM; until then, "navigate to a
  known tile" is not a reliable rig primitive. Details in
  `tools/oracle/README.md` §Navigation.
- **Blind clicking silently burns the turn.** Every map click while a unit is
  active issues a goto order, so a session of exploratory clicking ends with
  no activatable unit (`W`/`Enter` dead, arrow keys consume no randomness) —
  which reads as "the key doesn't work" rather than "the turn is spent".
  Before concluding a key is broken, check whether the turn still has orders
  to give.
- **Test record framing with a cross-record invariant, not with fields
  inside one record.** Every field the walkers "proved" sat at a
  consistent offset, and a 92-B record framed 34 bytes late still reads
  plausible numbers. What exposed it was a relation *between* tables:
  units on a city tile should be homed in that city (0/342 in the old
  framing, 349/349 in the true one). Run such a check (home city, owner,
  geography) whenever a record boundary comes from arithmetic alone.
- **A "settled" screen-vs-save conflict can be settled the wrong way.** The
  T1-4 Madrid (53,9) vs (43,37) question was closed in favour of the save
  by a RAM patch that changed nothing on screen. A null result is not a
  proof; the screen was right.
- **When local rustfmt wants to change files you didn't touch, believe it.**
  A session assumed "this machine's rustfmt formats differently" and
  reverted the changes each time; CI's `cargo fmt --check` had been red
  for many commits on exactly those files. Check `gh run list` before
  explaining a check failure away, and gate commits on fmt + clippy +
  `make test` together (a `&&` chain that skips the test result commits
  red code).
- **A "flaky" wizard can be a deterministic cycle.** The game-type list
  remembers its last choice, and Up/Down wrap around its 8 items. A fixed
  `Up×4` therefore flips between two items on alternate runs:
  - Original ↔ Midgard: every other run never saw "Select Size of World";
  - Fantasy ↔ Load a Saved Game: the "Select Game To Load" failure.

  Retries hid it, and the first seeded "standard game" runs of 2026-11-04
  were Fantasy games. Select list items by reading the state
  (`launcher_pick.py`), never by relative key counts.
- **Put probe buffers where the game does not write, and check that the
  counter only grows.** The first `rand` call-site log sat in a zero
  region the civ-placement setup clears (`0x6077a0..0x607c50`). The log
  silently restarted mid-run, and the map-generator draws went missing
  from the capture.
- **"The newest save" is not "this run's save".** `ls -t | head -1`
  copied yesterday's autosave three times while the real saves went to
  another game folder. Harvest with `find -newer <marker touched at run
  start>` and fail when nothing is fresh.
- **Restoring files under a still-running game does not stick.** The
  oracle restored the install's autosaves on exit, then the game it left
  running autosaved over them. Stop the process that writes before
  restoring what it writes (`seeded_start.sh` now does; `KEEP_GAME=1`
  opts out).
- **Never read decoded arithmetic through a noise filter.** Hiding
  `lea eax,[eax+eax*2]` / `shl eax,3` lines made the record-address
  arithmetic readable, but it also hid a real `· 3` in the corruption
  formula, and the replay failed for a "wrong input" that was never wrong.
  Filter for reading, then re-read every formula unfiltered before
  trusting it.
- **Two framing errors can cancel for one field and hide the bug for the
  rest.** The save's tile records were read 2 bytes late, and an index
  `−1` plus a `+4` byte offset brought terrain back onto the right byte,
  so every terrain check passed. The road/rail bits read through the
  same frame were another tile's fog, and a fog layer around an empire
  looks like a road network. Prove a record layout against the game's
  own accessor or a cross-table invariant (city flag ⇔ city record),
  never by "the picture looks right".


- **Live ToTPP knowledge can bypass the old packed civ bytes.** The
  `49c820` entry still looks original, but middle-of-function jumps call
  ToTPP. After a hut grant the original `5f6900+civ*3348` compatibility
  bytes stayed unchanged; the relocated tech-owner masks changed. Derive
  their base from `54bb88`'s patched immediate (subtract tech index 55).
  `hut_entry.py` does this; see the live hut evidence.
- **Reward windows consume the same CRT generator as hut logic.** A gold
  entry has three gameplay draws plus 176 presentation draws in the
  observed UI. Controlled headless comparisons isolate the identified
  presentation call sites into a separate CRT state, record all remaining
  callers, and restore the hooks. Document that isolation explicitly;
  comparing only the final global state without call sites is misleading.

## Documentation the agent cannot discover is documentation that does not exist

Paid for in the owner's separate goal2 recreation: the agent authored a
user manual by walking through the running game — a captured behavioral
spec — but the file was never referenced from the project's agent
orientation (AGENTS.md), so later sessions never found it and programmed
their own thing, drifting from the captured behavior. The failure is not
authorship but wiring: an agent only consults what its orientation file
names. Two rules follow for this repo:

- **Wire every behavioral reference into `AGENTS.md`.** The orientation
  file is the session's discovery surface; anything behavioral that
  sessions should consult (manual, controls reference, screen contracts,
  evidence index) must be listed there with its role, not merely exist
  in the tree.
- **State the hierarchy when wiring.** A player-facing guide describes
  shipped behavior; it is not the UI spec. `AGENTS.md`'s entry for
  `docs/manual.md` names the order explicitly: original observation,
  then `docs/ui/screen-contracts.md`, then the guide — so a later
  session cannot mistake the description for the spec and implement to
  match itself.
- **A user guide needs real screenshots, captured and linked in the
  same change.** The guide's first draft described behavior without a
  single image of the running client — to a new player the look and
  feel was simply undefined, the same gap as the goal2 manual whose
  screenshots existed but were not correctly linked. Capture from the
  real clients (the browser harness drives the actual artwork), commit
  the PNGs next to the guide, and reference them with relative links
  in the same commit so neither half can drift. The capture script
  (`tools/manual_screenshots.cjs`) is committed with the shots so the
  images stay reproducible.
- **The guide's look-and-feel reference is the original game, not the
  engine.** Engine screenshots alone document what we have built; the
  reader expects the original Civilization: Test of Time. Owner
  decision (2026-11-25): the guide may commit original-game screenshots
  as the look-and-feel reference — an explicit exception to the
  original-screenshots-stay-outside-Git practice, scoped to the user
  guide's reference section. The November 2025 interface-study
  captures (launch, map, city window, Civilopedia) now sit in
  `docs/manual/screenshots/original/`, and every engine screenshot is
  captioned as the current implementation. Without that framing the
  guide quietly redefines the target as whatever the engine happens to
  render today.
