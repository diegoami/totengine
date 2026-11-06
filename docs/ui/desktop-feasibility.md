# Native desktop feasibility result

The isolated [SDL2 spike](../../tools/desktop-spike/README.md) passes #126's
first-platform feasibility checks on native Linux and native Windows. SDL2
0.38.0 bindings with SDL2 2.32.10 and fontdue 0.9.5 are a viable choice for the
first desktop adapter. It is not a playable desktop client or a port of the
simulated nine-screen design.

[Retained numeric evidence](../evidence/desktop-feasibility.json) identifies the
source files, platform/toolchain, reports, assets evidence and limits. Linux was
Ubuntu 26.04.1 under WSL2 with an isolated authenticated Xvfb/X11 display. Windows
was the actual Windows 10.0.19045.7807 host reached through WSL interop, with the
SDL Windows driver, not Wine. No original game process was launched or driven.

Both platforms built and launched a standalone application window, drew owned
Original terrain/unit/city samples via the shared decoder, rendered text from
an owned system font, accepted/edited a name, traversed and accepted/cancelled
a modal, handled focus loss/return, resized/scaled, and closed. The assembled
packages passed real OS input checks without Cargo, a browser or an HTTP host.
Linux used XTest on its private display. Windows guarded each scan-code SendInput
against its child's foreground window; legacy SendKeys failed and is not used.
The repeated test reports preserve the distinction between OS input and internal
fixtures. Frame counts and startup focus events are timing-dependent metadata.

Two environment traps were resolved: Linux's WSL shared X11 socket directory
cannot host new sockets, so the isolated test uses authenticated TCP Xvfb; the
available LLVM cross compiler needs the `windows-gnullvm` Rust target and its
`libunwind.dll` at runtime. The plain GNU target needed absent GCC libraries.
No global toolchain/linker configuration was changed. Missing resource and font
diagnostics passed. Private modal/map captures were visually inspected; original
pixels and all executable/DLL packages stay outside Git.

The spike includes a synthetic mode and display-free SDL dummy-driver check for
CI, a reproducible package assembler and redistributed dependency/runtime notices.
It ships no fonts or game assets. System font fallback is explicit and overridable.
There is no game/session fork and production crates still have no SDL dependency.

This resolves the toolkit feasibility prerequisite for #130. Shared resource
atlas, adjacency, owner colors and full presentation remain #128; browser #129,
desktop #130 and assembled checkpoint/human-route acceptance #131 remain open.
No sustained campaign usability, IME/accessibility, full native renderer parity,
Wayland/distro matrix or completed playable-client claim follows from this spike.
