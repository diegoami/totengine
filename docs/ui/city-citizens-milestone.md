# v0.13 — city workers and specialists on both clients

Implementation scope for [milestone 11](https://github.com/diegoami/totengine/milestone/11),
following the owner's instruction to continue after v0.12. Stories #135–#137.
This selects a narrow part of the already reviewed city design, not the whole
city/economics package or a fixed remaining release schedule. Design basis:
[city screen contract](screen-contracts.md#city-management-and-production),
[reviewed prototype](client-design.md), decision 0005 and the owner's immediate
worker/specialist-edit choice. Both actual clients ship together.

## Accepted interaction carried forward

Open a city workspace from the own-city map panel. Keep the map workspace as its
return destination. Use a dense 1024×768 logical layout: output/storage/mood and
existing production control at left, the worked radius with tile yields and
citizen role controls in the center, selected-tile details and city navigation
at right. Both adapters render the same frame and actions. Installed terrain,
city and citizen art remain local; authored labels/markers communicate state.
At larger windows the shared canvas scales as in v0.12.

Inspection, city navigation and closing the workspace are inert. Worker and
specialist edits commit immediately through validated commands, refresh outputs
and mood, and stay committed after Close. There is no global city draft or
Apply/Cancel layer. Existing production selection retains its own explicit
accept/cancel picker. Required research/founding still blocks unrelated edits;
inspection remains possible. City-bound controls carry the city identity so
queued input cannot edit a different city after navigation.

## Engine and evidence boundary

Add own-city details to HumanView, never a mutable WorldState or spectator
payload. Include radius coordinates/known terrain/worked/availability/yields,
citizen roles, signed food debt/storage and growth threshold, shield/output,
finance, support and mood. Unknown/foreign-city private information stays absent.
Views never call a mutating totals refresh. Reuse the existing native-checked
allocation, availability, yield, finance and happiness routines after edits;
do not advance growth, treasury, production, RNG or turn during a city edit.

Native edit reconnaissance found the worker transition in unpatched civ2.bak
`0x4dddc6..0x4ddec9` and specialist cycling in `0x4dd19f..0x4dd24d`, with
the population-five restriction at `0x4dd1d8`. The retained `citizen-edits.json` / `citizen-edit-vectors.txt` compare 1,548
original-execution cases. They declare UI-only context/message substitutions
and compare unchanged packed transitions, rather than claim full city parity.
Existing 1,024 allocation vectors and saved-input availability/totals evidence
remain regression gates. No original executable bytes, sprites or saves enter
Git. Defensive ownership/phase/entity/index checks are engine API policy.

Orders and HumanSession commands record city identity, radius/specialist slot
and results. Checkpoint replay must retain accepted edits and reject stale,
foreign, invalid or changed-retry commands atomically. Checkpoints are bound to
the exact engine version; v0.12 remains explicitly supported by its own binaries,
without promising cross-version `.cth` migration.

## Acceptance and exclusions

Automated authored tests cover blocked/overlapping/off-map tiles, population
conservation, all supported specialist roles, the population-five restriction,
food surplus/debt and mood feedback, own-only views, wrong/stale/duplicate input,
and replay/partial-opponent/pending-choice checkpoints. Both real adapters run
the same generated-game citizen route and compare subsequent command/results
and full checkpoints, including production cancellation and city navigation.
The owner exercises the new citizen-management route on both clients; record
actual feedback and fix blockers before independent candidate review. Existing
ten-round v0.12 acceptance is not relabeled as acceptance of the new controls.

Buying/selling, cross-category production-switch penalties, full support/
improvement/happiness-report panels, rates/government/diplomacy, native SAV and
scenario import/export, multiple maps, complete native visual parity and full
campaign acceptance remain later work. The full original-AI #117 and pluggable
AI gate remain behind completion of the entire playable-client track. The owner
launches milestone review; main stops with the filled prompt at the candidate.
