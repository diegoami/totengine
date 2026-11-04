# Decision 0005 — specify the original UI before scheduling the playable client

Status: accepted, owner discussion 2026-11-04. Supersedes the earlier
assumption that v0.11.0 delivers a complete minimal playable SDL2 client.

The browser observer is implemented, but it does not establish the design
of a faithful playable client. We do not yet have a screen-by-screen
specification or an owner-reviewed interaction design for that client.
Planning it as one milestone was unsupported.

## Decision

v0.11.0 begins the client track with UI discovery, a concrete design for
owner review, and decomposition into implementation milestones. It does
not promise a complete playable client. The first implementation
deliverable, subsequent milestone count and release versions must be
defined after inspecting the original UI and reviewing the design.

1. Inspect the original game's screens, dialogs, keyboard shortcuts and
   workflows, including scenario-specific behavior. Record evidence and
   distinguish observations from proposed changes. Follow the oracle
   README and lessons before driving a reference session.
2. Produce screen layouts and interaction walkthroughs for owner review
   before substantial client implementation. Cover map navigation and
   unit orders, city management and production, research, government,
   diplomacy, reports, game setup, and save/load. Record what is still
   unknown. The observer UI is useful infrastructure, not the client spec.
3. Define successive implementation milestones from that reviewed design.
   Candidate slices are map navigation/unit orders, city management,
   research/government/diplomacy, and reports/setup/save/load. These are
   planning categories, not a fixed milestone count or promised ordering.
4. Validate complete human play across the assembled slices, followed by
   usability and compatibility work. A short autoplay or twenty-turn smoke
   test alone does not establish that the client is usable or faithful.

Original-like built-in AI remains the v0.10.0 target (decision 0002).
External AI protocols and tournaments stay after the playable client;
their version remains unassigned. They must not delay client delivery.

## Tracking

GitHub milestones/issues remain the live queue. Move the broad minimal
playable-client story #94 out of v0.11.0 until the UI work defines its
actual scope. Discovery/design/decomposition is tracked in #107 under
v0.11.0.
Do not assign later client versions or claim a known delivery schedule
before that work is complete. This is planning, not a release candidate.
