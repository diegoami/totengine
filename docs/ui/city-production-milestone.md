# v0.14 — city production decisions on both clients

Milestone 12 / #148–#151 follows the owner's instruction to merge after v0.13
and continue. It selects the next part of the reviewed city screen contract;
it does not complete the city UI or change the full-AI gate.

Both browser and standalone desktop must show the current item, shield cost,
investment, remaining shields and an estimate based on current net production.
Production rows remain draft choices until explicit acceptance. Show the
invested-shield consequence before committing. Cancellation spends no shields,
gold, production time or RNG. Original keyboard meanings remain authoritative.

Buying uses an engine-derived, state-specific quote and explicit confirmation.
Show cost, current/remaining treasury and that the unit/improvement appears
during production processing, not instantly on purchase. Insufficient funds,
disorder/unsupported context and stale quotes are visible, inert rejections.
Single-city improvement sales show the building, refund and lost effects before
confirmation; refresh finance/mood/output immediately. No implicit bulk sale.
Palace, wonders and non-building spaceship projects are not ordinary sale rows.

Quotes and commands use stable own-city identity and revision. Engine code
re-derives every price and rejects stale/wrong-owner/pending-choice input before
mutation. No adapter may submit an arbitrary cost or shield baseline. Repeated
requests remain idempotent; real accepted commands/results replay and survive
checkpoints. Views/inspection never execute an economic transaction or a turn.

## Native evidence and context gates

`city-production-v1.json` and its vectors currently establish 4,091 cases: 2,866 arithmetic cases plus 344 confirmed-buy
ledger/shield writes, 102 confirmed-sale building/refund/flag writes, 125 funds/
disorder admission cases, six already-sold gate cases in non-editor play and
648 accepted transitions across 162 prepared switch sequences. Confirmed buys
also compare rebasing the captured production budget.
Retention uses an explicitly captured initial category/shield budget.
The native current-city accessor and item-name formatting are the only UI
substitutions in those isolated fragments. These isolated fragments do not establish full dialog legality, full refresh
or the entire native city-window context lifecycle; auxiliary native queued-unit
caches are outside the compared fields.

In particular, the original captures an initial production class and shield
budget separately from the current build; switching computes from that context.
The engine now owns this private context and records it through validated
orders. Capture happens on the first accepted switch; inspection and draft
cancellation remain immutable. Buying rebases it, closing/navigation/end-turn
clear it (the map quick picker closes on acceptance), and checkpoint replay restores it and reopens the owning city. Tests
cover repeated switches, budget restoration, close/reopen, buy and replay. The
legacy autonomous production order is unchanged. Actual Chromium/Linux/Windows
checks now match all nine production scenario checkpoints. The owner accepted
the requested both-client production routes on 2026-11-07 (“great, it works, let
us continue”). Independent review remains required before release.

## Acceptance and later work

Generate recorded-play fixtures for purchase, sale and invested production.
Check confirmation/cancel, insufficient funds, stale/entity-reuse rejection,
second sale, next-turn production, city navigation, pending-choice save/reload
and following orders. Run actual Chromium/Linux/Windows input and compare full
checkpoints. Package both adapters with notices and no original assets/fonts.
The owner must try the new production route on both clients; v0.13 citizen-route
acceptance is not relabeled as acceptance here. Independent owner-launched
review gates the release, and main prints the prompt without launching review.

Bulk sales, full support/happiness/improvement reports, rates/government,
diplomacy, native SAV/scenarios/multiple maps, full original UI/AI and sustained
full-campaign acceptance remain later work. There is no fixed remaining count
of playable-client milestones.
