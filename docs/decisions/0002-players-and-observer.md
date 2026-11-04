# Decision 0002 — players behind an interface; an observer before the playable client

*Status: accepted (owner decision, 2026-11-04).*

## Context

The owner's goal is **autonomous headless play with every civ run by an
AI**, and watching it. As far as we know the original has neither a
spectator mode nor an interface for outside AIs; neither is verified.

The plan had the first UI arrive with T2:
- a full playable SDL2 client (map, city screen, orders, dialogs);
- planned 6–12 months after the headless core;
- AI only as one built-in implementation (RG-4).

## Decision

1. **Every civ is driven by a player implementation.** The engine gives
   each player a per-civ *observation* (its fog-limited view: map, units,
   cities, treasury, research). The player returns *orders* (move, attack,
   found city, build, research, …). The engine validates and applies them.
   - A game is reproducible from (save/seed + build + order log).
   - The built-in AI (RG-4) is the first implementation.
   - "All players AI" is the default setup.
   - A human player in T2 is one more implementation.
2. **Pluggable AIs, from the first observer release:**
   - in-process Rust players behind a trait;
   - an **out-of-process protocol** (JSON over stdio or a socket), so AIs
     can be written in any language, LLM agents included;
   - a headless tournament runner (AI vs AI, many seeds), built on the
     engine's determinism.
3. **An observer UI comes before the playable client** as its own
   milestone (v0.4.0):
   - **browser-based**, fed by the observation/replay stream;
   - spectator view with the full map or one civ's fog;
   - turn stepping, play/pause, and replay of a finished game;
   - read-only: no city screen, no order UI.

   It cannot ship the original art, so it uses our own simple tiles unless
   it loads them from the user's install locally.
4. **T2 stays the full playable classic client**, SDL2 native: Windows
   10/11, Linux and macOS, with the original art read from the user's
   install at runtime. It reuses the observer's map model.

## Consequences

- RG-4 is built against the player interface, not wired into the turn
  loop.
- The interface is new work, scheduled before RG-4: observation, order
  set, validation, order log.
- BACKLOG gains Epic OB (observer + pluggable AIs) for v0.4.0. T2 moves
  after it.
- v0.3.0 still ends with a headless all-AI campaign (RG-7). Its golden
  run is an order log plus fingerprints.
- The original game's GUI, for reference: Win32
  (`user32`/`GDI32`/`COMCTL32`/`comdlg32`) plus DirectDraw (`DDRAW`), with
  `WINMM` audio and AVI video. The art sits in resource DLLs (`Tiles.dll`,
  `Civ2Art.dll`, …) and the rule folders' `.bmp`/`.spr` sheets. This comes
  from a string scan of `civ2.exe` (its import table did not parse under
  objdump).
