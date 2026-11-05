//! Derived numeric campaign metrics. No original records/assets are emitted.
//! `ai_curve <save> [turns=50] [seed=3]` or `ai_curve --saved <directory>`.
use engine::{ai::AiV1, player::Game, WorldState};
use std::{env, fs, path::Path};

fn row(source: &str, w: &WorldState) {
    let scores = w.power_scores().expect("supported power settings");
    for (civ, score) in scores.iter().enumerate().skip(1) {
        let cities: Vec<_> = w
            .cities
            .iter()
            .filter(|c| c.city_id != 0 && c.owner & 15 == civ as u16)
            .collect();
        let population: u32 = cities.iter().map(|c| c.size as u32).sum();
        let units = w
            .units
            .iter()
            .filter(|u| u.id != 0 && u.owner as usize == civ)
            .count();
        let improvements: u32 = cities
            .iter()
            .map(|c| c.buildings.iter().map(|x| x.count_ones()).sum::<u32>())
            .sum();
        println!(
            "{source}\t{}\t{civ}\t{}\t{population}\t{units}\t{}\t{}\t{score}\t{improvements}",
            w.turn,
            cities.len(),
            w.research[civ].acquired,
            w.treasury[civ]
        );
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();
    println!(
        "source\tturn\tciv\tcities\tpopulation\tunits\tadvances\ttreasury\tpower\timprovements"
    );
    if args.get(1).is_some_and(|a| a == "--saved") {
        let dir = Path::new(&args[2]);
        let mut saved = std::collections::BTreeMap::new();
        for turn in 1..=26 {
            let path = dir.join(format!("turn{turn:02}.sav"));
            if !path.exists() {
                continue;
            }
            let save = tot_kit::parse_sav_file(&path).unwrap();
            let mut w = engine::boot(&save, 3);
            w.refresh_city_statistics();
            saved.insert(w.turn, w);
        }
        let save = tot_kit::parse_sav_file(&dir.join("final_Ph_Auto.SAV")).unwrap();
        let mut w = engine::boot(&save, 3);
        w.refresh_city_statistics();
        saved.insert(w.turn, w);
        for world in saved.values() {
            row("oracle", world);
        }
        return;
    }
    let path = Path::new(&args[1]);
    let turns: usize = args.get(2).map_or(50, |n| n.parse().unwrap());
    let seed: u64 = args.get(3).map_or(3, |n| n.parse().unwrap());
    let nearby = path.parent().unwrap().join("Rules.txt");
    let rules_path = if nearby.exists() {
        nearby
    } else {
        Path::new(&env::var("CIVTOT_HOME").unwrap()).join("Original/Rules.txt")
    };
    let rules = tot_kit::parse_rules(&tot_kit::latin1(&fs::read(rules_path).unwrap()));
    let name = path.file_name().unwrap().to_str().unwrap();
    let mut g = Game::new(name, &fs::read(path).unwrap(), seed, rules, None).unwrap();
    for civ in 1..8 {
        g.players[civ] = Some(Box::new(AiV1::new(&g.rules, seed, civ as u8)));
    }
    g.world.refresh_city_statistics();
    row("ai-v1", &g.world);
    for _ in 0..turns {
        g.play_turn();
        row("ai-v1", &g.world);
    }
    if let Ok(log) = env::var("ENGINE_CURVE_LOG") {
        fs::write(log, g.log()).unwrap();
    }
}
