//! Per-civ research against real files; self-skips without CIVTOT_HOME.
use engine::player::Game;
use std::path::PathBuf;

fn home() -> Option<PathBuf> {
    std::env::var_os("CIVTOT_HOME").map(PathBuf::from)
}

#[test]
fn fallrome_costs_match_original_x86_and_scenario_tail() {
    let Some(home) = home() else {
        eprintln!("skipping: CIVTOT_HOME not set");
        return;
    };
    let save = tot_kit::parse_sav_file(&home.join("FallRome/ROMFALL.scn")).unwrap();
    assert_eq!(save.research_settings.scenario, Some((100, -5)));
    assert_eq!(save.research_settings.flags, 0xd0);
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(home.join("FallRome/Rules.txt")).unwrap(),
    ));
    let world = engine::boot(&save, 3);
    // Original bytes executed by tools/research_check.py --emulate;
    // docs/evidence/research.json records the source binary's hash.
    let costs = [11025, 4025, 3006, 3675, 3850, 7700, 2128];
    for (civ, cost) in (1..8).zip(costs) {
        assert_eq!(world.research_cost(&rules, civ), Some(cost));
    }
}

#[test]
fn fifty_turn_research_ai_discovers_and_replays_on_a_standard_save() {
    let Some(home) = home() else {
        eprintln!("skipping: CIVTOT_HOME not set");
        return;
    };
    let save = std::fs::read(home.join("Original/Ph_Auto2.SAV")).unwrap();
    let text = tot_kit::latin1(&std::fs::read(home.join("Original/Rules.txt")).unwrap());
    let mut g = Game::new("Ph_Auto2.SAV", &save, 3, tot_kit::parse_rules(&text), None).unwrap();
    let start = g.world.techs;
    for civ in 1..8 {
        g.players[civ] = Some(Box::new(engine::ai::AiV1::new(&g.rules, 3, civ as u8)));
    }
    for _ in 0..50 {
        g.play_turn();
    }
    assert!(g.log().contains(" research "));
    let gained: u32 = g
        .world
        .techs
        .iter()
        .zip(start.iter())
        .flat_map(|(a, b)| a.iter().zip(b.iter()))
        .map(|(a, b)| (a & !b).count_ones())
        .sum();
    assert!(gained > 0, "all-AI science must produce new advances");
    let replay = Game::replay(
        "Ph_Auto2.SAV",
        &save,
        tot_kit::parse_rules(&text),
        None,
        g.log(),
    )
    .unwrap();
    assert_eq!(replay.world.checksum(), g.world.checksum());
    assert_eq!(replay.world.research, g.world.research);
    eprintln!("50 turns: {gained} new advances; research and full checksum replay exactly");
}
