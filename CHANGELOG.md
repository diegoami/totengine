# Changelog

All notable changes to ToTEngine. The format follows Keep a Changelog; releases
are tags `vX.Y.Z` gated by the [milestone review](docs/milestone-review.md).

## [0.1.0] — Tier 0: format toolchain & reconnaissance

First release: ToTEngine reads the original game's data — every claim proven
against real game files, evidence committed under `docs/evidence/`.

### Formats decoded & proven

- **`Rules.txt`** — all 22 sections to JSON (`totctl rules`).
- **`.MP` maps** — both flavors: stock ToT 3-byte slots *and* the ToTPP-era
  paired-slot variant (AnticChina); terrain/river/specials/improvements/void;
  renders as PNG (`totctl map`, `--scale`, overlays via `scnmap`).
  World.mp renders as Earth with correct biome latitudes; river bit 0x80
  corrected from an earlier mis-attribution.
- **`.SAV`/`.SCN` base (`0x32`) front** — section walk verified byte-exact:
  settings, civs, transporters, turn/year; embedded map (visibility 7 B +
  map 6 B per tile; road 0x40 / rail 0x80); unit records (x/y, type→@UNITS,
  owner with 0x10 leader bit, per-save re-assigned ids — garrisons-on-cities
  145/146, ships-on-ocean 32/32); city records (name, x/y, size).
- **ToTPP `0x33` extension** — tail framing (`LUAE` + serialized Lua state
  as plain text + `EVNT` trailer) and the mid-region, byte-exact on the
  four-file Warring States ladder: `[32+15×surface per-tile arrays][40 B
  units][4 B][92 B cities][fixed post-city]`; 0x33 unit layout differs from
  the 0x32 base (id@0 sequential, logical signed coords, type@18, owner@19).
  ToTPP writes plain 0x32 for non-Lua scenarios (oracle-verified).

### Toolchain

- Rust workspace `tot-kit | engine | lua-host | totctl | totui`; MP decoder
  ported and cross-validated byte-for-byte against the Python probe; CI on
  fmt + clippy + tests.
- `tools/totctl.py` — reference probe (rules/map/save/diff/scnmap),
  stdlib-only, `CIVTOT_HOME` for the install root.
- Golden-file harness (`make test`): 9-item evidence corpus, machine-
  normalized, self-skipping in CI.
- **Wine oracle rig** (`tools/oracle/`): the original game driven headless
  under Xvfb (custom XTest tool + Win32 SendInput injector), automated turn
  loop harvesting same-session saves; annotated diffs. Findings documented
  (launcher listbox needs double-click; X-level keys never reach the
  DirectDraw surface; classic exe's cdaudio CD-check is unrunnable headless).

### Project

- Language decision (Rust) recorded; prior art license-audited; MIT license;
  ToTPP Lua API vendored for Tier 3; public at `github.com/diegoami/totengine`.
- Docs: feasibility (evidence discipline), plan (tiers), backlog (T0 complete,
  12/12), milestone-review process, environment/provisioning guide.

### Known open (not defects)

- 0x33 refinements: per-tile band sub-layout, post-city internals, unit flag
  bytes; unit-start formula proven on one map.
- Multi-map (Midgard-style) unit scan in the 0x32 walker.
