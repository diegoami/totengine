#[test]
fn original_city_destruction_state() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(std::path::PathBuf::from) else {
        return;
    };
    let save = tot_kit::parse_sav_file(&home.join("Original/Ph_Auto2.SAV")).unwrap();
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(home.join("Original/Rules.txt")).unwrap(),
    ));
    let mut cases = 0;
    for line in include_str!("../../../docs/evidence/city-destruction-vectors.txt")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let v: Vec<i64> = line
            .split_whitespace()
            .map(|s| s.parse().unwrap())
            .collect();
        let mut w = engine::boot(&save, 1);
        w.human_mask = v[0] as u8;
        let target = v[1] as usize;
        for u in &mut w.units {
            u.home = target as u8;
        }
        w.units[0].type_id = 9;
        for c in &mut w.cities {
            c.route_count = 3;
            c.route_commodities = [-1, 0, 1];
            c.route_partners = [target as i16, ((target + 1) % 10) as i16, target as i16];
        }
        w.wonders = std::array::from_fn(|k| if k % 3 == 0 { target as i16 } else { -1 });
        let mut map = w.map.clone();
        for change in include_str!("../../../docs/evidence/city-destruction-map-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let m: Vec<usize> = change
                .split_whitespace()
                .map(|x| x.parse().unwrap())
                .collect();
            if m[0] != v[0] as usize || m[1] != target {
                continue;
            }
            let (i, field, after) = (m[2], m[3], m[5] as u8);
            match field {
                1 => map.improvements[i] = after,
                2 => map.site_bits[i] = after >> 5,
                5 => {
                    map.owner[i] = after >> 4;
                    map.fertility[i] = after & 15;
                }
                _ => panic!("unexpected tile field {field}"),
            }
        }
        assert!(w.destroy_surface_city(&rules, target));
        assert_eq!(
            w.map.improvements, map.improvements,
            "unit/city markers, humans {} city {target}",
            v[0]
        );
        assert_eq!(
            w.map.fertility, map.fertility,
            "fertility invalidation, humans {} city {target}",
            v[0]
        );
        assert_eq!(
            w.map.site_bits, map.site_bits,
            "site clearing, humans {} city {target}",
            v[0]
        );
        assert_eq!(
            w.map.owner, map.owner,
            "tile ownership, humans {} city {target}",
            v[0]
        );
        let mut actual: Vec<i64> = w.city_counts.iter().map(|&x| x as i64).collect();
        for u in &w.units {
            actual.extend([i64::from(u.id != 0), u.home as i64]);
        }
        for c in &w.cities {
            actual.extend([c.city_id as i64, c.flags as i64, c.route_count as i64]);
            actual.extend(c.route_commodities.map(i64::from));
            actual.extend(c.route_partners.map(i64::from));
        }
        actual.extend(w.wonders.map(i64::from));
        let differences: Vec<_> = actual
            .iter()
            .zip(&v[2..])
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .collect();
        assert!(
            differences.is_empty(),
            "humans {} city {target}: {differences:?}",
            v[0]
        );
        cases += 1;
    }
    assert_eq!(cases, 30);
}
