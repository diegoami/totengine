//! Native numeric reader/exploration vectors, plus asset-free snapshot checks.
use engine::saved_knowledge::*;

fn map() -> tot_kit::SaveFile {
    sized_map(16, 12)
}

fn sized_map(width: i16, height: i16) -> tot_kit::SaveFile {
    let mut save = engine::tests_support::synthetic_save();
    save.version = 0x32;
    save.game_flags = 0;
    let n = width * height / 2;
    save.map_header.raw = [width, height, n, 1, 0, 0, 0, 0];
    save.maps.truncate(1);
    save.maps[0].tile_fields = (0..n)
        .flat_map(|i| {
            [
                if i % 3 == 0 { 10 } else { 1 },
                (0x10 | ((i % 4) * 0x20)) as u8,
                0,
                1,
                1,
                240,
            ]
        })
        .collect();
    save.maps[0].visibility = vec![0x80; 7 * n as usize];
    save.units.clear();
    save.cities.clear();
    save
}

#[test]
fn original_reader_and_exploration_vectors_match() {
    let mut counts = [0; 5];
    for line in include_str!("../../../docs/evidence/import-knowledge-vectors.txt")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let (kind, values) = line.split_once(' ').unwrap();
        let v: Vec<usize> = values
            .split_whitespace()
            .map(|n| n.parse().unwrap())
            .collect();
        match kind {
            "G" => {
                assert_eq!(
                    city_shown(v[0] as u8, v[1] as u8, v[2] != 0, v[3] as u8, v[4] as u8),
                    v[5] != 0,
                    "{line}"
                );
                assert_eq!(
                    unit_shown(v[0] as u8, v[1] as u8, v[2] != 0, v[3] as u8),
                    v[6] != 0,
                    "{line}"
                );
                counts[0] += 1;
            }
            "S" => {
                assert_eq!(
                    displayed_city_size(v[0] as u8, v[1] as u8, v[2] as u8, v[3] as u8),
                    v[4] as u8,
                    "{line}"
                );
                counts[1] += 1;
            }
            "R" => {
                let mut save = map();
                let tile = 6 * 8 + 3;
                save.maps[0].tile_fields[6 * tile + 1] = v[1] as u8;
                save.maps[0].tile_fields[6 * tile + 4] = 1 << v[0];
                save.maps[0].visibility[(v[0] - 1) * 96 + tile] = v[2] as u8;
                let view = SavedMapKnowledge::read(&save, v[0] as u8).unwrap();
                assert_eq!(view.tiles[tile].unwrap().improvements, v[3] as u8, "{line}");
                assert!(view.units.is_empty() && view.cities.is_empty());
                counts[2] += 1;
            }
            "E" => {
                let mut save = map();
                if v[5] != 0 {
                    save.game_flags = 0x8000;
                }
                let tiles = explored_by_unit(
                    &save,
                    Scout {
                        x: v[3] as u16,
                        y: v[4] as u16,
                        domain: v[1] as u8,
                        extended: v[2] != 0,
                    },
                )
                .unwrap();
                assert_eq!(tiles.len(), v[6], "{line}");
                assert_eq!(tiles, v[7..], "{line}");
                counts[3] += 1;
            }
            "T" => {
                let mut save = sized_map(v[6] as i16, v[7] as i16);
                if v[5] != 0 {
                    save.game_flags = 0x8000;
                }
                let tiles = explored_by_unit(
                    &save,
                    Scout {
                        x: v[3] as u16,
                        y: v[4] as u16,
                        domain: v[1] as u8,
                        extended: v[2] != 0,
                    },
                )
                .unwrap();
                assert_eq!(tiles.len(), v[8], "{line}");
                assert_eq!(tiles, v[9..], "{line}");
                counts[4] += 1;
            }
            _ => panic!("unexpected vector: {line}"),
        }
    }
    assert_eq!(counts, [378, 189, 140, 144, 96]);
}

