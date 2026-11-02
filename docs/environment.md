# Reference environment & provisioning

What a fresh machine needs to continue this work, and the verified quirks of
the box the Tier-0 findings were made on. Operational oracle-rig details live
in [`tools/oracle/README.md`](../tools/oracle/README.md) — this file is about
*setting the machine up*, not driving the rig.

## The original game (required, not in the repo)

The engine is a recreation that reads the user's own ToT install; nothing from
it is ever committed (see `docs/feasibility.md` evidence rules).

- **Install**: Civilization II: Test of Time 1.1 + **ToTPP** at
  `/mnt/d/games/CIVTOT` (a mounted NTFS data drive on the reference box).
- **Wine prefix**: `~/.wine_civ2tot` (Wine 10.0 verified), with drive
  `d:` → `/mnt/d/games/CIVTOT`, i.e. the game sees itself as `D:\`.
- **Materials used by probes/tests** (all under the install):
  `World.mp`, `Battle.mp`, `Rules.txt`, `FallRome/` (classic macro events;
  `ROMFALL.scn`), `Midgard/` (ToTPP Lua; autosave `Oc_Auto.SAV`), and
  `Scenario/china/` (*Warring States*, heavy ToTPP Lua:
  `Warring States.scn`, `AnticChina.MP`, the 0x33 saves `deiQin.sav` /
  `deiYan.sav`).
- Tools take the install root from **`CIVTOT_HOME`** (never hard-coded in
  library code; the golden corpus self-skips when it is unset, so CI works
  without the game).

## Toolchain (reference box: Ubuntu Linux, **no sudo**)

| Piece | Where / how | Notes |
|---|---|---|
| Rust | rustup; `cargo` at `~/.cargo/bin/cargo` | `make` falls back to that path |
| Python | 3.14, **stdlib only** — `tools/totctl.py` has no pip deps | |
| gcc + libX11/libXtst dev headers | system | needed to build `tools/oracle/xtool` |
| **llvm-mingw** | `/home/diego/review-tmp/llvm-mingw` (from the tarball next to it); **not on PATH** — use its `bin/clang` by full path | needed to build `tools/oracle/winject.exe`; any recent llvm-mingw release works |
| Xvfb | system (`Xvfb :99`) | |
| Wine | 10.0 | prefix above |
| `gh` CLI | authed as `diegoami` | issues/milestones/releases |

No sudo is why nothing was `apt install`ed ad hoc: `xtool` exists because
xdotool couldn't be installed, pycdlib was unpacked from a wheel into scratch,
and llvm-mingw was downloaded as a tarball.

## Scratch space: `/home/diego/review-tmp`

Pre-created and approved for external access; **not part of the repo**. Ours:

- `llvm-mingw/` + `llvm-mingw.tar.xz` — the cross toolchain (see table above)
- `pylib/` + `pycdlib.whl` — pip-less install; `build_cd.py` needs
  `PYTHONPATH=/home/diego/review-tmp/pylib`
- `totcd.iso` — the fake-CD image `build_cd.py` produced (dead end for the
  classic exe's cdaudio check; kept as evidence, see oracle README)
- `oracle-runs/` — harvested oracle saves (ephemeral; reproduce with
  `turnloop.sh`)

The directory is **shared with unrelated projects** (hundreds of `harness-*` /
`guard-*` entries) — never wipe it blindly.

## Binaries that are NOT in git (rebuild after cloning)

`.gitignore` excludes `tools/oracle/xtool` and `tools/oracle/winject*.exe`;
only sources are committed. Rebuild (commands also in the oracle README):

```sh
cd tools/oracle
gcc -O2 -o xtool xtool.c -lX11 /usr/lib/x86_64-linux-gnu/libXtst.so.6
/home/diego/review-tmp/llvm-mingw/bin/clang --target=x86_64-pc-windows-gnu \
    -O2 -o winject.exe winject.c -luser32
```

## Verified box quirks

- **Repo name vs checkout dir**: the GitHub repo is `diegoami/totengine`
  (renamed from `civtotre`; old URLs redirect), but the reference checkout is
  still `/home/diego/projects/civtotre`. Any tooling that assumes the checkout
  dir equals the repo name breaks — pass explicit working directories.
- **Git identity is configured** on the box (`Diego Amicabile
  <diego.amicabile@gmail.com>`); plain `git commit` works. Early commits
  carrying explicit `-c user.*` overrides did not need them.
- `pkill -f <pat>` with a literal matching string in the *same command line*
  kills your own shell — use `[p]at` bracket patterns (bit us repeatedly
  during oracle work).
- Oracle-rig behavioral quirks (launcher listbox needs **double-click**,
  X-level keys never reach the game's DirectDraw surface — use winject,
  no-WM focus handling, classic exe unusable headless) are in the oracle
  README, not here.
