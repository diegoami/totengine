//! RG-8 acceptance on real data (env-gated on CIVTOT_HOME): all-random
//! play on FallRome (with its Events.txt) and on a standard-game save
//! replays bit-exact from save + seed + order log (decision 0003).

use engine::events::EventEngine;
use engine::player::{Game, RandomPlayer};

fn read(rel: &str) -> Option<Vec<u8>> {
    let home = std::path::PathBuf::from(std::env::var_os("CIVTOT_HOME")?);
    std::fs::read(home.join(rel)).ok()
}

fn play_and_replay(save_rel: &str, rules_rel: &str, events_rel: Option<&str>, turns: usize) {
    let (Some(save), Some(rules_txt)) = (read(save_rel), read(rules_rel)) else {
        eprintln!("skipping: {save_rel} not available");
        return;
    };
    let rules = || tot_kit::parse_rules(&tot_kit::latin1(&rules_txt));
    let events = || {
        events_rel
            .and_then(read)
            .map(|t| EventEngine::new(tot_kit::parse_events(&tot_kit::latin1(&t)).unwrap()))
    };
    let mut g = Game::new(save_rel, &save, 7, rules(), events()).unwrap();
    for civ in 1..8 {
        g.players[civ] = Some(Box::new(RandomPlayer::new(7, civ as u8)));
    }
    for _ in 0..turns {
        g.play_turn();
    }
    let log = g.log().to_string();
    assert!(
        log.contains("= moved") || log.contains("= attack"),
        "{save_rel}: units acted"
    );
    let r =
        Game::replay(save_rel, &save, rules(), events(), &log).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(r.world.checksum(), g.world.checksum(), "{save_rel}");
}

#[test]
fn fallrome_random_play_replays() {
    play_and_replay(
        "FallRome/ROMFALL.scn",
        "FallRome/Rules.txt",
        Some("FallRome/Events.txt"),
        8,
    );
}

#[test]
fn standard_game_random_play_replays() {
    play_and_replay("Original/Ph_Auto2.SAV", "Original/Rules.txt", None, 15);
}
