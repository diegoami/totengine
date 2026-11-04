//! #87: finance computed from original-game trade/shield records. Isolates
//! finance from route/connection work (#84/#85); never commits input saves.
use std::path::PathBuf;

fn check(save: &str, rules: &str) {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(PathBuf::from) else {
        eprintln!("skipping: CIVTOT_HOME not set");
        return;
    };
    let sav = tot_kit::parse_sav_file(&home.join(save)).unwrap();
    let rules = tot_kit::parse_rules(&tot_kit::latin1(&std::fs::read(home.join(rules)).unwrap()));
    let w = engine::boot(&sav, 1);
    assert!(!w.cities.is_empty());
    let mut specialists = 0;
    let mut capitalization = 0;
    for (i, c) in w.cities.iter().enumerate() {
        let out = w.split_city_trade(
            &rules,
            i,
            c.gross_trade as i32,
            (c.gross_shields as i32 - w.unit_support(&rules, i)).max(0),
        );
        assert_eq!(
            (out.science, out.taxes),
            (c.net_science as i32, c.net_taxes as i32),
            "{save}: city {i} {}",
            c.name
        );
        specialists += (c.specialists != 0) as usize;
        capitalization += (c.building == -39) as usize;
    }
    if save == "FallRome/ROMFALL.scn" {
        assert_eq!(w.cities.len(), 146);
        assert!(specialists > 0 && capitalization > 0);
    }
}

#[test]
fn all_fallrome_city_science_and_tax_records_match() {
    check("FallRome/ROMFALL.scn", "FallRome/Rules.txt");
}

#[test]
fn standard_city_science_and_tax_records_match() {
    check("Original/Ph_Auto2.SAV", "Original/Rules.txt");
}
