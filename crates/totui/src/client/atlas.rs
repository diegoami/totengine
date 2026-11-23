use super::{Client, Draw};
use crate::resources::{self, Mask, Rect, Resources};
use std::collections::BTreeMap;
use std::path::Path;

// Interior cells of the magenta grid in the measured classic GIF sheet.
pub(super) const CLASSIC_CELLS: [(&str, u32, u32); 5] = [
    ("classic-food", 1, 305),
    ("classic-shields", 16, 305),
    ("classic-science", 31, 320),
    ("classic-tax", 16, 320),
    ("classic-luxury", 1, 320),
];
const CLASSIC_SIZE: u32 = 14;
const CLASSIC_BACKDROP: [u8; 3] = [255, 159, 163];

const CLASSIC_UNIT_COUNT: u8 = 54;

pub struct Sprite {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}
pub struct Atlas {
    pub sprites: BTreeMap<String, Sprite>,
}
impl Atlas {
    pub(super) fn load_for_install(resources: &Resources, home: &Path) -> Result<Self, String> {
        let mut atlas = Self::load(resources)?;
        if let Some(sheet) = resources::classic_icon_sheet(home) {
            let icons: Option<BTreeMap<_, _>> = CLASSIC_CELLS
                .into_iter()
                .map(|(name, x, y)| {
                    let rect = Rect {
                        x,
                        y,
                        w: CLASSIC_SIZE,
                        h: CLASSIC_SIZE,
                    };
                    let mut image = resources::crop(&sheet, rect, Mask::default()).ok()?;
                    for pixel in image.pixels_mut() {
                        if pixel.0[..3] == CLASSIC_BACKDROP {
                            pixel[3] = 0;
                        }
                    }
                    let visible = image.pixels().filter(|pixel| pixel[3] != 0).count();
                    (1..(CLASSIC_SIZE * CLASSIC_SIZE) as usize)
                        .contains(&visible)
                        .then(|| {
                            (
                                name.to_string(),
                                Sprite {
                                    w: rect.w,
                                    h: rect.h,
                                    rgba: image.into_raw(),
                                },
                            )
                        })
                })
                .collect();
            if let Some(icons) = icons {
                atlas.sprites.extend(icons);
            }
        }
        if let Some(units) =
            resources::classic_unit_sheet(resources, home).and_then(|sheet| classic_units(&sheet))
        {
            atlas.sprites.extend(units);
        }
        Ok(atlas)
    }

    pub fn load(resources: &Resources) -> Result<Self, String> {
        if resources.isotrident.is_some() {
            return super::isotrident::atlas(resources);
        }
        let mut sprites = BTreeMap::new();
        let mut add = |name: String, sheet: &str, rect: Rect, mask: Mask| -> Result<(), String> {
            let image = resources
                .crop(sheet, rect, mask)
                .map_err(|e| format!("unsupported Original atlas: {name}: {e}"))?;
            sprites.insert(
                name,
                Sprite {
                    w: image.width(),
                    h: image.height(),
                    rgba: image.into_raw(),
                },
            );
            Ok(())
        };
        for id in 0..11 {
            for v in 0..3 {
                add(
                    format!("terrain-{id}-{v}"),
                    "terrain1",
                    Rect {
                        x: 1 + v * 65,
                        y: 1 + id * 33,
                        w: 64,
                        h: 32,
                    },
                    Mask {
                        diamond: true,
                        ..Default::default()
                    },
                )?;
            }
        }
        for (name, row) in [("forest", 4), ("hills", 8), ("mountains", 6)] {
            for v in 0..8 {
                add(
                    format!("{name}-{v}"),
                    "terrain2",
                    Rect {
                        x: 1 + v * 65,
                        y: 1 + row * 33,
                        w: 64,
                        h: 32,
                    },
                    Mask {
                        matte: true,
                        ..Default::default()
                    },
                )?;
            }
        }
        for id in 0..resources.rules.unit_names().len().min(80) as u32 {
            add(
                format!("unit-{id}"),
                "units",
                Rect {
                    x: 1 + (id % 9) * 65,
                    y: 1 + (id / 9) * 65,
                    w: 64,
                    h: 64,
                },
                Mask {
                    matte: true,
                    unit: true,
                    ..Default::default()
                },
            )?;
        }
        // Measured first-row People.bmp crops, following the reviewed art preview.
        for (role, column) in [(0, 2), (1, 4), (2, 9), (3, 10)] {
            add(
                format!("citizen-{role}"),
                "people",
                Rect {
                    x: 1 + column * 28,
                    y: 1,
                    w: 27,
                    h: 32,
                },
                Mask {
                    matte: true,
                    ..Default::default()
                },
            )?;
        }
        for size in 0..2 {
            add(
                format!("city-{size}"),
                "cities",
                Rect {
                    x: 1 + size * 65,
                    y: 88,
                    w: 64,
                    h: 48,
                },
                Mask {
                    matte: true,
                    ..Default::default()
                },
            )?;
        }
        for (name, sheet, rect) in [
            (
                "chrome",
                "dialog",
                Rect {
                    x: 1,
                    y: 1,
                    w: 92,
                    h: 92,
                },
            ),
            (
                "panel",
                "city",
                Rect {
                    x: 430,
                    y: 20,
                    w: 110,
                    h: 110,
                },
            ),
        ] {
            add(name.into(), sheet, rect, Mask::default())?;
        }
        for key in ["chrome", "panel"] {
            for pixel in sprites.get_mut(key).unwrap().rgba.as_chunks_mut::<4>().0 {
                for channel in &mut pixel[..3] {
                    *channel = (u16::from(*channel) * 45 / 100) as u8;
                }
            }
        }
        Ok(Self { sprites })
    }
}

