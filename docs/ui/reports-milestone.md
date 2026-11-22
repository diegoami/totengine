# Milestone 19 — own-civilization reports on both clients (#201, #202)

The eight report families and Civilopedia shipped in v0.19.0. Milestone 19
still needs owner acceptance on both clients. The scope is the report fields
the engine computes and verifies, shared by the browser and desktop clients.

## Admitted families

| Family | Key | Columns | Source |
| --- | --- | --- | --- |
| City status | F1 | city, size, food surplus, shields, science, tax, luxury, build item, shields stored/required, turns | `engine::human::city_status` from the filtered `HumanView` (city economy, production view) |
| Defense | F2 | unit type, attack, defense, hit points, firepower, movement, active count, in-production count | `engine::human::defense`: own units and own cities' builds, with the loaded rules' unit statistics |
| Attitude (tab *Mood*) | F4 | city, size, happy, content, unhappy citizens, entertainers, tax collectors, scientists | `engine::human::attitude`: `city_economy` mood plus the citizen view's specialist roles; content is the remainder |
| Trade | F5 | city, credited taxes, science, luxury, improvement maintenance, improvement count; totals row (taxes, upkeep, net, science, luxury, treasury) | `engine::human::trade` |
| Wonders | F7 | wonder, host city | `engine::human::wonders`: own wonder hosts only, names from the `@ENDWONDER` row comments (physical row ids); expiry and foreign wonders are not shown |
| Ranked cities | F8 | rank, city, civ, size, wonders | `engine::human` public figures: the world's five largest surface cities, ties in slot order. Public in the original (U23), so foreign cities are listed |
| Score | F9 | civ, score, advances, population, treasury | `WorldState::power_scores` (the native-decoded power score) with its standard components (3×advances + 8×population + treasury÷32); scenario-objectives mode is reported in the footer |
| Demographics | F11 | category, you, best, best civ, rank | public figures per living civilization: population, cities, military units, advances, treasury |

Field sets follow the inspected original reports (discovery U05 city status, U23
defense/trade/ranked/demographics, U24 score). **Layout, column order, labels and
the sort/paging controls are ours**: the original's exact column set, sorting and
drill-down behavior were not traced, so no native-parity claim is made (the
demographics categories in particular are our own selection). The separate
F3 known-contact panel belongs to milestone 18. The casualty timeline,
supply/demand and Spaceships (F12) remain outside this package.

## Rules for the views

- The public families (F8/F9/F11) go through `HumanSession::any_report` and use
  the world's *public figures* — the data the original itself reports for every
  civilization (U23/U24): city sizes for the ranking, the native power score
  and its components, population/cities/units/advances/treasury for
  demographics. They never read private foreign state beyond those public
  figures, and are exactly as inert as the own-civ views (test
  `public_reports_list_every_civilization_and_stay_inert`).
- Built only from the human's own filtered `HumanView` and the loaded rules, so a report
  cannot expose foreign hidden data; `HumanSession::report` journals nothing, consumes no
  randomness and bumps no revision (test
  `reports_are_inert_own_civ_views_that_agree_with_the_quote`).
- Trade totals agree with `HumanSession::preview_rates` for the current split (taxes,
  maintenance and science), and credited taxes follow the treasury rule (none in
  Anarchy or disorder). The improvement count includes the Palace.
- Rows carry the stable own-city `Entity` link the client uses to drill down.
- Presentation (sort column/direction, selection, page, which report is open) is client
  state, not checkpointed; reload returns to the map and rebuilds equivalent data.

## Client behavior (shared controller, `totui/src/client/reports.rs`)

- F1/F2/F4/F5/F7 or the **Advisors** menu open a report; F1/F2/F4/F5/F7 or the tabs switch
  family. Reports are blocked while a required decision is pending.
- Up/Down select, PageUp/PageDown page (12 rows), **S** cycles the sort column, a header
  button sorts that column and flips direction on a second click; rows sort case-insensitively
  for text and numerically for numbers, ties keep ascending city-slot order.
- **Enter** (or *Open city*) on a city row opens the city workspace; **Esc** there returns
  to the same report, family, sort and selection. Defense rows have no city and say so.
- Esc closes. Orange rows are cities in disorder. Long names are clipped with an ellipsis.

## Evidence and acceptance

