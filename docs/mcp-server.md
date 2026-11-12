# MCP server — headless read-only slice (#250, stage 1)

Status: **implemented as a tooling slice; not released, not owner-accepted.**
This is stage 1 of the [MCP agent interface](https://github.com/diegoami/totengine/issues/250)
as staged in [decision 0008](decisions/0008-scenario-platform.md): read-only
inspection and existing-policy runs over a local transport. Client-attached
advisory access (stage 2) and acting agents (stage 3, behind the complete-client
gate with #92) are not part of this slice.

## Running

```sh
totctl mcp [--root PATH]...
```

Speaks MCP over the stdio transport (newline-delimited JSON-RPC 2.0; protocol
versions 2024-11-05, 2025-03-26 and 2025-06-18 are negotiated at `initialize`).
stdout carries protocol messages only; diagnostics go to stderr. The transport
is local by construction, per the issue's constraint.

**Filesystem reads are bounded to roots**: each `--root` is canonicalized at
startup; when no `--root` is given, `CIVTOT_HOME` is used if it names a
directory. Every requested path is canonicalized and must resolve inside a
root; with no roots at all, file tools reject every request. The server
**never writes** to the filesystem — campaign checkpoints and order logs come
back inline (see `campaign_run`), so attaching a client cannot touch owner
previews, games or saves.

## Tools

| Tool | Arguments | Equivalent CLI |
| --- | --- | --- |
| `rules_summary` | `path` | `totctl rules <Rules.txt>` |
| `map_summary` | `path` | `totctl mp <file.mp>` |
| `events_summary` | `path` | `totctl events <Events.txt>` |
| `save_summary` | `path`, `rules?`, `turns=0`, `seed=1` | `totctl boot <file.sav\|scn>` |
| `run_turns` | `path`, `turns=50`, `seed=1`, `rules?`, `no_rules=false` | `totctl run <file.sav\|scn> --turns N` |
| `campaign_run` | `rules`, `policy?`, `turns=100`, setup options (`seed`, `width`, `height`, `land_percent`, `civs`, `difficulty`), `resume_checkpoint?`, `include_checkpoint?`, `include_log?` | `totctl campaign <Rules.txt> ...` |
| `campaign_observe` | as `campaign_run` (no `include_*`), plus `view="all"`, `from?`, `count=1` (max 8) | `totctl campaign <Rules.txt> ... --stream OUT` |
| `recording_info` | `path` | `totctl record info <bundle.totrec>` |
| `recording_verify` | `path`, `rules` | `totctl record verify <bundle.totrec> <Rules.txt>` |
| `recording_log` | `path`, `from_turn?`, `to_turn?`, `civ?` | lines of the run's `totctl campaign --log` file |
| `session_start` | as `campaign_run` setup, or `resume_checkpoint` | `totctl campaign` report of the starting state |
| `session_advance` | `session`, `rounds` (1..1000) | `totctl campaign --turns k` report after k rounds in total |
| `session_observe` | `session`, `view="all"` | one observer v1 frame (events since the previous call) |
| `session_checkpoint` | `session`, `format="checkpoint"\|"recording"` | `--checkpoint` text, or a `.totrec` recording |
| `session_close` | `session` | — |
| `worldgen_summary` | `seed?`, `width?`, `height?`, `land_percent?` (no path; nothing written) | `totctl worldgen <out.mp> ...` minus its final `map` line |

Every tool's first result block is byte-identical to the equivalent CLI stdout:
the tools and the CLI commands share the same report functions
(`crates/totctl/src/main.rs`). `campaign_run` adds the checkpoint text as a
second block when `include_checkpoint` is set (what `--checkpoint` would write)
and the order log as a final block when `include_log` is set (what `--log`
would write); `resume_checkpoint` takes a checkpoint text from a previous run
and rejects setup options, mirroring the CLI's `--resume`. Sibling `Rules.txt`/
`Events.txt` discovery next to a save behaves exactly like the CLI (the parent
directory is inside a root whenever the save path is).

### Recordings (`recording_*`)

`recording_info` and `recording_verify` return exactly what `totctl record
info|verify` print for a [recording bundle](recording-bundles.md). `recording_log`
returns the canonical order log of a recorded campaign without replaying it,
sliced by turn and optionally by civ: `T <turn> C <civ> …` lines plus the
`R <n> <checksum>` lines closing turn n−1, always with the 3 header lines. The
unfiltered result is byte-identical to the run's `--log` file. It is full-state
content: every civilization's orders. Recovery writes a file, so it stays
CLI-only (`totctl record recover`).

### Live sessions (`session_*`)

Each `campaign_*` call re-simulates from setup or from a checkpoint. Sessions keep
a campaign in memory instead, so an assistant can explore step by step: start,
advance a few rounds, observe, advance again. Determinism is unchanged: after k
rounds in total, in any split, `session_advance` returns the report
`totctl campaign --turns k` prints, and `session_checkpoint` returns the same
checkpoint text. A session resumed from that checkpoint continues identically.
`session_observe` uses one recorder per session, so a frame's events are those
since the session's previous observation. `session_checkpoint` with
`format: "recording"` returns a `.totrec` bundle (stop reason `stopped`) that
`recording_verify` and `totctl record verify` accept. At most 4 sessions are
open; they live only in the server process and end with it or with
`session_close`. Nothing is written to disk.

### Observation (`campaign_observe`)

`campaign_observe` runs or resumes a generated campaign like `campaign_run`
and returns [observer stream v1](browser-observer.md#stream-v1) frames: frame 0
is the state the call starts from, frame *k* the state after round *k*. The
first block is the campaign report (identical to `campaign_run`), then one
block per frame in the window `from`..`from+count` (default: the last frame).

- `view: "all"`: each frame block is byte-identical to the corresponding line
  of `totctl campaign ... --stream OUT`. An all-views frame of the default
  80×50 world is about 130 KB, hence the 8-frame window.
- `view: "spectator"`: only the full-state view. This is a spectator archive
  product, explicitly full state, not a player observation.
- `view: "1"`..`"7"`: only that civilization's fogged view, which is the
  observation the engine gives that civ's player. Every other frame field
  (turn, year, checksum, events) is unchanged.

Observation never alters the run: the report, checkpoint and log match a run
without it, and a resumed observation reproduces the uninterrupted run's frame
checksums. Like `campaign_run`, each call re-simulates from setup or from the
supplied checkpoint; there is no server-side session state.

Errors are reported as tool errors (`isError: true`) with the CLI's message
text; malformed requests are JSON-RPC errors. Unknown arguments are rejected
(`deny_unknown_fields`).

## Tests and acceptance

`crates/totctl/src/mcp.rs` unit tests cover the handshake and version
negotiation, notification handling, the tool list, path bounding (escape and
no-roots rejection), argument validation, the NDJSON serve loop, and campaign
byte-identity: a 3-turn run resumed for 5 turns from the inline checkpoint
reaches the same fingerprint as a single 8-turn run, and the tool text equals
the shared report the CLI prints. The CLI refactor behind this slice was
verified byte-identical against the pre-refactor binary (rules, mp, campaign,
resume, error paths, checkpoint/log writes). The observation follow-up was
checked the same way for `worldgen` (stdout and map bytes) and `campaign`
(builtin-1 and ai-v1, checkpoint/log/resume, round-limit and overwrite errors)
over three seeds.

Session tests check that 3 + 4 rounds equal one 7-round run byte for byte
(report and checkpoint), that a resumed session continues identically,
per-view observation and events-since-last-call, exported recordings verifying
through the recording tools, the 4-session limit, and argument errors. A stdio
test drives a session across messages in a real `totctl mcp` process.
`recording_log` tests check the unfiltered slice against the `--log` file and a
turn/civ window.

Further unit tests cover `campaign_observe` windows, the default last frame,
per-civ and spectator filtering, resumed-frame checksums and argument errors;
`builtin-1` runs and resumes through the tool using the authored
`builtin-settings.txt` fixture; and `worldgen_summary` matches the shared
report, also with no roots configured. `crates/totctl/tests/mcp_stdio.rs`
spawns `totctl mcp` over real stdio and checks that its frames equal a
`totctl campaign --stream` file and its worldgen summary equals `totctl
worldgen` stdout.

The issue's headless acceptance — *an MCP client inspects a corpus entry and
runs/resumes a seeded campaign byte-identically to `totctl`* — is met for the
generated-campaign corpus by the tests above; **inspection of install corpus
entries (`.SCN`/`.SAV` walks) through the MCP transport is not yet evidenced
here** (no `CIVTOT_HOME` in the authoring environment) and follows the usual
install-backed check.

## Not in this slice

Client attach (loopback advisory access to `totplay`/`totdesktop`, #255; design
proposal in [mcp-client-attach.md](mcp-client-attach.md)), observer
queries over `.SAV`/`.SCN` games and the scenario inspector (#254, which needs
install-backed tests), an agent playing a civilization (#256) or delegated
actions in a human game (#257), recording bundles (#186), author-assistance lint tools (#208), acting/delegated
orders, and any original-asset responses. No milestone, version or release is
assigned; owner acceptance and the independent review process are unchanged.
