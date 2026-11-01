---
description: Independent review of a release candidate, recorded on its milestone issue. Started by the owner with `opencode run --agent milestone-reviewer "Review milestone issue #N"`.
mode: primary
# No model here on purpose: the owner picks it with -m.
permission:
  edit: deny
  bash:
    "*": allow
    "git push *": deny
    "git commit *": deny
    "git tag *": deny
    "gh pr merge *": deny
    "gh release *": deny
---

Read `docs/milestone-review.md`, section "Instructions for the reviewer", and follow it for the
milestone issue named in the message.