- Engine and client unit tests; the recorded 15-step route `generated_reports_route`
  (F1/F2/F4/F5/F7, sort, header click, drill-down and return, toolbar button) replayed through real
  SDL (Xvfb/XTest) and Chromium input with byte-identical checkpoints before and after —
  inspection must not change the game (`tools/check_government_route.{py,cjs}`, reused runner).
- Not claimed: Windows input, packaged archives, large-list/small-window behavior on real
  input (paging is unit-tested only), custom-scenario labels beyond the loaded rules,
  owner acceptance, independent review.

## Civilopedia lookup (slice 3)

Reference text comes from the selected profile's own `Describe.txt` (the profile folder first,
then the install root), so every shipped ruleset and scenario folder shows its own
descriptions. The file documents its format in its header: `@NAME_DESCRIPTIONS` sections
(advances, improvements, wonders, units, terrain and resources, governments, concepts), an
`@@NAME_INDEX` block mapping rules entry order to a description number (`-1` none, `-2`
terminator), `@@Title` entries with blank-line separated paragraphs, `;` comments and a
terminator line. `tot_kit::Pedia` decodes it without rewriting text. Findings across the
nine files in the reference install ([structure summary](../evidence/pedia-structure-v1.json),
counts and hashes only, produced by the independent `tools/pedia_inventory.py` and
checked by `crates/tot-kit/tests/pedia_install.rs`):

- All nine files parse; each has the seven documented sections. Several files omit the
  trailing comma on some index lines and indent others, so the comma is optional in the
  decoder (the native loader's tolerance is unverified).
- Index values past the entries of a section are tolerated as "no description" (none exist in
  the reference install; the decoder counts them).
- Description text is almost entirely ASCII; the decoder uses the Rules decoder's Latin-1 policy.
- The text is copyrighted and is never committed: tests use authored synthetic text, and the
  real-install recorder reads it at run time only.

The **Civilopedia** menu opens
category tabs, a paged title list and a scrollable reading view (Left/Right category, Up/Down
select or scroll, PageUp/PageDown, Enter reads/goes back, Esc steps back then closes). In the
required research prompt a **Civilopedia** button describes the highlighted advance.
Enter or Escape returns to the same research choice, page and keyboard focus. The
footer names the selected file, including the root fallback when used. The open
demo reports the lookup as unavailable. Generated and legacy format-2 checkpoints
keep their original resource identity. Format-3 scenario checkpoints bind the
selected text in their dependency manifest. Word wrap is approximate (a fixed character width), not the
original's layout. Not covered: media, hyperlinks between entries, search, unit/terrain
pictures, and the "About" entry.

Evidence: unit tests plus a recorded 15-step real-input route (`generated_pedia_route`)
replayed through SDL and Chromium with byte-identical checkpoints.

## Scenario report and reference walkthrough

Use a private install copy containing the root and Original image and text
resources. The fixture author adds a new `Scenario/AuthorReference` directory
with an authored save and an authored `Describe.txt`.

```sh
export CIVTOT_HOME=/path/to/private/install
export TOTENGINE_PEDIA_OUT=/path/to/private/reference-fixtures
cargo test -p totui --lib scenario_pedia_route -- --ignored
export TOTENGINE_PEDIA_FIXTURES="$TOTENGINE_PEDIA_OUT"
export TOTENGINE_PEDIA_COMPACT=1
cargo build -p totui --features desktop --bins
python3 tools/check_client_linux.py
```

For Chromium, start `totplay --no-open --port=8844` with the same private
install and a scratch `TOTENGINE_SAVES` directory. Run
`tools/check_client_browser.cjs` with `TOTENGINE_CLIENT_ORIGIN` set to the
loopback address, `TOTENGINE_CLIENT_SAVES` set to the host's scratch save folder,
and the same fixture and compact-layout variables. Both runners use a
960×720 window for this walkthrough.

Load `pedstart.cth`, highlight a research topic, and open Civilopedia.
Verify the authored text and its source path, scroll, and use Enter to
return to the same choice. After deciding on research, browse to the second
page of the standalone index and check that Enter reads the selected topic.
Open City status, change the sort, drill into Reference Grove, and return to
the same report. Reload the pending checkpoint and repeat the contextual
lookup with Escape. The runners compare snapshots of the pending decision
and the unchanged game after reference browsing and reporting.
The [40-step evidence record](../evidence/client-scenario-reference-route-v1.json)
retains the authored fixture, route and checkpoint hashes. Provenance and
navigation were checked through real Chromium and Linux SDL input.
