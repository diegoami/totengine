# MCP advisor attached to a running game — design proposal

Status: **proposal for owner review; nothing implemented.** Implementation:
[#255](https://github.com/diegoami/totengine/issues/255). This is stage 2 of
[#250](https://github.com/diegoami/totengine/issues/250) as staged in
[decision 0008](decisions/0008-scenario-platform.md): an MCP client attaches,
with the player's explicit consent, to a game they are playing in `totplay`
(browser) or `totdesktop` (desktop). It **sees what the player sees** and gives
advice. It cannot act. Acting agents are stage 3 and stay behind the
complete-client gate with #92.

## User route

1. The player starts a game in either client and turns on **Allow advisor**.
   It is off by default and never remembered across launches without asking.
2. The client shows that an advisor feed is open, and while one is connected it
   shows an indicator naming it. One click turns the feed off and disconnects.
3. In Claude Desktop, Claude Code or another MCP client, the player has
   registered `totctl mcp --attach` once. The assistant can then answer
   questions such as "what should Rome build?", "why is Athens in disorder?"
   or "which research do you recommend?" from the player's current view.
4. Suggestions stay suggestions. The player carries them out with the normal
   controls; nothing the advisor says becomes an order.

## Architecture (recommended): a feed in each client, MCP in `totctl`

```text
MCP client ──stdio──▶ totctl mcp --attach ──loopback, token──▶ advisor feed (thread)
                                                                   ▲ publishes snapshots
                                                   totplay / totdesktop main loop (owns HumanSession)
```

- **Clients publish, they don't serve the session.** After every accepted input,
  decision or opponent progress, the client serializes an advisor snapshot from
  `HumanSession::view()` and the report/city views and hands it to a small feed
  thread (`Arc<Mutex<Snapshot>>`). The feed thread answers reads from that
  snapshot only, so it never locks or touches the live game, consumes RNG or
  changes the revision. The same code serves both clients. Desktop gains a
  loopback listener only while the feed is on.
- **One MCP implementation.** `totctl mcp --attach` stays a stdio server, the
  transport MCP clients launch most easily and the one already tested. It reads
  the feed and exposes advisor tools beside, or instead of, the headless tools.
- **Feed transport.** Loopback TCP on an ephemeral port, a per-launch random
  token and a size cap. Requests are read-only: the feed has no write route at
  all. The port and token are written to an owner-only file in the clients' data
  folder (`advisor.json`), which `--attach` reads. The token never appears in
  URLs or page content.

Alternatives considered:

| Option | Why not first |
| --- | --- |
| MCP over HTTP inside `totplay` | Browser only; desktop would need its own server; two MCP stacks to test |
| Exposing the browser's existing `/view` routes | They are the human client's protocol, with a page token and origin checks; the advisor needs a narrower, versioned, read-only schema |
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
resources. That includes sprites, sounds and **Civilopedia text**, which comes
from the player's own `Describe.txt` and is excluded under #250's
"no original assets" rule.

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

- `totui::client` gains the snapshot publisher and the feed thread. This
  probably touches `crates/totui/src/client/mod.rs`, so it **requires the
  scenario-engine baseline refresh with `CIVTOT_HOME`**. The final step therefore
  belongs on the owner's machine, or in a session with the install.
- `engine::human` should need no changes: the views already exist and are inert.
- `totctl mcp --attach` adds the advisor tools; the headless tools stay unchanged.

## Questions for the owner

1. Is the recommended architecture (client feed + `totctl mcp --attach`) acceptable?
2. Where should **Allow advisor** live? A setup option, an in-game menu toggle, or both?
3. May the advisor read the player's Civilopedia text? It is proposed as excluded.
4. Should one feed allow several advisors at once, or only one connection?
