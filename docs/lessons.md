# Lessons learned (process & rig)

Distilled operational lessons — each one was paid for. The *format findings*
live in `docs/evidence/README.md` (decoded fields, structure claims); this
file is about how to run experiments and work the repo without repeating
mistakes. See also `tools/oracle/README.md` for rig mechanics.

## Process discipline

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

## Wizard & navigation (oracle)

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
