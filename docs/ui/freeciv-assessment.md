# Freeciv reference and Isotrident fallback

Validation/provenance: [retained numeric record](../evidence/isotrident-fallback-v1.json).
Packaged Chromium, Linux SDL and Windows SDL each completed ten rounds and
save/reload routes without a ToT install; all seven retained checkpoints match
across the three clients. The actual-pack shared test also covers selecting and
moving the normally produced guard. Workspace tests, Python tests, goldens,
fmt and strict default/desktop Clippy pass. No new human acceptance or release
review is claimed.

Owner requested this assessment and approved Isotrident as a fallback on
2026-11-08, to support a future web application without requiring users to own
Test of Time. This is an implementation candidate, not a released milestone.

## Findings from the installed distribution

Inspected source `~/projects/freeciv/freeciv-3.2.7`, installed data under
`installed/share/freeciv` and binaries under `installed/bin`. Both server and
GTK client report 3.2.7. The GTK client opened its own window on an isolated
Xvfb with `GDK_BACKEND=x11`, Isotrident and sound disabled; it and the display
were stopped. This was a startup check, not an interactive city-screen study.

| Use | Value to ToTEngine | Boundary |
| --- | --- | --- |
| Running client | High: accessible reference for city management, reports, production lists, diplomacy, focus and input workflows | Study task flows; original ToT/manual and accepted owner choices still define our controls and fidelity |
| Isotrident artwork | High: redistributable 64×32 isometric terrain, units, cities and citizen icons | Requires semantic sprite mapping and its own license/credits; not a drop-in ToT sheet layout |
| Ruleset documentation | Useful checklist for mechanics, dependencies and edge cases | Not proof of original ToT behavior |
| Server/client and AI documentation | Useful architectural comparison for filtered views, action validation and asynchronous UI | No GPL game code copied into our MIT implementation; full AI work remains gated behind playable clients |
| Saves/scenarios | Possible future interoperability research | Freeciv scenario files are its own save format; no native ToT compatibility established by this inspection |

Freeciv does **not** have exactly the same rules as ToT. Local
`doc/README` explicitly describes intentional differences. Under `data/civ2`,
`units.ruleset:12` says approximate, `governments.ruleset:14` marks
Fundamentalism incomplete, and buildings/technology/effects descriptions mark
incompleteness. `game.ruleset:460,470` retains unanswered Civ2 trade questions;
`buildings.ruleset:505` describes an unimplemented Hydro Plant condition.
These are Freeciv's own compatibility caveats, not new ToT format claims.
Continue deriving original rules from original-binary evidence.

## License and provenance

Freeciv is GPL-2.0-or-later free software, not public-domain artwork.
The distribution's `COPYING` and `doc/README` are the license sources.
The selected sheets are unmodified PNGs; their `.spec` files retain artists and
editable crop definitions. For example, terrain credits include Tim F. Smith;
units include Tatu Rissanen; city definitions list Jerzy Klek and other artists;
the terrain2 sheet itself retains additional credits. Keep all of these files.

The owner-approved exception to the prior-art reference-only rule is narrowly
for a **separate artwork pack**. No Freeciv C/Lua game code or ruleset is copied.
ToTEngine's own code stays MIT; the artwork pack retains GPL-2.0-or-later.
Packages/web distributions must deliver its notices and editable artwork source
alongside the art. The pack includes the original PNG/spec files and upstream
source archive URL; transformed or externally sourced future assets require a
new provenance check. Do not label the combined distribution wholly MIT.

## Implemented candidate

`tools/prepare_isotrident.py` extracts six specified PNG/spec pairs plus the
tileset description, Freeciv README and GPL notice. It independently derives
381 sprite records, with content hashes. No original ToT data enters the pack.
Output stays outside Git. The package tool accepts `--isotrident-pack`, verifies
the exact file inventory/hashes and puts it beside both client binaries.

Both clients use the same adapter and RGBA/hit-test sprites. Existing complete
ToT resources take precedence. If artwork is missing but ToT rules exist, the
adapter preserves those rules. An explicitly invalid ToT path fails; it never
silently switches balance. Blank setup uses the **Open demo** rules embedded in
our MIT code, plus the redistributable art. The demo is independently authored
from our campaign fixture: three units, seven enabled improvement slots and
twelve research choices. Reserved classic slots are disabled. This demonstrates
install-free play; it is not full original ToT rules or a Freeciv ruleset.
Checkpoint identity includes rules, profile, manifest and all decoded sheets.

The initial renderer uses single terrain variants, open-water cells without
matched shorelines, two city sizes and shared neutral city styles. Unsupported
unit labels receive a visible neutral marker. Full Freeciv coast blending,
directional unit animations and all city styles are not claimed. Selection,
blinking, commands and economics remain our existing implementation.

## Run and package

```sh
python3 tools/prepare_isotrident.py \
  --source ~/projects/freeciv/freeciv-3.2.7 \
  --out /tmp/tot-isotrident
TOTENGINE_ISOTRIDENT=/tmp/tot-isotrident cargo run -p totui --bin totplay
```

Clear the optional ToT directory and choose **Start open demo**. The same
`TOTENGINE_ISOTRIDENT` setting works for `totdesktop`. A pack in `isotrident/`
beside the executable is discovered automatically. To use ToT rules with fallback
art, enter the installation directory or a Rules.txt path at setup. Packages use
the existing `tools/package_clients.py` arguments plus
`--isotrident-pack /tmp/tot-isotrident`. The supplied Freeciv binary is not a
runtime dependency.

This enables resource delivery for a future hosted app. The present `totplay`
host is loopback-only and owns one shared session. Internet hosting still needs
an explicitly designed per-user session/save boundary, resource delivery and
deployment configuration. No public service was deployed or hosting contract
changed here. A complete independently distributed standard rules profile also
remains separate from the deliberately small demo.
