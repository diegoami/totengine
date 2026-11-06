use engine::campaign::{AiPolicy, Campaign};
use engine::newgame::NewGameConfig;
use engine::player::{Game, Player};
fn rules() -> tot_kit::Rules {
    tot_kit::parse_rules(&format!(
        "{}\n{}",
        include_str!("fixtures/campaign-rules.txt"),
        include_str!("fixtures/builtin-settings.txt")
    ))
}
#[test]
fn builtin_campaign_restores_policy_private_rng_and_order_replay() {
    let config = NewGameConfig {
        gameplay_seed: 7,
        world: engine::worldgen::WorldConfig {
            seed: 7,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut full = Campaign::with_policy(config, rules(), AiPolicy::BuiltIn1).unwrap();
    full.advance(100).unwrap();
    assert!(full.game.log().contains("research "));
    assert!(full.game.log().contains("= founded"));
    let replay = Game::replay_generated(config, rules(), full.game.log()).unwrap();
    assert_eq!(replay.world.checksum(), full.game.world.checksum());
    let mut prefix = Campaign::with_policy(config, rules(), AiPolicy::BuiltIn1).unwrap();
    prefix.advance(37).unwrap();
    let saved = prefix.checkpoint();
    assert!(saved.starts_with("totengine-campaign 2\n"));
    assert!(saved.contains("\npolicy builtin-1\n"));
    let mut resumed = Campaign::restore(rules(), &saved).unwrap();
    assert_eq!(resumed.policy(), AiPolicy::BuiltIn1);
    resumed.advance(63).unwrap();
    assert_eq!(resumed.checkpoint(), full.checkpoint());
    assert!(Campaign::restore(
        rules(),
        &saved.replace("policy builtin-1", "policy builtin-2")
    )
    .is_err());
    assert!(
        Campaign::restore(rules(), &saved.replace("policy builtin-1", "policy ai-v1")).is_err()
    );
}
#[test]
fn builtin_research_cannot_read_foreign_knowledge_or_caches() {
    let rules = rules();
    let world = engine::newgame::generate(Default::default(), &rules).unwrap();
    let mut altered = world.clone();
    altered.techs[2] = [255; 13];
    altered.research[2].acquired = 90;
    altered.ai_state.reputation[2] = 9;
    altered.ai_state.city_landmasses[2] = [255; 256];
    altered.diplomacy[1][2] = 8192;
    let a = world.observe(&rules, 1, None);
    let b = altered.observe(&rules, 1, None);
    assert_eq!(a, b);
    let mut first = engine::ai::BuiltInAi::new(&rules, 19, 1).unwrap();
    let mut second = engine::ai::BuiltInAi::new(&rules, 19, 1).unwrap();
    assert_eq!(first.act(&a), second.act(&b));
}

fn tactical_observation() -> (tot_kit::Rules, engine::player::Observation) {
    use engine::player::{TileView, UnitView};
    let mut rules = rules();
    rules.sections.get_mut("UNITS").unwrap().rows[1][5] = "6a".into();
    let world = engine::newgame::generate(Default::default(), &rules).unwrap();
    let mut obs = world.observe(&rules, 1, None);
    obs.w2 = 14;
    obs.h = 10;
    obs.seen = vec![
        Some(TileView {
            terrain: 1,
            river: false,
            special: false,
            road: false,
            rail: false
        });
        70
    ];
    obs.research.target = Some(0);
    obs.research.available = vec![0];
    obs.units = vec![UnitView {
        index: 0,
        type_id: 1,
        owner: 1,
        x: 4,
        y: 4,
        moves_left: 3,
        veteran: false,
    }];
    obs.cities.clear();
    obs.city_economy.clear();
    (rules, obs)
}
#[test]
fn field_army_routes_to_and_attacks_a_visible_city_but_keeps_a_garrison() {
    use engine::player::{CityView, Order, OrderResult, UnitView};
    let (rules, mut obs) = tactical_observation();
    let city = CityView {
        index: 0,
        name: "Target".into(),
        owner: 2,
        x: 8,
        y: 4,
        size: 1,
        building: None,
        shield_box: None,
        output: None,
    };
    obs.cities.push(city.clone());
    obs.units.push(UnitView {
        index: 1,
        type_id: 2,
        owner: 2,
        x: 8,
        y: 4,
        moves_left: 0,
        veteran: false,
    });
    let mut ai = engine::ai::BuiltInAi::new(&rules, 7, 1).unwrap();
    assert_eq!(
        ai.act(&obs),
        Order::Move {
            unit: 0,
            to: (6, 4)
        }
    );
    obs.units[0].x = 6;
    obs.last_result = Some(OrderResult::Moved);
    assert_eq!(
        ai.act(&obs),
        Order::Move {
            unit: 0,
            to: (8, 4)
        }
    );
    let mut guarded = obs.clone();
    guarded.cities[0] = CityView {
        index: 1,
        owner: 1,
        x: 6,
        y: 4,
        ..city
    };
    let mut keeper = engine::ai::BuiltInAi::new(&rules, 7, 1).unwrap();
    assert_eq!(keeper.act(&guarded), Order::EndTurn);
}
#[test]
fn settler_keeps_its_route_after_deferring_a_poor_founding_site() {
    use engine::player::{Order, OrderResult};
    let (rules, mut obs) = tactical_observation();
    obs.units[0].type_id = 0;
    for tile in obs.seen.iter_mut().flatten() {
        tile.terrain = 0;
    }
    obs.seen[4 * 7 + 2].as_mut().unwrap().terrain = 4;
    obs.seen[4 * 7 + 4].as_mut().unwrap().terrain = 2;
    let mut ai = engine::ai::BuiltInAi::new(&rules, 7, 1).unwrap();
    assert_eq!(
        ai.act(&obs),
        Order::Move {
            unit: 0,
            to: (6, 4)
        }
    );
    obs.units[0].x = 6;
    obs.last_result = Some(OrderResult::Moved);
    assert_eq!(
        ai.act(&obs),
        Order::Move {
            unit: 0,
            to: (8, 4)
        }
    );
    obs.units[0].x = 8;
    obs.last_result = Some(OrderResult::Moved);
    assert_eq!(ai.act(&obs), Order::FoundCity { unit: 0 });
}

#[test]
fn army_occupies_a_cleared_city_and_transfers_its_economy() {
    use engine::player::{Order, OrderResult};
    let rules = rules();
    let mut w = engine::newgame::generate(Default::default(), &rules).unwrap();
    w.map.terrain.fill(1);
    w.map.visible.fill(255);
    let mut city = engine::boot(&engine::tests_support::synthetic_save(), 1).cities[0].clone();
    city.map_id = 0;
    city.city_id = 1;
    city.owner = 2;
    city.original_owner = 3;
    city.x = 8;
    city.y = 4;
    city.size = 3;
    city.buildings = [255; 5];
    city.shield_box = 30;
    let mut refuge = city.clone();
    refuge.city_id = 2;
    refuge.x = 12;
    refuge.size = 4;
    w.cities = vec![city, refuge];
    let mut army = w.units[0].clone();
    army.id = 1;
    army.type_id = 1;
    army.owner = 1;
    army.x = 6;
    army.y = 4;
    army.moves_left = 3;
    army.home = 255;
    let mut defender = army.clone();
    defender.id = 2;
    defender.owner = 2;
    defender.x = 8;
    defender.home = 0;
    let mut displaced = defender.clone();
    displaced.id = 3;
    displaced.type_id = 2;
    displaced.x = 12;
    w.units = vec![army, defender, displaced];
    w.treasury[2] = 1000;
    w.treasury[1] = 20;
    w.refresh_city_statistics();
    let before = w.checksum();
    let occupy = Order::OccupyCity { unit: 0, city: 0 };
    assert_eq!(
        w.apply_order(&rules, None, 1, occupy),
        OrderResult::Rejected("city defended")
    );
    assert_eq!(
        w.checksum(),
        before,
        "rejected occupation must not spend moves/RNG"
    );
    assert_eq!(
        w.apply_order(&rules, None, 2, occupy),
        OrderResult::Rejected("not your unit")
    );
    assert_eq!(w.checksum(), before);
    w.units[1].id = 0; // the preceding battle has cleared the target
    let mut obs = w.observe(&rules, 1, None);
    obs.research.target = Some(0);
    obs.research.available = vec![0];
    let mut ai = engine::ai::BuiltInAi::new(&rules, 7, 1).unwrap();
    assert_eq!(ai.act(&obs), occupy);
    assert_eq!(
        w.apply_order(&rules, None, 1, occupy),
        OrderResult::Occupied {
            city: 0,
            razed: false
        }
    );
    assert_eq!((w.units[0].x, w.units[0].y), (8, 4));
    assert_eq!(
        (
            w.cities[0].owner,
            w.cities[0].original_owner,
            w.cities[0].size
        ),
        (1, 2, 2)
    );
    assert_eq!(w.cities[0].shield_box, 0);
    assert!([1, 4, 7, 11]
        .into_iter()
        .all(|i| !w.cities[0].has_building(i)));
    assert_eq!(w.treasury[1], 395);
    assert_eq!(w.treasury[2], 625);
    assert_eq!((w.city_counts[1], w.city_counts[2]), (1, 1));
    assert_eq!(
        w.units[2].home, 1,
        "lone defender at another city is rehomed"
    );
    assert!(w.units[2].id != 0);
    let tile = w.map.stored_index(8, 4).unwrap();
    assert_eq!(w.map.owner[tile], 1);
    assert_eq!(
        w.observe(&rules, 1, None)
            .cities
            .iter()
            .find(|c| c.index == 0)
            .unwrap()
            .owner,
        1
    );
}

#[test]
fn size_one_occupation_destroys_city_routes_and_wonders_without_killing_attacker() {
    use engine::player::{Order, OrderResult};
    let rules = rules();
    let mut w = engine::newgame::generate(Default::default(), &rules).unwrap();
    w.map.terrain.fill(1);
    w.human_mask = 2;
    let mut city = engine::boot(&engine::tests_support::synthetic_save(), 1).cities[0].clone();
    city.map_id = 0;
    city.city_id = 1;
    city.owner = 2;
    city.original_owner = 2;
    city.x = 8;
    city.y = 4;
    city.size = 1;
    let mut other = city.clone();
    other.city_id = 2;
    other.x = 12;
    other.owner = 1;
    other.route_count = 1;
    other.route_partners[0] = 0;
    w.cities = vec![city, other];
    w.wonders[14] = 0;
    let mut army = w.units[0].clone();
    army.type_id = 1;
    army.owner = 1;
    army.x = 6;
    army.y = 4;
    army.moves_left = 3;
    army.home = 255;
    w.units = vec![army];
    assert_eq!(
        w.apply_order(&rules, None, 1, Order::OccupyCity { unit: 0, city: 0 }),
        OrderResult::Occupied {
            city: 0,
            razed: true
        }
    );
    assert_eq!(w.cities[0].city_id, 0);
    assert_eq!(w.cities[1].route_count, 0);
    assert_eq!(w.wonders[14], -2);
    assert!(w.units[0].id != 0);
    assert_eq!((w.units[0].x, w.units[0].y), (8, 4));
    assert_eq!((w.city_counts[1], w.city_counts[2]), (1, 0));
}
