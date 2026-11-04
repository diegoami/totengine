//! #75 acceptance on real data (env-gated on CIVTOT_HOME): a 100-turn
//! all-AI (AI v0) game on a standard-game save — every civ alive at the
//! start founds at least one city, cities grow in number, and the game
//! replays bit-exact from save + seed + order log.

use engine::ai::AiV0;
use engine::player::Game;

#[test]
fn hundred_turn_all_ai_game_expands_and_replays() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(std::path::PathBuf::from) else {
        eprintln!("skipping: CIVTOT_HOME not set");
        return;
    };
    let (Ok(save), Ok(rules_txt)) = (
        std::fs::read(home.join("Original/Ph_Auto2.SAV")),
        std::fs::read(home.join("Original/Rules.txt")),
    ) else {
        eprintln!("skipping: no standard-game autosave");
        return;
    };
    let rules = || tot_kit::parse_rules(&tot_kit::latin1(&rules_txt));
    let mut g = Game::new("Ph_Auto2.SAV", &save, 3, rules(), None).unwrap();
    let start_civs = g.world.living_civs();
    let start_cities = g.world.cities.len();
    for civ in 1..8u8 {
        g.players[civ as usize] = Some(Box::new(AiV0::new(&g.rules, 3, civ)));
    }
    for _ in 0..100 {
        g.play_turn();
    }
    let log = g.log().to_string();
    for civ in &start_civs {
        assert!(
            log.contains(&format!("C {civ} found "))
                && log.lines().any(|l| l.starts_with("T ")
                    && l.contains(&format!(" C {civ} found "))
                    && l.contains("= founded")),
            "civ {civ} founded no city"
        );
    }
    assert!(g.world.cities.len() > start_cities + start_civs.len());
    let r =
        Game::replay("Ph_Auto2.SAV", &save, rules(), None, &log).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(r.world.checksum(), g.world.checksum());
}
