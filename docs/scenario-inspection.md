# Scenario inspection and the private corpus

Milestone 15 / #192. This is a read-only author/developer tool, built on the
existing Python format probe. It does not execute macros/Lua, boot a game,
advance turns, import into either playable client or grant runtime admission.
The first entry proposal is [scenario-entry-contract.md](scenario-entry-contract.md).

```sh
export CIVTOT_HOME=/path/to/CIVTOT
python3 tools/scenario_inventory.py --out /tmp/scenario-report.json
# Compact evidence keeps counts, eight diagnostic examples and full-report hash:
python3 tools/scenario_inventory.py --compact --out /tmp/scenario-summary.json
# A different manifest may select other private inputs under the same root:
python3 tools/scenario_inventory.py --manifest /path/to/manifest.json
# Admission-sensitive automation must request its actual required level:
python3 tools/scenario_inventory.py --require simulation --out /tmp/blocked.json
```

Output paths must be new. The final command currently returns 1 for every
input, deliberately: this inspector has no runtime-conformance admission.
Default exit 0 means the declared inputs were inspected and a report produced,
not that a scenario is playable. Exit 1 means unavailable/malformed required
inputs or an unsatisfied requested level; exit 2 means configuration/output
failure. Existing files, including saves, cannot be overwritten through --out.
Legacy `totctl play/run/observe` are unchanged; the report is not retroactive
enforcement on those experimental/model runners.

## Manifest schema 1

[scenario-corpus.json](scenario-corpus.json) declares five entries by paths
relative to CIVTOT_HOME. It contains no original data. Each entry has a unique
bounded `id`, `kind`, explicit `save`/`rules`, an `events` path or null, `lua`
file list, ordered `art_roots`, and `declared_features`. A single
`first_entry_candidate` may select an input for contract work. Its selection
and `first_entry_profile` do not assert compatibility.

Paths use POSIX separators, remain within the chosen installation after symlink
resolution and reject absolute/traversal/drive paths. Lists, manifest size and
source files are bounded (64 entries, manifests/baseline 1 MiB, individual
sources 16 MiB); directory inventory stops beyond 4,096 children. Files are read
into immutable snapshots before parsing/hash generation. No scenario directory
is added to Python's module path and no supplied code is imported/executed.

Scanning stops before exceeding 4,096 diagnostics, 128 distinct phase/token
keys or 100,000 lines per entry; a report shares budgets of 8,192 diagnostics,
1,024 phase/token keys and 500,000 lines across entries. One additional reserved
`scan_limit_exceeded` marker per entry identifies overflow. Remaining entries
still receive their file/probe reports. Counts/coverage on an interrupted scan
are prefix inventories, not complete totals: `scan_complete` is false and
`overflow` identifies the budget. Compact mode retains that state and bounded
vocabularies; it does not disguise incomplete scans as complete inspection.

Structural macro diagnostics (unclosed/misplaced blocks, unterminated payloads,
malformed locations) and scan-limit failures also mark the entry's inspection
incomplete and yield exit 1. Unsupported-but-parseable semantics alone do not.
Reserved `ENDTEXT`/`ENDLOCATIONS` outside their payloads are structural errors
in every phase. Inside TEXT, directive/other-terminator text stays literal;
inside LOCATIONS, a non-coordinate line remains a malformed location.
After `@ENDEVENTS`, scanning continues under the same budgets: non-comment
content is diagnosed but never counted as an active event/trigger/action.
Blank/comment tails are allowed. This is lexical inspection policy, not a
claim about the original game's tolerance for trailing content.
Location fields accept an optional minus and up to ten ASCII decimal digits;
this lexical safety bound is not evidence of the native coordinate range.

Lua files/modules, map modes and author objectives are explicitly declared, not
inferred from arbitrary code or prose. A null events path is an omitted input,
not proof that no native macro is required. The listed Lua files and selected
sheets are not a complete dependency closure. Text/audio/video/module references
and native resource search behavior need a separate audit before gameplay.

## Report schema 1

