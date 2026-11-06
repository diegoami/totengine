//! Numeric policy comparison with independent order replay and generated
//! checkpoint continuation. Usage: INPUT(.SAV/.scn/Rules.txt) TURNS SEED DIFFICULTY POLICY.
use engine::{
    ai::{AiV1, BuiltInAi},
    campaign::{AiPolicy, Campaign},
    newgame::NewGameConfig,
    player::Game,
    WorldState,
};
use std::path::Path;
fn totals(w: &WorldState) -> [i64; 7] {
    let cities: Vec<_> = w.cities.iter().filter(|c| c.city_id != 0).collect();
    [
        cities.len() as i64,
        cities.iter().map(|c| i64::from(c.size)).sum(),
        w.units.iter().filter(|u| u.id != 0).count() as i64,
        w.research.iter().map(|r| i64::from(r.acquired)).sum(),
        cities
            .iter()
            .map(|c| {
                c.buildings
                    .iter()
                    .map(|b| i64::from(b.count_ones()))
                    .sum::<i64>()
            })
            .sum(),
        w.treasury.iter().map(|&t| i64::from(t)).sum(),
        w.living_civs().len() as i64,
    ]
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let path = Path::new(&args[1]);
    let turns: u32 = args[2].parse().unwrap();
    let seed: u32 = args[3].parse().unwrap();
    let difficulty: u8 = args[4].parse().unwrap();
    let policy = AiPolicy::parse(&args[5]).unwrap();
    let generated = path
        .extension()
        .is_some_and(|s| s.eq_ignore_ascii_case("txt"));
    let mut source = std::fs::read(path).unwrap();
    if !generated {
        let mut editor = tot_kit::SavEditor::new(&source).unwrap();
        editor.set_difficulty(difficulty);
        source = editor.into_bytes();
    }
    let rules_bytes = if generated {
        source.clone()
    } else {
        std::fs::read(path.parent().unwrap().join("Rules.txt")).unwrap()
    };
    let rules = tot_kit::parse_rules(&tot_kit::latin1(&rules_bytes));
    let config = NewGameConfig {
        gameplay_seed: seed,
        difficulty,
        world: engine::worldgen::WorldConfig {
            seed,
            ..Default::default()
        },
        ..Default::default()
    };
    let (initial, game, continuation) = if generated {
        let mut full = Campaign::with_policy(config, rules.clone(), policy).unwrap();
        let initial = totals(&full.game.world);
        full.advance(turns).unwrap();
        let mut prefix = Campaign::with_policy(config, rules.clone(), policy).unwrap();
        prefix.advance(turns / 2).unwrap();
        let mut resumed = Campaign::restore(rules.clone(), &prefix.checkpoint()).unwrap();
        resumed.advance(turns - turns / 2).unwrap();
        assert_eq!(resumed.checkpoint(), full.checkpoint());
        (initial, full.game, "true")
    } else {
        let mut game = Game::new("audit", &source, u64::from(seed), rules.clone(), None).unwrap();
        assert_eq!(game.world.difficulty, difficulty);
        let initial = totals(&game.world);
        for civ in 1..8 {
            game.players[civ] = Some(match policy {
                AiPolicy::AiV1 => Box::new(AiV1::new(&rules, u64::from(seed), civ as u8)),
                AiPolicy::BuiltIn1 => {
                    Box::new(BuiltInAi::new(&rules, u64::from(seed), civ as u8).unwrap())
                }
            });
        }
        for _ in 0..turns {
            game.play_turn();
        }
        (initial, game, "null")
    };
    let replay = if generated {
        Game::replay_generated(config, rules, game.log()).unwrap()
    } else {
        // The typed editor patched only the in-memory source difficulty.
        let saved = tot_kit::parse_sav(&source).unwrap();
        assert_eq!(
            saved.difficulty, difficulty,
            "saved cohort difficulty must match the source header"
        );
        Game::replay("audit", &source, rules, None, game.log()).unwrap()
    };
    assert_eq!(replay.world.checksum(), game.world.checksum());
    let count = |needle: &str| {
        game.log()
            .lines()
            .filter(|line| line.contains(needle))
            .count()
    };
    println!("{{\"policy\":\"{}\",\"generated\":{generated},\"seed\":{seed},\"difficulty\":{difficulty},\"requested_rounds\":{turns},\"rounds\":{},\"initial\":{:?},\"final\":{:?},\"attacks\":{},\"wins\":{},\"occupations\":{},\"founded\":{},\"research_orders\":{},\"rejected\":{},\"checksum\":\"{:016x}\",\"replay\":true,\"continuation\":{continuation}}}",
        policy.id(), game.log().lines().filter(|line| line.starts_with("R ")).count(), initial, totals(&game.world), count("= attack "), count("= attack won"), count("= occupied "), count("= founded "), count("= research-set"), count("= rejected "), game.world.checksum());
}
