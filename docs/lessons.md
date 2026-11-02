# Lessons learned (process & rig)

Distilled operational lessons — each one was paid for. The *format findings*
live in `docs/evidence/README.md` (decoded fields, structure claims); this
file is about how to run experiments and work the repo without repeating
mistakes. See also `tools/oracle/README.md` for rig mechanics.

## Process discipline

- **Never inline process-killing.** The pkill self-kill trap struck FOUR
  times: any command line containing both `pkill -f '[c]iv2'` and the literal
  string `civ2` (a later `wine civ2.exe`, a `grep civ2`, even `pgrep civ2`)
  kills its own shell mid-run. All process management lives in
  `tools/oracle/stop_game.sh` / `restart_game.sh`; interactive commands only
  ever call those scripts.
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

- **World-gen is deterministic given the wizard settings** (seed = settings
  vector, NOT the clock; three same-answer runs at different times were
  byte-identical, ~96 KB including the map). Consequences: (a) isolate a knob
  by diffing against a same-settings baseline, never across knobs — any
  setting change reshuffles the whole world (~8.4k bytes); (b) experiments
  are exactly reproducible, which is what makes byte-isolation meaningful.
- **Validate wizard variants mechanically at harvest**, before analysis:
  map-header dims at 29900 (100×80 default — a world-size change must move
  it), `save[684]` difficulty, `save[698]` civ-count candidate. If a value
  that should have moved didn't, the Downs were swallowed — redo the run
  rather than analyze garbage.
- Classify diff bytes three ways (A vs B, A vs C, B vs C): bytes that differ
  in every pair are seed-noise; bytes specific to one variant's comparisons
  are that knob's fields.
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
