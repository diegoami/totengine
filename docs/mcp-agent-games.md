# Multi-agent MCP games — design for #262

Status: **design, owner-reviewed decisions of 2026-11-15 embedded; not
implemented.** Program: [#256](https://github.com/diegoami/totengine/issues/256)
(agent play, experimental), sub-issue
[#262](https://github.com/diegoami/totengine/issues/262). Scope stays inside the
owner's 2026-11-12 relaxation: headless, generated-campaign agent play only.

Today one agent plays one civilization because `HumanSession` has a single
human seat. This note designs several external seats in one game, so several
LLM clients can play against each other and/or built-in AIs.

## Owner decisions (2026-11-15)

1. **Shared game folder.** Each LLM runs its own `totctl mcp` process (its own
   stdio registration in its own tool). The game lives in one explicitly
   created folder; processes hand the game over by reading and writing it
   between seat turns. One process may still hold several seats (useful for
   orchestration and tests), but cross-LLM play does not require it.
2. **Strict blocking at absent seats.** When the next seat's LLM is not
   connected, the game simply waits at that seat. No built-in stand-in ever
   plays an agent seat; recordings stay honest about who played.
3. **The server may write inside the game folder only.** Outside a game folder
   created by `agent_start`/`agent_resume`, the server still writes nothing.

## Engine: one driver, a set of seats

`HumanSession` is generalized from `human: u8` to a set of external seats
(`seats: BTreeSet<u8>`, non-empty, within `1..=civilizations`). Generalizing —
rather than adding a second driver — keeps one implementation of per-seat
mechanics, checkpoint replay and view building; the per-seat state that exists
today (fog memory, pending decision, goal, retry receipts, notices, action
count, last own result) becomes per-civ state keyed inside the session.

- **Turn order and suspension.** The opponent loop in `advance()` is unchanged
  except for its stop condition: it stops when the cursor reaches *any* seat's
  turn (lowest civ first, as civs already run in order 1..7). `end_turn` from a
  seat runs the built-in AIs between seats and stops at the next seat.
- **Skipping is not playing.** A civ with no player is skipped by the loop, so
  every seat must be a real stop point: each seat gets its `begin_turn`
  window, required-decision gating (research, founding) and notices exactly as
  the single human seat does today.
- **`Phase`.** The public enum gains `Phase::Seat(u8)`. Single-seat sessions
  keep emitting exactly today's phases (`Human` remains "your turn" for the
  one seat), so both clients' route logic is unaffected; in-repo matches on
  `Phase` are updated, and a multi-seat session never reports `Human`.
- **Per-seat filtered views, revisions, request IDs.** A view is built from
  one seat's fog memory only; a session-global `revision` still increments on
  every state change (it orders the journal, not the view). Request IDs and
  retry receipts stay per seat, as they are keyed by the requesting civ in the
  journal and receipts map.
- **Fog isolation between agents.** Seat M's view is filtered through seat
  M's `known_tiles`/`known_cities`; the MCP surface for a process claiming
  seat M never offers another seat's view or another seat's private state.
  This is tool-surface isolation over a shared local folder, not a security
  boundary (see *Trust boundary*).
- **Checkpoints.** Single-seat sessions keep emitting byte-identical
  `totengine-human 2`; a multi-seat session emits `totengine-human 3`, which
  adds one `seats <civ ...>` line and otherwise reuses the same header
  fields, journal and replay verification. Format 2 restores exactly as
  before; format 3 is refused by old readers, as today's unknown-line rule
  already does.
- **Replay without agents.** The journal already records the requesting civ
  per input and `advance` budgets; a multi-seat recording therefore replays
  with no agent present, exactly like single-seat ones.

## MCP: the shared game folder

A game folder is created once and never overwritten wholesale; state moves
forward by atomic file replacement (write `state.tmp`, rename over `state`).

- `game.json` — the manifest: seats (`civ`, agent name, model label as given),
  generated setup descriptor, Rules identity, resource identity, engine pin,
  creation parameters (seed, size, civs, difficulty), and the current
  position: whose seat's turn it is, or the terminal state.
- `state.chk` — the format-3 checkpoint at the current position.
- `notes/` — per-seat inert presentation notes (per-turn summaries, command
  statistics per agent, as in #263), appended per completed seat turn so any
  process can rebuild `agent_report` for the whole game.
- `lock` — advisory creation/claim marker (process id + seat claim + time).
  Refuses concurrent writers; stale locks (dead pid) are reported, never
  auto-stolen silently.

Tools (single-seat tools keep their names and semantics; `game` handles in
this section refer to a game folder):

- `agent_start {folder, rules, seats: [{civ, agent, model?}], seed, width,
  height, land_percent?, civs?, difficulty?}` — creates the folder (must not
  exist), writes the initial state at the first seat's turn, and claims the
  calling process's seat (`seat` argument) for immediate play. At least two
  civilizations must exist; seats may be any subset in `1..=civilizations`.
- `agent_resume {folder, agent, model?}` — opens the folder read-state,
  verifies Rules/engine/resource identities, refuses if it is not a seat's
  turn or the seat is already actively claimed elsewhere, claims the seat
  whose turn it is, and returns that seat's first view. A process may hold
  several seats only by resuming each in turn when it is their turn.
- `agent_view` / `agent_map` / `agent_submit` / `agent_quote` — unchanged,
  addressed to the claimed seat. `end_turn` advances to the next seat,
  atomically writes the folder and answers with `waiting_for: {seat, agent}`
  (or the terminal state). If the next seat is held by the same process, the
  response says so and its view is immediately available.
- `agent_report` — unchanged for live seat reports; `agent_report {path}`
  extends to a game folder, joining every seat's notes with the manifest.
- `agent_checkpoint {format: "recording"}` — a `.totrec` for the whole game
  so far, naming every agent in the producer/notes, replay-verified without
  agents by `recording_verify`/`totctl record verify`.
- Absent seat (owner decision 2): any `agent_resume` while another seat's
  turn is pending returns that fact as an error ("seat M (agent X) has the
  turn"); nothing plays the seat, and the folder just waits.

## Trust boundary

The folder contains full game state (as any local save does). Fog isolation
holds at the tool surface — a well-behaved client sees only its seat — but an
operator with filesystem access can read everything. This design targets
same-owner and cooperative multi-LLM experiments. Adversarial play between
mutually distrusting owners needs a hosted referee process that holds the
state and serves only filtered views (a future shape along #255's loopback
HTTP direction); it is explicitly out of scope here.

## Consequences

- `human.rs` (baseline-tracked) changes: full `make test` with `CIVTOT_HOME`,
  audit and baseline refresh are part of the work, on an install-backed
  machine.
- Single-seat checkpoints, recordings and MCP tools remain byte-identical;
  the clients and #259's quickstart are unaffected.
- AI opponents stay `builtin-1` (the checkpoint policy line is unchanged);
  choosing opponent policies is #261 and stays separate.

## Tests

- Engine: two seats in one generated game; per-seat fog isolation (seat M
  cannot see seat N's private state through its view); per-seat required
  decisions; elimination skips a seat's turn; turn limit; format-3 checkpoint
  restores; single-seat sessions still emit and restore format 2
  byte-identically.
- MCP: two in-process servers sharing one folder play consecutive seat turns
  with correct hand-over, blocking at the absent seat, atomic state writes and
  intact fog per server; a stdio test drives two real `totctl mcp` processes
  through one folder; recordings name both agents and verify by replay.
- Determinism: the same seat transcripts produce byte-identical state.chk and
  recordings regardless of which process played which seat.

## Not in this slice

- Choosing opponent policy per game (#261).
- Agent seats in experiment manifests (#184 integration).
- More than one connection per process, hosted referee / HTTP transport,
  cross-owner adversarial isolation (#255 direction).
- Scenario starts (#197) and client-attached advisor use of these seats.
