# Isolated desktop feasibility (#126)

This is an ordinary SDL2 application window, with no browser or HTTP dependency.
It uses the shared Rust resource loader to draw owned Original terrain, a unit
and city, or authored synthetic fixtures when `CIVTOT_HOME` is unset. It is not
a playable client. Its separate Cargo workspace keeps the candidate toolkit out
of production until feasibility is established.

The native Linux and Windows runtime, real OS input and packaged-launch checks
passed; [evidence and limits](../../docs/ui/desktop-feasibility.md) records the
specific environments. SDL2 is now a viable first-client implementation choice,
not a claim that the normal desktop client is done.

## Build and launch

Linux with SDL2 development libraries installed:

```sh
cargo build --manifest-path tools/desktop-spike/Cargo.toml
CIVTOT_HOME=/path/to/CIVTOT tools/desktop-spike/target/debug/tot-desktop-spike
```

The reference machine has only the runtime. A temporary development link works:

```sh
mkdir -p /tmp/tot-sdl-lib
ln -s /usr/lib/x86_64-linux-gnu/libSDL2-2.0.so.0 /tmp/tot-sdl-lib/libSDL2.so
RUSTFLAGS='-L native=/tmp/tot-sdl-lib' cargo build --manifest-path tools/desktop-spike/Cargo.toml
```

Windows can build with a normal Rust/SDL2 development setup. On the reference
WSL machine the available cross compiler is LLVM, so use the matching Rust target
rather than `windows-gnu` (which requires GCC runtime libraries):

```sh
rustup target add x86_64-pc-windows-gnullvm
# Set these to your extracted official SDL2-devel and llvm-mingw toolchain.
export CARGO_TARGET_X86_64_PC_WINDOWS_GNULLVM_LINKER=/path/to/llvm-mingw/bin/x86_64-w64-mingw32-clang
export CARGO_TARGET_X86_64_PC_WINDOWS_GNULLVM_RUSTFLAGS='-L native=/path/to/SDL2/x86_64-w64-mingw32/lib'
cargo build --manifest-path tools/desktop-spike/Cargo.toml --target x86_64-pc-windows-gnullvm
```

Put the matching `SDL2.dll` and LLVM `libunwind.dll` beside the Windows executable.
The first failed runtime launch demonstrated that the latter is required; a
cross-compile alone is insufficient. Official SDL runtime used:
[SDL 2.32.10 release](https://github.com/libsdl-org/SDL/releases/tag/release-2.32.10).
Bindings: [rust-sdl2](https://github.com/Rust-SDL2/rust-sdl2).

`N` opens city-name entry; Backspace edits; Tab/Shift-Tab traverses the field and
buttons; Enter accepts or activates the focused cancel button; Escape cancels
the modal or closes the main window. The window is resizable with a 960×640
logical canvas. Lost focus suspends text input. DejaVu Sans (Linux) or Arial
(Windows) is read from the system; set `TOTENGINE_FONT` to an owned TTF if needed.
Missing installations/art/fonts get a diagnostic before opening the window.

## Verification

```sh
# Synthetic fixtures can run in CI without a display or game install.
SDL_VIDEODRIVER=dummy tools/desktop-spike/target/debug/tot-desktop-spike --self-test /tmp/spike.json
# Real X11 OS input on an isolated authenticated Xvfb (needs libX11/libXtst).
python3 tools/desktop-spike/check_linux.py
CIVTOT_HOME=/path/to/CIVTOT python3 tools/desktop-spike/check_linux.py
```

On Windows PowerShell, after building and supplying the two runtime DLLs:

```powershell
$env:CIVTOT_HOME = 'D:\games\CIVTOT' # or unset for synthetic fixtures
powershell -NoProfile -ExecutionPolicy Bypass -File tools\desktop-spike\check_windows.ps1
```

The Windows checker enumerates only its child's window and checks its foreground
ownership before sending each key. It uses real scan-code `SendInput`; legacy
.NET `SendKeys` did not deliver the SDL input in this environment. Linux uses
XTest on a private display. Both type/edit `athena`, traverse/accept/cancel a
modal, lose/regain focus, resize and close. The report's `input_method` separates
these checks from the internal scripted fixture. `--self-test` queues SDL keys/
window events but directly dispatches text fixtures because rust-sdl2 cannot
queue TextInput. It checks an accented name and rejects unfocused text.

`--external-test REPORT.json` waits for OS input and checks that route. The
checkers accept `TOTENGINE_SPIKE_BINARY`/`TOTENGINE_SPIKE_REPORT` (Linux) and
`-Binary`/`-Report` (Windows) for packaged-launch tests. Reports are numeric/text
only. Optional `TOTENGINE_SPIKE_CAPTURE=/private/path/prefix` writes BMP captures
at frames 20 and 35 for private visual inspection; **never commit installed-art
captures**. No evidence files or artwork are written by normal launch.

## Package

`package.py --binary PATH --out NEW_DIRECTORY --rust-docs PATH` assembles the
executable and all notices. `--rust-docs` is the toolchain's `share/doc/rust`
directory. Windows additionally requires `--sdl-runtime PATH/SDL2.dll` and
`--unwind-runtime PATH/libunwind.dll`. Linux uses the system SDL2 runtime.
The output must be new; no game artwork or font files are copied. Both assembled
packages were launched without Cargo and passed the OS input route. They remain
private scratch, not production release downloads.

Keep [notices](notices/README.md) and shared decoder notices with distributions.
The final client still needs a complete resource atlas/presentation model,
installation/setup workflows, engine input/choices, persistence widgets and both
client acceptance. IME composition and accessibility remain unproven here.
