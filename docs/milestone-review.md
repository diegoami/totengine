# Milestone review

A milestone is a release tag `vX.Y.Z` on `main`. Before the tag, one big review covers
everything since the previous tag. No script forces it: the owner starts it by calling a
model's CLI directly, on whatever model they pick, in a fresh session. Adapted from
malpaco's `docs/milestone-review.md`, which adapted it from Geoclick2027's release review.

## How a milestone happens

1. The owner calls a milestone, or the main session proposes one when a coherent set of
   work has landed (a tier or slice closed — e.g. "v0.1.0 = Tier 0 complete").
2. The version bump and the release notes land on `main`; that commit is the **candidate**.
3. The main session opens a **milestone issue** (label `milestone`) with: the proposed tag,
   the candidate's full SHA, the previous tag (or "first tag"), the issues/PRs merged since
   (including work merged without a review), the checks already run on the candidate, the
   **claims to verify with `file:line`** — for this repo, the evidence artifacts in
   `docs/evidence/` and what they assert — and the owner's decisions that are not defects.
   From then on `main` takes only fixes for the milestone's findings until it is tagged.
4. The owner starts the review from any checkout of this repository (a cloud session's clone
   is fine), for example:

   ```sh
   opencode run -m minimax/MiniMax-M3#thinking --agent milestone-reviewer "Review milestone issue #N"
   claude -p --model opus "Read docs/milestone-review.md and review milestone issue #N as it says."
   codex exec "Read docs/milestone-review.md and review milestone issue #N as it says."
   ```

   The reviewer must be a model family that implemented none of the candidate.
5. The reviewer opens one issue per reproduced finding and posts one verdict, AGREE or
   BLOCK, on the milestone issue. Nothing is pasted back.
6. BLOCK: the findings are fixed in ordinary PRs, the candidate moves to the new `main`
   commit (the issue says so) and the owner starts a re-review. After a third round without
   AGREE the owner decides.
7. AGREE: the main session tags **exactly the reviewed SHA** and publishes the release
   (GitHub release with notes; artifacts only once there are artifacts worth attaching).
   The owner may also tag without a review; the issue says so.

## Instructions for the reviewer

You are the independent reviewer of a ToTEngine release candidate. You implemented none of
it: verify everything against the code and the data, not commit messages or docs.

**Input.** The milestone issue number. `gh issue view <n> --comments` gives the tag,
candidate SHA, previous tag, issues/PRs, checks and claims; for a re-review, the earlier
verdict and the fix PRs. If any of these is missing, say so on the issue and stop.

**Set up.** `git fetch origin --tags`. If the working tree is clean, `git checkout --detach
<sha>`, then run the checks the claims name (today: `python3 -m py_compile tools/totctl.py`
and re-running the evidence commands in `docs/evidence/README.md` against
`/mnt/d/games/CIVTOT` when present — note any that cannot run here), and when you finish,
check out the branch you started on. If the tree is not clean, do not touch it: read the
candidate with `git show <sha>:<path>` and say which checks you could not run.

**Review** `git diff <previous tag>..<sha>` (or the whole tree, for a first tag) and follow
it into any file it touches or relies on. The standard is `docs/plan.md`'s principles and
`docs/feasibility.md`'s evidence discipline. Look hardest at:

- **format claims vs evidence:** every claim about `.MP`, `.SAV`/`.SCN`, `Rules.txt`
  decoding must trace to an artifact in `docs/evidence/` or a check that re-derives it;
  a parser change without regenerated evidence is a finding;
- **oracle honesty:** assertions about the original game's behavior marked as verified must
  say how (which save, which diff, which run under Wine);
- **no original game assets in the repo** — sprites, sounds, maps, saves from the install
  must never be committed; evidence artifacts must be derived data (JSON summaries, PNG
  renders we generated) and small;
- **the Wine oracle's absolute paths** (`/mnt/d/games/CIVTOT`) never hard-coded into
  library code — tools only, behind `CIVTOT_HOME`;
- tests that pass without the behaviour they claim, difficulty labels that the story's
  reality contradicts (a finding, not a defect), and anything copied from prior-art
  repositories without the license check (T0-12) recorded.

Aim at what CI cannot see. Out-of-scope problems elsewhere count too; mark them so.

**Write to GitHub yourself, and only this; never edit, commit, push, tag or merge:**

- one issue per reproduced finding: label `review` plus one of `bug`, `tests`, `design`,
  `cleanup`, `documentation`; MUST-FIX or follow-up in the title's prefix; linking the
  milestone issue and SHA;
- always exactly one verdict comment on the milestone issue: AGREE, or BLOCK when any
  finding is MUST-FIX; the SHA reviewed, your model, whether you checked out the SHA, the
  checks you ran and their results, the issues you opened, and what you checked and found
  clean. A review that finds nothing still leaves this record.

Write comment and issue bodies to a file and pass `--body-file` (UTF-8, no BOM — never
build bodies with shell here-strings or `Out-File`). If posting fails, print the complete
text instead and say why.
