# First native save/scenario entry contract

Milestone 15 / #193. Contract identifier: **classic-surface-032-v1**.
This freezes the first implementation boundary for #197/#198 and the dependency
checks needed by experiments #203/#204. It implements no loader or client screen.
[Inspection](scenario-inspection.md) and its pinned corpus provide source facts;
existing UI contracts/prototype remain the planning basis.

## Selected candidate and current outcome

The first reference candidate is `Original/Ph_Auto2.SAV` with the explicit
Original Rules/events/resource roots in [scenario-corpus.json](scenario-corpus.json).
Exact input hashes are in [the corpus summary](evidence/scenario-inventory-v1.json).
The observed 0x0032 save has framed surface data, 38 units, ten cities and no
native transporter records. This selects a bounded stock Original surface
profile before custom calendars, multiple maps or modern scripts.

**It is not admitted for human entry or faithful simulation today.** Standard
Original events already require an unsupported compound/endgame branch, and
native active-phase/movement/fog import requires further evidence. The other
corpus inputs expose additional independent blockers. The first loader may
only activate this profile when every required predicate below is verified;
otherwise it supplies an actionable rejection. Selecting a candidate does not
waive missing behavior or authorize stripping its events to make it pass.

SAV and SCN are separately labeled inputs of the same proposed classic-format
boundary; a suffix or shared magic cannot establish their native startup/phase
semantics. A private controlled SCN must be independently pinned and verified
against these predicates before the SCN branch is activated. FallRome is a
follow-on custom-scenario candidate; Roman Civil Wars, Midgard and Warring States
remain negative/extended cases, not substitute positive fixtures.

## Admission predicates

| Area | Required predicate / rejection |
| --- | --- |
| Format/records | Verified native ToT 1.1 (0x0032) framing; bounded sizes/counts, valid identities/owners/references/coordinates. Extended or unlocated framing is rejected, not coerced into classic records. |
| Map context | Proven single active surface context, no unmodeled map/transfer requirements, compatible terrain/domain rules. Header index 3 alone is insufficient. Non-surface coordinates/native transport state require a later map profile. |
| Rules/resources | Explicit compatible stock Original rules and resources for the first profile; every required override/dependency bound by identity. Missing/ambiguous/mismatched inputs reject. Custom labels/currency/dates/text and sprite requirements require measured support before admitting that custom profile. |
| Events/extensions | Complete supported trigger/action/parameter/condition/timing and persistence semantics for every required feature. Unknown, partially modeled or skipped behavior rejects continuation; no silent Lua disable, ignored compound/delay/flag or substitute generic objective. |
| Human/phase | Verified live selected civ 1–7, ownership/human-mask context and a supported human-input phase. Unknown mid-round, native dialog or pending-choice state rejects until mapped. Do not assume every autosave is turn-start. |
| Snapshot state | Preserve saved remaining movement, health/orders/home/stack identities, city stores/production, treasury/rates/research/government and admitted map flags. Do not call a generic new-turn refresh that resets spent movement or collects income during load. Cache normalization needs declared, measured semantics. |
| Knowledge | Initialize only the selected civ's admitted visible/remembered state. Native full save data is not a human observation. Do not reconstruct memory by revealing hidden current units/cities; unverified native fog memory requires a rejection or a separately labeled experimental profile. |
| Opponents/RNG | Explicit engine policy/configuration and seed initialization; the first profile does not claim original AI mission/cache/shared-RNG continuation. Native RNG import needs its own measured mapping. Initialization and later replay must be reproducible. No privileged driver is approved. |
| Identity/atomicity | Bind all required original input/resource identities; validate an immutable captured snapshot in a temporary session. Source changes, cancellation, stale epoch/revision or preflight failure cannot replace/mutate the active game. |

Predicate activation requires retained original-data/reference evidence and
implementation tests. The inspector's keyword list or `first_profile_structure`
booleans are insufficient. Existing source checks establish emitted-but-skipped
GIVETECHNOLOGY, partial CREATEUNIT placement, absent Lua hosting and other gaps;
fixing one handler does not establish all scenario behavior.

## Resolution and presentation policy

For the first stock candidate, select `Original/Rules.txt` and the explicitly
chosen installation's complete Original resources, as already used by generated
play. No accidental fallback to authored demo rules is allowed. The optional
artwork path may only preserve the same original Rules and declared identities;
unsupported/mismatched scenario units cannot be silently renamed to stock roles.

