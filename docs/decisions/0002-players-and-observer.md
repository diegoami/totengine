# Decision 0002 — players behind an interface; observing and playing as the two main goals

*Status: accepted (owner decision, 2026-11-04; amended the same day: observing and playing are both main goals — observation is one of the main reasons the project exists; release ladder in BACKLOG.md).*

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
2. **Pluggable AIs** (v0.10.0):
   - in-process Rust players behind a trait;
   - an **out-of-process protocol** (JSON over stdio or a socket), so AIs
     can be written in any language, LLM agents included;
   - a headless tournament runner (AI vs AI, many seeds), built on the
     engine's determinism.
3. **The observer is the first UI** (v0.7.0), because watching needs
   less UI than playing:
   - **browser-based**, fed by the observation/replay stream;
   - spectator view with the full map or one civ's fog;
   - turn stepping, play/pause, and replay of a finished game;
   - read-only: no city screen, no order UI.

   It cannot ship the original art, so it uses our own simple tiles unless
   it loads them from the user's install locally.
4. **Observing and playing are both main goals.** The owner's note that
   the original probably lacks an observer mode is a reason to build it,
   not a ranking.
   - The SDL2 native client (Windows 10/11, Linux, macOS; original art
     read from the user's install at runtime) is where humans play.
   - Its first release is minimal (T2-1): map, units, basic orders, end
     turn, simple city view, observer mode.
   - The full classic client (city screen, menus, sound) follows.
   - Order: browser observer (v0.7.0), pluggable AIs (v0.10.0), the
     SDL2 client (v0.11.0). The client can move earlier on the owner's
     call.

## Consequences

- Every AI (RG-10 v0, RG-4 v1) is built against the player interface,
  not wired into the turn loop.
- The interface (RG-8) is headless new work in v0.5.0 (observation,
  order set, validation, order log), tested with a random-but-legal
  player; AI v0 (RG-10) follows in v0.6.0.
- BACKLOG gains Epic OB (observer + pluggable AIs) and T2-1. The
  release ladder there spreads the work over small releases (v0.3.0 …
  v0.11.0) so that each review stays one round.
- The headless all-AI campaign (RG-7, v0.9.0) has a golden run made of
  an order log plus fingerprints.
- The original game's GUI, for reference: Win32
  (`USER32`/`GDI32`/`COMCTL32`/`comdlg32`) plus DirectDraw (`DDRAW`), with
  `WINMM` audio and AVI video (`AVIFIL32`, `MSVFW32`).
  - Source: the import table of the unpatched `civ2.bak`, which also
    imports `ADVAPI32`, `KERNEL32` and `XDaemon`.
  - `TILES`, `cv`, `pv`, `ss`, `mk` and `timerdll` appear only as strings,
    so they are loaded at runtime.
  - The art sits in resource DLLs (`Tiles.dll`, `Civ2Art.dll`, …) and the
    rule folders' `.bmp`/`.spr` sheets.
