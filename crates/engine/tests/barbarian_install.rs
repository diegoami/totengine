//! Sustained original-runtime arrival phases; no AI/city/pollution round claim.
#[test]
fn same_seed_arrivals_match_sixty_four_live_original_phases() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(std::path::PathBuf::from) else {
        return;
    };
    let save = tot_kit::parse_sav_file(&home.join("Original/Ph_Auto2.SAV")).unwrap();
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(home.join("Original/Rules.txt")).unwrap(),
    ));
    let mut w = engine::boot(&save, 1);
    w.difficulty = 4;
    w.human_mask = 254;
    w.seed_word = (w.seed_word & 0xff00) | 3;
    for civ in 1..8 {
        w.treasury[civ] = 32000;
    }
    let mut cases = 0;
    let mut units = 0;
    for line in include_str!("../../../docs/evidence/barbarian-live-vectors.txt")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let (input, expected) = line.split_once('|').unwrap();
        let values: Vec<u32> = input
            .split_whitespace()
            .map(|x| x.parse().unwrap())
            .collect();
        w.turn = values[0] as i16;
        assert_eq!(
            w.combat_rng.state(),
            values[1],
            "continuous mechanics state before turn {}",
            w.turn
        );
        let new = w.barbarian_arrivals(&rules);
        let actual: Vec<Vec<u32>> = new
            .iter()
            .map(|&i| {
                let u = &w.units[i];
                vec![
                    i as u32,
                    u.type_id as u32,
                    u.x as u32,
                    u.y as u32,
                    u.orders as u32,
                    u.veteran as u32,
                ]
            })
            .collect();
        let expected: Vec<Vec<u32>> = expected
            .split_whitespace()
            .map(|s| s.split(',').map(|x| x.parse().unwrap()).collect())
            .collect();
        assert_eq!(
            actual, expected,
            "native allocation/arrival turn {}",
            w.turn
        );
        assert_eq!(
            w.combat_rng.state(),
            values[2],
            "final mechanics state turn {}",
            w.turn
        );
        units += new.len();
        cases += 1;
    }
    assert_eq!(cases, 64);
    assert_eq!(units, 200);
}
