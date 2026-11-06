# Isolated desktop spike notices

Versions follow the adjacent Cargo.lock, inspected from downloaded packages on
2026-11-06. Shared image notices are reused from docs/third-party. No fonts are
bundled: Linux reads a system DejaVu font, Windows a system Arial font, or the
user supplies TOTENGINE_FONT. Font files and original art never enter packages.

| Crate | Selected license | Notice |
| --- | --- | --- |
| allocator-api2-0.2.21 | MIT | allocator-api2-0.2.21.txt |
| autocfg-1.5.1 | see shared decoder notices | ../../../docs/third-party/autocfg-1.5.1.txt |
| bitflags-1.3.2 | MIT | bitflags-1.3.2.txt |
| bytemuck-1.25.2 | see shared decoder notices | ../../../docs/third-party/bytemuck-1.25.2.txt |
| byteorder-lite-0.1.0 | see shared decoder notices | ../../../docs/third-party/byteorder-lite-0.1.0.txt |
| cfg-if-1.0.5 | MIT | cfg-if-1.0.5.txt |
| color_quant-1.1.0 | see shared decoder notices | ../../../docs/third-party/color_quant-1.1.0.txt |
| core_maths-0.1.1 | MIT | core_maths-0.1.1.txt |
| equivalent-1.0.3 | MIT | equivalent-1.0.3.txt |
| foldhash-0.1.5 | Zlib | foldhash-0.1.5.txt |
| fontdue-0.9.5 | MIT | fontdue-0.9.5.txt |
| gif-0.14.2 | see shared decoder notices | ../../../docs/third-party/gif-0.14.2.txt |
| hashbrown-0.15.5 | MIT | hashbrown-0.15.5.txt |
| image-0.25.10 | see shared decoder notices | ../../../docs/third-party/image-0.25.10.txt |
| lazy_static-1.5.1 | MIT | lazy_static-1.5.1.txt |
| libc-0.2.191 | MIT | libc-0.2.191.txt |
| libm-0.2.16 | MIT | libm-0.2.16.txt |
| moxcms-0.8.1 | see shared decoder notices | ../../../docs/third-party/moxcms-0.8.1.txt |
| num-traits-0.2.19 | see shared decoder notices | ../../../docs/third-party/num-traits-0.2.19.txt |
| pxfm-0.1.30 | see shared decoder notices | ../../../docs/third-party/pxfm-0.1.30.txt |
| sdl2-0.38.0 | MIT | sdl2-0.38.0.txt |
| sdl2-sys-0.38.0 | MIT bindings + zlib SDL | sdl2-0.38.0.txt and SDL2-2.32.10.txt |
| ttf-parser-0.25.1 | MIT | ttf-parser-0.25.1.txt |
| version-compare-0.1.1 | MIT | version-compare-0.1.1.txt |
| weezl-0.1.12 | see shared decoder notices | ../../../docs/third-party/weezl-0.1.12.txt |

SDL2 runtime 2.32.10: [zlib notice](SDL2-2.32.10.txt).
Windows LLVM libunwind and linked runtime: retain [LLVM/toolchain notice](llvm-mingw.txt)
and [MinGW runtime notice](mingw-runtime.txt). No GPL game implementation code
is used. SDL's HIDAPI uses its BSD license alternative; do not select its GPL
alternative when rebuilding SDL. This spike uses the official runtime binaries.

Additional SDL permissive component notices: [HIDAPI BSD](SDL2-HIDAPI-BSD.txt) and [YUV conversion](SDL2-yuv2rgb.txt).
