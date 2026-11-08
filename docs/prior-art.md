# Prior art & licensing policy

Checked 2026-11-01 (T0-12). License facts verified via the GitHub API `license`
field plus the repos' own LICENSE files where present.

| Project | What it is | License | May we copy code? | Policy for ToTEngine |
|---|---|---|---|---|
| [C-evo-x](https://github.com/donmccaughey/C-evo-x), pinned `8e2e203e7766bc5524e989137b00d110a517238d` | C-evo 1.2.0 game, standard AI and development kits | Code has explicit **public-domain** declarations; bundled help excludes graphics/sounds | Source adaptation candidate after per-file audit; none adopted | Downloaded outside Git; assess AI/client adoption around ToT scenario compatibility. Do not reuse bundled media under the broad root declaration. [Assessment](ai-client-reuse-assessment.md). |
| [Freeciv](https://github.com/freeciv/freeciv) 3.2.7 | Civilization-inspired game and Isotrident graphics | **GPL-2.0-or-later** | No game code copying under this project's policy | Owner approved a separately licensed Isotrident artwork fallback on 2026-11-08. Keep PNG/spec source, credits and GPL notice in its own pack; MIT engine code is independent. [Assessment and limits](ui/freeciv-assessment.md). |
| [axx0/Civ2-clone](https://github.com/axx0/Civ2-clone) | C# re-implementation of Civ2; loads ToT saves | **GPL-3.0** | **No** (unless ToTEngine relicenses GPL-3.0, which we won't) | Read for *format facts* only (facts are not copyrightable); cite it in evidence when a finding agrees/disagrees with its behavior. No code, no files. |
| [richardpenman/civ2](https://github.com/richardpenman/civ2) | C reconstruction of Civ2 (new, Oct 2026) | **MIT** | Yes, with attribution | Same as above by preference: independent implementation; if we ever copy a function, keep the copyright line and cite the commit. |
| [ProfGarfield/ZeroBraneForTOTPP](https://github.com/ProfGarfield/ZeroBraneForTOTPP) | ZeroBrane fork with ToTPP Lua API autocomplete | **MIT** (inherits ZeroBrane Studio's LICENSE) | Yes, with the MIT notice | `docs/refs/totpp-api-zerobrane.lua` is vendored from it; an MIT notice is prepended to the file (required for substantial portions). |
| [pkulchenko/ZeroBraneStudio](https://github.com/pkulchenko/ZeroBraneStudio) | Upstream IDE | MIT | Yes | Not needed; noted as license source for the fork. |
| ToTPP (TheNamelessOne), CivFanatics threads | The patch project itself; public documentation posts | none stated (public forum posts) | No code (not open-source); quoting small doc excerpts is fine | Reference URLs only; the machine-readable API contract we keep comes from the MIT-licensed ZeroBrane fork above. |
| Mercator's Apolyton SAV/SCN thread | Community reverse-engineering reference | public forum posts | n/a (it is documentation) | Cite the thread for every structural claim we rely on; our evidence artifacts re-derive the claims locally. |

## Rules for this repo

1. **No original game assets, ever** (sprites, maps, saves, sounds from
   `/mnt/d/games/CIVTOT`) — only derived data (JSON summaries, PNG renders we
   generate) as small evidence artifacts.
2. **GPL-3.0 code never enters this repo.** If a GPL project's behavior must be
   matched, the behavior is re-derived from the game (oracle) or from public
   format documentation, then implemented from scratch; the derivation path is
   recorded in `docs/evidence/`.
3. **MIT/BSD-licensed code may be vendored** with the upstream notice kept in
   the file header and provenance recorded in the adjacent README.
4. ToTEngine itself is MIT (see `LICENSE`); format facts and evidence are
   published under the same license as the repo.
