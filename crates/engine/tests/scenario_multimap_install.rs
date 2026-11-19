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
    let live = |u: &&tot_kit::UnitRecord| u.id() != 0;
    let on_zero = sav
        .units
        .iter()
        .filter(live)
        .filter(|u| u.map_id() == 0)
        .count();
    let others = sav
        .units
        .iter()
        .filter(live)
        .filter(|u| u.map_id() != 0)
        .count();
    assert_eq!(alive, on_zero);
    assert_eq!(world.units.len(), sav.units.len(), "indices kept");
    for (c, r) in world.cities.iter().zip(&sav.cities) {
        assert_eq!(c.city_id != 0, r.map_id() == 0, "{}", r.name());
    }
    let rules = tot_kit::parse_rules(&String::from_utf8_lossy(
        &install("Midgard/Rules.txt").unwrap(),
    ));
    let p = engine::scenario::preflight(&bytes, &rules, None, false, false).unwrap();
    assert!(
        p.gaps[0].starts_with(&format!(
            "Only the first of 4 maps is played: {others} units and 4 cities"
        )),
        "{:?}",
        p.gaps
    );
}

/// ToTPP `0x33` (#331): maps are `14 × surface + 2` (one extra byte per
/// tile after the classic tile records); units, cities and the scenario
/// parameter block are where `0x32` has them.
#[test]
fn warring_states_frames_two_totpp_maps_with_classic_records() {
    for rel in [
        "Scenario/china/Warring States.scn",
        "Scenario/china/Hu_Auto.SAV",
        "Scenario/china/deiYan.sav",
        "Scenario/china/deiQin.sav",
    ] {
        let Some(bytes) = install(rel) else {
            eprintln!("skipping {rel}: not installed");
            continue;
        };
        let sav = tot_kit::parse_sav(&bytes).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert_eq!(sav.version, 0x33, "{rel}");
        assert_eq!(sav.maps.len(), 2, "{rel}");
        let surface = sav.map_header.surface();
        assert_eq!(
            sav.unit_region.0,
            29916 + 2 * (14 * surface + 2) + 7056 + 10240
        );
        let (mut placed, mut live) = (0, 0);
        for u in sav.units.iter().filter(|u| u.id() != 0) {
            live += 1;
            let m = usize::from(u.map_id());
            if tile_flags(&sav, m, u.x(), u.y()).is_some_and(|f| f & 0x01 != 0) {
                placed += 1;
            }
        }
        // Live units on a unit-flagged tile of their own map: 1441/1441,
        // 1425/1425, 1403/1440 (deiYan) and 1633/1635; why deiYan's 37
        // tiles lack the flag is not decoded.
        assert!(100 * placed >= 97 * live, "{rel}: {placed}/{live}");
        let flagged = sav
            .cities
            .iter()
            .filter(|c| {
                tile_flags(&sav, usize::from(c.map_id()), c.x(), c.y()).unwrap() & 0x02 != 0
            })
            .count();
        // The pristine scenario has four cities whose tile lacks the flag.
        assert!(flagged + 4 >= sav.cities.len(), "{rel}: {flagged}");
        assert!(sav.cities.iter().all(|c| c.map_id() == 1), "{rel}");
        if sav.research_settings.flags & 0x80 != 0 {
            let cal = sav.calendar.unwrap();
            assert_eq!((cal.increment, cal.start_year), (-3, -1404), "{rel}");
            // The saved year advances by the increment's size each turn.
            assert_eq!(sav.year, -1402 + 3 * (sav.turn - 1), "{rel}");
        }
    }
}

#[test]
fn warring_states_opens_on_its_first_map() {
    let (Some(bytes), Some(rules)) = (
        install("Scenario/china/Warring States.scn"),
        install("Scenario/china/Rules.txt"),
    ) else {
        eprintln!("skipping: Warring States not installed");
        return;
    };
    let rules = tot_kit::parse_rules(&String::from_utf8_lossy(&rules));
    let p = engine::scenario::preflight(&bytes, &rules, None, false, true).unwrap();
    assert!(
        p.gaps[0].starts_with("Only the first of 2 maps is played: 120 units and 120 cities"),
        "{:?}",
        p.gaps
    );
    assert!(p.civs.len() >= 7, "{:?}", p.civs);
}

/// The located 0x33 calendar reproduces the years the game itself saved:
/// Hu_Auto (turn 1) advanced to turn 2 and 11 gives deiYan's and deiQin's.
#[test]
fn warring_states_calendar_matches_the_saved_years() {
    let Some(bytes) = install("Scenario/china/Hu_Auto.SAV") else {
        eprintln!("skipping: Warring States not installed");
        return;
    };
    let mut world = engine::boot(&tot_kit::parse_sav(&bytes).unwrap(), 0);
    let mut years = Vec::new();
    for _ in 0..10 {
        years.push(world.advance_turn().year);
    }
    assert_eq!((years[0], years[9]), (-1399, -1372), "{years:?}");
}