#[test]
fn saved_projection_hides_current_changes_and_uses_remembered_sizes() {
    let mut save = map();
    let mut unit = tot_kit::UnitRecord { raw: [0; 40] };
    unit.raw[..2].copy_from_slice(&4u16.to_le_bytes());
    unit.raw[2..4].copy_from_slice(&4u16.to_le_bytes());
    unit.raw[9] = 2;
    unit.raw[11] = 2;
    unit.raw[30..34].copy_from_slice(&1u32.to_le_bytes());
    save.units.push(unit);
    let tile = 4 * 8 + 2;
    save.maps[0].tile_fields[6 * tile + 1] = 0x31; // current road+rail+unit
    save.maps[0].tile_fields[6 * tile + 4] = 2;
    save.maps[0].visibility[tile] = 0x10; // viewer remembers just a road
    let mut city = tot_kit::CityRecord { raw: [0; 92] };
    city.raw[..2].copy_from_slice(&8u16.to_le_bytes());
    city.raw[2..4].copy_from_slice(&4u16.to_le_bytes());
    city.raw[10] = 2;
    city.raw[11] = 40;
    city.raw[14] = 2;
    city.raw[16] = 3;
    city.raw[86..90].copy_from_slice(&1u32.to_le_bytes());
    save.cities.push(city);
    let city_tile = 4 * 8 + 4;
    save.maps[0].tile_fields[6 * city_tile + 1] = 2;
    save.maps[0].tile_fields[6 * city_tile + 4] = 2;
    let a = SavedMapKnowledge::read(&save, 1).unwrap();
    assert_eq!(a.tiles[tile].unwrap().improvements, 0x10);
    assert_eq!(a.tiles.iter().flatten().count(), 2);
    assert_eq!(a.units.len(), 1);
    assert_eq!(a.cities[0].size, 3);
    save.maps[0].tile_fields[6 * tile + 1] = 0x11;
    save.cities[0].raw[11] = 7;
    assert_eq!(
        a,
        SavedMapKnowledge::read(&save, 1).unwrap(),
        "unobserved live improvements/city size cannot overwrite saved memory"
    );
    save.units[0].raw[11] = 0;
    save.cities[0].raw[14] = 0;
    let b = SavedMapKnowledge::read(&save, 1).unwrap();
    assert!(
        b.units.is_empty() && b.cities.is_empty(),
        "exploration alone cannot expose foreign entities"
    );
    assert!(
        a.tiles
            .iter()
            .enumerate()
            .filter(|(_, v)| v.is_none())
            .count()
            > 80
    );
    save.cities[0].raw[10] = 1;
    save.cities[0].raw[11] = 255;
    assert!(SavedMapKnowledge::read(&save, 1)
        .err()
        .unwrap()
        .contains("signed display range"));
    save.units[0].raw[9] = 0x12;
    assert!(SavedMapKnowledge::read(&save, 1)
        .err()
        .unwrap()
        .contains("flagged owner"));
}

#[test]
fn unsupported_saved_map_contexts_are_rejected() {
    let save = map();
    for civ in [0, 8, 255] {
        assert!(SavedMapKnowledge::read(&save, civ).is_err());
    }
    let mut bad = save.clone();
    bad.version = 0x33;
    assert!(SavedMapKnowledge::read(&bad, 1).is_err());
    bad = save.clone();
    bad.maps.push(bad.maps[0].clone());
    assert!(SavedMapKnowledge::read(&bad, 1).is_err());
    bad = save.clone();
    bad.maps[0].visibility.pop();
    assert!(SavedMapKnowledge::read(&bad, 1).is_err());
    bad = save.clone();
    bad.map_header.raw[0] = 17;
    assert!(
        SavedMapKnowledge::read(&bad, 1).is_err(),
        "odd logical width is outside the bounded context"
    );
    assert!(explored_by_unit(
        &save,
        Scout {
            x: 0,
            y: 0,
            domain: 4,
            extended: true
        }
    )
    .is_err());
}

