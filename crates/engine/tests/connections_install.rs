//! Original x86-derived capital-connection cases, not engine-generated expectations.
#[test]
fn connections_match_original_execution() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        return;
    };
    let home = std::path::PathBuf::from(home);
    let vectors = include_str!("../../../docs/evidence/connection-vectors.txt");
    for (save_path, rules_path, expected_count) in [
        ("Original/Ph_Auto2.SAV", "Original/Rules.txt", 5),
        ("FallRome/ROMFALL.scn", "FallRome/Rules.txt", 118),
    ] {
        let save = tot_kit::parse_sav(&std::fs::read(home.join(save_path)).unwrap()).unwrap();
        let rules = tot_kit::parse_rules(&tot_kit::latin1(
            &std::fs::read(home.join(rules_path)).unwrap(),
        ));
        let w = engine::boot(&save, 1);
        let mut count = 0;
        for line in vectors.lines().filter(|l| l.starts_with(save_path)) {
            let f: Vec<_> = line.split_whitespace().collect();
            let i: usize = f[1].parse().unwrap();
            let k: usize = f[2].parse().unwrap();
            let expected: i32 = f[3].parse().unwrap();
            let (a, b) = (&w.cities[i], &w.cities[k]);
            assert_eq!(
                w.road_connection(&rules, (a.owner & 15) as u8, (a.x, a.y), (b.x, b.y)),
                expected,
                "{save_path} city {i} to {k}"
            );
            assert_eq!(
                w.capital_connection(&rules, i),
                expected,
                "capital {save_path} city {i}"
            );
            count += 1;
        }
        assert_eq!(count, expected_count);
    }
}
