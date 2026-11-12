# MCP advisor attached to a running game — design

Status: **design with owner decisions of 2026-11-12; not implemented.** Implementation:
[#255](https://github.com/diegoami/totengine/issues/255). This is stage 2 of
[#250](https://github.com/diegoami/totengine/issues/250) as staged in
[decision 0008](decisions/0008-scenario-platform.md): an MCP client attaches,
with the player's explicit consent, to a game they are playing in `totplay`
(browser) or `totdesktop` (desktop). It **sees what the player sees** and gives
advice. It cannot act. Acting in a human game is stage 3b (#257) and stays
behind the complete-client gate. Only headless agent play (#256) was relaxed;
see decision 0008.

## User route

1. The player starts a game in either client and turns on **Allow advisor**
   from the **in-game menu** (owner decision). It can be turned on and off at
   any time during play; it is off by default and not remembered across launches.
2. The client shows that the advisor endpoint is open, and while an advisor is
   connected it shows an indicator naming it. One click turns it off and
   disconnects.
3. The client shows the endpoint address and token to paste into Claude
   Desktop, Claude Code or another MCP client. The assistant can then answer
   questions such as "what should Rome build?", "why is Athens in disorder?"
   or "which research do you recommend?" from the player's current view.
4. Suggestions stay suggestions. The player carries them out with the normal
   controls; nothing the advisor says becomes an order.

## Architecture (owner decision): MCP inside each client

```text
MCP client ──Streamable HTTP, loopback, bearer token──▶ /mcp endpoint in totplay or totdesktop
                                                          │ reads published snapshots only
                                            client main loop (owns HumanSession)
```

The owner chose an MCP endpoint served by each client, with no separate
`totctl` process. To avoid two MCP stacks:

- **One shared MCP core.** Extract the JSON-RPC/MCP handling now in
  `crates/totctl/src/mcp.rs` (initialize, version negotiation, tools/list,
  tools/call, error mapping) into a small shared crate (proposed `tot-mcp`).
  `totctl mcp` uses it over stdio; the clients use it over HTTP. The tools differ;
  the protocol code is the same and tested once.
- **One shared advisor implementation.** The advisor tools, snapshot type and
  HTTP handling live in `totui::client`, used by both `totplay` and
  `totdesktop`. A shared test feeds the same session state to both and requires
  identical responses.
- **Clients publish, the endpoint never touches the session.** After every
  accepted input, decision or opponent progress, the client serializes an
  advisor snapshot from `HumanSession::view()` and the report/city views into an
  `Arc<Mutex<Snapshot>>`. The endpoint answers only from that snapshot, so it
  cannot lock or change the live game, consume RNG or change the revision.
- **Transport.** MCP Streamable HTTP (`POST /mcp`, JSON responses; no server-sent
  events needed for read-only tools) on loopback.
  - `totplay` serves it on a **separate listener**, not the browser page's port,
    so the page token and origin checks of the human protocol stay unchanged.
  - `totdesktop` starts the same listener thread only while the advisor is on.
  - Both use an ephemeral port, a per-enable random bearer token, a request size
    cap, and `Origin` rejection for browser-originated requests.
  - There is no write route.
- **One advisor at a time** by default: a second client with the token is
  refused while one is connected. This can be relaxed later.

Alternatives considered:

| Option | Status |
| --- | --- |
| Feed in each client + `totctl mcp --attach` over stdio | Proposed first; owner chose in-client MCP instead |
| Exposing the browser's existing `/view` routes | Rejected: that is the human client's protocol, and the advisor needs a narrower, versioned, read-only schema |
| Reading checkpoints | **Rejected:** replaying a human checkpoint rebuilds the complete game, so it is full state, not the player's view |

## What the advisor may see

Only data already filtered for the human civilization:

- Phase, turn, year, revision, and any pending decision with its legal choices
  (research, founding name, government, rate review).
- Own civilization: treasury, rates and caps, government, research and goal,
  known advances.
- Own cities: the existing filtered city info, production, citizens, support and
  happiness views.
- Own units, plus the fog-limited tiles and foreign units/cities the player can
  see now or remembers (last-seen memory), exactly as rendered.
- Notices since a given revision.
- Reports (`HumanSession::report` families), which are already derived from
  the filtered view.

Never exposed: the spectator view, other civilizations' private state, AI
policy state, RNG, order logs or checkpoints, installation paths, and original
resources such as sprites and sounds.

**Civilopedia text (owner decision: opt-in).** It comes from the player's own
`Describe.txt`, so it is original game text. It is available only through a
second, separate switch, **Share Civilopedia text**, which is off by default and
only offered while the advisor is on. Only then does a `game_pedia {topic}` tool
appear. This is a deliberate, player-chosen exception to #250's "no original
assets" rule, limited to text the player owns. Images and sounds are never sent.

## MCP tools (stage 2)

| Tool | Returns |
| --- | --- |
| `game_status` | Client kind, revision, phase, turn/year, pending decision summary, advisor feed version |
| `game_overview` | Own civilization summary (treasury, rates, government, research/goal, city and unit counts) |
| `game_cities` / `game_city {id}` | Filtered city list / one city's info, production, citizens, support, happiness |
| `game_map {x, y, radius}` | Visible and remembered tiles, units and cities around a point, bounded in size |
| `game_decision` | The pending decision with its legal options, or none |
| `game_notices {since_revision}` | Notices after a revision |
| `game_report {family}` | One own-civilization report |
| `game_pedia {topic}` | Civilopedia entry; only when **Share Civilopedia text** is on |

Every response carries the snapshot's `revision`, so an answer can say which
state it refers to. If the game has moved on, the client can mark the
suggestion as stale.

## Consistency, privacy and recording

- **Inert by construction.** The feed holds no reference to the session.
  Acceptance includes byte-identical checkpoints for the same inputs with the
  advisor attached and with it off.
- **Fog test.** In an authored scenario where an opponent moves out of sight,
  the advisor's `game_map` must match the client's rendering, and the hidden
  move must not appear in any advisor response.
- **Recording (#207).** Attach and detach can be written to the recording's
  inert presentation stream. Advisor text is not recorded unless the player saves
  it as a note.
- **Both clients.** Browser and desktop must produce identical advisor responses
  for the same session state, with a shared test, plus owner acceptance on both.

## Implementation impact

- New shared MCP core crate, extracted from `totctl`. `totctl mcp` behaviour and
  tests must stay unchanged; that refactor is safe to do first, without the install.
- `totui::client` gains the snapshot publisher, advisor tools, HTTP endpoint and
  menu toggles. This probably touches `crates/totui/src/client/mod.rs`, so it
  **requires the scenario-engine baseline refresh with `CIVTOT_HOME`**. The
  final step therefore belongs on the owner's machine, or in a session with the
  install.
- `engine::human` should need no changes: the views already exist and are inert.

## Owner decisions (2026-11-12)

1. Architecture: **MCP inside each client** (rather than the feed + `totctl`
   proposal), implemented with a shared MCP core and shared advisor code.
2. **Allow advisor** lives in the **in-game menu**.
3. Civilopedia text: **yes, opt-in**, behind its own switch.
4. Several advisors at once: not decided; the default is one connection at a time.
