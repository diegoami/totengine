//! Original post-processing city memory, independent of economic phase ticks.
use engine::saved_knowledge::{city_phase_memory_after, SavedMapKnowledge};
use tot_kit::{CityRecord, SaveFile};

fn snapshot(owner: u8, size: u8, known: u8, old: [u8; 8]) -> SaveFile {
    let mut save = engine::tests_support::synthetic_save();
    save.version = 0x32;
    save.multiplayer = 0;
    save.map_header.raw = [12, 10, 60, 1, 0, 0, 0, 0];
    save.maps.truncate(1);
    save.maps[0].tile_fields = [1, 0x10, 0, 1, 0, 240].repeat(60);
    save.maps[0].visibility = vec![0x10; 7 * 60];
    save.units.clear();
    save.cities.clear();
    for i in 0..3 {
        let mut raw = std::array::from_fn(|j| (17 * j + 29 * i) as u8);
        raw[..2].copy_from_slice(&(2 * i as u16).to_le_bytes());
        raw[2..4].copy_from_slice(&2u16.to_le_bytes());
        raw[4..6].copy_from_slice(&0u16.to_le_bytes());
        raw[10] = owner;
        raw[11] = size;
        raw[14] = known;
        raw[15..23].copy_from_slice(&old);
        raw[86..90].copy_from_slice(&(i as u32 + 1).to_le_bytes());
        save.cities.push(CityRecord { raw });
        save.maps[0].tile_fields[6 * (2 * 6 + i) + 1] = 0x12;
        save.maps[0].tile_fields[6 * (2 * 6 + i) + 5] = owner << 4;
    }
    save
}

#[test]
fn original_phase_memory_vectors_match_both_state_adapters() {
    let numbers = |s: &str| {
        s.split_whitespace()
            .map(|n| n.parse::<i32>().unwrap())
            .collect::<Vec<_>>()
    };
    let mut cases = 0;
    for line in include_str!("../../../docs/evidence/import-city-memory-vectors.txt")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let (input, expected) = line.split_once('|').unwrap();
        let v = numbers(input);
        let expected = numbers(expected);
        let (owner, allies, apollo, size, slot, known) = (
            v[0] as u8,
            v[1] as u8,
            v[2] as i16,
            v[3] as u8,
            v[4] as usize,
            v[5] as u8,
        );
        let old = std::array::from_fn(|j| v[6 + j] as u8);
        let mut save = snapshot(owner, size, known, old);
        for a in 0..8 {
            for b in 0..8 {
                let mut value = 0x104
                    | if a == owner as usize && allies & (1 << b) != 0 {
                        8
                    } else {
                        0
                    };
                if b == owner as usize && a != owner as usize {
                    value = 0x104 | if allies & (1 << a) == 0 { 8 } else { 0 };
                }
                save.diplomacy[a][b] = value;
            }
        }
        save.wonders[25] = apollo;
        let out = city_phase_memory_after(&save, slot).unwrap();
        assert_eq!(
            out.cities[slot].raw[14..23]
                .iter()
                .map(|&n| i32::from(n))
                .collect::<Vec<_>>(),
            expected,
            "{line}"
        );
        let mut restored = out.clone();
        restored.cities[slot].raw[15..23].copy_from_slice(&old);
        assert_eq!(
            restored, save,
            "only selected city's remembered sizes change: {line}"
        );
        assert_eq!(city_phase_memory_after(&out, slot).unwrap(), out);
        let mut world = engine::boot(&save, 1234);
        let original_hash = world.checksum();
        world.refresh_city_phase_memory(slot).unwrap();
        assert_eq!(world.cities[slot].known_to, known);
        assert_eq!(
            world.cities[slot].known_sizes.as_slice(),
            &out.cities[slot].raw[15..23]
        );
        world.cities[slot].known_sizes = old;
        assert_eq!(
            world.checksum(),
            original_hash,
            "world adapter touched other state: {line}"
        );
        cases += 1;
    }
    assert_eq!(cases, 1248);
}

#[test]
fn size_refresh_does_not_discover_cities_or_overwrite_unrelated_memory() {
    let mut save = snapshot(2, 9, 0, [3; 8]);
    save.wonders[25] = -1;
    save.diplomacy = [[0; 8]; 8];
    save.diplomacy[2][4] = 8;
    save.maps[0].tile_fields[6 * 12 + 4] = 1 << 4;
    let out = city_phase_memory_after(&save, 0).unwrap();
    assert_eq!(out.cities[0].known_to(), 0);
    assert_eq!(out.cities[0].known_size(4), Some(9));
    assert_eq!(out.cities[0].known_size(0), Some(3));
    assert_eq!(
        out.cities[0].known_size(2),
        Some(3),
        "own slot is not implicitly refreshed"
    );
    assert!(
        SavedMapKnowledge::read(&out, 4).unwrap().cities.is_empty(),
        "a size update alone does not discover the city"
    );
    save.wonders[25] = -2;
    let out = city_phase_memory_after(&save, 0).unwrap();
    assert_eq!(out.cities[0].raw[15..23], [3, 9, 9, 9, 9, 9, 9, 9]);
    assert_eq!(out.cities[0].known_to(), 0);
}

#[test]
fn unsupported_city_memory_contexts_reject_without_partial_world_changes() {
    let save = snapshot(2, 7, 0, [3; 8]);
    for kind in 0..7 {
        let mut bad = save.clone();
        match kind {
            0 => bad.cities[0].raw[86..90].fill(0),
            1 => bad.cities[0].raw[4] = 1,
            2 => bad.cities[0].raw[10] = 0x12,
            3 => bad.cities[0].raw[11] = 0,
            4 => bad.cities[0].raw[11] = 128,
            5 => bad.multiplayer = 1,
            6 => bad.version = 0x33,
            _ => unreachable!(),
        }
        let before = bad.clone();
        assert!(city_phase_memory_after(&bad, 0).is_err());
        assert_eq!(bad, before);
        if kind < 5 {
            let mut world = engine::boot(&bad, 1234);
            let old = world.checksum();
            assert!(world.refresh_city_phase_memory(0).is_err());
            assert_eq!(world.checksum(), old);
        }
    }
    assert!(city_phase_memory_after(&save, 999).is_err());
    let mut world = engine::boot(&save, 1234);
    let old = world.checksum();
    assert!(world.refresh_city_phase_memory(999).is_err());
    assert_eq!(world.checksum(), old);
}
