# ToTEngine mission

Owner direction, 2026-11-08. [Delivery plan](plan.md) · [Live backlog](../BACKLOG.md) ·
[Decision 0008](decisions/0008-scenario-platform.md); MCP interface [#250](https://github.com/diegoami/totengine/issues/250).

**Build an open, native recreation of Civilization II: Test of Time for playing,
observing, testing and creating scenarios, on desktop and in the browser.**
ToT scenario compatibility is the distinguishing purpose. Standard games remain
a product target and provide essential mechanics and campaign tests.

## Product targets

| Target | Intended experience | Completion evidence |
| --- | --- | --- |
| Desktop and browser play | The same native ToT simulation through an ordinary desktop application or a playable browser client, using the user's original game resources. | Sustained human standard/scenario play on both surfaces, equivalent decisions and save/resume; no original executable needed at runtime. |
| Headless all-AI simulation | Run every civilization under AI to test scenarios and compare seeds, difficulty, policy configuration and other declared parameters. | Reproducible run manifests, comparable outcomes, event traces, checkpoints and explicit failure/stop reasons. |
| Observation | Watch live games and recorded runs, including scenarios; inspect turns, events, objectives and maps through full spectator or per-civ views. | Observation agrees with the admitted headless run; recorded inspection never executes it again. |
| Faithful UI plus QoL | Recreate ToT's full screens, controls, decisions, reports, audio and scenario presentation; offer optional improvements and alternative UI settings. City production queues and rally points are explicit candidates. | A screen/feature coverage matrix, original-reference evidence and owner-tested routes in both clients; enhancements have separate contracts/settings. |
| AI choice | Select different versioned policies for factions and experiments, eventually including external plugins and suitable C-evo/Freeciv-derived candidates. | Legal orders, declared knowledge/budgets, scenario-aware evaluation and reproducible private policy state. Native-AI reproduction has separate fidelity criteria. |
| Scenario workbench | Open and inspect scenarios, diagnose unsupported features, test-run changes and help authors create them. Grow validation/diff tools toward safe editing/export and richer authoring. | A declared compatibility corpus, actionable diagnostics, reproducible tests and round-trip evidence for claimed original-format writes. |
| Session recording | Help players, observers and authors record sessions, revisit events, annotate findings and share reproducible evidence. | Versioned session bundles and recoverable recordings; later audio/video capture and clips are separately tested deliverables. |

These are targets, not claims that every feature is implemented. Parsing a file,
rendering a map, running autoplay and completing a human scenario establish
different levels of support. Current capabilities belong in the delivery plan
and release readiness records.

## Compatibility and resources

The main ToT experience requires the user's original installation/resources,
including scenario overrides. Ship no proprietary sprites, maps, saves, sounds
or scenario files. The native Rust engine implements gameplay; the original
game/manual and measured executable behavior define compatibility.

The browser is a required playable surface. Its accepted local Rust host is a
valid delivery route. Browser-contained execution and public hosting need
separate engineering decisions, including user resource access and session/save
isolation. A local client does not establish a public hosted service.

The approved, separately licensed Isotrident fallback remains an optional
artwork route. It does not replace ToT rules or establish scenario compatibility.
The small Open demo is a demonstration, not a complete redistributable ToT game.
A full independent standard rules profile remains separate scope.

Classic macro events, ToTPP/Lua and ToT's multiple-map modes are explicit
compatibility tracks. Preserve custom names, dates, currency, rules, resources,
objectives, restrictions and event timing. Report unsupported behavior rather
than silently substituting rules or disabling scenario logic.

## What Freeciv and C-evo change

They supply concrete AI, client, rendering and workflow candidates to evaluate.
Use their available sources and runnable references to avoid unnecessary
reinvention. Adoption must preserve ToTEngine's scenario semantics, validated
commands, visibility and replay contracts, and satisfy both delivery surfaces.
Their rules are not ToT evidence, and their AI strength remains to be measured.

The [source assessment](ai-client-reuse-assessment.md) and
[prior-art policy](prior-art.md) govern reuse: C-evo source requires per-file
provenance review and its media is not cleared; Freeciv code/client adoption
needs an explicit licensing/integration decision. No port or license change is
implied by this mission.

Exact native-AI reproduction remains a separately tracked compatibility program
([#117](https://github.com/diegoami/totengine/issues/117)), not the sole route to
capable opponents or a prerequisite for all AI alternatives. Preserve the
existing gate: complete playable-client delivery precedes full-AI ports and
external plugins. Source study and interface planning may proceed now.

## Delivery discipline

Use one deterministic simulation for human play, AI runs and observation;
preserve gameplay and policy identities in recordings. Develop original UI
coverage and optional QoL explicitly, without silently changing default rules
or controls. Release small coherent slices with both-client human acceptance
where relevant and independent owner-launched milestone review. This mission
assigns no version, calendar date or fixed remaining milestone count.
