# Decision 0002 — players behind an interface; observing and playing as the two main goals

Current mission/roadmap: [decision 0008](0008-scenario-platform.md), accepted
2026-11-08. Historical release scope below remains recorded; its future schedule
is superseded where inconsistent. Both-client UI fidelity and client-first AI
implementation gates remain; native-AI reproduction is not the only AI route.

*Status: accepted (owner decision, 2026-11-04; amended the same day: observing and playing are both main goals — observation is one of the main reasons the project exists; release ladder: GitHub milestones v0.5.0–v0.11.0).*

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
2. **Pluggable AIs** (after the playable client; version unassigned):
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
   - Client work spans multiple milestones. v0.11.0 starts UI discovery,
     concrete designs for owner review, and milestone decomposition
     (decision 0005). There is no validated client specification yet;
     implementation scope and later release versions remain unassigned.
   - The full classic client (city screen, menus, sound) follows.
   - Order: browser observer (v0.7.0), original-like built-in AI
     (v0.10.0), client discovery/design (v0.11.0), successive client
     implementation milestones, then pluggable AIs.
   - Owner amendment (2026-11-04): by v0.10.0, the built-in opponent
     should resemble the original game's AI. The v0.8.0 AI v1 is a
     foundation; deterministic autoplay alone does not meet that target.
     Compare expansion, city/economy decisions, research and military
     behavior against the original on controlled positions. Document
     supported behavior and remaining differences; full decision-for-decision
     parity is not claimed without evidence. External AI protocols and
     tournaments must not delay the playable client.

## Consequences

- Every AI (RG-10 v0, RG-4 v1) is built against the player interface,
  not wired into the turn loop.
- The interface (RG-8) is headless new work in v0.5.0 (observation,
  order set, validation, order log), tested with a random-but-legal
  player; AI v0 (RG-10) follows in v0.6.0.
- The backlog gains Epic OB (observer + pluggable AIs) and T2-1, now
  GitHub milestones and issues (#74–#97). The
  release ladder there spreads the work over small releases. OB-3/OB-4
  follow the playable client; v0.10.0 now targets the built-in AI's
  resemblance to the original, rather than the external-AI infrastructure.
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
