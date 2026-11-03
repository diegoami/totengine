//! v0.2.0 headline, env-gated on CIVTOT_HOME: a STANDARD game (no
//! scenario) boots and runs headless. `Original/Ph_Auto.SAV` is the
//! rotating autosave of the standard-game lab (evidence §T1-4), so only
//! properties are asserted, never its exact bytes (the oracle may rewrite
//! it).

#[test]
fn standard_game_boots_and_runs_fifty_turns() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(std::path::PathBuf::from) else {
        eprintln!("skipping: CIVTOT_HOME not set");
        return;
    };
    let Ok(raw) = std::fs::read(home.join("Original/Ph_Auto.SAV")) else {
        eprintln!("skipping: no standard-game autosave");
        return;
    };
    let sav = tot_kit::parse_sav(&raw).expect("standard-game save parses");
    let rules = tot_kit::parse_rules(&String::from_utf8_lossy(
        &std::fs::read(home.join("Rules.txt")).expect("global Rules.txt"),
    ));
    let run = || {
        let mut w = engine::boot(&sav, 3);
        w.start_turn(&rules);
        let mut grew = 0;
        let mut sums = Vec::new();
        for _ in 0..50 {
            grew += w.run_turn(&rules, None).grew.len();
            sums.push(w.checksum());
        }
        (w, grew, sums)
    };
    let (w, grew, a) = run();
    let (_, _, b) = run();
    assert_eq!(a, b, "deterministic");
    assert!(!w.cities.is_empty() && !w.units.is_empty());
    // every city's owner is a real civ slot, and every city is on the map
    assert!(w.cities.iter().all(|c| c.owner <= 7));
    assert!(w
        .cities
        .iter()
        .all(|c| w.map.terrain_at(c.x, c.y).is_some()));
    assert!(grew > 0, "cities grow over 50 turns");
    assert_eq!(w.turn, sav.turn + 50);
}
