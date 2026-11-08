//! Independent adapter for the separately licensed Freeciv artwork pack.
use super::{Atlas, Sprite};
use crate::resources::Resources;
use image::{imageops, Rgba, RgbaImage};
use std::collections::BTreeMap;

fn named(r: &Resources, tag: &str) -> Result<RgbaImage, String> {
    let s = r
        .isotrident
        .as_ref()
        .and_then(|m| m.get(tag))
        .ok_or_else(|| format!("Isotrident sprite missing: {tag}"))?;
    let [x, y, w, h] = s.rect;
    Ok(imageops::crop_imm(&r.sheets[&s.sheet], x, y, w, h).to_image())
}
fn sprite(image: RgbaImage) -> Sprite {
    Sprite {
        w: image.width(),
        h: image.height(),
        rgba: image.into_raw(),
    }
}
fn normalize(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
fn unit_tag(name: &str) -> String {
    // Rules IDs are never Freeciv sprite positions. Match semantic labels, with
    // explicit aliases for the authored campaign fixture and common ToT labels.
    let name = normalize(name);
    let tag = match name.as_str() {
        "pioneers" => "settlers",
        "runners" | "scouts" => "explorer",
        "guards" => "phalanx",
        "legion" | "legions" => "legion",
        "mechinf" | "mechanizedinfantry" => "mech_inf",
        "alpinetroops" => "alpine_troops",
        "stealthfighter" => "stealth_fighter",
        "stealthbomber" => "stealth_bomber",
        "aegiscruiser" => "aegis_cruiser",
        "cruisemsl" | "cruisemissile" => "cruise_missile",
        "nuclearmsl" | "nuclearmissile" => "nuclear",
        "partisans" => "partisan",
        other => other,
    };
    format!("u.{tag}_Idle")
}

pub(super) fn atlas(r: &Resources) -> Result<Atlas, String> {
    let mut sprites = BTreeMap::new();
    for (id, tag) in [
        "desert",
        "plains",
        "grassland",
        "forest",
        "hills",
        "mountains",
        "tundra",
        "arctic",
        "swamp",
        "jungle",
    ]
    .iter()
    .enumerate()
    {
        let image = named(r, &format!("t.l0.{tag}1"))?;
        for v in 0..3 {
            sprites.insert(format!("terrain-{id}-{v}"), sprite(image.clone()));
        }
    }
    // The initial fallback uses open-water cells. Coast matching is a separate
    // presentation refinement; gameplay still uses the engine's actual map.
    let mut water = RgbaImage::new(64, 32);
    for (tag, x, y) in [("u", 0, 0), ("r", 32, 0), ("l", 0, 16), ("d", 32, 16)] {
        imageops::overlay(
            &mut water,
            &named(r, &format!("t.l0.coast_cell_{tag}000"))?,
            x,
            y,
        );
    }
    for (x, y, p) in water.enumerate_pixels_mut() {
        if ((f64::from(x) + 0.5 - 32.) / 32.).abs() + ((f64::from(y) + 0.5 - 16.) / 16.).abs() > 1.
        {
            p[3] = 0;
        }
    }
    for v in 0..3 {
        sprites.insert(format!("terrain-10-{v}"), sprite(water.clone()));
    }
    // Isotrident's base terrain already contains these features.
    for name in ["forest", "hills", "mountains"] {
        for v in 0..8 {
            sprites.insert(format!("{name}-{v}"), sprite(RgbaImage::new(64, 32)));
        }
    }
    for (id, name) in r.rules.unit_names().iter().take(80).enumerate() {
        let mut image = RgbaImage::new(64, 64);
        if let Ok(source) = named(r, &unit_tag(name)) {
            let scaled = imageops::resize(&source, 48, 48, imageops::FilterType::Nearest);
            imageops::overlay(&mut image, &scaled, 8, 12);
        } else {
            // Visible neutral marker for an unsupported custom unit, rather
            // than silently presenting another named unit's artwork.
            for y in 20..58 {
                for x in 14..50 {
                    let edge =
                        x == 14 || x == 49 || y == 20 || y == 57 || x == y - 6 || x + y == 70;
                    image.put_pixel(
                        x,
                        y,
                        Rgba(if edge {
                            [235, 211, 127, 255]
                        } else {
                            [45, 45, 45, 255]
                        }),
                    );
                }
            }
        }
        sprites.insert(format!("unit-{id}"), sprite(image));
    }
    for (id, tag) in [(0, "city.european_city_0"), (1, "city.european_city_2")] {
        sprites.insert(format!("city-{id}"), sprite(named(r, tag)?));
    }
    for (role, tag) in [
        (0, "citizen.content_0"),
        (1, "specialist.entertainer_0"),
        (2, "specialist.taxman_0"),
        (3, "specialist.scientist_0"),
    ] {
        sprites.insert(
            format!("citizen-{role}"),
            sprite(imageops::resize(
                &named(r, tag)?,
                27,
                32,
                imageops::FilterType::Nearest,
            )),
        );
    }
    for key in ["chrome", "panel"] {
        sprites.insert(
            key.into(),
            sprite(RgbaImage::from_pixel(8, 8, Rgba([39, 46, 40, 255]))),
        );
    }
    Ok(Atlas { sprites })
}
