# Milestone 19 — own-civilization reports on both clients (#201, #202)

Status: **first slice open as PR #247, extended with Attitude and Wonders on a stacked
branch; not released, not owner-accepted.** Scope is deliberately small: five report
families whose fields the engine already computes and verifies, shared by the browser and
desktop clients.

## Admitted families

| Family | Key | Columns | Source |
| --- | --- | --- | --- |
| City status | F1 | city, size, food surplus, shields, science, tax, luxury, build item, shields stored/required, turns | `engine::human::city_status` from the filtered `HumanView` (city economy, production view) |
| Defense | F2 | unit type, attack, defense, hit points, firepower, movement, active count, in-production count | `engine::human::defense`: own units and own cities' builds, with the loaded rules' unit statistics |
| Attitude (tab *Mood*) | F4 | city, size, happy, content, unhappy citizens, entertainers, tax collectors, scientists | `engine::human::attitude`: `city_economy` mood plus the citizen view's specialist roles; content is the remainder |
| Trade | F5 | city, credited taxes, science, luxury, improvement maintenance, improvement count; totals row (taxes, upkeep, net, science, luxury, treasury) | `engine::human::trade` |
| Wonders | F7 | wonder, host city | `engine::human::wonders`: own wonder hosts only, names from the `@ENDWONDER` row comments (physical row ids); expiry and foreign wonders are not shown |

Field sets follow the inspected original reports (discovery U05 city status, U23
defense/trade). **Layout, column order, labels and the sort/paging controls are ours**:
the original's exact column set, sorting and drill-down behavior were not traced, so no
native-parity claim is made. Not in this slice: Foreign (F3),
Ranked cities (F8), Score (F9), Demographics (F11), Spaceships (F12), the defense
casualty timeline, supply/demand, and the Civilopedia/reference lookup.

## Rules for the views

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

- F1/F2/F4/F5/F7 or the bottom-row **Reports** button open a report; F1/F2/F4/F5/F7 or the tabs switch
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
