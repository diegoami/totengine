//! Compare the original's saved happiness using its stored trade inputs.
#[test]
fn saved_city_happiness_records() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        return;
    };
    let home = std::path::PathBuf::from(home);
    let vectors = include_str!("../../../docs/evidence/happiness-saved-vectors.txt");
    for (name, stored_exact) in [("Original/Ph_Auto2.SAV", 10), ("FallRome/ROMFALL.scn", 131)] {
        let save = tot_kit::parse_sav(&std::fs::read(home.join(name)).unwrap()).unwrap();
        let rules = tot_kit::parse_rules(&tot_kit::latin1(
            &std::fs::read(
                home.join(std::path::Path::new(name).parent().unwrap())
                    .join("Rules.txt"),
            )
            .unwrap(),
        ));
        let w = engine::boot(&save, 1);
        let cases: Vec<Vec<&str>> = vectors
            .lines()
            .filter(|l| l.starts_with(name))
            .map(|l| l.split_whitespace().collect())
            .collect();
        assert_eq!(cases.len(), w.cities.len());
        let mut stored_matches = 0;
        for (i, c) in w.cities.iter().enumerate() {
            let luxuries = w
                .split_city_trade(
                    &rules,
                    i,
                    c.gross_trade as i32,
                    c.gross_shields as i32 - w.unit_support(&rules, i),
                )
                .luxuries;
            let h = w.city_happiness_with_luxuries(&rules, i, luxuries);
            let v = &cases[i];
            assert_eq!(v[1].parse::<usize>().unwrap(), i);
            assert_eq!(
                luxuries,
                v[2].parse::<i32>().unwrap(),
                "{name} city {i} luxuries"
            );
            assert_eq!(
                h,
                engine::happiness::Happiness {
                    happy: v[3].parse().unwrap(),
                    unhappy: v[4].parse().unwrap(),
                    angry: v[5].parse().unwrap(),
                },
                "{name} city {i}: current arithmetic vs original execution"
            );
            let expected = (save.cities[i].raw[84] as i32, save.cities[i].raw[85] as i32);
            stored_matches += ((h.happy, h.unhappy) == expected) as usize;
        }
        // The original bytes reproduce the same 15 FallRome stored
        // discrepancies. Complete-turn parity is a separate acceptance.
        assert_eq!(stored_matches, stored_exact);
    }
}

#[test]
fn disorder_affects_the_following_production_phase() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        return;
    };
    let home = std::path::PathBuf::from(home);
    let save =
        tot_kit::parse_sav(&std::fs::read(home.join("FallRome/ROMFALL.scn")).unwrap()).unwrap();
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(home.join("FallRome/Rules.txt")).unwrap(),
    ));
    let mut w = engine::boot(&save, 1);
    w.wonders = [-1; 28];
    w.units.clear();
    w.difficulty = 5;
    w.human_mask = 2;
    w.city_counts[1] = 200;
    w.science_rates[1] = 5;
    w.tax_rates[1] = 5;
    w.cities[0].buildings = [0; 5];
    w.cities[0].specialists = 0;
    w.cities[0].building = 0;
    w.cities[0].flags = 0;
    w.cities[0].shield_box = 0;
    assert!(w.city_output(&rules, 0).shields > 0);
    w.run_turn(&rules, None);
    assert_ne!(w.cities[0].flags & 1, 0);
    assert!(w.cities[0].unhappy > w.cities[0].happy);
    // Original 0x4c6208 runs before 0x4c998e detects this turn's riot.
    let collected = w.cities[0].shield_box;
    assert!(collected > 0);
    w.run_turn(&rules, None);
    assert_eq!(
        w.cities[0].shield_box, collected,
        "existing disorder stops production"
    );
    w.governments[1] = 4;
    w.run_turn(&rules, None);
    assert_eq!(w.cities[0].flags & 1, 0);
    assert_eq!(w.cities[0].unhappy, 0);
    assert_eq!(
        w.cities[0].shield_box, collected,
        "recovery happens after production too"
    );
    w.run_turn(&rules, None);
    assert!(w.cities[0].shield_box > collected);
}
