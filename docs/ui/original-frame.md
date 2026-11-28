# The original's top-band map frame (decision 0009, #439)

Status: first delivery on both clients — the map screen's top band. The
floating city window is a follow-up slice (see *Remaining* below). Owner
acceptance on both clients is still open.

## Reference

- `docs/manual/screenshots/original/map.png` (1024×768): the frame this slice
  targets. The user guide's reference section is the owner's scoped exception
  for original screenshots; no original art is committed.
- `docs/manual/screenshots/original/moving-units.png` (1024×742): the Moving
  Units band with a selected unit, captured 2026-11-27 from the original
  ToTPP build (load `Original/Ph_Auto2.SAV`, accept the difficulty plate,
  resize the world window; no in-world input needed). It shows the icon,
  `Moves: 1`, the order `NONE`, the nationality `Spanish` and
  `Archers (Hills)`, with no `Loc:` line.
- [Discovery U04](discovery.md): "Menu strip; empire figures upper left,
  selected-unit details and mode across top, overview upper right, diamond map
  below."
- [Original controls](original-controls.md) for keys and modes.
- Measured from the capture: a light menu bar at the top, then a stone band
  with the status panel at the left, the info panel across the middle and a
  dark circular overview at the right; the map runs the full width below.

## What the clients now draw

Both clients draw one shared `Draw` list (`totui::client`), so a change lands
in the browser and desktop clients together.

- **Menu bar**: the original's light bar with dark text and the client's
  existing menus (Game, Kingdom, View, Orders, Advisors, World, Civilopedia).
  The serif treatment is shared by both clients.
- **Top band** (`chrome.rs`), three stone panels over a full-width map:
  - **Status panel**: population (`10,000 People`), cities/units with the
    scenario date, gold with the tax/luxury/science digits (`2 Gold 4.0.6`)
    and a drawn research flask with topic and progress.
  - **Info panel**: the mode centered (`Viewing Pieces` / `Moving Units` /
    `Opponents Playing`), the inspected tile and terrain, the active unit,
    the `End of Turn (Press ENTER)` prompt (the prompt area is the click
    target), the stack's `Activate unit N` keys and two stone scroll keys.
  - **Moving Units branch**: with a unit under orders the panel shows the
    icon with a health bar over it, `Moves:`, the order and the nationality
    beside it, and `Unit` / `(Terrain)` under it, and no `Loc:` line or
    bottom prompt, as the capture above. `Viewing Pieces` keeps the inspected
    tile.
  - **Circular overview**: the known world clipped to a dark disc, unit/city
    dots, the map view outline, and a click that recentres the map.
- **Full-width map** below the band; the map's stone surround stays visible at
  the edges. The provisional right column, its city buttons and the bottom
  status bar are gone from the map screen. The client's one-line feedback
  stays as map text over the stone surround.
- **Cities open from their tile** (single click selects; double-click opens),
  as in the original. The sidebar's `Manage city` / `Choose production`
  entries are gone; `Kingdom > Cities` still cycles city windows.
- **Opt-ins keep their owner decisions and are re-placed**:
  - the order strip stays under the map (off by default);
  - the message dock becomes a floating stone panel over the map's right side
    (off by default);
  - the new-messages overlay is unchanged;
  - the wide-screen side column (on by default from 1264 logical pixels) sits
    below the band at the right, and the map ends before it.

## Type and texture

The original's UI face reads as Times New Roman, the Windows serif of its era;
the install exposes no font resource of its own. Both clients therefore use a
Times-style face rather than a generic serif: the browser asks for Times New
Roman, then Liberation Serif / Tinos (metric-compatible clones), then DejaVu
Serif / Georgia; the desktop client loads `times.ttf` on Windows and
`LiberationSerif-Regular.ttf` on Linux, then DejaVu Serif, falling back to the
sans face when none is present. This keeps the original's widths and shapes as
closely as a system font can; it still does not claim the original's bitmap
rasterisation, which remains a gap.

