# Classic snapshot record and normalization check

Milestone 17 / #197, next bounded part of slices 1–2. This check does not
admit `classic-surface-032-v1` or change experimental checkpoint recipes.
The [entry contract](scenario-entry-contract.md) remains authoritative.

Before booting a prospective classic snapshot, check its exact record extent,
single-map shape, live identities/owners/coordinates, unit types, homes,
reciprocal same-tile stack links, city occupancy, active route references and
wonder references. Subject to the existing parser's framing/name checks, dead
slots and unused route fields may retain stale bytes; they do not become live
references. Each rejected context identifies its
record/field. Bounds are deliberately conservative engine preparation policy,
not a claim that the original rejects every excluded save.

The preparation check uses the existing parser and proven record accessors.
Retained native framing/stack/knowledge evidence establishes those accessors;
a pinned installed-input check and authored corruptions establish this new
validation policy. No original assets or save bytes enter public evidence.

Separately measure the current schema-2 constructor against raw decoded
snapshot fields and a booted world. Compare all city records, civ finances,
research/government, map exploration/memory and unit identities/health/orders/
homes/stack positions. The complete world may differ from boot only in movement
allowances minus saved spent thirds (saturating at zero); both seeded RNGs
must retain zero draws. Record movement rows and initial state assertions in
derived evidence, including whether a required research choice is synthesized.
This is an engine normalization measurement, not native prompt timing.

Acceptance requires rejection tests for invalid extents, duplicate identities,
bad map/owner/type/coordinates, orphan occupancy, stale homes/routes/wonders,
broken/cross-tile/cyclic stacks and negative sentinel values. Valid deleted
slots and inactive route fields must remain tolerated. The pinned Original
candidate must pass these record checks and the field-preservation measurement;
experimental schema-1/schema-2 and generated checkpoint regressions must pass.

Knowledge continuation, pending-choice admission, complete stock dependency
closure, an admitted initializer and #198's private snapshot lifecycle/atomic
both-client entry remain separate work. Passing this check grants no runtime
admission, fresh-turn normalization or native AI/RNG continuation permission.

The tail-byte count is an extent observation, not a requirement to reproduce
every native UI/AI cache. Follow-up work must identify the bytes/loader paths
relevant to the contract's required phase and pending-choice predicates;
unrelated opaque bytes do not automatically add new milestone prerequisites.

The returned stack positions describe live records only. A future admitted
constructor must install that checked order rather than trusting the legacy
boot walk, which also follows deleted-slot link payloads. This slice neither
rewrites those payloads nor changes legacy reconstruction; its complete-world
measurement is specific to the pinned positive input.
