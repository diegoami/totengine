# Reusing C-evo and Freeciv for ToT scenario play

First source assessment, 2026-11-08. The owner asked to download/analyze C-evo
and clarified the product direction: ToT scenario compatibility is our niche;
stronger AI, an existing client or better UI workflows are candidates for
adoption. Reproducing the original opponent is not the only approach to assess.
Neither competitor has been benchmarked against ToT here. No AI/client port or
runtime change is included in this assessment.

## Roadmap integration

The owner's seven product targets are now recorded in [mission.md](mission.md),
[plan.md](plan.md) and decision 0008. Live programs: AI adoption/evaluation
[#187](https://github.com/diegoami/totengine/issues/187), optional workflows
[#188](https://github.com/diegoami/totengine/issues/188), and scenario compatibility
[#183](https://github.com/diegoami/totengine/issues/183). These organize this
assessment into delivery work; they do not start a port, change licensing or
remove the complete-client gate for full-AI/plugin implementation.

## Downloaded reference and scope

C-evo-x is checked out at
`/home/diego/review-tmp/c-evo-x-reference`, pinned to
`8e2e203e7766bc5524e989137b00d110a517238d` from
[donmccaughey/C-evo-x](https://github.com/donmccaughey/C-evo-x).
This fork assembles the C-evo 1.2.0 game, standard AI and development kits;
it is not claimed to be an untouched original release archive.
The checkout is about 29 MiB including assets and Git metadata. Its standard-AI
directory has nine source files / 9,943 lines; this count includes support and
protocol code, not just decision logic. All downloaded code/assets stay outside
this repository. [Source identities and observations](evidence/cevo-source-assessment-v1.json).

The official New Horizons page's source-download link and a guessed release
archive both returned HTTP 404. No New Horizons archive was obtained or analyzed.
The working reference is the pinned C-evo-x checkout. Freeciv observations below
come from the already installed 3.2.7 source tree. No downloaded build scripts,
game binaries or scenario scripts were executed. No C-evo compiler/build/runtime
test was performed; Delphi/VCL portability is assessed from source.

## License correction

`Project/readme.txt:14` explicitly dedicates the code to the public domain.
The root `LICENSE` makes a broad public-domain statement and lists contributors.
However, `Resources/Help/help.txt:157–165` explicitly excludes graphics and
sounds and directs users to obtain graphics authors' permission for other uses.
Treat C-evo source as a promising adaptation candidate, subject to a per-file
audit, and **do not treat the artwork/sounds as cleared for our distribution**.
No source has been copied or translated into ToTEngine. Retain provenance and
individual contributors if a later task adopts eligible code; for example,
`Project/IPQ.pas` credits Rassim Eminli separately.

Freeciv's inspected AI headers carry GPL-2.0-or-later. Actual code/client reuse
would need an explicit integration and licensing decision under our current MIT
policy; this assessment does not approve GPL code copying. A separate process
does not automatically settle licensing. Isotrident remains the approved artwork
fallback, with its own license and credits.

## What the C-evo source actually supplies

| Area and source | Observed design | Use for ToTEngine |
| --- | --- | --- |
| `Project/LocalPlayer/LocalPlayer.pas`, `docs/protocol/c/aidev.html:92–178`, `Project/Protocol.pas:1059` | Human UI and AI are competitor modules using the same server command callback. Player context holds own and known-enemy lists, map memory and reports. | Strong architectural reference for shared human/AI capabilities. Keep our validated orders and filtered observations; native pointers are not an enforceable security boundary. |
| `Project/GameServer.pas:3712–3743`, `Project/Protocol.pas:73` | Commands have preview/execution forms. Buying validates city state, resources and cost centrally. The manual says movement previews suppress failures caused by hidden enemies. | Useful validation checklist: previews must be inert and must not become hidden-information probes. Reuse the idea, not C-evo's purchase formula. |
| `Project/LocalPlayer/CityScreen.pas:997–1370` | Own-city mutation guard, tile assignment, support inspection, production category/switch confirmations, and buy/sale queries followed by server calls. | High immediate value for completing workflow contracts and tests. City panel roles and keyboard conventions still follow the owner's ToT requirements. |
| `Project/LocalPlayer/IsoEngine.pas` and `Term.pas:4370–4420` | Terrain/shore/object/unit/city passes, connection masks, paint bounds; timer-driven blinking with modal handling and active-application checks for scrolling. | Useful reference for coast matching, layering and input/presentation separation. GDI/VCL drawing calls are not a browser/SDL renderer. |
| `AI_Template/Project/Lib/Sprawl.cs`, `AddressPriorityQueue.cs`, `Project/IPQ.pas` | Priority-queue traversal, backtraces, travel state with turns and remaining moves, domain-specific costs and explicit treaty/blocking/ZoC options. | Good candidate design for go-to and route previews. Recompute edge costs from ToT mechanics and scenario rules; do not transplant encoded C-evo movement values. |
| `StdAI/AI.pas:1052–1200`, `ToolAI.pas` | Map analysis, attack/patrol, returning units, settler work, city production and tax adjustment; helper routines for job assignment and sea-transport group plans. | Substantial concrete AI to evaluate/adapt, especially coordination and transport planning. It is a different game's policy, not a reproduction of ToT's opponent. |
| `StdAI/AI.pas:23–63,201–354,2159`, `Project/Protocol.pas` | Fixed research-order arrays, C-evo unit-model design/quality categories, numeric improvement and wonder meanings, production priorities. | Main adapter work: replace fixed C-evo semantics with capabilities and ToT scenario definitions. Direct array/ID translation would be unsafe semantically. |
| `Project/GameServer.pas:449–480`, `StdAI/AI.pas:82`, `AI_Template/Project/Persistent.cs` | Native DLL callbacks and private AI data versions/sizes; separate persistent data structures. | Useful persistence checklist. Preserve our deterministic policy checkpoints; do not adopt the Windows DLL ABI or raw pointers as a web/portable plugin contract. |

The inspected C-evo human interface and AI are separated logically, which helps
adaptation. Its game server is an in-process native component, **not a ready web
backend**. Its protocol also warns that list indices can change between turns;
our existing entity identities and stale-input checks must survive any adapter.

## Freeciv comparison and adoption choices

Freeciv's `doc/README.AI` places logic in both `ai/` and `server/advisors`.
`ai/default/daicity.h` exposes direct Freeciv city/player/effect/advance types and
AI caches. The module interface is therefore not a standalone language-neutral
player protocol. The source includes domestic, technology, diplomacy, military,
air and ferry planners; those categories establish coverage, not playing strength.

| Option | Assessment | Main work before adoption |
| --- | --- | --- |
| Adopt UI workflows | Highest immediate return; compatible with both existing clients | Translate task flows into shared screen/command contracts and real-input regressions |
| Adapt selected C-evo algorithms | Promising source reuse route after per-file provenance checks | Separate generic planning from C-evo rule IDs, unit designs, costs and native memory |
| Adapt Freeciv AI | Worth a bounded prototype; broad planner coverage | Substantial Freeciv world/advisor/action adaptation plus an explicit GPL integration decision |
| Adopt a whole client | Keep as a real candidate, not a promised shortcut | C-evo needs a portable rendering/input layer; Freeciv needs investigation of protocol/ruleset/view translation. Both must expose ToT-specific state and scenario dialogs. Browser and standalone desktop remain required. |
| Put ToT imports into another engine | Highest risk to the niche | Loading a map/save is insufficient: ToT events, timing, custom mechanics, maps and persistence must also survive. No claim that either existing engine can execute our ToT scenario corpus. |

These are engineering judgments from the inspected coupling, not measured port
estimates. The recommended first adoption experiment is a small planning slice
behind our observation/order boundary, keeping ToTEngine authoritative for
simulation and events. The experiment is not started by this source assessment.
Do not force an original-AI port merely to obtain capable opponents; retain
original behavior as a compatibility baseline where a scenario depends on it.
This does not silently cancel the scoped original-reproduction program #117.

## What the adapter and acceptance tests must preserve

1. Scenario definitions: custom unit roles/flags, costs, technology prerequisites,
   government/effect meanings, disabled actions and map identity. Infer abilities
   from definitions, not English names or hardcoded standard-game slots.
2. Observations: own/visible/last-seen state and explicit unknowns. A route or
   attack preview must not disclose hidden defenders or scripted future events.
3. Commands: requests go through our authority and event timing. An AI cannot
   directly mutate units or invent a building just because its source game has it.
4. Persistence: private planner state, mission identity, randomness and adapter
   version must replay and resume through engine checkpoints.
5. Scenario goals and constraints: victory objectives, protected/capturable units,
   reinforcements, changing technologies, multiple maps and house rules need
   explicit semantics or documented limitations. A general AI cannot reliably
   infer every author's intention from arbitrary event scripts or prose.
6. Evaluation: identical starts, declared knowledge/handicaps, fixed budgets and
   seeds, multiple factions and held-out scenarios. Measure objective completion,
   illegal/stalled orders, economic/military competence and replay correctness.
   Source availability or reputation alone is not evidence of stronger play.

## First additional community scenario

The owner authorized downloading community scenarios and explicitly directed
installation under the actual CivTot scenario directory. **Roman Civil Wars**
by techumseh was fetched from its [Scenario League page](https://sleague.civfanatics.com/index.php/Roman_Civil_Wars),
with the separate sound archive. It is installed at
`D:\games\CIVTOT\Scenario\Roman Civil Wars\RomCivWars.SCN`
(`/mnt/d/games/CIVTOT/Scenario/Roman Civil Wars/RomCivWars.SCN`).
All 43 scenario/audio files were hash-verified after installation into a new
folder. Existing scenario files and root Original assets were not overwritten.
The separate IMP/global UI replacement pack was not installed. The author's
sprite-disable instructions were not applied to the shared installation.

The format probe reads a ToT 1.1 (`0x0032`) file, 202,069 bytes, with 159 cities
and 618 units. This is a static parse, not a native launch or engine play pass.
The installed readme describes capturable senators as victory points, delayed
successor leaders, seasonally granted/removed Navigation, and special airbase/
fortress house rules. The macro file contains 121 `CREATEUNIT` action lines and
69 `UNITKILLED` condition lines. These author-described mechanisms make useful
adapter tests; their full native behavior has not been re-derived here.
Archives, extracted game files and full probe output stay outside Git; only
hashes, counts and this analysis are retained. The Linux 7-Zip build could list
the RARs but could not decompress them; installed Windows UnRAR extracted them
successfully. No game or downloaded scenario script was run.