#[test]
fn installed_original_snapshot_matches_the_native_reader_projection() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        return;
    };
    let path = std::path::Path::new(&home).join("Original/Ph_Auto2.SAV");
    let save = tot_kit::parse_sav_file(&path).unwrap();
    let view = SavedMapKnowledge::read(&save, save.player_civ).unwrap();
    assert_eq!(view.tiles.iter().flatten().count(), 59);
    assert_eq!(
        view.units.iter().map(|u| u.slot).collect::<Vec<_>>(),
        [6, 9, 18, 26]
    );
    assert_eq!(
        view.cities
            .iter()
            .map(|c| (c.slot, c.size))
            .collect::<Vec<_>>(),
        [(5, 2)]
    );
    assert!(
        view.tiles[12 * view.width + 27].is_none(),
        "Madrid's unexplored working-radius tile remains unknown"
    );
    let world = engine::boot(&save, 1);
    assert_eq!(
        view.engine_sight_additions(&world).unwrap(),
        [12 * view.width + 27]
    );
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(std::path::Path::new(&home).join("Original/Rules.txt")).unwrap(),
    ));
    let preflight =
        engine::scenario::preflight(&std::fs::read(path).unwrap(), &rules, None, false, false)
            .unwrap();
    assert!(
        preflight
            .gaps
            .iter()
            .any(|g| g.contains("starts from saved map knowledge")),
        "{:?}",
        preflight.gaps
    );
    assert!(preflight
        .gaps
        .iter()
        .any(|g| g.contains("later play uses engine visibility/contact rules")));
}

#[test]
fn installed_original_observation_matches_native_sparse_updates() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        return;
    };
    let folder = std::path::Path::new(&home).join("Original");
    let original = tot_kit::parse_sav_file(&folder.join("Ph_Auto2.SAV")).unwrap();
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(folder.join("Rules.txt")).unwrap(),
    ));
    let vectors = include_str!("../../../docs/evidence/import-pinned-observation.txt");
    let delta = |before: &[u8], after: &[u8]| {
        assert_eq!(before.len(), after.len());
        before
            .iter()
            .zip(after)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, (a, b))| format!("{i}:{a}:{b}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    let mut count = 0;
    for line in vectors.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<_> = line.split_whitespace().collect();
        let [slot, x, y, tiles, memory] = fields.as_slice() else {
            panic!("invalid native vector: {line}");
        };
        let slot = slot.parse::<usize>().unwrap();
        let mut prepared = original.clone();
        if *x != "-" {
            let (x, y) = (x.parse::<u16>().unwrap(), y.parse::<u16>().unwrap());
            let unit = &mut prepared.units[slot];
            let index = |x, y| {
                original
                    .map_header
                    .stored_index(usize::from(x), usize::from(y))
                    .unwrap()
            };
            let old = 6 * index(unit.x(), unit.y());
            let new = 6 * index(x, y);
            unit.raw[..2].copy_from_slice(&x.to_le_bytes());
            unit.raw[2..4].copy_from_slice(&y.to_le_bytes());
            let map = &mut prepared.maps[0].tile_fields;
            map[old + 1] &= !1;
            map[old + 5] |= 0xf0;
            map[new + 1] |= 1;
            map[new + 5] = map[new + 5] & 0x0f | (original.player_civ << 4);
        }
        let output =
            engine::saved_knowledge::unit_observation_after(&prepared, &rules, slot, false)
                .unwrap();
        let before = &prepared.maps[0];
        let after = &output.maps[0];
        assert_eq!(
            delta(&before.tile_fields, &after.tile_fields),
            tiles.trim_matches('-'),
            "{line}"
        );
        assert_eq!(
            delta(&before.visibility, &after.visibility),
            memory.trim_matches('-'),
            "{line}"
        );
        assert_eq!(prepared.units, output.units);
        assert_eq!(prepared.cities, output.cities);
        assert_eq!(prepared.diplomacy, output.diplomacy);
        count += 1;
    }
    assert_eq!(count, 8);
}
