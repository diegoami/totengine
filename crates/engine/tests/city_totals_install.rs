//! Native fixed-worker city math, including sequential route-cache refresh.
#[test]
fn original_city_totals() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(std::path::PathBuf::from) else {
        eprintln!("skipping: CIVTOT_HOME not set");
        return;
    };
    let mut mismatches = Vec::new();
    let mut cases = 0;
    for name in ["Original/Ph_Auto2.SAV", "FallRome/ROMFALL.scn"] {
        let save = tot_kit::parse_sav_file(&home.join(name)).unwrap();
        let rules_path = std::path::Path::new(name)
            .parent()
            .unwrap()
            .join("Rules.txt");
        let rules = tot_kit::parse_rules(&tot_kit::latin1(
            &std::fs::read(home.join(rules_path)).unwrap(),
        ));
        for mode in ["fixed", "auto"] {
            let mut w = engine::boot(&save, 1);
            for line in include_str!("../../../docs/evidence/city-total-vectors.txt")
                .lines()
                .filter(|l| l.starts_with(name))
            {
                let mut fields = line.split_whitespace();
                fields.next();
                if fields.next() != Some(mode) {
                    continue;
                }
                let city: usize = fields.next().unwrap().parse().unwrap();
                let expected: Vec<i64> = fields.map(|s| s.parse().unwrap()).collect();
                if mode == "auto" {
                    w.refresh_city_totals(&rules, city, false).unwrap();
                }
                w.cities[city].specialists = engine::workers::normalize_specialists(
                    w.cities[city].specialists,
                    w.cities[city].worked,
                );
                w.cities[city].route_trade = w.pre_route_trade(&rules, city);
                let output = w.city_output(&rules, city);
                let support = w.unit_support(&rules, city);
                let finance =
                    w.split_city_trade(&rules, city, output.trade, output.shields - support);
                let mood = w.city_happiness(&rules, city);
                let c = &w.cities[city];
                let owner = (c.owner & 15) as u8;
                let settlers = w
                    .units
                    .iter()
                    .filter(|u| {
                        u.id != 0
                            && u.owner == owner
                            && u.home as usize == city
                            && rules.unit_role(u.type_id as usize) == Some(5)
                    })
                    .count();
                let actual = [
                    support as i64,
                    settlers as i64,
                    c.worked as i64,
                    c.specialists as i64,
                    (c.flags & !engine::founding::CITY_REFRESH) as i64,
                    c.food_box as i16 as i64,
                    c.route_trade as i64,
                    finance.science as i64,
                    finance.taxes as i64,
                    output.trade as i64,
                    output.food as i64,
                    output.shields as i64,
                    mood.happy as i64,
                    mood.unhappy as i64,
                ];
                for (field, (&a, &e)) in actual.iter().zip(&expected).enumerate() {
                    if a != e {
                        mismatches.push(format!(
                            "{name} {mode} city {city} field {field}: engine {a}, original {e}"
                        ));
                    }
                }
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 312);
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

/// Exercise positive recurring routes through final corruption and finance,
/// rather than comparing isolated route arithmetic on uniform maps.
#[test]
fn positive_routes_and_final_corruption_match_full_original_totals() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(std::path::PathBuf::from) else {
        return;
    };
    let save = tot_kit::parse_sav_file(&home.join("Original/Ph_Auto2.SAV")).unwrap();
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(home.join("Original/Rules.txt")).unwrap(),
    ));
    let mut cases = 0;
    let mut corrupted = 0;
    for gov in 0..7 {
        for humans in [0, 254] {
            let mut w = engine::boot(&save, 1);
            w.governments = [gov; 8];
            w.human_mask = humans;
            w.science_rates = [4; 8];
            w.tax_rates = [5; 8];
            let count = w.cities.len();
            for (i, c) in w.cities.iter_mut().enumerate() {
                c.route_trade = (64 + 16 * i) as i16;
                c.route_count = 3;
                c.route_commodities = [0, 1, 2];
                c.route_partners = std::array::from_fn(|k| ((i + k + 1) % count) as i16);
            }
            for line in include_str!("../../../docs/evidence/city-route-total-vectors.txt")
                .lines()
                .filter(|l| !l.starts_with('#'))
            {
                let v: Vec<i64> = line
                    .split_whitespace()
                    .map(|x| x.parse().unwrap())
                    .collect();
                if v[0] != gov as i64 || v[1] != humans as i64 {
                    continue;
                }
                let i = v[2] as usize;
                w.cities[i].specialists = engine::workers::normalize_specialists(
                    w.cities[i].specialists,
                    w.cities[i].worked,
                );
                w.cities[i].route_trade = w.pre_route_trade(&rules, i);
                let output = w.city_output(&rules, i);
                let support = w.unit_support(&rules, i);
                let finance = w.split_city_trade(&rules, i, output.trade, output.shields - support);
                let mood = w.city_happiness_with_luxuries(&rules, i, finance.luxuries);
                let c = &w.cities[i];
                let settlers = w
                    .units
                    .iter()
                    .filter(|u| {
                        u.id != 0
                            && u.owner as u16 == c.owner & 15
                            && u.home as usize == i
                            && rules.unit_role(u.type_id as usize) == Some(5)
                    })
                    .count();
                let route_arrows: i32 = w.trade_route_income(&rules, i, c.route_trade).iter().sum();
                let actual = [
                    support as i64,
                    settlers as i64,
                    c.worked as i64,
                    c.specialists as i64,
                    (c.flags & !engine::founding::CITY_REFRESH) as i64,
                    c.food_box as i16 as i64,
                    c.route_trade as i64,
                    finance.science as i64,
                    finance.taxes as i64,
                    output.trade as i64,
                    output.food as i64,
                    output.shields as i64,
                    mood.happy as i64,
                    mood.unhappy as i64,
                    route_arrows as i64,
                    output.corruption as i64,
                ];
                assert_eq!(actual, &v[3..], "government {gov} humans {humans} city {i}");
                assert!(route_arrows > 0);
                corrupted += usize::from(output.corruption > 0);
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 140);
    assert_eq!(corrupted, 40);
}
