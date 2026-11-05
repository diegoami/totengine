//! Controlled native city-phase and full headless-round economic comparisons.
//! No Wine saves, research, ordinary builds or AI campaign acceptance claim.
#[test]
fn controlled_fifty_turn_city_sequence() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(std::path::PathBuf::from) else {
        return;
    };
    let save = tot_kit::parse_sav_file(&home.join("Original/Ph_Auto2.SAV")).unwrap();
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(home.join("Original/Rules.txt")).unwrap(),
    ));
    let mut w = engine::boot(&save, 1);
    w.human_mask = 255;
    w.difficulty = 0;
    w.science_rates = [0; 8];
    w.tax_rates = [10; 8];
    for c in &mut w.cities {
        c.specialists = 0;
        c.building = -39;
        c.flags = 0;
        c.size = 1;
        c.food_box = 0;
        c.worked = 1 << 20;
        c.buildings = [c.buildings[0] & 2, 0, 0, 0, 0];
    }
    for u in &mut w.units {
        u.home = 255;
    }
    let mut mismatches = Vec::new();
    let mut cases = 0;
    let mut routes = engine::city_phase::FoodRouteState::default();
    for line in include_str!("../../../docs/evidence/city-phase-vectors.txt")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let v: Vec<i64> = line
            .split_whitespace()
            .map(|s| s.parse().unwrap())
            .collect();
        w.turn = v[0] as i16;
        let k = v[1] as usize;
        let step = w.run_city_phase(&rules, k, &mut routes).unwrap();
        assert!(!step.destroyed && !step.famined && !step.revolution);
        assert!(step.built.is_none() && step.disbanded.is_empty());
        let c = &w.cities[k];
        let actual = [
            c.size as i64,
            c.food_box as i16 as i64,
            c.shield_box as i16 as i64,
            c.worked as i64,
            c.specialists as i64,
            (c.flags & !0x80000) as i64,
            c.route_trade as i64,
            c.net_science as i64,
            c.net_taxes as i64,
            c.gross_trade as i64,
            c.gross_food as i64,
            c.gross_shields as i64,
            c.happy as i64,
            c.unhappy as i64,
            w.treasury[(c.owner & 15) as usize] as i64,
        ];
        // 0x4cad56 also updates the original's settler-job planner and its
        // 0x80000 city flag. That planner is outside this economic comparison;
        // retain the native flag in the artifact instead of fabricating it.
        let mut economic = v[2..].to_vec();
        economic[5] &= !0x80000;
        for (field, (&a, &e)) in actual.iter().zip(&economic).enumerate() {
            if a != e {
                mismatches.push(format!(
                    "turn {} city {k} field {field}: engine {a}, original {e}",
                    w.turn
                ));
            }
        }
        cases += 1;
    }
    assert_eq!(cases, 500);
    assert!(
        mismatches.is_empty(),
        "{} mismatches:\n{}",
        mismatches.len(),
        mismatches
            .iter()
            .take(25)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn full_round_loop_matches_controlled_native_city_economics_for_fifty_turns() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(std::path::PathBuf::from) else {
        return;
    };
    let save = tot_kit::parse_sav_file(&home.join("Original/Ph_Auto2.SAV")).unwrap();
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(home.join("Original/Rules.txt")).unwrap(),
    ));
    let mut w = engine::boot(&save, 1);
    w.human_mask = 255;
    w.difficulty = 0;
    w.science_rates = [0; 8];
    w.tax_rates = [10; 8];
    w.seed_word &= 0xff00; // Barbarian level zero; no arrivals in this city-only control.
    for c in &mut w.cities {
        c.specialists = 0;
        c.building = -39;
        c.flags = 0;
        c.size = 1;
        c.food_box = 0;
        c.worked = 1 << 20;
        c.buildings = [c.buildings[0] & 2, 0, 0, 0, 0];
    }
    for u in &mut w.units {
        u.home = 255;
    }
    let rows: Vec<Vec<i64>> = include_str!("../../../docs/evidence/city-round-vectors.txt")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split_whitespace().map(|x| x.parse().unwrap()).collect())
        .collect();
    assert_eq!(rows.len(), 500);
    for round in rows.as_chunks::<10>().0 {
        assert_eq!(w.turn as i64, round[0][0]);
        let report = w.run_turn(&rules, None);
        assert!(
            report.built.is_empty()
                && report.famined.is_empty()
                && report.disbanded_units.is_empty()
                && report.discoveries.is_empty()
                && report.barbarian_arrivals.is_empty()
        );
        for row in round {
            let k = row[1] as usize;
            let c = &w.cities[k];
            let actual = [
                c.size as i64,
                c.food_box as i16 as i64,
                c.shield_box as i16 as i64,
                c.worked as i64,
                c.specialists as i64,
                (c.flags & !0x80000) as i64,
                c.route_trade as i64,
                c.net_science as i64,
                c.net_taxes as i64,
                c.gross_trade as i64,
                c.gross_food as i64,
                c.gross_shields as i64,
                c.happy as i64,
                c.unhappy as i64,
            ];
            let mut expected = row[2..16].to_vec();
            expected[5] &= !0x80000;
            assert_eq!(actual, expected[..], "turn {} city {k}", row[0]);
        }
        for civ in 1..8 {
            if let Some(row) = round
                .iter()
                .rfind(|r| w.cities[r[1] as usize].owner & 15 == civ as u16)
            {
                assert_eq!(
                    w.treasury[civ] as i64, row[16],
                    "turn {} civ {civ} treasury",
                    row[0]
                );
            }
        }
    }
}
