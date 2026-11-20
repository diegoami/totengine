//! Native normal-context update vectors and transactional rejection checks.
use engine::saved_knowledge::{
    clear_unit_stack_visibility, unit_knowledge_after, unit_observation_after,
    unit_observation_after_reset, SavedMapKnowledge, UnitObservationError,
};
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

fn contact_context(v: &[usize]) -> (SaveFile, Rules) {
    let base = [
        4,
        v[0],
        1,
        v[1],
        1,
        v[2],
        v[3],
        1,
        1,
        0,
        6,
        6,
        255,
        if v[11] != 0 { 2 } else { 0 },
    ];
    let (mut save, _) = context(&base);
    let (dx, dy) = OFFSETS[v[2]];
    let positions = [(6, 6), ((6 + dx) as usize, (6 + dy) as usize)];
    for (group, (x, y)) in positions.into_iter().enumerate() {
        let i = y * 8 + x / 2;
        save.maps[0].tile_fields[6 * i] = if v[4] & (1 << group) != 0 { 10 } else { 1 };
    }
    for (slot, u) in save
        .units
        .iter_mut()
        .enumerate()
        .filter(|(_, u)| u.id() != 0)
    {
        u.raw[17] = if slot % 2 == 0 { v[5] as u8 } else { 3 };
        if v[10] != 0 && slot < 2 {
            u.raw[8] = 9;
        }
    }
    save.human_mask = v[7] as u8;
    save.diplomacy[4][5] = v[8] as u32;
    save.diplomacy[5][4] = v[8] as u32;
    let mut text = String::from("@UNITS\n");
    for id in 0..10 {
        let d = if id == 1 { v[1] } else { v[0] };
        text.push_str(&format!(
            "Unit,nil,{d},1,0,1,1,1,1,1,0,{},nil,000000000000001\n",
            v[6]
        ));
    }
    (save, parse_rules(&text))
}

fn submarine_context(v: &[usize]) -> (SaveFile, Rules) {
    let (mut save, _) = contact_context(&v[..13]);
    let (extended, subs, detectors, mode, tail_order, mixed_domain) =
        (v[13], v[14], v[15], v[16], v[17], v[18]);
    let mut text = String::from("@UNITS\n");
    for id in 0..4 {
        let mut domain = v[if id < 2 { 0 } else { 1 }];
        if id % 2 == 1 && mixed_domain & (1 << (id / 2)) != 0 {
            domain = 0;
        }
        let flags = usize::from(extended & (1 << id) != 0)
            | if subs & (1 << id) != 0 { 8 } else { 0 }
            | if detectors & (1 << id) != 0 {
                0x4000
            } else {
                0
            };
        text.push_str(&format!(
            "Unit,nil,{domain},1,0,1,1,1,1,1,0,{},nil,{flags:015b}\n",
            v[6]
        ));
    }
    for (slot, u) in save
        .units
        .iter_mut()
        .enumerate()
        .filter(|(_, u)| u.id() != 0)
    {
        u.raw[8] = slot as u8;
        u.raw[11] = 1;
        if slot % 2 == 1 {
            u.raw[17] = tail_order as u8;
        }
    }
    if mode == 1 {
        save.units[3].raw = [0; 40];
        save.units[2].raw[28..30].copy_from_slice(&(-1i16).to_le_bytes());
    }
    let reverse: &[usize] = if mode == 2 {
        &[2]
    } else if mode == 3 {
        &[0, 2]
    } else {
        &[]
    };
    for &first in reverse {
        save.units[first].raw[26..28].copy_from_slice(&((first + 1) as i16).to_le_bytes());
        save.units[first].raw[28..30].copy_from_slice(&(-1i16).to_le_bytes());
        save.units[first + 1].raw[26..28].copy_from_slice(&(-1i16).to_le_bytes());
        save.units[first + 1].raw[28..30].copy_from_slice(&(first as i16).to_le_bytes());
    }
    for c in save.cities.iter_mut().filter(|c| c.city_id() != 0) {
        c.raw[14] = 1;
    }
    (save, parse_rules(&text))
}

