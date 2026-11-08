# Decision 0008 — ToT scenario platform, AI choice and reusable workflows

Status: accepted product direction, owner request 2026-11-08. Records the seven
targets in [mission.md](../mission.md); delivery is in [plan.md](../plan.md).
No release version, calendar schedule, AI port or license change is assigned.

## Context

The initial T0–T4 plan predates released playable clients and still defers author
tools and QoL behind a distant final phase. It also mixes historical estimates,
completed work and future promises. The owner now explicitly requires native
and browser ToT play using original assets, headless all-AI scenario testing,
scenario observation, a complete faithful UI with optional enhancements,
pluggable/different AIs, scenario opening/test/creation assistance, and recording
of playing and observation sessions.

Freeciv and C-evo sources make AI, client and workflow reuse worth evaluating.
The owner identifies ToT scenario compatibility as the niche; neither source
availability nor similar rules prove that another engine can run our scenarios.

## Decision

1. Organize delivery by product workstreams, shared mechanics and admitted
   scenario profiles, rather than a strictly sequential T0–T4 ladder. Keep
   standard games, both playable clients and scenario compatibility explicit.
2. Make scenario experiments, observation, recording and author assistance
   first-class programs. Build useful bounded slices with existing policies;
   they do not depend on external AI transport or a complete native-AI port.
3. Retain ToT's full interface/controls as the compatibility baseline and the
   existing accepted prototype/contracts as the planning basis. Review proposed
   additions against reference behavior. City production queues, rally points
   and alternative UI settings are explicit optional scope, not deferred
   automatically until all other work is complete.
4. Evaluate eligible C-evo/Freeciv planners, whole clients and workflows as real
   alternatives. Preserve the authoritative ToT simulation, scenario resources/
   events, filtered views, validated orders and versioned replay/private state.
   Benchmark strength separately from fidelity. Do not copy GPL game code or
   assume C-evo media is cleared; existing provenance/licensing rules remain.
5. Keep #117's exact original-AI reproduction as its own fidelity program, with
   unchanged proof obligations. It is not the sole route to capable opponents
   or a prerequisite for all other AIs. #118 is first within that program only.
6. Preserve the earlier client-first implementation gate: complete playable
   clients precede full-AI ports and external plugins. Read-only source studies,
   interface/benchmark design and existing-policy scenario tooling can proceed.
   This decision does not approve a privileged compatibility driver under 0003.
7. The main ToT experience requires original resources. Preserve the approved
   Isotrident fallback as optional artwork; Open demo is a limited demonstration.
   Full independent rules and public hosting remain separately scoped.
8. Record reproducible gameplay and observation sessions with explicit resource,
   engine, policy and scenario identities. Separate camera/perspective/notes
   from canonical orders/events. Stage audiovisual capture as a later recording
   package; current logs do not establish a complete video recorder.

## Superseded and retained commitments

This replaces the old plan's calendar ranges, sequential phase schedule and
blanket ban on new QoL before T4. It updates decision 0002's roadmap ordering and
broadens the AI planning context of 0007, without rewriting their historical
release acceptance. Decision 0005's both-client delivery, original UI study,
reviewed design and meaningful human acceptance remain in force. Decision 0003's
normal observation/order authority remains in force. The complete client and
AI gates require admitted coverage and explicit limits, not a renamed prototype.

GitHub programs #183–#190 extend the existing queue. Existing macro/mechanics,
client, ToTPP and AI issues are reused and cross-linked; planning does not close
unfinished implementation. Actual release candidates still use small coherent
themes and independent owner-launched review at the exact candidate SHA.

## Milestone alignment — owner follow-up, 2026-11-08

The owner then requested using this backlog to update milestones. Named future
packages 14–29 now carry bounded implementation/preparation stories, with no
numeric release versions or due dates. [The package map](../milestones.md)
records dependencies, the complete-client AI gates and remaining unscheduled
original-feature work. Historical milestones remain unchanged. Program epics
remain open across their child milestones; this is not a fixed final release
count, frozen review candidate or implementation launch.
