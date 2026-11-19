# Layout B: classic frame with optional panels

Owner decision, 2026-11-17: of the three layout options on the design canvas
(A classic, B classic plus optional panels, C full-window map), the owner chose
**B** ("go with B"). It refines decision A+B from #266: the original-faithful
frame is the baseline, and the conveniences stay opt-in under Settings, off by
default. With every option off, B looks like A.

The canvas (private to the owner) holds the mock boards: a wireframe of the
v0.20.0 map screen, then map, city and science boards for each option. The
mocks use flat drawn tiles only; no original artwork was published or
committed. Screenshots of the v0.20.0 client stayed outside the repository.

## What changes

Today's map screen spreads commands over two button bars. The top bar mixes a
mode (Viewing/Moving) with actions (Found, Save, Load). The bottom bar mixes
game screens (Reports, Civilopedia) with application actions (New game, Close
client). The sidebar stacks text and buttons of uneven width, and there is no
minimap. Layout B replaces this with one frame:

| Area | Baseline (always) | Optional (Settings, off by default) |
|---|---|---|
| Top | Menu bar: Game, Kingdom, View, Orders, Advisors, World, Civilopedia. Every current command lives in a menu with its key shown. | — |
| Map | Map fills the area left of the right column. | Tooltips and path preview (already shipped in v0.20.0). |
| Under the map | — | Order strip: buttons for the active unit's available orders, with keys. |
| Right column, top | Minimap of the known world with the view rectangle; a click recentres. | — |
| Right column, middle | Civilization status: name, year and turn, treasury, tax/lux/sci rates, research topic and progress. Active unit box: name, veteran, moves, home city, terrain, others on the tile. | — |
| Right column, bottom | "End of Turn (Press ENTER)" prompt when no unit can move; an End turn button. | Message log dock: the last turns' notices, and a link to the full log. |
| Side column (16:9 and wider, #337) | — | On by default when the stage is at least 1264 logical pixels wide (owner, 2026-11-19): the latest turn's messages wrapped in full, earlier ones newest first, and All messages. Replaces the dock and the new-messages overlay; the map is narrower by the column. City window and dialogs unchanged. |
| City | One window over the dimmed map: title with Previous/Next, citizens strip, resources, radius map, food and production boxes with Buy/Change, improvements, supported and present units. | Tile tooltips, sorting in the side panel. |
| Science advisor | One window: portrait slot (from the install), research progress, goal, known advances. | Turn estimates for the goal path, sortable "available next" list. |

## Boundaries

- Presentation only. No new orders, rules, checkpoint, receipt or recording
  changes. Menus and the order strip list only commands the client already
  has; orders the engine does not support yet (fortify, sentry, terrain work)
  are not shown until they exist.
- Keys stay as they are; menus show them.
- Both clients (browser and desktop) land together, as for every client
  milestone.
- The real-input scripts (`tools/check_client_*.{cjs,py,ps1}`) follow the new
  chrome and are run to completion (lesson from #307).
- `crates/totui/src/client/mod.rs` is baseline-tracked: each PR that touches it
  refreshes `docs/scenario-engine-baseline.json` and the inventory.

## Delivery

GitHub milestone "Layout B: classic frame with optional panels — both
clients": #310 menu bar, #311 right column (minimap, status, unit box), #312
optional order strip, #313 optional message dock, #314 city window, #315
science advisor window, #316 both-client acceptance and real-input checks.
Suggested order: #310 and #311 together (they replace the same chrome), then
#314, #315, #312, #313, and #316 last.