#[test]
fn native_submarine_detector_and_mixed_stack_vectors_match() {
    let numbers = |s: &str| {
        s.split_whitespace()
            .map(|n| n.parse::<usize>().unwrap())
            .collect::<Vec<_>>()
    };
    let mut counts = [0; 2];
    let mut review_cases = 0;
    for line in include_str!("../../../docs/evidence/import-submarine-vectors.txt")
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let (input, expected) = line.split_once('|').unwrap();
        let (kind, input) = input.trim().split_once(' ').unwrap();
        let v = numbers(input);
        let (save, rules) = submarine_context(&v);
        let (expected_kind, expected) = expected.trim().split_once(' ').unwrap();
        counts[usize::from(expected_kind == "C")] += 1;
        if kind == "F" {
            review_cases += 1;
        }
        assert_observation(save, rules, &v, expected_kind, numbers(expected), line);
    }
    assert_eq!(counts, [1435, 17]);
    assert_eq!(review_cases, 12);
}

#[test]
fn broader_observation_keeps_the_legacy_submarine_guard() {
    let mut v = [
        0, 0, 1, 0, 3, 255, 0, 16, 13, 0, 0, 0, 0, 15, 4, 0, 1, 255, 0,
    ];
    let (save, rules) = submarine_context(&v);
    assert!(unit_knowledge_after(&save, &rules, 0).is_err());
    let observed = unit_observation_after(&save, &rules, 0, false).unwrap();
    assert_eq!(
        observed.units[2].visibility() & 16,
        0,
        "single submarine stays hidden without a detector"
    );
    v[14] = 0;
    let (ordinary, rules) = submarine_context(&v);
    assert!(
        unit_knowledge_after(&ordinary, &rules, 0).is_ok(),
        "the legacy failure was the submarine guard, not an unrelated fixture error"
    );
}

#[test]
fn native_stack_reset_order_wakes_and_contact_boundaries_match() {
    let numbers = |s: &str| {
        s.split_whitespace()
            .map(|n| n.parse::<usize>().unwrap())
            .collect::<Vec<_>>()
    };
    let mut counts = [0; 3];
    for line in include_str!("../../../docs/evidence/import-contact-vectors.txt")
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let (input, expected) = line.split_once('|').unwrap();
        let (kind, input) = input.trim().split_once(' ').unwrap();
        let v = numbers(input);
        if kind == "R" {
            let (save, _) = context(&[4, 0, 0, 0, 0, 1, 0, 0, 1, 0, 6, 6, v[1], 0]);
            let out = clear_unit_stack_visibility(&save, v[0]).unwrap();
            assert_eq!(
                out.units
                    .iter()
                    .map(|u| usize::from(u.visibility()))
                    .collect::<Vec<_>>(),
                numbers(expected)
            );
            let mut restored = out;
            for (u, old) in restored.units.iter_mut().zip(&save.units) {
                u.raw[11] = old.raw[11];
            }
            assert_eq!(restored, save);
            counts[0] += 1;
            continue;
        }
        let (save, rules) = contact_context(&v);
        let (expected_kind, expected) = expected.trim().split_once(' ').unwrap();
        counts[if expected_kind == "C" { 2 } else { 1 }] += 1;
        assert_observation(save, rules, &v, expected_kind, numbers(expected), line);
    }
    assert_eq!(counts, [24, 1261, 19]);
}

