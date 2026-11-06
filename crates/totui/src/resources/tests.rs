use super::*;
use image::{DynamicImage, Rgba};
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "totengine-resources-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(path.join("Original")).unwrap();
        fs::write(path.join("Rules.txt"), b"@COSMIC\n3\n").unwrap();
        for stem in ["Terrain1", "Units", "Cities"] {
            fs::write(path.join(format!("{stem}.BMP")), encoded(ImageFormat::Bmp)).unwrap();
        }
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn encoded(format: ImageFormat) -> Vec<u8> {
    let image = DynamicImage::ImageRgba8(RgbaImage::from_fn(4, 4, |x, y| {
        Rgba([(x * 50) as u8, (y * 50) as u8, 0, 255])
    }));
    let mut output = Cursor::new(Vec::new());
    image.write_to(&mut output, format).unwrap();
    output.into_inner()
}

#[test]
fn bmp_and_gif_decode_and_invalid_data_is_bounded() {
    for format in [ImageFormat::Bmp, ImageFormat::Gif] {
        let image = decode(&encoded(format)).unwrap();
        assert_eq!(image.dimensions(), (4, 4));
        assert_eq!(image.get_pixel(0, 0).0, [0, 0, 0, 255]);
        assert_eq!(image.get_pixel(3, 3).0, [150, 150, 0, 255]);
    }
    assert!(decode(b"not an image").is_err());
    let mut bad = encoded(ImageFormat::Bmp);
    bad[18..22].copy_from_slice(&100_000i32.to_le_bytes());
    assert!(decode(&bad).is_err());
    assert!(decode(&bad[..20]).is_err());
}

#[test]
fn profile_precedence_case_rules_and_content_identity() {
    let f = Fixture::new();
    fs::write(f.0.join("Original/rUlEs.TxT"), b"@COSMIC\n7\n").unwrap();
    fs::write(f.0.join("Original/Units.GIF"), encoded(ImageFormat::Gif)).unwrap();
    let a = Resources::load(&f.0, "oRiGiNaL").unwrap();
    assert!(a.sources["units"].relative.ends_with("Original/Units.GIF"));
    assert!(a.sources["terrain1"].relative.ends_with("Terrain1.BMP"));
    assert!(a.sources["rules"].relative.ends_with("Original/rUlEs.TxT"));
    assert_eq!(a.rules.section("COSMIC").unwrap().rows[0][0], "7");
    fs::write(f.0.join("Original/units.bmp"), encoded(ImageFormat::Bmp)).unwrap();
    let b = Resources::load(&f.0, "Original").unwrap();
    assert!(b.sources["units"].relative.ends_with("Original/units.bmp"));
    assert_ne!(a.identity(), b.identity());
    assert_eq!(
        b.identity(),
        Resources::load(&f.0, "original").unwrap().identity()
    );
    fs::write(f.0.join("Original/units.bmp"), b"broken preferred BMP").unwrap();
    assert!(Resources::load(&f.0, "Original")
        .err()
        .unwrap()
        .contains("units"));
}

#[test]
fn traversal_missing_and_ambiguous_resources_have_diagnostics() {
    let f = Fixture::new();
    for profile in [
        "",
        ".",
        "..",
        "Original/../Original",
        "/Original",
        "C:/Original",
        "Original//",
    ] {
        assert!(Resources::load(&f.0, profile).is_err(), "{profile}");
    }
    fs::remove_file(f.0.join("Cities.BMP")).unwrap();
    assert!(Resources::load(&f.0, "Original")
        .err()
        .unwrap()
        .contains("cities"));
    #[cfg(target_os = "linux")]
    {
        fs::write(f.0.join("CITIES.bmp"), encoded(ImageFormat::Bmp)).unwrap();
        fs::write(f.0.join("cities.BMP"), encoded(ImageFormat::Bmp)).unwrap();
        assert!(Resources::load(&f.0, "Original")
            .err()
            .unwrap()
            .contains("ambiguous"));
    }
}

