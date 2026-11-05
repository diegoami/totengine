//! Derived radius availability from original reference-save execution.
#[test]
fn saved_worker_availability_matches_original_helpers() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(std::path::PathBuf::from) else {
        eprintln!("skipping: CIVTOT_HOME not set");
        return;
    };
    let mut cases = 0;
    for name in ["Original/Ph_Auto2.SAV", "FallRome/ROMFALL.scn"] {
        let save = tot_kit::parse_sav_file(&home.join(name)).unwrap();
        let path = std::path::Path::new(name)
            .parent()
            .unwrap()
            .join("Rules.txt");
        let rules =
            tot_kit::parse_rules(&tot_kit::latin1(&std::fs::read(home.join(path)).unwrap()));
        let w = engine::boot(&save, 1);
        for line in include_str!("../../../docs/evidence/worker-availability-vectors.txt")
            .lines()
            .filter(|l| l.starts_with(name))
        {
            let mut fields = line.split_whitespace();
            fields.next();
            let city: usize = fields.next().unwrap().parse().unwrap();
            let expected: Vec<u8> = fields.map(|s| s.parse().unwrap()).collect();
            assert_eq!(w.worker_availability(&rules, city), expected[..], "{line}");
            cases += 1;
        }
    }
    assert_eq!(cases, 156);
}
