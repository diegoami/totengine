//! Native normal-context update vectors and transactional rejection checks.
use engine::saved_knowledge::{unit_knowledge_after, SavedMapKnowledge};
use tot_kit::{parse_rules, Rules, SaveFile};

const OFFSETS: [(i32, i32); 25] = [
    (1, -1),
    (2, 0),
    (1, 1),
    (0, 2),
    (-1, 1),
    (-2, 0),
    (-1, -1),
    (0, -2),
    (2, -2),
    (2, 2),
    (-2, 2),
    (-2, -2),
    (1, -3),
    (3, -1),
    (3, 1),
    (1, 3),
    (-1, 3),
    (-3, 1),
    (-3, -1),
    (-1, -3),
    (0, 0),
    (4, 0),
    (-4, 0),
    (0, 4),
    (0, -4),
];

fn context(v: &[usize]) -> (SaveFile, Rules) {
    let mut save = engine::tests_support::synthetic_save();
    save.version = 0x32;
    save.multiplayer = 0;
    save.game_flags = if v[9] != 0 { 0x8000 } else { 0 };
    save.map_header.raw = [16, 12, 96, 1, 0, 0, 0, 0];
    save.maps.truncate(1);
    save.maps[0].tile_fields = (0..96)
        .flat_map(|i| {
            [
                if i % 3 == 0 { 10 } else { 1 },
                0x10 | ((i % 4) * 0x20),
                0,
                1,
                1,
                240,
            ]
        })
        .collect();
    save.maps[0].visibility = vec![128; 96 * 7];
    save.units.clear();
    save.cities.clear();
    save.diplomacy = [[13; 8]; 8];
    let (civ, other) = (
        v[0] as u8,
        if v[13] & 1 != 0 { v[0] } else { v[0] % 7 + 1 } as u8,
    );
    let (x, y) = (v[10] as i32, v[11] as i32);
    let (dx, dy) = OFFSETS[v[5]];
    let (mut nx, ny) = (x + dx, y + dy);
    if v[9] == 0 {
        if nx < 0 {
            nx += 16;
        } else if nx >= 16 {
            nx -= 16;
        }
    }
    for (group, (ux, uy, owner)) in [(x, y, civ), (nx, ny, other)].into_iter().enumerate() {
        let present = group == 0 || ((0..16).contains(&ux) && (0..12).contains(&uy));
        for j in 0..2 {
            let slot = 2 * group + j;
            let mut u = tot_kit::UnitRecord { raw: [0; 40] };
            if present && (group == 0 || v[13] < 2) {
                u.raw[..2].copy_from_slice(&(ux as u16).to_le_bytes());
                u.raw[2..4].copy_from_slice(&(uy as u16).to_le_bytes());
                u.raw[8] = group as u8;
                u.raw[9] = owner;
                u.raw[11] = v[12] as u8;
                u.raw[17] = 255;
                u.raw[26..28]
                    .copy_from_slice(&(if j == 0 { -1 } else { slot as i16 - 1 }).to_le_bytes());
                u.raw[28..30]
                    .copy_from_slice(&(if j == 0 { slot as i16 + 1 } else { -1 }).to_le_bytes());
                u.raw[30..34].copy_from_slice(&(slot as u32 + 1).to_le_bytes());
            }
            save.units.push(u);
        }
        let mut c = tot_kit::CityRecord { raw: [0; 92] };
        if present {
            let i = uy as usize * 8 + ux as usize / 2;
            if group == 0 || v[13] < 2 {
                save.maps[0].tile_fields[6 * i + 1] |= 1;
            }
            save.maps[0].tile_fields[6 * i + 5] = owner << 4;
            if v[6] & (1 << group) != 0 {
                save.maps[0].tile_fields[6 * i + 1] =
                    (save.maps[0].tile_fields[6 * i + 1] & !0xc0) | 2;
                c.raw[..2].copy_from_slice(&(ux as u16).to_le_bytes());
                c.raw[2..4].copy_from_slice(&(uy as u16).to_le_bytes());
                c.raw[10] = owner;
                c.raw[11] = 5 + 2 * group as u8;
                c.raw[14] = v[12] as u8;
                c.raw[15..23].copy_from_slice(&[40, 41, 42, 43, 44, 45, 46, 47]);
                c.raw[86..90].copy_from_slice(&(group as u32 + 1).to_le_bytes());
            }
        }
        save.cities.push(c);
    }
    if v[7] != 0 {
        save.maps[0].tile_fields[6 * (y as usize * 8 + x as usize / 2) + 4] |= 1 << other;
    }
    let rules = parse_rules(&format!("@UNITS\nScout,nil,{},1,0,{},1,1,1,1,0,6,nil,{:015b}\nOther,nil,{},1,0,1,1,1,1,1,0,6,nil,{:015b}\n",v[1],v[8],v[2],v[3],v[4]));
    (save, rules)
}

