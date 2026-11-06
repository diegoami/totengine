# Browser and desktop client: implementation audit and proposed architecture

Architecture plan for #107, 2026-11-06, audited against `373d418`. The owner
accepted local browser hosting and the both-client first scope after this audit. This completes the initial
code/dependency audit; it does not select a release version or claim that a
human client is implemented. Both #94 (desktop) and #125 (browser) are required.
Read with the [screen contracts](screen-contracts.md) and
[delivery proposal](client-milestones.md).

## Implementation progress after v0.11

The audit below is the planning baseline. The shared human driver and checkpoint
engine foundation now exist in `engine::human`; see [session contract and
checks](human-session.md). `totui::resources` provides profile Rules/art selection,
BMP/GIF decoding and the measured crop masks; see [loader status](asset-preview.md#shared-rust-loader-milestone-10-implementation).
The remaining desktop feasibility, full atlas/presentation, two adapters and
assembled human acceptance are still milestone 10 work. The design prototype
remains simulated and is not connected by this foundation change.

## Findings from the audited code

| Area | Existing implementation | Consequence for client work |
| --- | --- | --- |
| Input and recording | `Game::submit` and `Game::close_turn` in `crates/engine/src/player.rs` are private. `play_turn` synchronously calls each living civilization's `Player::act` until EndTurn or `ACTION_CAP`, then closes the round. | A browser request or desktop input cannot suspend this loop. Calling public `WorldState::apply_order` directly would omit Game's order recording and turn scheduling. Introduce a resumable driver inside the engine boundary; do not implement two client schedulers. |
| Human commands | `Order` has Move, OccupyCity, SetProduction, SetResearch, FoundCity and EndTurn. FoundCity has only a unit index. | Name entry, wait/skip, workers, specialists, buy/sell, rates and government need explicit commands/session semantics. Treat activation and viewport movement as presentation state, not fake EndTurn orders. |
| Choice timing | `research::add_research` clears a completed target; `turn::run_turn` completes all city phases, events and next-turn initialization in one call. `Game::close_turn` discards its `TurnSummary`. | Retain results and deliver required decisions. For the first slice, prompts occur at defined boundaries after atomic commands/round processing; this does not claim the original game's intra-city interruption timing. Mid-phase choices later require serializable phase continuations, rather than blocking callbacks into widgets. |
| Visibility | `WorldState::observe` builds a civilization's filtered observation. `observer::Recorder::capture` builds a spectator view plus every civilization view, with a global checksum and log-derived events. | Do not serve observer snapshots/log endpoints to the human browser and merely hide unwanted fields in JavaScript. Build a human view on the engine side; filter notifications as well as tiles/units. Retain the existing observer as a separate tool. |
| Persistence | `campaign::Campaign` checkpoints replay generated autonomous campaigns, including private built-in policy state. Restoration costs O(turns). | Reuse replay/fingerprint concepts, but a human checkpoint must also restore the current civilization, phase, pending decision and transcript. Existing campaign format is not a human save. No native SAV export is implied. |
| Asset interpretation | `tot-kit` has map/save/rules/events readers, no bitmap/sprite decoder. `tools/preview_client.py` resolves an explicit profile/root BMP/GIF policy; `asset-preview.js` delegates image decoding to the browser and crops measured examples. | Production needs shared resolution/decoding/crop semantics. The Python/JS visual pass is not a portable Rust renderer, an exhaustive atlas or proof of native fallback precedence. |
| Platforms | Engine depends only on tot-kit; lua-host is a placeholder. totui uses standard TCP/I/O for a local observer. | Native Rust hosting is a direct fit for current engine code. A browser-contained engine would need a separate WASM, file-access and persistence audit; no such build has been tested. |

These are code findings, not newly decoded original-game format claims. Existing
format and artwork evidence remains in `docs/evidence/`. Checks of this machine
found GCC but no `pkg-config` or CMake commands on PATH; SDL development/linking
and Windows packaging have not been demonstrated here. Do not count a proposed
desktop toolkit as a successful platform spike.

## Recommended first architecture

Use **one Rust human-session implementation with two presentation adapters**:

```text
installed resources -> shared resolver/decoder -> atlas + presentation data
                                                 |                 |
                                        native desktop        browser Canvas/DOM
                                                 |                 |
                                            direct calls      loopback protocol
                                                 |                 |
                                          shared human session
                                      validated views / commands
                                           engine + opponents
```

Recommend a **local Rust host for the first browser client**, launched as an
ordinary supplied executable which opens the local browser address. It owns the
engine session, resource access and checkpoint files. This is actual browser
play, with the human using the browser for all game input. It is not a hosted
website, multiplayer server or requirement to run Python/development tools.
An independently hosted/download-free browser edition remains an additional
delivery decision; do not claim it from this architecture.

Recommend **a native SDL2 window for the desktop feasibility spike**, calling
the same Rust session directly. It must run without a browser or local HTTP
server. SDL2 was #94's original candidate and fits sprite/window/input work;
menus, text entry, focus, scaling and dialogs still need implementation and
testing. It does not supply a complete game interface. The toolkit becomes an
implementation choice only after the spike passes both Windows and Linux
acceptance; failure should trigger a focused alternative evaluation before
production code depends on it. No toolkit dependency is added by this document.

Upstream sources checked for this recommendation: [Rust-SDL2](https://github.com/Rust-SDL2/rust-sdl2)
documents its MIT bindings and system/bundled build arrangements;
[SDL installation](https://wiki.libsdl.org/SDL2/Installation) documents Linux
and Windows setup; [SDL licensing](https://www.libsdl.org/license.php) identifies
SDL 2.0 and newer as zlib licensed. Retain the actual dependency notices and
audit any image/font dependencies when selected. This is a feasibility
recommendation, not code borrowed from other game recreations.

Browser and desktop share rules, legality, session/decision state, resource
interpretation and checkpoint semantics. Each still needs its own widgets,
input/focus, text layout, file dialogs and packaging tests. Reusing sprite data
does not mean that Canvas and SDL draw calls are shared. Keep the engine free
of window, HTTP, installation-path and image-decoder dependencies.

## Shared session contract to implement first

The following names are proposed, not APIs that currently exist:

- `HumanSession::view()` returns the human's current observation, phase,
  revision, filtered notices and any pending decision. Reading it does not
  advance gameplay or consume randomness.
- `submit(revision, request_id, command)` validates ownership, phase, entity
  identity and legality, then records accepted gameplay through the common
  order boundary. Reject wrong/stale submissions without gameplay mutation.
  Duplicate request IDs return the recorded response without executing twice.
  An explicit EndTurn may advance opponents; polling or reconnecting may not.
- `advance(budget)` performs bounded opponent/phase work and returns progress,
  a decision or human input. The budget limits work per call, never the human's
  thinking time. No busy loop or automatic EndTurn represents waiting.
- Decision responses carry a stable ID, revision and allowed typed choice.
  Founding uses an explicit validated name; research choices use engine legal
  lists. Reject an unrelated map command while a required choice is pending.
- Checkpoints bind supported format version, setup/import identity, Rules and
  resource identities, human civilization, command/decision transcript, current
  phase/decision, and opponent policies/private state. UI viewport/preferences
  are separate from gameplay. Restore into a temporary session and replace the
  active one only after complete validation.

Keep autonomous `play_turn` semantics and existing order logs replayable while
adding this driver. Do not widen the normal `Player` observation or give it the
world/RNG. New human-only decision data is still filtered for its civilization;
it does not create the trusted AI compatibility driver rejected as an implicit
amendment of decision 0003.

Round-boundary choice delivery is a first-slice engine limitation. A discovery
occurring in one city's processing must not silently acquire a human-selected
new target for the remaining cities. Define and test what happens to subsequent
science before offering the choice; report that behavior to the player and
reviewer. If this makes the proposed first slice misleading or unusable, split
city-phase suspension into its prerequisite rather than hide it behind the UI.

## Assets and transport

Extract a Rust resource resolver with an explicit supported profile policy and
provenance. Reproduce case-insensitive matching, ambiguity/missing-resource
diagnostics and measured profile/root selection from the prototype as **our
policy**, not an assertion about all original scenario search paths. Resolve
Rules, artwork and scenario identities consistently; do not mix an imported
scenario's units with unlabelled standard art. Unsupported layouts get a
specific diagnostic until measured and supported.

Decode measured BMP variants and required GIF fallback into common RGBA/crop
data. Derive fixtures/checks from owned files, retaining only numeric results or
our own synthetic images in Git. Include transparency, owner recoloring,
font/text and adjacency work in the production rendering backlog; the five
sample crops do not prove them. Browser and desktop compare numeric crop/mask
results and render the same engine-known map with the same tile geometry.

The browser host serves only its client, selected assets and human-session
protocol on loopback. Mutation requests need a per-launch token, same-origin
checks, bounded input and revision/request validation; no mutation through GET.
Two tabs and duplicate/retried requests must not issue extra orders. A reconnect
retrieves the pending decision; it does not restart or advance the game. Never
expose arbitrary installation paths, the spectator feed, complete world data or
private replay/policy state as human-client endpoints. The read-only prototype's
HTTP server is not sufficient for this mutation protocol.

## Concrete next work and evidence

These are implementation stories for the owner-accepted
[first-slice scope](client-milestones.md#proposed-first-slice-for-owner-review).
They are prerequisites of both client umbrellas, not separate releases:

1. **Desktop feasibility:** open/close/resize a normal Windows and Linux window;
   draw owned Original-profile terrain/unit/city art; type and edit a city name;
   navigate a modal by keyboard; handle focus loss and scaling. Record build,
   runtime dependencies and distribution/notices. Synthetic-only CI plus a
   private installed-art run; no original assets in packages. No engine fork.
2. **Shared human driver:** pause for an order, founding and research; reject
   wrong owner/stale/duplicate responses; run built-in opponents and return to
   human input. Retain turn results and test replay/RNG equality without a
   window. Exercise a discovery during city processing, not only at setup.
3. **Shared resources and human views:** build actual fog-filtered map/unit/city
   views and consistent profile resolution/decoding. Compare both consumers;
   verify hidden tiles/foreign private fields never enter the human payload.
4. **Browser adapter:** launch packaged host, select installation/start game,
   move/found/choose research/end turn through browser input; test duplicate
   submits, reconnect and missing/unsupported resources. No dev server required.
5. **Desktop adapter:** the same gameplay route in its own application window,
   with keyboard/mouse/focus and installation errors. Compare canonical engine
   commands/results to the browser route; no HTTP/browser launch dependency.
6. **Human checkpoint and assembled acceptance:** save/resume before and during
   required choices on both surfaces, reject invalid imports atomically, and
   complete the first-slice human route with retained replay results. Only then
   consider a candidate for that slice's independent review.

The toolkit spike and resource audit can inform each other; both adapters depend
on the shared driver and views. Do not deliver one and quietly remove the other.
Full city economics, government/diplomacy, reports, scenarios/multiple maps and
sustained campaign usability remain subsequent packages in the delivery plan.
