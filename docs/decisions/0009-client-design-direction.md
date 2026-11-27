# Decision 0009 — the original game is the client design target

Status: accepted, owner direction 2026-11-27. Read this before any client
design or presentation work. It supersedes, **for look and layout only**, the
v0.11 design proposal ([decision 0005](0005-playable-client-planning.md),
[client design](../ui/client-design.md), [prototype](../ui/client-prototype.html)),
the frame of [Layout B](../ui/layout-b.md), and the "planning basis" wording
of [decision 0008](0008-scenario-platform.md) §Decision 3. It does not change
engine, session, checkpoint or both-client requirements.

## Why this record exists

The owner reports that sessions keep defending the first client's design and
fall back to its decisions. Those decisions were **a provisional client to get
started**, not the target. The v0.11 prototype was accepted only "as a
prototype" (2026-11-06). Its own text says it "is a design change for review,
not a faithful pixel copy" and uses "our own dark palette". Every later client
milestone built on that chrome.

Layout B (2026-11-17) was meant to restore the original frame, but its design
canvas was drawn without original screenshots. Its option A note says "right
column like the original". That is wrong for Test of Time. The reference
inventory had already recorded the real frame in
[discovery U04](../ui/discovery.md): "empire figures upper left, selected-unit
details and mode across top, overview upper right, diamond map below". The
original screenshots added to the [user guide](../manual.md) on 2026-11-25
confirm U04. The right column comes from Civilization II, not Test of Time.

## Decision

1. **The original Test of Time interface is the design target**, for both
   clients. The reference is the original-game screenshots in
   [`docs/manual/screenshots/original/`](../manual/screenshots/original/)
   (shown in [the guide](../manual.md#the-look-and-feel-the-original-game)), the
   reference inventory [discovery.md](../ui/discovery.md) with its capture
   manifest, and [original controls](../ui/original-controls.md). Where a
   ToTEngine screen looks different, the difference is remaining work, not
   intent.
2. **The frame is the original's top band**, chosen by the owner on
   2026-11-27 over keeping Layout B's right column. It has a menu bar, then
   three stone-textured panels across the top: the status panel, the info
   panel and the circular overview. The full-width map sits below them.
   Layout B's right column is reclassified as provisional. Delivery:
   [#439](https://github.com/diegoami/totengine/issues/439).
3. **Layout B's opt-in conveniences remain**, as Settings options that are off
   by default. They are the order strip, message dock, tooltips, path preview,
   sorting and goal estimates. With every option off, the client should look
   like the original. The owner's wide-screen side column (2026-11-19, on by
   default from 1264 logical pixels) stays, because the original has no
   wide-screen layout. Its placement must fit the original frame rather than
   the provisional right column.
4. **Where the original gives no answer** (wide screens, browser specifics,
   QoL, modern input), the agent proposes a design with mocks next to
   original screenshots for the owner to choose. Record the result in
   `docs/ui/` and link it from this record. A design that claims to be
   "like the original" must cite the reference it was drawn from.

## Order of authority for client design

| Rank | Source | Binding for |
| --- | --- | --- |
| 1 | Explicit, dated owner decisions (list below) | What they decide |
| 2 | The original game: screenshots, discovery captures, original controls, a live reference session | Look, layout, windows, keys, workflows |
| 3 | Owner-chosen designs for gaps the original leaves open (Layout B's opt-in panels and the wide-screen column) | Those gaps only |
| 4 | [Screen contracts](../ui/screen-contracts.md) | Engine/session boundaries: decisions, filtered views, validated commands, checkpoints. Not visual layout. |
| 5 | v0.11 proposal, prototype, schematic PNGs, asset preview, and the shipped v0.12–v0.21 chrome | Historical only; never a reason to keep a layout |

The [user guide](../manual.md) describes shipped behavior. It is not a spec,
except for its original-game reference section.

### Owner decisions that stay binding

- Both a playable browser client and a standalone desktop client, delivered
  together; local browser hosting (2026-11-06).
- Immediate empty-tile moves; confirmation for attacks and captures. City
  worker and specialist edits commit immediately, like the original.
  Production pickers and buy/sell keep explicit confirmations (2026-11-06).
- Revolution Escape cancels. The government chooser is required, with no
  cancel. Rate Cancel discards the draft. Native research-list rules; the
  goal persists in checkpoints (2026-11-09).
- UI work takes priority (2026-11-15).
- Conveniences are opt-in under Settings and off by default (#266 A+B,
  reaffirmed by Layout B on 2026-11-17). The wide-screen side column is on by default (2026-11-19).
- The original-game screenshots are the guide's look-and-feel reference
  (2026-11-25), with the target frame chosen here (2026-11-27).
- Isotrident is optional fallback art for installs without ToT
  (2026-11-08); the user's original resources always take precedence.

Anything not in this list that came from the v0.11 proposal or later chrome
is provisional. Examples are button bars, sidebar buttons such as "Manage
city", the boxed full-window city layout, and the dark palette and sans
font. Change it toward the original without asking for permission.

## Visible gaps (from the committed screenshots, 2026-11-27)

These differences between `docs/manual/screenshots/original/` and
`docs/manual/screenshots/` are a starting list, not an exhaustive audit.

**Map screen.** The original has the top band of three stone panels. The
**status panel** shows population, cities/units count with the date, gold
with tax/lux/sci digits and a research icon. The **info panel** shows the
mode ("Viewing Pieces"), location and terrain, "End of Turn (Press ENTER)"
and the arrow buttons. The **circular overview** sits upper right. The map
is full width below, and the text uses a serif face. The captured menu bar
also lists Cheat and Map. ToTEngine shows a right column with sans text,
city buttons in the sidebar, a bottom status line and no Cheat/Map menus.

**City window.** The original draws floating stone panels over the map. City
Resources (top left) has icon rows for Net Food, Taxes, Luxury, Science and
Production. Build (left) has the unit picture, a progress bar, turns, and
Buy with its price. The large radius map with yield icons fills the centre.
The title panel shows name, population and treasury, with Info, Support,
Improvements, Happy and Close plus an info pane. Food Storage (bottom right)
shows Next Citizen, and the citizen strip with a portrait runs along the
bottom. ToTEngine shows a full-window boxed layout with text rows. Its radius
map uses numeric yields, and it adds Previous/Next, Save, Load and Research
buttons that the original city window does not have.

## Working rules

- Moving a screen toward the original look and layout is pre-authorized
  work. Do not ask for permission to replace provisional chrome, and do not
  defend it as an accepted design.
- Ground each claim about the original in a reference: a screenshot, a
  discovery capture or a live reference session. Read the
  [oracle README](../../tools/oracle/README.md) and [lessons](../lessons.md)
  before driving the original. Original screenshots for design work stay
  outside Git; the guide's reference set is the owner's scoped exception.
  Committed mocks draw no original art.
- Presentation changes keep the existing boundaries. There are no new rules
  or orders. Both clients change together. The real-input scripts follow the
  new chrome, and `crates/totui/src/client/mod.rs` changes refresh the
  scenario baseline (see [Layout B §Boundaries](../ui/layout-b.md#boundaries)).
- A departure from the original is an opt-in setting, off by default, unless
  the owner decides otherwise and the decision is recorded above.
- Owner acceptance on both clients still closes client milestones.
