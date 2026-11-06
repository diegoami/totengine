use crate::resources::{Mask, Rect, Resources};
use std::collections::BTreeMap;

pub struct Sprite {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}
pub struct Atlas {
    pub sprites: BTreeMap<String, Sprite>,
}
impl Atlas {
    pub fn load(resources: &Resources) -> Result<Self, String> {
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
