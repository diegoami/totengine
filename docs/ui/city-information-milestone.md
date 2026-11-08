# v0.15 — city information panels on both clients

Milestone 13 / #157–#159 follows the owner's instruction to continue after the
v0.14 release. It selects the remaining information panels from the reviewed
city screen contract, not government/diplomacy or the complete client/AI gates.
Both browser and ordinary standalone desktop shipped in v0.15.0 at exactly
`948b0d2b94d22315190c664340d080062629c8ea`, after owner panel approval and
third-round independent AGREE #162. #157–#159 and milestone 13 are closed;
non-blocking #176–#179 remain open. Both clients remain required for later work.

## Reference and implementation boundary

Read screen-contracts.md city management, discovery U21/U22 and the selection
study before implementation. The retained reference has resources/build/buy at
left, radius centrally, panel selector and selected information at right,
citizens along the bottom and food storage separately. Existing accepted
client controls and immediate edits remain. The fresh isolated copied-install
study is recorded in city-panels-study-v1.json; original images stay outside Git. Native
ToTPP UI observations do not establish unpatched vanilla UI equivalence.

Info retains the accepted tile inspector and adds own units present. Native
commodity supply/demand text is observed but its workflows remain later work. Support distinguishes
units supported by this city from units currently present; cost indicators use
the existing shield/settler-food kernels. Improvements lists owned buildings,
upkeep and local wonders with active/expired status. Sale remains the existing
explicit quote/confirmation, never a side effect of opening the panel. Happy
uses the five proven normalized stages, omits unchanged rows like the observed
reference, and shows final mood, with specialists kept separate. Do not invent per-stage arithmetic or hide a cost distinction behind
a single unexplained total.

The presentation reuses the accepted client layout: left resources and
production, central radius/citizens, right information selector/content. The
right-hand Info/Support/Improvements/Happy selector is presentation state;
changing it does not issue an engine command. Retain it across city navigation
and accepted immediate edits. Opening a new session defaults to Info. Loading
keeps the current panel when the city workspace remains open; the selected
panel is not gameplay checkpoint data. Existing recorded production context
still restores/clears according to v0.14 rules.

Lists are inspection-only unless an existing validated explicit sale action is
selected. Support rows must not wake, activate, move, disband or close the city.
Those original routes need further observation/commands. List pagination and
long names must remain readable without clipping or retargeting a stale city.
Keep original keyboard meanings; Tab/Enter reaches the new explicit controls.

## Engine views and arithmetic

Only live own surface-city data is exposed through HumanView. Supported and
present unit identities must remain distinct, with stable IDs, owner filtering
and actual ordinal shield costs. A present unit supported elsewhere must not
be shown as consuming this city's support. Owned wonders are separate from
the ordinary packed building mask. Inspection must leave checksum, RNG,
production context, treasury, stock and recorded orders unchanged.

Happiness now exposes snapshots from the same kernel used for final mood:
base, luxuries, buildings, military, wonders. Existing final arithmetic remains
unchanged. `happiness-stages-v1.json` / vectors capture 1,280 original normalized
snapshots across 256 synthetic contexts (240 have changing stages). The native
probe uses the existing explicit garrison input boundary; it does not establish
map-input derivation or original citizen-icon interpretation.

## Acceptance

#157 covers reference/numeric evidence and immutable filtered views; #158 covers
the shared panels; #159 covers recorded-play fixtures, actual browser/Linux/
Windows input, packaging and new both-client owner acceptance. Check navigation
across two cities, support away/present/no-home distinctions, panel retention,
worker/specialist and sale refresh, confirmation cancel, stale input, save/load
and following production/movement. Full checkpoint comparisons prove that
inspection is inert. Existing v0.14 acceptance does not accept these new panels.
Independent owner-launched review still gates the release.

Later: bulk sale, support unit actions, city renaming/full reports/commodity
workflows, rates/government, diplomacy, original saves/scenarios/multiple maps,
sea/air/cargo and sustained complete-campaign acceptance. Complete original AI
and pluggable AI remain behind complete playable-client delivery. No fixed
remaining client milestone count is implied.
