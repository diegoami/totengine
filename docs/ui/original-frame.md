# The original's top-band map frame (decision 0009, #439)

Status: delivered on both clients — the map screen's top band and the
floating city window. Owner acceptance on both clients is still open.

## Reference

- `docs/manual/screenshots/original/map.png` (1024×768): the frame this slice
  targets. The user guide's reference section is the owner's scoped exception
  for original screenshots; no original art is committed.
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

## City window (second slice)

The city screen floats stone panels over the map, as the original:

- **City Resources** top left: Net Food, Net Taxes, Net Luxury, Net Science
  and Net Production with their own markers, plus the trade/happy/upkeep
  detail the Layout B window showed.
- **Build** at the left: the current item, progress bar, the unit picture,
  shields and turns, and `Choose production` / `Buy production` with its
  price / `Improvements / sell`.
- **Radius map** in the centre, enlarged toward the original's size, with the
  worker markers, the selected-tile outline and per-tile yields.
- **Title** top right: the name, `Population N`, `Treasury: N Gold` and the
  PgUp/PgDn city keys.
- **Info / Support / Improvements / Happy** stacked keys and the information
  pane at the right, with the same lists, sorting, paging and sale
  confirmations.
- **Food Storage** bottom right: the bar, the store and `Next Citizen`.
- **Citizen strip** along the bottom: portrait, worker/specialist counts and
  the specialist role keys.

Every Layout B panel, command and immediate-edit rule is kept; the footer
Save/Load/Research and Previous/Next city buttons are gone, as the original's
city window does not show them (Ctrl+S/Ctrl+L, F6, PageUp/PageDown and the
menus keep those commands). As in the capture, the menu bar and the band's
first lines (status, `Loc:`, the centre prompt) stay visible above the city
surface; the surface then covers the band before the panels begin. The cut
sits on a band text line boundary (`city::CITY_TOP`) so no prompt is left
half-visible between panels; a client test pins that.

Gaps: the install exposes no resource or improvement icon sheet here, so the
City Resources markers and tile yields are drawn or numeric rather than the
original's sprites; the city bitmap font is the same gap as the map screen;
the exact panel stone crop is approximated as on the map screen.

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
- **Selected-unit details**: the original's documented layout ("selected-unit
  details and mode across top") places more detail in the info panel than the
  current unit name, moves and health lines. The full branch needs a reference
  capture with a selected unit.
- **Panel textures**: the band uses the install's `City.gif` stone centred on
  the capture's measured tone; the exact source tile and native tiling are not
  identified, so the speckle pattern is not necessarily the same pixels.
- **Window chrome**: the original capture includes its OS window border; the
  browser/desktop stage has none.
- **Menu-bar title**: the client's scenario/rules title and date at the right
  of the menu bar are ours; the original's window title carries them.

## Verification in this slice

- `cargo test -p totui`: 87 passed, 12 ignored (install-backed), including the
  menu/band route test, the city-window panels and the real-input route tests.
- The desktop client rendered the band, the research dialog and the serif TTF
  path under Xvfb + xfwm4. Automated desktop input was blocked when the client
  started before any window manager; with xfwm4 running first, XTest input
  reaches the window, and the city route (load a checkpoint, double-click the
  city tile, inspect the panels, choose production) was driven and captured on
  the desktop client too. The initial-focus guard still applies to a client
  launched without a window manager.
- The retained packaged-client fixtures are version-bound; the panel/selection
  scripts were updated to the new route and text but were not rerun here (the
  retained v0.15 checkpoints do not load in this engine version).
- `python3 -m unittest discover -s tools -p 'test_*.py'`: 90 passed.
- `docs/scenario-engine-baseline.json` and
  `docs/evidence/scenario-inventory-v1.json` refreshed for the presentation
  changes in `client/mod.rs`, `chrome.rs`, `desktop.rs` and `play.js`; the audit
  still reports `matches_current_source` and no coverage change.
- Screenshots taken against the original capture at 1024×768 and on a 16:9
  stage; the real-input scripts (`tools/check_client_{browser,linux,windows}`,
  `check_city_panels`, `manual_screenshots`) now open a city from its map tile
  instead of clicking the removed sidebar button, and locate the city title
  and panels by content rather than the old fixed coordinates.

## Acceptance checklist (#439)

Map screen:
- [x] menu bar with the client's menus;
- [x] top band: status panel, info panel, circular overview over a full-width
      map;
- [x] provisional right column, sidebar city buttons and bottom status bar
      removed; feedback stays as map text;
- [x] cities open from their map tile (double-click), the original's route;
- [x] opt-ins preserved and re-placed; wide-screen message column at the right;
- [ ] `Cheat` and `Map` menu entries (the client has no commands for them);
- [ ] the original's bitmap font (serif system face approximated);
- [ ] the selected-unit info branch from a reference capture;
- [ ] `Loc:` plane number and the arrows' measured behavior.

City window:
- [x] floating stone panels: City Resources, Build, radius map, title/info
      pane, Food Storage, citizen strip;
- [x] every Layout B list, page, sort, sale confirmation and immediate
      worker/specialist edit kept; footer-only buttons removed with their keys
      and menus retained;
- [ ] the install's resource/improvement icons (drawn markers and numeric
      yields instead; the classic-art branch #380 would supply icons);
- [ ] the exact panel stone crop.

Both clients:
- [x] one shared Draw list; browser walkthrough at 1024 and 1365;
- [x] desktop client renders the band, dialogs and serif TTF path under Xvfb;
- [x] desktop city route driven with real X input under Xvfb + xfwm4 (the
      window manager must start before the client);
- [ ] owner acceptance on both clients, including the final side-by-side
      comparison.
