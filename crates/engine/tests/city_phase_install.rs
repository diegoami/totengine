//! Controlled full-native city-phase comparison. This is not the `run_turn`
//! acceptance gate: no Wine saves, research, ordinary builds or AI campaign.
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
        w.cities[k].flags &= 0xffbfffbb;
        let context = w.city_growth_context(&rules, k).unwrap();
        assert!(
            context.food_box >= 0,
            "controlled case reached unsupported starvation"
        );
        let growth = engine::growth::prepare_growth(context);
        w.cities[k].size = growth.size as u16;
        w.cities[k].food_box = growth.food_box as u16;
        let pre = w.refresh_city_totals(&rules, k, false).unwrap();
        let surplus = pre.output.food
            - w.cities[k].size as i32 * rules.cosmic_food_per_citizen() as i32
            - pre.settlers * pre.settler_food;
        w.cities[k].food_box = (w.cities[k].food_box as i16 as i32 + surplus) as u16;
        assert!(w.production_step(&rules, k).is_none());
        let post = w.refresh_city_totals(&rules, k, true).unwrap();
        // 0x4c9c2d: balanced cities with shield output and no food deficit
        // preserve their specialists on the next AI processing pass.
        if post.mood.happy == post.mood.unhappy
            && post.output.shields - post.support > 0
            && surplus >= 0
        {
            w.cities[k].flags |= 0x4000;
        } else {
            w.cities[k].flags &= !0x4000;
        }
        assert!(
            post.mood.unhappy <= post.mood.happy,
            "controlled case reached disorder"
        );
        assert!(
            post.mood.happy < (w.cities[k].size as i32 + 1) / 2 || w.cities[k].size <= 2,
            "controlled case reached celebration"
        );
        w.cities[k].flags &= !1;
        w.process_city_treasury(&rules, k, post.finance.taxes);
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
