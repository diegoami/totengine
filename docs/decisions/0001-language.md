# Decision 0001 — the engine is written in Rust

*Status: accepted (owner decision, 2026-11-01, in the T0-5 story).* Options were
Rust, C++17, C# (axx0/Civ2-clone's stack), or staying on Python probes. The owner
chose Rust.

## Context

docs/plan.md §3 defers the language choice to Phase 0. The engine must: parse
binary game formats byte-precisely, run a deterministic headless simulation,
embed **exactly Lua 5.3.6** (the version ToTPP ships, verified in the local
install), emit/consume JSON tooling, and eventually drive a 2D isometric client.

## Decision

**Rust**, workspace layout:

| crate | owns |
|---|---|
| `tot-kit` | format readers/writers: Rules.txt, .MP, .SAV/.SCN (0x32 → 0x33), art, sound |
| `engine` | rules model, deterministic turn kernel, combat, macro events |
| `lua-host` | Lua 5.3 embedding + the ToTPP `civ.*` API (contract: docs/refs/totpp-api-zerobrane.lua) |
| `totctl` | the CLI (successor of the Python probe `tools/totctl.py`, which stays as reference) |
| `totui` | the Tier-2 graphical client (placeholder until then) |

Key dependencies: `mlua` (Lua 5.3 feature pin), `serde`/`serde_json`, `clap`,
`thiserror`; SDL2 or wgpu decided when Tier 2 starts.

## Consequences

- The Python probes remain the fast-reverse-engineering loop; Rust `tot-kit`
  replaces them feature-by-feature, gated by the golden-file tests (T0-9).
- Byte-precise save writing (`0x32`) is safer in Rust (no hidden platform types).
- No local Rust toolchain existed on the dev box at decision time; installed via
  rustup (stable, minimal profile). CI (GitHub Actions) builds and tests the
  workspace on every push.
- Reversal cost grows after T1-2; revisit only if Lua embedding or determinism
  proves problematic.