#[test]
fn reset_rejects_broken_chains_and_observation_reports_unsupported_inputs() {
    let (save, rules) = contact_context(&[0, 0, 1, 0, 0, 3, 0, 16, 5, 0, 0, 0]);
    let mut bad = save.clone();
    bad.units[1].raw[26..28].copy_from_slice(&(-1i16).to_le_bytes());
    assert!(clear_unit_stack_visibility(&bad, 0).is_err());
    assert!(unit_observation_after(&bad, &rules, 0, false).is_err());
    assert!(clear_unit_stack_visibility(&save, 999).is_err());
    bad = save.clone();
    bad.version = 0x33;
    assert!(matches!(
        unit_observation_after(&bad, &rules, 0, false),
        Err(UnitObservationError::Unsupported(_))
    ));
    // Native reset works from a tail, including units with sentry orders.
    let reset = clear_unit_stack_visibility(&save, 1).unwrap();
    assert_eq!(reset.units[0].visibility(), 0);
    assert_eq!(reset.units[1].visibility(), 0);
    assert_eq!(reset.units[2].visibility(), 255);
    assert_eq!(reset.units[1].orders(), 3);
    let (legacy, mut legacy_rules) = context(&[4, 0, 1, 0, 1, 1, 0, 0, 1, 0, 6, 6, 255, 0]);
    for row in &mut legacy_rules.sections.get_mut("UNITS").unwrap().rows {
        row[11] = "unknown".into();
    }
    assert!(unit_knowledge_after(&legacy, &legacy_rules, 0).is_ok());
    assert!(matches!(
        unit_observation_after(&legacy, &legacy_rules, 0, false),
        Err(UnitObservationError::Unsupported(_))
    ));
}

fn assert_observation(
    save: SaveFile,
    rules: Rules,
    v: &[usize],
    expected_kind: &str,
    expected: Vec<usize>,
    line: &str,
) {
    let before = save.clone();
    let result = if v[12] != 0 {
        unit_observation_after_reset(&save, &rules, 0, v[9] != 0)
    } else {
        unit_observation_after(&save, &rules, 0, v[9] != 0)
    };
    if expected_kind == "C" {
        assert_eq!(
            result,
            Err(UnitObservationError::ContactRequired {
                observer: expected[0] as u8,
                other: expected[1] as u8,
                x: expected[2] as u16,
                y: expected[3] as u16,
                map: expected[4] as u8
            }),
            "{line}"
        );
    } else {
        let out = result.unwrap_or_else(|e| panic!("{line}: {e}"));
        let mut got: Vec<_> = out
            .units
            .iter()
            .map(|u| usize::from(u.visibility()))
            .collect();
        got.extend(out.units.iter().map(|u| usize::from(u.orders())));
        got.extend(out.cities.iter().map(|c| usize::from(c.known_to())));
        got.extend(
            out.cities
                .iter()
                .map(|c| usize::from(c.known_size(4).unwrap())),
        );
        let explored: Vec<_> = (0..96)
            .filter(|&i| out.maps[0].visible_to(i) & 16 != 0)
            .collect();
        got.push(explored.len());
        got.extend(&explored);
        assert_eq!(got, expected, "{line}");
        let mut restored = out.clone();
        for i in 0..96 {
            assert_eq!(
                out.maps[0].tile_fields[6 * i + 4],
                save.maps[0].tile_fields[6 * i + 4] | if explored.contains(&i) { 16 } else { 0 }
            );
            restored.maps[0].tile_fields[6 * i + 4] = save.maps[0].tile_fields[6 * i + 4];
            for civ in 1..8 {
                assert_eq!(
                    out.maps[0].visibility[(civ - 1) * 96 + i],
                    if civ == 4 && explored.contains(&i) {
                        save.maps[0].improvements(i)
                    } else {
                        128
                    }
                );
            }
        }
        restored.maps[0]
            .visibility
            .clone_from(&save.maps[0].visibility);
        for (u, old) in restored.units.iter_mut().zip(&save.units) {
            u.raw[11] = old.raw[11];
            u.raw[17] = old.raw[17];
        }
        for (c, old) in restored.cities.iter_mut().zip(&save.cities) {
            c.raw[14] = old.raw[14];
            c.raw[19] = old.raw[19];
        }
        assert_eq!(
            restored, save,
            "only knowledge and order fields may change: {line}"
        );
    }
    assert_eq!(
        save, before,
        "contact must not leak a partially changed snapshot"
    );
}
