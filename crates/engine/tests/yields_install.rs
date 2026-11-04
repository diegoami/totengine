//! T1-4 tile yields against game-written city records (env-gated on
//! CIVTOT_HOME). Evidence §T1-4 tile yields.

fn load(rel: &str, rules_rel: &str) -> Option<(engine::WorldState, tot_kit::Rules)> {
    let home = std::path::PathBuf::from(std::env::var_os("CIVTOT_HOME")?);
    let sav = tot_kit::parse_sav(&std::fs::read(home.join(rel)).ok()?).ok()?;
    let rules = tot_kit::parse_rules(&tot_kit::latin1(&std::fs::read(home.join(rules_rel)).ok()?));
    Some((engine::boot(&sav, 1), rules))
}

/// ROMFALL: food has no post-processing, so the worked-tile sum equals
/// the record (144/146); trade is stored after corruption, so it is never
/// above the tile sum.
#[test]
fn romfall_food_matches_and_trade_never_exceeds() {
    let Some((w, rules)) = load("FallRome/ROMFALL.scn", "FallRome/Rules.txt") else {
        eprintln!("skipping: CIVTOT_HOME not set");
        return;
    };
    let mut food_exact = 0;
    for (i, c) in w.cities.iter().enumerate() {
        let y = w.worked_yield(&rules, i);
        food_exact += (y[0] == c.gross_food as i32) as usize;
        assert!(
            c.gross_trade as i32 <= y[2],
            "{}: trade {} > tile sum {}",
            c.name,
            c.gross_trade,
            y[2]
        );
    }
    assert_eq!((food_exact, w.cities.len()), (144, 146));
}

/// A standard game's autosave (Original rules): every city's food is
/// exact, and food/shields/trade all match in at least 90 % of cities
/// (the rest: corruption/waste, not decoded yet). The rotating autosave
/// can be rewritten by the oracle, so only these properties are asserted.
#[test]
fn standard_game_tile_sums_match_records() {
    let Some((w, rules)) = load("Original/Ph_Auto2.SAV", "Original/Rules.txt") else {
        eprintln!("skipping: no standard-game autosave");
        return;
    };
    let mut all = 0;
    for (i, c) in w.cities.iter().enumerate() {
        let y = w.worked_yield(&rules, i);
        assert_eq!(y[0], c.gross_food as i32, "{} food", c.name);
        all += (y
            == [
                c.gross_food as i32,
                c.gross_shields as i32,
                c.gross_trade as i32,
            ]) as usize;
    }
    assert!(!w.cities.is_empty());
    assert!(all * 10 >= w.cities.len() * 9, "{all}/{}", w.cities.len());
}
