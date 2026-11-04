# Backlog

**The backlog lives on GitHub** (owner decision, 2026-11-04):
- **Releases** are [milestones](https://github.com/diegoami/totengine/milestones):
  v0.5.0 player interface → v0.6.0 AI v0 → v0.7.0 browser observer →
  v0.8.0 AI v1 + economy depth → v0.9.0 generated-world campaign →
  v0.10.0 original-like built-in AI → v0.11.0 minimal SDL2 client →
  pluggable AIs (later, version unassigned). Each has one theme
  and a few stories, so every review stays one round (decision 0002).
- **Stories** are [issues](https://github.com/diegoami/totengine/issues). The
  story ID (`T1-4`, `RG-8`, `OB-2`, …) is in the title; the body holds the
  acceptance criteria. Issues without a milestone are "later".
- **Epics:**
  [#19 T0](https://github.com/diegoami/totengine/issues/19) ·
  [#20 T1](https://github.com/diegoami/totengine/issues/20) ·
  [#21 T2](https://github.com/diegoami/totengine/issues/21) ·
  [#22 T3](https://github.com/diegoami/totengine/issues/22) ·
  [#23 RG](https://github.com/diegoami/totengine/issues/23).

This file keeps the working conventions only. The story tables it used to
hold (stories T0-1 … RG-11 with their acceptance and status notes) are in
git history up to tag `v0.4.0`; [docs/plan.md](docs/plan.md) holds the
strategic context.

Labels: `p0`/`p1`/`p2` priority, `diff-easy`/`diff-hard` difficulty (below),
`t0`…`t3` tier.

## Assigning work: difficulty and agents

Two difficulty levels — a ToTEngine-local convention for picking the agent and its
effort level. It is deliberately independent of the review process being adopted;
when stories become harness tasks, the difficulty decides the assignment, the
harness decides the review.

| Diff | Meaning | Agent assignment |
|------|---------|------------------|
| **E** | Low uncertainty: mechanical, well-specified, tooling or docs. Format known, acceptance testable in one pass. | `general` agent, standard effort — or `explore` (medium) for pure-research stories. One review pass. |
| **H** | Reverse-engineering, parity-sensitive engine work, or architectural. Unknowns that only the oracle/experiments resolve. | `general` agent at **high effort**, design note agreed before code. Cross-model review; second opinion on milestone-critical stories. |

Rules of thumb: anything gated by *oracle parity* is H regardless of size; anything
whose acceptance is "it parses/renders and tests pass" is E unless the format itself
is unknown. An H story that grows two failed attempts gets split before a third.

## Releases & milestone reviews

Releases are tags `vX.Y.Z` on `main`, gated by an independent milestone
review before the tag: one big review per release by a model family that
implemented none of it, verdict AGREE/BLOCK on a milestone issue, findings
as ordinary issues, tag on exactly the reviewed SHA. The process:
[docs/milestone-review.md](docs/milestone-review.md).

Released:
- `v0.1.0`: Tier 0 complete (2026-11-02, 5 review rounds);
- `v0.2.0`: headless game loop (2 rounds);
- `v0.3.0`: reproducible new game (1 round);
- `v0.4.0`: minimal production (1 round).

## Standing tasks

- Keep `docs/evidence/` regenerated on every format-affecting change (T0-9 makes
  this automatic).
- Every format claim gets an evidence artifact or oracle test — no folklore.
- No original game assets in the repo, ever.
- Difficulty is re-assessed when a story starts; a wrong guess is a finding, not a
  failure — record it on the story.