The band stone is derived from the install's own `City.gif` panel crop that the
client already used for its panels (`atlas.rs`). The reference capture shows a
dark, visibly speckled band, not a smooth one: a sampled status-panel region
(`docs/manual/screenshots/original/moving-units.png`, 20,100–210,135) averages
about 60 grey with a standard deviation near 20, and the sunken info panel is
equally dark. The two variants (`band` for raised stone, `panel-mid` for the
sunken info panel and the side column) therefore keep the source speckle
around those measured mid tones. The atlas evidence
(`docs/evidence/client-atlas-v2.json`) is regenerated for the sprite hashes.

The original menu bar's entries are specified by the install's own
`Menu.txt` (`@GAME`, `@KINGDOM`, `@VIEW`, `@ORDERS`, `@ADVISORS`, `@WORLD`,
`@CHEAT`, `@MAP`, `@PEDIA`); the client's menu labels and shortcuts follow it
where a command exists (for example `Build &Road|r`, `&Fortify|f`,
`Toggle Cheat Mode|Ctrl+K`).

## Explicit remaining differences

- **Menus**: the captured bar also shows `Cheat` and `Map`. `Menu.txt` lists
  their native entries (`Toggle Cheat Mode`, `Create Unit`, `Reveal Map`, the
  scenario editors; `Select Map`, `Import Map`, `Export Map`). The client has
  no commands for them yet — no cheat/scenario-edit contract and no multiple
  map support — so they are omitted rather than drawn inert.
- **Fonts**: a Times-style system face with matching widths, not the
  original's bitmap rasterisation (above).
- **Map planes**: the original's `Loc:` line carries the plane number and the
  arrows switch planes; the client has one surface plane, so `Loc:` omits the
  number. The arrows scroll the map horizontally; that behavior is inferred,
  not measured from the reference.
- **Selected-unit details**: the branch follows the capture's stack (icon,
  health bar, `Moves:`, order, nationality, then `Unit` / `(Terrain)`), but
  the panel is our stone texture rather than the original's, and the stack's
  `Activate unit N` keys occupy the panel's right side where the reference
  leaves it empty. The order names are mapped only for the measured bytes
  (`0xff` none, `1` fortifying, `2` fortified); other orders show a dash
  until their names are decoded.
- **Panel textures**: the band uses the install's `City.gif` stone centred on
  the capture's measured tone; the exact source tile and native tiling are not
  identified, so the speckle pattern is not necessarily the same pixels.
- **Window chrome**: the original capture includes its OS window border; the
  browser/desktop stage has none.
- **City window**: still the boxed v0.14–v0.21 layout. The original's floating
  stone panels (City Resources, Build, radius map, title/panels, Food Storage,
  citizen strip) are the next #439 slice.
- **Menu-bar title**: the client's scenario/rules title and date at the right
  of the menu bar are ours; the original's window title carries them.

## Verification in this slice

- `cargo test -p totui`: 87 passed, 12 ignored (install-backed), including the
  menu/band route test and the real-input route tests.
- `python3 -m unittest discover -s tools -p 'test_*.py'`: 90 passed.
- `docs/scenario-engine-baseline.json` and
  `docs/evidence/scenario-inventory-v1.json` refreshed for the presentation
  changes in `client/mod.rs`, `chrome.rs`, `desktop.rs` and `play.js`; the audit
  still reports `matches_current_source` and no coverage change.
- Screenshots taken against the original capture at 1024×768 and on a 16:9
  stage; the real-input scripts (`tools/check_client_{browser,linux,windows}`,
  `check_city_panels`, `manual_screenshots`) now open a city from its map tile
  instead of clicking the removed sidebar button.
- The Moving Units branch has a client test: with a unit active it draws
  `Moves:`, the order, the nationality, the unit name and `(Terrain)`, and no
  `Loc:`; after `v` (Viewing Pieces) the `Loc:` line returns. The new view
  fields are `Observation::unit_orders` (own visible units only, so foreign
  orders stay hidden) and `Observation::civ_names`.
