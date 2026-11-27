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
            // The top band (#439) uses the same stone, lightened: the
            // original's frame is a pale stone panel over the map, with a
            // darker inset for the info panel.
            (
                "band",
                "city",
                Rect {
                    x: 430,
                    y: 20,
                    w: 110,
                    h: 110,
                },
            ),
            (
                "panel-mid",
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
        // The source stone is a high-contrast speckle meant for small
        // surfaces. Flatten it toward a quiet mid-grey so band text reads and
        // the frame matches the original's smooth stone: keep a third of the
        // variation around a chosen mid tone.
        for (key, mid, contrast) in [("band", 138i32, 32i32), ("panel-mid", 76i32, 40)] {
            let sprite = sprites.get_mut(key).unwrap();
            let mut sum = 0u64;
            let mut n = 0u64;
            for pixel in sprite.rgba.as_chunks_mut::<4>().0 {
                for channel in &pixel[..3] {
                    sum += u64::from(*channel);
                    n += 1;
                }
            }
            let mean = sum
                .checked_div(n)
                .map_or(128, |v| v.min(i32::MAX as u64) as i32);
            for pixel in sprite.rgba.as_chunks_mut::<4>().0 {
                for channel in &mut pixel[..3] {
                    let v = mid + (i32::from(*channel) - mean) * contrast / 100;
                    *channel = v.clamp(0, 255) as u8;
                }
            }
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
