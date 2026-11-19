//! Multi-map framing (#331), re-derived from the installed Midgard files:
//! the map count is header `raw[7] + 1` (the original's writer `0x56fa9d`
//! and loader `0x56fcb1` loop maps `0..=raw[7]`), each unit's map is byte
//! +4 and each city's map word +4. Skips without `CIVTOT_HOME`.
use std::path::PathBuf;

fn install(rel: &str) -> Option<Vec<u8>> {
    let home = std::env::var("CIVTOT_HOME").ok()?;
    std::fs::read(PathBuf::from(home).join(rel)).ok()
}

/// Tile byte +1 of `(x, y)` on map `m`: `0x01` unit, `0x02` city.
fn tile_flags(sav: &tot_kit::SaveFile, m: usize, x: u16, y: u16) -> Option<u8> {
    let t = sav.map_header.stored_index(x.into(), y.into())?;
    sav.maps.get(m)?.tile_fields.get(6 * t + 1).copied()
}

#[test]
fn midgard_frames_four_maps_and_places_every_record_on_its_own_map() {
    for rel in [
        "Midgard/Midgard.SCN",
        "Midgard/Oc_Auto.SAV",
        "Midgard/Oc_Auto2.SAV",
    ] {
        let Some(bytes) = install(rel) else {
            eprintln!("skipping {rel}: not installed");
            continue;
        };
        let sav = tot_kit::parse_sav(&bytes).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert_eq!(sav.map_header.raw[7], 3, "{rel}");
        assert_eq!(sav.maps.len(), 4, "{rel}");
        let mut per_map = [0usize; 4];
        for u in &sav.units {
            let m = usize::from(u.map_id());
            if u.x() >= sav.map_header.raw[0] as u16 {
                continue; // off-board (negative coordinates)
            }
            per_map[m] += 1;
            assert!(
                tile_flags(&sav, m, u.x(), u.y()).unwrap() & 0x01 != 0,
                "{rel}: unit at ({}, {}) map {m}",
                u.x(),
                u.y()
            );
        }
        assert!(per_map.iter().all(|&n| n > 0), "{rel}: {per_map:?}");
        for c in &sav.cities {
            let m = usize::from(c.map_id());
            assert!(m < 4, "{rel}: {}", c.name());
            // Oc_Auto2's Edgetown (map 0) is the one city whose tile has
            // no city flag in that save; why is not decoded.
            if c.name() != "Edgetown" {
                assert!(
                    tile_flags(&sav, m, c.x(), c.y()).unwrap() & 0x02 != 0,
                    "{rel}: {}",
                    c.name()
                );
            }
        }
    }
}

#[test]
fn midgard_boots_map_zero_and_the_preflight_says_so() {
    let Some(bytes) = install("Midgard/Midgard.SCN") else {
        eprintln!("skipping: Midgard not installed");
        return;
    };
    let sav = tot_kit::parse_sav(&bytes).unwrap();
    let world = engine::boot(&sav, 0);
    let alive = world.units.iter().filter(|u| u.id != 0).count();
    let on_zero = sav.units.iter().filter(|u| u.map_id() == 0).count();
    assert_eq!(alive, on_zero);
    assert_eq!(world.units.len(), sav.units.len(), "indices kept");
    for (c, r) in world.cities.iter().zip(&sav.cities) {
        assert_eq!(c.city_id != 0, r.map_id() == 0, "{}", r.name());
    }
    let rules = tot_kit::parse_rules(&String::from_utf8_lossy(
        &install("Midgard/Rules.txt").unwrap(),
    ));
    let p = engine::scenario::preflight(&bytes, &rules, None, false, false).unwrap();
    let others = sav.units.len() - on_zero;
    assert!(
        p.gaps[0].starts_with(&format!(
            "Only the first of 4 maps is played: {others} units and 4 cities"
        )),
        "{:?}",
        p.gaps
    );
}

#[test]
fn totpp_extended_files_are_still_rejected_with_a_reason() {
    let Some(bytes) = install("Scenario/china/Warring States.scn") else {
        eprintln!("skipping: Warring States not installed");
        return;
    };
    let rules = tot_kit::Rules::default();
    let e = engine::scenario::preflight(&bytes, &rules, None, false, false).unwrap_err();
    assert!(e.contains("0x33"), "{e}");
}