#[test]
fn native_normal_unit_knowledge_transitions_match() {
    let mut cases = 0;
    for line in include_str!("../../../docs/evidence/import-visibility-vectors.txt")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let (input, expected) = line.split_once('|').unwrap();
        let numbers = |s: &str| {
            s.split_whitespace()
                .map(|n| n.parse::<usize>().unwrap())
                .collect::<Vec<_>>()
        };
        let v = numbers(input);
        let expected = numbers(expected);
        let (save, rules) = context(&v);
        let out = unit_knowledge_after(&save, &rules, 0).unwrap_or_else(|e| panic!("{input}: {e}"));
        let mut result: Vec<_> = out
            .units
            .iter()
            .map(|u| usize::from(u.visibility()))
            .collect();
        result.extend(out.cities.iter().map(|c| usize::from(c.known_to())));
        result.extend(
            out.cities
                .iter()
                .map(|c| usize::from(c.known_size(v[0] as u8).unwrap())),
        );
        let explored: Vec<_> = (0..96)
            .filter(|&i| out.maps[0].visible_to(i) & (1 << v[0]) != 0)
            .collect();
        result.push(explored.len());
        result.extend(&explored);
        assert_eq!(result, expected, "{input}");
        let mut unchanged = out.clone();
        for i in 0..96 {
            let observed = explored.contains(&i);
            assert_eq!(
                out.maps[0].tile_fields[6 * i + 4],
                save.maps[0].tile_fields[6 * i + 4] | if observed { 1 << v[0] } else { 0 }
            );
            unchanged.maps[0].tile_fields[6 * i + 4] = save.maps[0].tile_fields[6 * i + 4];
            for civ in 1..8 {
                let at = (civ - 1) * 96 + i;
                assert_eq!(
                    out.maps[0].visibility[at],
                    if civ == v[0] && observed {
                        save.maps[0].improvements(i)
                    } else {
                        128
                    }
                );
            }
        }
        unchanged.maps[0]
            .visibility
            .clone_from(&save.maps[0].visibility);
        for (u, old) in unchanged.units.iter_mut().zip(&save.units) {
            u.raw[11] = old.raw[11];
        }
        for (c, old) in unchanged.cities.iter_mut().zip(&save.cities) {
            c.raw[14] = old.raw[14];
            c.raw[15 + v[0]] = old.raw[15 + v[0]];
        }
        assert_eq!(unchanged, save, "only knowledge fields may change: {input}");
        // OR writes and cache refresh are idempotent in an unchanged world.
        assert_eq!(unit_knowledge_after(&out, &rules, 1).unwrap(), out);
        cases += 1;
    }
    assert_eq!(cases, 768);
}

#[test]
fn unsupported_updates_fail_without_mutating_the_snapshot() {
    let v = [4, 0, 1, 0, 1, 1, 2, 0, 1, 0, 6, 6, 1, 0];
    let (save, rules) = context(&v);
    for kind in 0..13 {
        let mut bad = save.clone();
        match kind {
            0 => bad.version = 0x33,
            1 => bad.multiplayer = 1,
            2 => bad.units[0].raw[9] = 0x14,
            3 => bad.units[0].raw[17] = 3,
            4 => bad.units[0].raw[28..30].copy_from_slice(&0i16.to_le_bytes()),
            5 => bad.units[1].raw[26..28].copy_from_slice(&(-1i16).to_le_bytes()),
            6 => bad.units[0].raw[8] = 99,
            7 => bad.maps[0].tile_fields[6 * (6 * 8 + 3) + 1] &= !1,
            8 => bad.maps[0].tile_fields[6 * (6 * 8 + 3) + 5] = 0,
            9 => bad.diplomacy[4][5] = 0,
            10 => bad.cities[1].raw[11] = 128,
            11 => bad.units[0].raw[5] = 1,
            12 => bad.maps[0].visibility.pop().map(|_| ()).unwrap(),
            _ => unreachable!(),
        }
        let before = bad.clone();
        assert!(
            unit_knowledge_after(&bad, &rules, 0).is_err(),
            "case {kind}"
        );
        assert_eq!(bad, before);
    }
    let invisible = parse_rules("@UNITS\nScout,nil,0,1,0,1,1,1,1,1,0,6,nil,000000000001000\n");
    assert!(unit_knowledge_after(&save, &invisible, 0).is_err());
    assert!(unit_knowledge_after(&save, &rules, 999).is_err());
}

#[test]
fn observed_city_size_persists_outside_later_scout_sight() {
    let v = [4, 0, 0, 0, 0, 1, 2, 0, 1, 0, 6, 6, 1, 0];
    let (mut save, rules) = context(&v);
    let first = unit_knowledge_after(&save, &rules, 0).unwrap();
    assert_eq!(
        SavedMapKnowledge::read(&first, 4).unwrap().cities[0].size,
        7
    );
    save = first;
    // The other civ grows after we leave. Its remembered size is retained.
    save.cities[1].raw[11] = 9;
    for slot in 0..2 {
        save.units[slot].raw[..2].copy_from_slice(&0u16.to_le_bytes());
    }
    save.maps[0].tile_fields[6 * (6 * 8 + 3) + 1] &= !1;
    save.maps[0].tile_fields[6 * (6 * 8) + 1] |= 1;
    save.maps[0].tile_fields[6 * (6 * 8) + 5] = 4 << 4;
    let later = unit_knowledge_after(&save, &rules, 0).unwrap();
    assert_eq!(
        SavedMapKnowledge::read(&later, 4).unwrap().cities[0].size,
        7
    );
    assert_eq!(later.cities[1].size(), 9);
    assert_eq!(later.units[2].visibility(), save.units[2].visibility());
    assert_eq!(later.units[0].visibility(), save.units[0].visibility());
}