Future custom packages must supply their declared rules/event/text dependencies.
Package overrides take priority only through an explicit sheet/text-role policy;
common Original UI-sheet fallback must be distinguished from unit/terrain art
that may depend on custom definitions. The current inspector deliberately shows
missing `dialog` in some declared roots; that is a resolution-policy question,
not proof the original game lacks its shared dialog sheet. Measure native lookup
and freeze that role map before broadening admission. Never copy a global UI
replacement pack over the owner's installation as part of import.

All filesystem work lives in the adapter/resource layer with user-supplied roots,
not absolute paths in engine libraries. The accepted browser edition uses local
Rust hosting; public hosting/uploads/per-user isolation require their separate
contract. Incomplete dependency closure remains a blocker, not a heuristic pass.

## Both-client walkthroughs to implement

1. **Positive entry after predicates pass.** The player selects **Open ToT save /
   scenario**, separately from **Load engine checkpoint**, and supplies the
   original resource root. Preview identifies the selected input/profile and
   meaningful compatibility results. Choose the supported human faction and
   explicit engine-opponent/seed context, confirm, then commit the temporary
   session. Return to original-style map/unit input; native required choices
   remain explicit. Do not show the author's full-world report as human fog data.
2. **Unsupported scenario.** Selecting Warring States explains that required
   ToTPP/Lua behavior is unsupported. Midgard explains multiple-map requirements;
   Roman Civil Wars exposes unsupported event/goal features. Offer inspect/report
   and back, not an unlabeled partial-play button that drops logic.
3. **Bad/mismatched resources.** Missing rules, ambiguous sheets, changed input,
   wrong version or truncated data identify the selected relative file and
   corrective choice. Cancel/back preserves the current game, selection and
   pending decision. Hashes/low-level details belong in inspection/details.
4. **Loss/stale work.** Bind async file/preflight results and confirmation to a
   session/input epoch and exact choices. Pause bounded opponent scheduling while
   replacing a session; cancellation resumes the original phase without another
   EndTurn. A queued click/key cannot target newly imported units or choices.

Use original controls and actual standalone desktop/browser resource selection,
focus and error behavior. These are future routes, not simulated acceptance.
After implementation, the owner checks both new routes; existing v0.16 acceptance
covers neither native imports nor these missing mechanics.

## Persistence, experiments and recording

Existing human checkpoint format 2 always reconstructs a generated world; it
cannot represent a native-import initializer. Do not fake an imported snapshot
by editing its generated seed/setup descriptor. Before native import lands,
#197 must define a versioned initializer/checkpoint extension with:

- Immutable input snapshot identities (and a private preserved snapshot/reference
  policy), rules/art/text/event dependency identities and the admitted profile.
- Declared normalized state/phase/human owner and initial visibility memory.
- Explicit policy/configuration/seed identities and private continuation state.
- Ordered accepted decisions/events and the same resource/knowledge/RNG checks
  required by existing checkpoint/replay semantics.

Never overwrite original inputs or embed them in public repository evidence or
distributed recordings. Missing/changed snapshots or dependencies fail atomically.
A bounded headless experiment and observation of the admitted profile must agree
on events and next checkpoints; full-state observer data remains explicitly
separate from human views. Native SAV/SCN export is a different writer/original
round-trip gate, not implied by importing or saving an engine checkpoint.

## Evidence matrix and next implementation dependencies

| Case | Current evidence | Required before entry/continuation |
| --- | --- | --- |
| Original standard candidate | Pinned bytes, classic probe counts, stock resources, one macro block | Native human/phase/spent-move/fog mapping; compound/endgame event support; import initializer |
| FallRome | 64 macro blocks, 710 units/146 cities in existing framing | Technology effects and other required macro/context/presentation semantics; custom date/text policy |
| Roman Civil Wars | Installed hashes, 258 macro blocks, 618 units/159 cities | Delay/flag/randomize/removal/aggression/audio/objective and author-restriction semantics |
| Midgard | Partial surface framing, declared multiple maps, 12 transporter records | Correct complete layout, map state/transfer simulation and event semantics |
| Warring States | Partial extended framing and declared Lua files | Extended state, Lua API/lifecycle/serialization and dependency closure |
| Malformed/missing/escaped inputs | Authored unit tests and report error paths | Preserve rejection/atomicity in the shared importer and both adapters |

#192/#193 define inventory and this conditional boundary, not the mechanics or
client implementation. #197/#198 remain blocked on these explicit gaps. Full
macro work #16, diplomacy/capture/endgame/map/ToTPP prerequisites and scenario
platform #183 stay separate; no complete scenario, native AI or client is closed
by this contract. Read-only source study of C-evo/Freeciv does not alter the gate.
