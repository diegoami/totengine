# Client decoder dependency notices

The shared Rust resource loader uses image 0.25.10 with default features disabled
and only BMP/GIF enabled. It contains no original game data. Versions below are
pinned by the workspace Cargo.lock. License facts and notices were checked against
the downloaded crates' Cargo.toml and license files on 2026-11-06.

Upstream: [image-rs/image](https://github.com/image-rs/image),
[image API](https://docs.rs/image/0.25.10/image/). No prior-art game code was copied.

| Crate/version | License selected for distribution | Retained notice |
| --- | --- | --- |
| image 0.25.10 | MIT | [notice](image-0.25.10.txt) |
| gif 0.14.2 | MIT | [notice](gif-0.14.2.txt) |
| color_quant 1.1.0 | MIT | [notice](color_quant-1.1.0.txt) |
| bytemuck 1.25.2 | MIT | [notice](bytemuck-1.25.2.txt) |
| byteorder-lite 0.1.0 | MIT | [notice](byteorder-lite-0.1.0.txt) |
| weezl 0.1.12 | MIT | [notice](weezl-0.1.12.txt) |
| num-traits 0.2.19 | MIT | [notice](num-traits-0.2.19.txt) |
| autocfg 1.5.1 (build dependency) | MIT | [notice](autocfg-1.5.1.txt) |
| moxcms 0.8.1 | BSD-3-Clause | [notice](moxcms-0.8.1.txt) |
| pxfm 0.1.30 | BSD-3-Clause | [notice](pxfm-0.1.30.txt) |

Include these notices with any distributed client binaries containing the
libraries, and update this inventory when the lockfile changes. This audit
selects no desktop toolkit and adds no font or image assets to the project.
