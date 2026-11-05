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