fn classic_units(sheet: &image::RgbaImage) -> Option<BTreeMap<String, Sprite>> {
    // Green grid intersections surround the six 9-column rows of Original units.
    if !(0..=6)
        .all(|row| (0..=9).all(|col| sheet.get_pixel(col * 65, row * 49).0 == [0, 255, 0, 255]))
    {
        return None;
    }
    (0..CLASSIC_UNIT_COUNT)
        .map(|id| {
            let id = u32::from(id);
            let rect = Rect {
                x: 1 + (id % 9) * 65,
                y: 1 + (id / 9) * 49,
                w: 64,
                h: 48,
            };
            let mut unit = resources::crop(sheet, rect, Mask::default()).ok()?;
            for pixel in unit.pixels_mut() {
                if pixel.0[..3] == [135, 83, 135] {
                    pixel[3] = 0;
                }
            }
            let visible = unit.pixels().filter(|pixel| pixel[3] > 0).count();
            if !(1..(rect.w * rect.h) as usize).contains(&visible) {
                return None;
            }
            // Align the bottom edges so the 48-pixel art has the same map
            // anchor, draw rectangle and hit-testing coordinates as ToT art.
            let mut padded = image::RgbaImage::new(64, 64);
            image::imageops::replace(&mut padded, &unit, 0, 16);
            Some((
                format!("classic-unit-{id}"),
                Sprite {
                    w: padded.width(),
                    h: padded.height(),
                    rgba: padded.into_raw(),
                },
            ))
        })
        .collect()
}

impl Client {
    pub(super) fn classic_units_available(&self) -> bool {
        self.atlas()
            .is_some_and(|atlas| atlas.sprites.contains_key("classic-unit-0"))
    }

    pub(super) fn unit_sprite(&self, type_id: u8) -> String {
        if self.enhancements.classic_units
            && type_id < CLASSIC_UNIT_COUNT
            && self.classic_units_available()
        {
            format!("classic-unit-{type_id}")
        } else {
            format!("unit-{type_id}")
        }
    }

    pub(super) fn classic_icons_active(&self) -> bool {
        self.enhancements.classic_icons
            && self
                .atlas()
                .is_some_and(|atlas| atlas.sprites.contains_key("classic-food"))
    }

    pub(super) fn classic_icon(&self, sprite: &str, x: f64, y: f64) -> Option<Draw> {
        self.classic_icons_active().then(|| Draw::Sprite {
            sprite: sprite.into(),
            x,
            y,
            w: f64::from(CLASSIC_SIZE),
            h: f64::from(CLASSIC_SIZE),
        })
    }
}
