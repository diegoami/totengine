# Client dependency notices

Inventory checked against Cargo.lock and downloaded crate manifests/license files,
2026-11-06. Includes build dependencies and target-only permissive dependencies.
The two clients use image with only BMP/GIF enabled, serde JSON, OS randomness,
atomic checkpoint creation, and optional SDL2/fontdue for the native client.
No game implementation was copied from prior-art projects. No artwork or fonts
are distributed. Retain these notices in binary packages.

| Dependency | Selected license | Retained notice |
| --- | --- | --- |
| allocator-api2-0.2.21 | MIT | [allocator-api2-0.2.21.txt](allocator-api2-0.2.21.txt) |
| autocfg-1.5.1 | MIT | [autocfg-1.5.1.txt](autocfg-1.5.1.txt) |
| bitflags-1.3.2 | MIT | [bitflags-1.3.2.txt](bitflags-1.3.2.txt) |
| bitflags-2.13.2 | MIT | [bitflags-2.13.2.txt](bitflags-2.13.2.txt) |
| bytemuck-1.25.2 | MIT | [bytemuck-1.25.2.txt](bytemuck-1.25.2.txt) |
| byteorder-lite-0.1.0 | MIT | [byteorder-lite-0.1.0.txt](byteorder-lite-0.1.0.txt) |
| cfg-if-1.0.5 | MIT | [cfg-if-1.0.5.txt](cfg-if-1.0.5.txt) |
| color_quant-1.1.0 | MIT | [color_quant-1.1.0.txt](color_quant-1.1.0.txt) |
| core_maths-0.1.1 | MIT | [core_maths-0.1.1.txt](core_maths-0.1.1.txt) |
| equivalent-1.0.3 | MIT | [equivalent-1.0.3.txt](equivalent-1.0.3.txt) |
| errno-0.3.14 | MIT | [errno-0.3.14.txt](errno-0.3.14.txt) |
| fastrand-2.5.0 | MIT | [fastrand-2.5.0.txt](fastrand-2.5.0.txt) |
| foldhash-0.1.5 | Zlib | [foldhash-0.1.5.txt](foldhash-0.1.5.txt) |
| fontdue-0.9.5 | MIT | [fontdue-0.9.5.txt](fontdue-0.9.5.txt) |
| getrandom-0.3.4 | MIT | [getrandom-0.3.4.txt](getrandom-0.3.4.txt) |
| gif-0.14.2 | MIT | [gif-0.14.2.txt](gif-0.14.2.txt) |
| hashbrown-0.15.5 | MIT | [hashbrown-0.15.5.txt](hashbrown-0.15.5.txt) |
| image-0.25.10 | MIT | [image-0.25.10.txt](image-0.25.10.txt) |
| itoa-1.0.18 | MIT | [itoa-1.0.18.txt](itoa-1.0.18.txt) |
| lazy_static-1.5.1 | MIT | [lazy_static-1.5.1.txt](lazy_static-1.5.1.txt) |
| libc-0.2.191 | MIT | [libc-0.2.191.txt](libc-0.2.191.txt) |
| libm-0.2.16 | MIT | [libm-0.2.16.txt](libm-0.2.16.txt) |
| linux-raw-sys-0.12.1 | MIT | [linux-raw-sys-0.12.1.txt](linux-raw-sys-0.12.1.txt) |
| memchr-2.8.3 | MIT | [memchr-2.8.3.txt](memchr-2.8.3.txt) |
| moxcms-0.8.1 | BSD-3-Clause | [moxcms-0.8.1.txt](moxcms-0.8.1.txt) |
| num-traits-0.2.19 | MIT | [num-traits-0.2.19.txt](num-traits-0.2.19.txt) |
| once_cell-1.21.4 | MIT | [once_cell-1.21.4.txt](once_cell-1.21.4.txt) |
| proc-macro2-1.0.107 | MIT | [proc-macro2-1.0.107.txt](proc-macro2-1.0.107.txt) |
| pxfm-0.1.30 | BSD-3-Clause | [pxfm-0.1.30.txt](pxfm-0.1.30.txt) |
| quote-1.0.47 | MIT | [quote-1.0.47.txt](quote-1.0.47.txt) |
| r-efi-5.3.0 | MIT | UEFI target only; absent from Windows/Linux builds and packages |
| rustix-1.1.6 | MIT | [rustix-1.1.6.txt](rustix-1.1.6.txt) |
| sdl2-0.38.0 | MIT | [sdl2-0.38.0.txt](sdl2-0.38.0.txt) |
| sdl2-sys-0.38.0 | MIT + Zlib | [sdl2-sys-0.38.0.txt](sdl2-sys-0.38.0.txt) |
| serde-1.0.229 | MIT | [serde-1.0.229.txt](serde-1.0.229.txt) |
| serde_core-1.0.229 | MIT | [serde_core-1.0.229.txt](serde_core-1.0.229.txt) |
| serde_derive-1.0.229 | MIT | [serde_derive-1.0.229.txt](serde_derive-1.0.229.txt) |
| serde_json-1.0.154 | MIT | [serde_json-1.0.154.txt](serde_json-1.0.154.txt) |
| syn-3.0.9 | MIT | [syn-3.0.9.txt](syn-3.0.9.txt) |
| tempfile-3.27.0 | MIT | [tempfile-3.27.0.txt](tempfile-3.27.0.txt) |
| ttf-parser-0.25.1 | MIT | [ttf-parser-0.25.1.txt](ttf-parser-0.25.1.txt) |
| unicode-ident-1.0.27 | MIT + Unicode-3.0 | [unicode-ident-1.0.27.txt](unicode-ident-1.0.27.txt) |
| version-compare-0.1.1 | MIT | [version-compare-0.1.1.txt](version-compare-0.1.1.txt) |
| wasip2-1.0.4+wasi-0.2.12 | MIT | [wasip2-1.0.4+wasi-0.2.12.txt](wasip2-1.0.4+wasi-0.2.12.txt) |
| weezl-0.1.12 | MIT | [weezl-0.1.12.txt](weezl-0.1.12.txt) |
| windows-link-0.2.1 | MIT | [windows-link-0.2.1.txt](windows-link-0.2.1.txt) |
| windows-sys-0.61.2 | MIT | [windows-sys-0.61.2.txt](windows-sys-0.61.2.txt) |
| wit-bindgen-0.57.1 | MIT | [wit-bindgen-0.57.1.txt](wit-bindgen-0.57.1.txt) |
| zmij-1.0.23 | MIT | [zmij-1.0.23.txt](zmij-1.0.23.txt) |

Windows packages retain SDL2 2.32.10's Zlib notice, HIDAPI's BSD alternative,
YUV conversion notice, LLVM/MinGW runtime notices, and Rust's generated
COPYRIGHT-library.html and licenses directory. The optional SDL Rust bindings
use the system runtime; bundled SDL compilation is disabled. Linux packages
require the distribution's SDL2 runtime. Fonts are read from the user's system.
UEFI r-efi is lockfile-only for these targets and is not distributed.

Runtime notices: [SDL2](SDL2-2.32.10.txt), [HIDAPI BSD](SDL2-HIDAPI-BSD.txt),
[YUV](SDL2-yuv2rgb.txt), [LLVM](llvm-mingw.txt), [MinGW](mingw-runtime.txt).
The package script includes all notices rather than pruning build-only entries.
