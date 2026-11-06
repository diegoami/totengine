# Prototype with installed game artwork

The owner accepted the schematic prototype and then directed the next visual
pass to use the artwork already in the game directory (2026-11-06). This local
preview does that. It remains a simulated design prototype, with authored
terrain/cities/names/values and no engine command connection. It does not choose
the production client toolkit or complete the human-client milestone.

## Run it

From the repository root:

```sh
CIVTOT_HOME=/path/to/CIVTOT python3 tools/preview_client.py --profile Original
```

Open the printed loopback URL (default `http://127.0.0.1:8765`). If that port is
occupied, use `--port 8766` or `--port 0` to print an automatically chosen port.
The reference-machine install is `/mnt/d/games/CIVTOT`; substitute that path
for the example above on that machine. `--home` can also supply the install.

Map and city views use installed terrain/relief/forest sprites, units, cities,
citizens, resource icons and city/dialog textures. Zoom, selection, movement
variants, city edits and the other prototype screens still operate on fixtures.
The overview is rendered from the same authored map. Returning between views
keeps the installed-art renderer active. Opening `client-prototype.html`
directly retains the original schematic mode, without loading installed files.

The four PNG previews committed alongside the design are the **schematic**
revision, not captures of this mode. Screenshots of the installed-art mode stay
outside Git. Review it locally with the owner's installation.

## Asset selection and boundaries

The tool reads a fixed allowlist: Terrain1/2, Units, Cities, City, Icons, People
and Dialog. In the selected profile directory, it prefers BMP over GIF, then
falls back to matching root files if that sheet is absent. Filename lookup is
case-insensitive and fails on ambiguity. `--profile` is an install-relative
folder; traversal and symlinks escaping the install are rejected. This is the
preview's explicit lookup policy, not a claim to have recovered every native
resource search rule.

Only the fixed design documents/scripts and selected artwork are served, on
IPv4 loopback. Save files, Rules, arbitrary install paths and arbitrary repo
files are not HTTP routes. The tool does not write to the installation or
copy/convert/export assets. The browser decodes the selected files and builds
cropped/masked canvases and texture URLs **in memory**. No external dependencies
are needed to run the server. The optional browser check uses Playwright
installed outside the repository.

`/assets/manifest` identifies relative source filenames and SHA-256 values.
No original image bytes, base64 art, extracted atlases or installed-art captures
are committed. [Numeric evidence](../evidence/client-artwork.json) records the
Original-profile source hashes, browser-decoded dimensions and five sample-mask
measurements. Re-run the browser check below against the local server to
rederive it.

## Measured visual recipes and limits

These recipes were inspected against the installed **Original** sheets. They
are examples for this prototype, not a complete atlas specification or original
renderer. The source identities in the evidence bind the observations.

| Sheet         | Recipe used                                                                                    | Limit                                                                                                                  |
| ------------- | ---------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Terrain1      | 64×32 base diamonds at `(1 + variant×65, 1 + terrain×33)`; three sample variants.              | Fixture geography and variants are authored. No native terrain adjacency/variant algorithm recovered.                  |
| Terrain2      | 64×32 examples with the same pitch; rows 4, 6 and 8 supply forest, mountain and hill overlays. | Full connections, coast/river blending, roads, special resources, all maps and extra layers remain unimplemented.      |
| Units         | 64×64 examples at `(1 + column×65, 1 + row×65)`, nine columns.                                 | A few standard fixture unit indices only; no animation, owner-color/flag pipeline or scenario Rules-to-sprite adapter. |
| Cities        | 64×48 examples at `(1 + size×65, 39 + style×49)`.                                              | A fixed style and two sizes, not a native civilization/style/population/walls selector.                                |
| People        | First-row 27×32 example faces at x=`1 + column×28`.                                            | Schematic role preview, not full mood/era/government citizen rendering.                                                |
| Icons         | Inspected 20×20 food, tax, luxury, science, shield/support examples.                           | Specific crop coordinates are in the renderer; no complete icon atlas claim.                                           |
| City / Dialog | Inspected city-panel and dialog texture fragments.                                             | Proposed layout using real textures; not native border/window composition.                                             |

Pure magenta is keyed out; unit examples also remove the observed purple
backdrop. A boundary flood clears the observed gray matte while preserving
isolated interior detail. Resource icons additionally clear their observed
pink matte. Terrain bases use a diamond mask. These measured RGB recipes are
not a universal palette-index/alpha decoder; other profiles can need different
masks/layouts. Selecting another profile changes the artwork sources, **not**
the fixture's Rules, names, mechanics or scenario compatibility claim.

The current crop checks assert foreground plus transparency, no visible magenta
and a transparent corner in five measured samples. They cannot prove native
visual parity, complete palette correctness, exact coasts or sustained gameplay.

## Verification

Install-free selection/exposure checks:

```sh
python3 -m unittest discover -s tools -p test_preview_client.py
```

With the server running and Playwright available through `NODE_PATH`:

```sh
node tools/test_client_artwork.cjs http://127.0.0.1:8765 /tmp/client-artwork.json
```

The optional JSON output contains numeric metadata only; no images are written.
The check verifies served-byte hashes against the manifest, five sprite-mask
samples, zoom, a simulated move, citizen-image replacement without duplicate
images, all nine main screens, desktop/1024 layout and absence of JS errors.
The standalone schematic browser flows also passed after the renderer was added.

## Shared Rust loader (milestone 10 implementation)

`totui::resources::Resources::load` now implements the explicit profile/root
selection in Rust, including profile Rules, ASCII case-insensitive resolution,
ambiguity/missing diagnostics, symlink containment, bounded reads and BMP/GIF
(first frame) decoding to RGBA. This is shared data for the forthcoming two
adapters; the Python preview and its simulated actions remain unchanged.
`Resources::identity()` binds selected relative source names, content lengths and
FNV fingerprints to human checkpoints. A corrupt preferred BMP produces an
explicit error, rather than silently selecting a lower-precedence GIF.

`Resources::crop` implements the measured magenta/purple/edge-matte/diamond
recipes above. Its install-backed test compares all five retained preview sample
opacity counts and transparent corners, including the food icon. Synthetic tests
cover both codecs, bounded/corrupt data, resource precedence, Rules consistency,
traversal, missing/ambiguous names, symlink escape, rectangle bounds and preserving
isolated interior gray details. Reproduce with:

```sh
CIVTOT_HOME=/path/to/CIVTOT cargo test -p totui resources
```

The new `image` dependency enables only BMP/GIF; redistributed notices and its
transitive dependency audit are in [third-party notices](../third-party/README.md).
No original bytes are retained by tests or in Git. Full Rules-to-sprite atlas,
adjacency, owner colors, font and both-adapter rendering are still #128 work;
these five measured examples are not complete native rendering support.
