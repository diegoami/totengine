//! Numeric trajectory of a fresh campaign; Rules are supplied by the caller.
use engine::{campaign::Campaign, newgame::NewGameConfig};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let bytes = std::fs::read(&args[1]).unwrap();
    let seed = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(7);
    let turns = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(100);
    let config = NewGameConfig {
        gameplay_seed: seed,
        world: engine::worldgen::WorldConfig {
            seed,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut campaign =
        Campaign::new(config, tot_kit::parse_rules(&tot_kit::latin1(&bytes))).unwrap();
    println!("round\tcities\tpopulation\tunits\tadvances\timprovements\ttreasury\tfingerprint");
    for round in 0..=turns {
        let w = &campaign.game.world;
        let cities: Vec<_> = w.cities.iter().filter(|c| c.city_id != 0).collect();
        println!(
            "{round}\t{}\t{}\t{}\t{}\t{}\t{}\t{:016x}",
            cities.len(),
            cities.iter().map(|c| c.size as usize).sum::<usize>(),
            w.units.iter().filter(|u| u.id != 0).count(),
            w.research
                .iter()
                .map(|r| r.acquired as usize)
                .sum::<usize>(),
            cities
                .iter()
                .map(|c| c.buildings.iter().map(|b| b.count_ones()).sum::<u32>())
                .sum::<u32>(),
            w.treasury.iter().map(|&t| t as i64).sum::<i64>(),
            w.checksum()
        );
        if round != turns {
            campaign.advance(1).unwrap();
        }
    }
    if std::env::var_os("CAMPAIGN_DEBUG_CITIES").is_some() {
        for city in &campaign.game.world.cities {
            eprintln!("{city:?}");
        }
    }
}