| Field | Meaning |
| --- | --- |
| `view` | `full` or `compact`; compact macros retain totals/histograms and bound examples to eight, with the exact full-report hash/size |
| `producer` | Inspector and format-probe byte hashes; no timestamp or absolute installation path |
| `manifest_sha256`, `baseline_sha256` | Exact declarations/audit identity used |
| `engine_audit` | Pinned v0.16 source ref/hashes and whether the audited files still match; not a compiled-binary or native-conformance check |
| `profile_candidate` | Proposed contract identifier, not an execution permission |
| `entries[].files` | Listed input role, relative path, bytes and SHA-256; no source bodies |
| `save_probe` | Existing walker results: version, raw game/header fields, counts where framed, and transport records. `structurally_probed`, `partial_layout` and `failed` are distinct. The zero-based header index 3/probe block inference is not an established active-map count. |
| `rules_probe` | Section/row inventory only; no claim that every rule/effect is implemented |
| `macro` | Lexical condition/action-token counts, partial baseline coverage and bounded line diagnostics; `scan_complete`/`overflow` identify interrupted inventories. Text/location payloads are not commands. Unknown parameters, compounds, skipped effects and malformed blocks remain visible. |
| `lua` | Declared source count; execution and inference flags remain false |
| `art` | BMP/GIF file identities chosen from declared roots in order, BMP before GIF within each root. Ambiguous roots are not silently replaced. No pixels are decoded. This is inspector policy, not native lookup evidence. Missing means missing in declared roots, not necessarily missing from the native game. |
| `dependency_closure` | Explicitly not established for complete gameplay/presentation |
| `first_profile_structure` | Narrow format/declaration/count predicates, separate from phase/event/fog or gameplay acceptance |
| `support` | Distinct inspection, engine boot, simulation, observation, human entry/campaign and native-export states |
| `blockers` | Concrete missing/unknown capabilities and acceptance gates |

`simulation` and `native_export` remain `not_admitted`, engine boot is `not_run`,
and observation/campaign are unvalidated for each input. Native human entry is
not implemented in the audited source. If those sources change, their audit
status is stale and human-entry assessment becomes `audit_required`.
Recognizing every keyword cannot elevate any of these fields. Full author-tool
metadata can contain global counts; it is not a human player's filtered view.

## Audited baseline and reproducible evidence

[scenario-engine-baseline.json](scenario-engine-baseline.json) catalogs directly
inspected implementation paths at v0.16.0. The SHA checks are conservative source
change detection; they do not prove runtime semantics, hook ordering, complete
parameter coverage or original-game equivalence. Baseline statuses distinguish
modelled subsets, partial handlers, emitted-but-skipped effects and no-state
presentation effects. Source references and exact hashes make that audit
reviewable rather than treating forum descriptions as implemented behavior.

The [derived summary](evidence/scenario-inventory-v1.json) pins all declared
inputs/selected sheets, report/source hashes, measured counts and bounded
examples. The complete line-diagnostic report is reproducible from the command
above and is retained privately, rather than committing a large report or any
original resources. Reports are deterministic for the same source/declaration/
input bytes. Re-run after changes; source mismatch never silently inherits old
coverage. The snapshot probe helpers preserve the old CLI/walk golden outputs.

| Corpus entry | Observed inspection | Important limitations |
| --- | --- | --- |
| Original standard `Ph_Auto2.SAV` | 0x0032; 38 units, 10 cities; one event block | Compound RECEIVEDTECHNOLOGY/RANDOMTURN condition and endgame/AVI actions are outside the modeled subset. Numeric year is raw, not a universal scenario date formatter. |
| FallRome | 0x0032; 710 units, 146 cities; 64 event blocks | Two GIVETECHNOLOGY tokens emit effects that are skipped by application; placement/presentation/hooks remain partial. |
| Roman Civil Wars | 0x0032; 618 units, 159 cities; 258 event blocks; inspection incomplete | Non-comment content follows ENDEVENTS; it is diagnosed, not counted as active logic. This lexical policy is not native-invalidity evidence. Delay/flag/randomized placement, technology removal, aggression/audio and author objectives need missing semantics. Installation/hash success is not play acceptance. |
| Midgard | 0x0032; surface framing only, 12 transport records; 151 event blocks; inspection incomplete | A location line fails the inspector's lexical grammar; this is not proof that the native input is invalid. Classic unit/city counts are not located by this probe. Declared multiple maps and terrain/flag/endgame semantics block continuation. No active-map count is inferred from the header. |
| Warring States | 0x0033 partial layout; five declared Lua files | Extended saves and Lua execution are not admitted. Absence of populated Lua state in a pristine file would not prove that Lua is unnecessary. |

The manifest selects the Original standard input for **first entry contract
work** because its classic surface/count predicates are satisfied. It remains
blocked for gameplay. This is a concrete candidate and gap list, not an admitted
scenario or a new native-play result. Unknown author objectives remain unknown.
