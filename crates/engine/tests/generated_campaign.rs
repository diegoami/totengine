use engine::newgame::NewGameConfig;
use engine::player::Game;

const RULES: &str = include_str!("fixtures/campaign-rules.txt");

fn configured(seed: u32) -> NewGameConfig {
    NewGameConfig {
        world: engine::worldgen::WorldConfig {
            seed,
            ..Default::default()
        },
        gameplay_seed: seed,
        ..Default::default()
    }
}

#[test]
fn fresh_world_has_only_valid_starting_units_and_no_saved_caches() {
    let rules = tot_kit::parse_rules(RULES);
    for seed in [0, 1, 7, 31, u32::MAX] {
        let config = configured(seed);
        let w = engine::newgame::generate(config, &rules).unwrap();
        assert!(w.cities.is_empty());
        assert_eq!(w.units.len(), 5);
        assert_eq!(w.living_civs(), vec![1, 2, 3, 4, 5]);
        assert_eq!(
            w.checksum(),
            engine::newgame::generate(config, &rules)
                .unwrap()
                .checksum()
        );
        assert!(w.map.visible.iter().all(|&v| v == 0));
        for u in &w.units {
            assert_eq!((u.x + u.y) % 2, 0);
            assert_ne!(w.map.terrain_at(u.x, u.y), Some(10));
            assert_eq!(u.home, 255);
            let at = w.map.stored_index(u.x, u.y).unwrap();
            assert_eq!(w.map.owner[at], u.owner);
            assert_eq!(w.map.improvements[at], 1);
        }
        for (i, &terrain) in w.map.terrain.iter().enumerate() {
            let region = w.map.landmass[i] as usize;
            assert!((1..255).contains(&region));
            assert_eq!(w.map.landmass_values[region] > 0, terrain == 10);
        }
    }
}

#[test]
fn generated_ai_campaign_founds_cities_and_replays() {
    let config = configured(7);
    let run = || {
        let mut game = Game::generated(config, tot_kit::parse_rules(RULES)).unwrap();
        for civ in 1..=config.civilizations {
            game.players[civ as usize] = Some(Box::new(engine::ai::AiV1::new(
                &game.rules,
                config.gameplay_seed as u64,
                civ,
            )));
        }
        for _ in 0..30 {
            game.play_turn();
        }
        game
    };
    let game = run();
    assert!(game.world.cities.iter().filter(|c| c.city_id != 0).count() >= 5);
    assert!(game.log().contains("= founded"));
    assert_eq!(game.log(), run().log());
    let replay = Game::replay_generated(config, tot_kit::parse_rules(RULES), game.log()).unwrap();
    assert_eq!(game.world.checksum(), replay.world.checksum());
    let changed = RULES.replace("3 ; road", "4 ; road") + "\n; different rules identity\n";
    assert!(Game::replay_generated(config, tot_kit::parse_rules(&changed), game.log()).is_err());
    assert!(
        Game::replay_generated(configured(8), tot_kit::parse_rules(RULES), game.log()).is_err()
    );
}

#[test]
fn invalid_setup_and_missing_settlers_fail_before_boot() {
    let rules = tot_kit::parse_rules(RULES);
    assert!(engine::newgame::generate(
        NewGameConfig {
            civilizations: 8,
            ..Default::default()
        },
        &rules
    )
    .is_err());
    assert!(engine::newgame::generate(
        NewGameConfig {
            difficulty: 6,
            ..Default::default()
        },
        &rules
    )
    .is_err());
    assert!(engine::newgame::generate(Default::default(), &tot_kit::Rules::default()).is_err());
    let no_settler = tot_kit::parse_rules(&RULES.replace(", 0, 5, nil,", ", 0, 0, nil,"));
    assert!(engine::newgame::generate(Default::default(), &no_settler).is_err());
}

#[test]
fn trailing_unit_annotation_does_not_exceed_classic_capacity() {
    let annotated = RULES.replace("@IMPROVE", ": Authored trailing unit annotation\n@IMPROVE");
    let rules = tot_kit::parse_rules(&annotated);
    assert_eq!(rules.unit_names().len(), 81);
    let annotated_world = engine::newgame::generate(configured(7), &rules).unwrap();
    let ordinary_world =
        engine::newgame::generate(configured(7), &tot_kit::parse_rules(RULES)).unwrap();
    assert_eq!(annotated_world.checksum(), ordinary_world.checksum());
}

#[test]
fn checkpoint_resume_restores_ai_state_and_rejects_tampering() {
    use engine::campaign::Campaign;
    let rules = || tot_kit::parse_rules(RULES);
    let config = configured(7);
    let mut uninterrupted = Campaign::new(config, rules()).unwrap();
    uninterrupted.advance(40).unwrap();
    let mut split = Campaign::new(config, rules()).unwrap();
    split.advance(17).unwrap();
    let checkpoint = split.checkpoint();
    let mut restored = Campaign::restore(rules(), &checkpoint).unwrap();
    restored.advance(23).unwrap();
    assert_eq!(restored.game.log(), uninterrupted.game.log());
    assert_eq!(restored.checkpoint(), uninterrupted.checkpoint());
    assert_eq!(restored.rounds(), 40);
    assert!(Campaign::restore(rules(), &checkpoint.replace("rounds 17", "rounds 18")).is_err());
    assert!(Campaign::restore(rules(), &checkpoint.replace("= founded", "= altered")).is_err());
    assert!(Campaign::restore(
        rules(),
        &checkpoint.replace("totengine-campaign 1", "totengine-campaign 2")
    )
    .is_err());
    assert!(Campaign::restore(
        tot_kit::parse_rules(&(RULES.to_string() + "\n; other Rules\n")),
        &checkpoint
    )
    .is_err());
}

#[test]
fn hundred_round_generated_campaign_matches_the_full_checkpoint_golden() {
    let mut campaign =
        engine::campaign::Campaign::new(configured(7), tot_kit::parse_rules(RULES)).unwrap();
    campaign.advance(100).unwrap();
    assert_eq!(
        campaign.checkpoint(),
        include_str!("../../../docs/evidence/generated-campaign-v1.ctc")
    );
    assert!(
        campaign
            .game
            .world
            .cities
            .iter()
            .filter(|c| c.city_id != 0)
            .count()
            > 5
    );
    assert!(
        campaign
            .game
            .world
            .research
            .iter()
            .map(|r| r.acquired as u32)
            .sum::<u32>()
            > 0
    );
}

#[test]
#[ignore = "nightly multi-seed autoplay"]
fn nightly_generated_campaigns_resume_and_replay_across_seeds() {
    use engine::campaign::Campaign;
    for seed in 0..32 {
        let config = configured(seed);
        let rules = || tot_kit::parse_rules(RULES);
        let mut uninterrupted = Campaign::new(config, rules()).unwrap();
        uninterrupted.advance(100).unwrap();
        let mut split = Campaign::new(config, rules()).unwrap();
        split.advance(37).unwrap();
        let mut restored = Campaign::restore(rules(), &split.checkpoint()).unwrap();
        restored.advance(63).unwrap();
        assert_eq!(
            uninterrupted.checkpoint(),
            restored.checkpoint(),
            "resume seed {seed}"
        );
        let replay = Game::replay_generated(config, rules(), uninterrupted.game.log()).unwrap();
        assert_eq!(
            uninterrupted.game.world.checksum(),
            replay.world.checksum(),
            "replay seed {seed}"
        );
        assert!(
            uninterrupted.game.log().contains("= founded"),
            "seed {seed}"
        );
    }
}
