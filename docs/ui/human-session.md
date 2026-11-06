# Shared human-session implementation

Milestone 10 work for #127 and the engine part of #131. This is shared engine
infrastructure; the browser and standalone desktop adapters are still required.
It does not turn the nine-screen design prototype into a playable client.

`engine::human::HumanSession::generated` starts a generated game with one
explicit human civilization and `builtin-1` opponents. The human mechanics mask
is applied before initialization and included in a distinct three-line
`totengine-human-orderlog 1` header. Autonomous generated logs keep their existing
header and behavior. `Game::replay_human` re-derives canonical orders, explicit
UTF-8 city names and round checksums without using a UI or AI policy.

## Input, phases and retry behavior

- `view()` returns the existing civilization-filtered observation, own entity
  identities, identities of known cities, own legal production choices, current
  phase/decision, last human order result and own round notices. It does not
  include the spectator payload, global checksum, foreign private state or the
  checkpoint transcript. Production pickers and validation share one legality
  boundary. Entity references pair a slot with its current identity.
- `submit(Request)` checks human owner, revision, phase, entity identity and
  required decision. Accepted requests increment the revision and receive a
  receipt. An exact retry returns its original receipt even after subsequent
  opponent work or restore; reusing its ID for different input fails. Rejected
  commands leave world, both RNGs, log, revision, decision and receipts unchanged.
  Transactions currently clone the world/events before executing a human order;
  this favors correctness over optimizing input cost.
- Research is a required choice at the human input boundary when a target is
  absent or a free advance awaits selection, provided legal choices remain.
  Founding opens an explicit name decision. Naming commits founding atomically;
  cancel leaves the settler intact. Names follow our client policy: trimmed,
  nonempty, at most 63 UTF-8 bytes, no control characters. This is not a native
  SAV name encoding/capacity claim. The order log encodes names as hex UTF-8.
- Only explicit EndTurn hands off to opponents. `advance(budget)` executes at
  most 256 work items per call, stopping at human input. A work item is one AI
  action, skipped civilization, or whole atomic round. It retains private AI
  state, last-result feedback and the existing 200-action guard for opponents.
  The human has no action cap or timeout. Polling `view` never advances a game;
  `advance` while awaiting human input is also inert.
- Turn results retain only own growth, starvation, destruction, disbands,
  production, discoveries and revolution notices. Foreign events and barbarian
  arrivals are not exposed as a spectator feed. Notices last until the next
  round result, including across save/load.

Research completion still occurs inside the existing atomic round. The completed
target's overflow is discarded by the existing research kernel; subsequent
cities bank their science without a target. The human chooses at their next
input boundary, with the bank retained. No new research target is selected on
their behalf. This is engine boundary timing, not a claim of native intra-city
UI interruption timing. A controlled two-human-city plus foreign-city test runs
the real city phases in descending order, verifies later-city banking and checks
that only the human's discovery enters the notices.

Elimination stops the driver. Forced human anarchy stops at the human boundary
with `UnsupportedGovernment`; adapters must display the unsupported required
choice and preserve a checkpoint, rather than continue or invent a government.
Government choice remains a prerequisite to continuing such a game. There is an
explicit 30,000-round limit, before the signed turn counter can wrap. The driver
currently retains up to 100,000 accepted input receipts. It does not implement
native victory rules or the later full-client management commands.

## Human checkpoints

`checkpoint(resource_identity)` emits `totengine-human 2`, binding exact engine
version, generated setup, human owner, policy ID, canonical Rules, adapter-supplied
resource manifest identity, world checksum, last-seen knowledge fingerprint and order-log identity. Its journal
records accepted human inputs and performed opponent work, including a partial
opponent turn. Restore reconstructs a temporary session, reruns that journal and
compares the canonical checkpoint in full. This rebuilds private AI RNG/state,
world RNG/state, revisions, decisions, retry receipts, notices and scheduler
position. Invalid input cannot partially replace the active session. The
checkpoint is private host data; never expose it through a human view endpoint.

This is an engine checkpoint, not native SAV export. It stores no artwork or
Rules contents. The adapter must supply the same Rules and actual resource
identity, provide file selection, and replace the active session only after
successful reconstruction. Restore is O(recorded work), with a 16 MiB input and
200,000-operation parser bound; instant snapshots and cross-version migrations
are not implemented. Resource identities/fingerprints detect accidental changes,
not deliberate cryptographic forgery. Both clients now provide named local slots and atomic replacement after successful
reconstruction; #131 also requires owner play and assembled acceptance.

## Verification

```sh
cargo test -p engine --lib human::tests
```

Nine authored engine tests cover inert polling/rejection/retries, changed
request IDs and stale/foreign/reused entity references, real movement, named
founding/cancellation, initial and later research, legal production and more
human actions than the AI cap, private-data filtering, unsupported government
and elimination, and invalid checkpoint replacement. Automated ten-round routes
with human civilizations 1, 3 and 5 compare one-item versus 256-item opponent
budgets and restore mid-opponent every round. Their world/log/view and subsequent
policy behavior agree; canonical world replay agrees too. Pending research and
founding checkpoints and retry receipt restoration are covered separately.

These automated routes supply choices in test code. They are not the required
actual ten-round human acceptance route in either client and do not close #131.

The human view now retains last-seen tile features and foreign-city identity,
name, owner and population. Explored tiles do not expose hidden later changes;
actual sight refreshes memory after each accepted command and opponent work item.
This memory is reconstructed by checkpoint replay and fingerprinted separately.
Foreign units remain current-sight-only. The existing autonomous Player
observation and AI trajectory contract are unchanged. A focused authored test
covers hidden city growth/rename/ownership, new cities on explored tiles, slot
reuse, roads and reacquisition. The 1/256-work-budget continuation test covers
knowledge reconstruction at partial opponent boundaries.
