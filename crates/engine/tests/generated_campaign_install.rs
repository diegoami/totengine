//! Fresh generated games with install Rules; no reference save input.
use engine::{campaign::Campaign, newgame::NewGameConfig, player::Game};

#[test]
fn original_rules_generated_campaign_expands_replays_and_resumes() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        return;
    };
    let bytes = std::fs::read(std::path::PathBuf::from(home).join("Original/Rules.txt")).unwrap();
    let rules = || tot_kit::parse_rules(&tot_kit::latin1(&bytes));
    let config = NewGameConfig {
        gameplay_seed: 7,
        world: engine::worldgen::WorldConfig {
            seed: 7,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut full = Campaign::new(config, rules()).unwrap();
    assert!(full.game.world.cities.is_empty());
    assert_eq!(full.game.world.research_settings.advance_count, 89);
    full.advance(100).unwrap();
    assert!(
        full.game
            .world
            .cities
            .iter()
            .filter(|c| c.city_id != 0)
            .count()
            > 5
    );
    assert!(
        full.game
            .world
            .research
            .iter()
            .map(|r| r.acquired as u32)
            .sum::<u32>()
            > 0
    );
    let replay = Game::replay_generated(config, rules(), full.game.log()).unwrap();
    assert_eq!(replay.world.checksum(), full.game.world.checksum());
    let mut prefix = Campaign::new(config, rules()).unwrap();
    prefix.advance(37).unwrap();
    let mut restored = Campaign::restore(rules(), &prefix.checkpoint()).unwrap();
    restored.advance(63).unwrap();
    assert_eq!(restored.checkpoint(), full.checkpoint());
}
