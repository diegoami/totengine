//! Derived terrain/river statistics for generated worlds and original saves.
//! Usage: worldgen_metrics [label=path.sav ...]; no original records emitted.
use engine::worldgen::{generate, WorldConfig};

fn report(label: &str, width: usize, height: usize, terrain: &[u8], rivers: usize) {
    let mut hist = [0usize; 11];
    for &t in terrain {
        hist[t as usize] += 1;
    }
    print!("{label}\t{width}\t{height}\t{}\t{rivers}", terrain.len());
    for n in hist {
        print!("\t{n}");
    }
    println!();
}

fn main() {
    println!("source\tlogical_width\theight\ttiles\trivers\tdesert\tplains\tgrassland\tforest\thills\tmountains\ttundra\tglacier\tswamp\tjungle\tocean");
    for arg in std::env::args().skip(1) {
        let (label, path) = arg.split_once('=').expect("expected label=path.sav");
        let save = tot_kit::parse_sav_file(std::path::Path::new(path)).unwrap();
        let map = &save.maps[0];
        let terrain: Vec<_> = (0..(map.tile_fields.len() / 6))
            .map(|i| map.terrain(i))
            .collect();
        report(
            label,
            save.map_header.raw[0] as usize,
            save.map_header.raw[1] as usize,
            &terrain,
            (0..(map.tile_fields.len() / 6))
                .filter(|&i| map.is_river(i))
                .count(),
        );
    }
    for seed in 0..16 {
        let config = WorldConfig {
            seed,
            ..WorldConfig::default()
        };
        let w = generate(config).unwrap();
        report(
            &format!("engine-v1-{seed}"),
            config.width as usize,
            config.height as usize,
            &w.terrain,
            w.river.iter().filter(|&&r| r).count(),
        );
    }
}
