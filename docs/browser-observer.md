# Browser observer (#80 / #81)

The browser watches AI v0 by default using the existing player API (decision 0002).
Add `--ai-v1` for the AI v1 expansion, economy, research and unit policy.
It renders the surface map, living units and cities with original geometric
tiles. It needs no JavaScript build, CDN, original artwork or npm packages.
The CLI still requires your own save and its sibling `Rules.txt`; a sibling
`Events.txt` is loaded as for `totctl play`.

```sh
cargo run -p totctl -- observe "$CIVTOT_HOME/Original/Ph_Auto2.SAV" --turns 100 --seed 3
```

Open the printed loopback URL (default `http://127.0.0.1:8000`). Use the
arrows to step, Play/Pause, the speed selector, Restart, or the timeline.
A request for the next uncached frame advances one complete engine turn;
pause stops new requests. An in-progress turn finishes. Previously generated
frames are cached, so stepping backward never runs the AI again. Multiple
browser tabs share one game. `--port 0` chooses a free port.

Select Full map or Civilization 1–7. Fog comes directly from
`WorldState::observe`: seen terrain persists, foreign units require current
sight, and known foreign cities remain visible. These are the engine's
visibility rules (decision 0003), not newly decoded original-game rules.
Point at the map to inspect terrain, city names and unit types. The details
panel lists canonical orders/results and turn checksums, including rejected
orders; it does not enumerate every internal production/event effect.

Save replay so far downloads all frames fetched by that tab, including all
fog views and the spectator view. Open replay loads that NDJSON locally in
the viewer; it does not upload to the server. The order-log link downloads
all orders executed by the server so far and is hidden during archive playback. A spectator archive contains the
full map by design; fog selection is a viewing control, not access control.

To generate a finished archive and canonical order log without a server:

```sh
cargo run -p totctl -- observe "$CIVTOT_HOME/Original/Ph_Auto2.SAV" \
  --turns 100 --seed 3 --stream /tmp/game.ndjson --log /tmp/game.log

# Watch a finished game's verified order-log replay:
cargo run -p totctl -- observe "$CIVTOT_HOME/Original/Ph_Auto2.SAV" \
  --replay /tmp/game.log

# Reconstruct its observation stream (must match the first byte for byte):
cargo run -p totctl -- observe "$CIVTOT_HOME/Original/Ph_Auto2.SAV" \
  --replay /tmp/game.log --stream /tmp/replayed.ndjson
cmp /tmp/game.ndjson /tmp/replayed.ndjson
```

Output files must not already exist. Replay uses the seed in the order log;
`--seed` and `--turns` apply only to live games. Reconstructing from a log
needs the same save, rules, events and engine build. NDJSON playback needs
only the archive and the viewer; it displays recorded frames without
re-simulating or certifying their provenance. Archives and logs are local
user data: do not commit them from original-game saves.

## Stream v1

`engine::observer::Recorder` captures boot, then each completed turn, without
changing world state or random generators. Each NDJSON line is a self-contained
snapshot: schema 1, turn, year, hex checksum, stored width/height, terrain/unit
names from the loaded rules, eight views and event strings. View 0 is the
spectator (`civ: null`); views 1–7 use those civ IDs. Tiles in stored order
are `null` or `[terrain, river, road, rail, special-suppressed]`. Logical coordinates satisfy
`x % 2 == y % 2`, with index `y * width + x/2`. Units carry stable slot ID,
type, owner, coordinates and veteran status; cities carry slot ID, name,
owner, coordinates and size. The initial frame has no events. Later frames
include the canonical log records since the previous capture.

`Game::replay_observed` invokes the recorder after boot and after each
verified turn; failed replays are discarded by the CLI. Observer replay
requires a log ending at a completed turn (or the boot header alone). The server binds
only to IPv4 loopback and serves embedded assets and in-memory data using
exact routes. It keeps history in memory, so very long games or large maps
can consume significant memory. This first observer has no orders UI and
shows only the engine's surface map.

## Validation

`cargo test --workspace` includes a synthetic 100-turn AI stream/replay
comparison, existing fog-boundary comparisons, and cached-frame checks.
With `CIVTOT_HOME` set, `crates/totctl/tests/observer_install.rs` additionally
exports 100 turns with `--ai-v1` on `Original/Ph_Auto2.SAV`, reconstructs the stream from
its order log, compares every byte of all 101 frames, and checks overwrite
refusal. `make test fmt clippy` also checks the existing golden corpus.

The optional real-browser smoke test needs Playwright and Chromium, installed
outside this dependency-free workspace, for example:

```sh
npm install --prefix /tmp/totengine-browser playwright
PLAYWRIGHT_BROWSERS_PATH=/tmp/totengine-browser/browsers \
  /tmp/totengine-browser/node_modules/.bin/playwright install chromium
# With an observer running for 100 turns:
NODE_PATH=/tmp/totengine-browser/node_modules \
  PLAYWRIGHT_BROWSERS_PATH=/tmp/totengine-browser/browsers \
  node tools/test_observer_browser.cjs http://127.0.0.1:8000
```

It checks live stepping, fog rendering, play/pause, restart during a seek, all 100 turns,
replay download/import, malformed archive rejection, and identical canvas pixels
on the final frame.
It writes a screenshot and replay to a temporary directory printed on success.
