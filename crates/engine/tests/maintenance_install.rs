#[test]
fn upkeep_matches_original_with_saved_knowledge_and_wonders() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        return;
    };
    let home = std::path::PathBuf::from(home);
    let vectors = include_str!("../../../docs/evidence/maintenance-saved-vectors.txt");
    for name in ["Original/Ph_Auto2.SAV", "FallRome/ROMFALL.scn"] {
        let save = tot_kit::parse_sav(&std::fs::read(home.join(name)).unwrap()).unwrap();
        let rules = tot_kit::parse_rules(&tot_kit::latin1(
            &std::fs::read(
                home.join(std::path::Path::new(name).parent().unwrap())
                    .join("Rules.txt"),
            )
            .unwrap(),
        ));
        let w = engine::boot(&save, 1);
        let mut count = 0;
        for line in vectors.lines().filter(|l| l.starts_with(name)) {
            let v: Vec<_> = line.split_whitespace().collect();
            assert_eq!(
                w.improvement_upkeep(&rules, v[1].parse().unwrap(), v[2].parse().unwrap()),
                v[3].parse::<i32>().unwrap(),
                "{line}"
            );
            count += 1;
        }
        assert_eq!(count, 245);
    }
}