#[cfg(unix)]
#[test]
fn symlinks_cannot_escape_selected_installation() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let other = Fixture::new();
    symlink(other.0.join("Units.BMP"), f.0.join("Original/Units.BMP")).unwrap();
    assert!(Resources::load(&f.0, "Original")
        .err()
        .unwrap()
        .contains("escapes"));
    fs::remove_file(f.0.join("Original/Units.BMP")).unwrap();
    symlink(&other.0, f.0.join("Outside")).unwrap();
    assert!(Resources::load(&f.0, "Outside")
        .err()
        .unwrap()
        .contains("escapes"));
}

#[test]
fn crop_masks_preserve_interior_detail_and_bound_rectangles() {
    let mut sheet = RgbaImage::from_pixel(7, 7, Rgba([132, 132, 132, 255]));
    for y in 1..6 {
        for x in 1..6 {
            sheet.put_pixel(x, y, Rgba([255, 0, 0, 255]));
        }
    }
    sheet.put_pixel(3, 3, Rgba([132, 132, 132, 255]));
    sheet.put_pixel(2, 2, Rgba([132, 82, 132, 255]));
    let r = Rect {
        x: 0,
        y: 0,
        w: 7,
        h: 7,
    };
    let crop = crop(
        &sheet,
        r,
        Mask {
            matte: true,
            unit: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(crop.get_pixel(0, 0)[3], 0);
    assert_eq!(crop.get_pixel(3, 3)[3], 255);
    assert_eq!(crop.get_pixel(2, 2)[3], 0);
    let outside = Rect { x: u32::MAX, ..r };
    assert!(super::crop(&sheet, outside, Mask::default()).is_err());
    let diamond = super::crop(
        &RgbaImage::from_pixel(64, 32, Rgba([0, 200, 0, 255])),
        Rect {
            x: 0,
            y: 0,
            w: 64,
            h: 32,
        },
        Mask {
            diamond: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(diamond.pixels().filter(|p| p[3] != 0).count(), 1024);
}

#[test]
fn installed_original_crops_match_retained_browser_measurements() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        eprintln!("SKIP: CIVTOT_HOME not set");
        return;
    };
    let resources = Resources::load(Path::new(&home), "Original").unwrap();
    let cases = [
        (
            "icons",
            Rect {
                x: 1,
                y: 224,
                w: 20,
                h: 20,
            },
            Mask {
                matte: true,
                pink: true,
                ..Default::default()
            },
            312,
        ),
        (
            "terrain1",
            Rect {
                x: 1,
                y: 67,
                w: 64,
                h: 32,
            },
            Mask {
                diamond: true,
                ..Default::default()
            },
            1024,
        ),
        (
            "terrain2",
            Rect {
                x: 1,
                y: 133,
                w: 64,
                h: 32,
            },
            Mask {
                matte: true,
                ..Default::default()
            },
            947,
        ),
        (
            "units",
            Rect {
                x: 131,
                y: 1,
                w: 64,
                h: 64,
            },
            Mask {
                matte: true,
                unit: true,
                ..Default::default()
            },
            787,
        ),
        (
            "cities",
            Rect {
                x: 1,
                y: 88,
                w: 64,
                h: 48,
            },
            Mask {
                matte: true,
                ..Default::default()
            },
            513,
        ),
    ];
    for (key, r, mask, expected) in cases {
        let image = resources.crop(key, r, mask).unwrap();
        assert_eq!(
            image.pixels().filter(|p| p[3] != 0).count(),
            expected,
            "{key}"
        );
        assert_eq!(image.get_pixel(0, 0)[3], 0);
        assert_eq!(
            image
                .pixels()
                .filter(|p| p[3] != 0 && p.0[..3] == [255, 0, 255])
                .count(),
            0
        );
    }
    assert_eq!(resources.sources.len(), 9);
    assert_eq!(resources.sheets["terrain1"].dimensions(), (640, 480));
    assert_eq!(resources.rules.unit_names().len(), 81);
}
