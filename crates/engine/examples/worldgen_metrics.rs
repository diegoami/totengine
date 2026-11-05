//! Derived terrain/river statistics for generated worlds and original saves.
//! Usage: worldgen_metrics [--seeds N] [label=path.sav ...]; numeric data only.
use engine::worldgen::{generate, WorldConfig, ALGORITHM_VERSION};

fn neighbors(width: usize, height: usize, i: usize) -> impl Iterator<Item = usize> {
    let stride = width / 2;
    let y = (i / stride) as i32;
    let x = (2 * (i % stride)) as i32 + (y & 1);
    [
        (1, -1),
        (2, 0),
        (1, 1),
        (0, 2),
        (-1, 1),
        (-2, 0),
        (-1, -1),
        (0, -2),
    ]
    .into_iter()
    .filter_map(move |(dx, dy)| {
        let ny = y + dy;
        (ny >= 0 && ny < height as i32)
            .then(|| ny as usize * stride + (x + dx).rem_euclid(width as i32) as usize / 2)
    })
}

fn report(label: &str, width: usize, height: usize, terrain: &[u8], rivers: usize, spatial: bool) {
    let mut hist = [0usize; 11];
    for &t in terrain {
        hist[t as usize] += 1;
    }
    print!("{label}\t{width}\t{height}\t{}\t{rivers}", terrain.len());
    for n in hist {
        print!("\t{n}");
    }
    if spatial {
        let mut edges = 0;
        let mut coast = 0;
        let mut same = 0;
        let mut seen = vec![false; terrain.len()];
        let mut components = 0;
        let mut largest = 0;
        for i in 0..terrain.len() {
            for j in neighbors(width, height, i) {
                edges += 1;
                coast += usize::from((terrain[i] == 10) != (terrain[j] == 10));
                same += usize::from(terrain[i] == terrain[j]);
            }
            if terrain[i] == 10 || seen[i] {
                continue;
            }
            components += 1;
            let mut stack = vec![i];
            seen[i] = true;
            let mut size = 0;
            while let Some(at) = stack.pop() {
                size += 1;
                for j in neighbors(width, height, at) {
                    if terrain[j] != 10 && !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
            largest = largest.max(size);
        }
        print!("\t{edges}\t{coast}\t{same}\t{largest}\t{components}");
    }
    println!();
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let spatial = args.iter().any(|a| a == "--spatial");
    print!("source\tlogical_width\theight\ttiles\trivers\tdesert\tplains\tgrassland\tforest\thills\tmountains\ttundra\tglacier\tswamp\tjungle\tocean");
    if spatial {
        print!("\tneighbor_edges\tcoast_edges\tsame_terrain_edges\tlargest_land_tiles\tland_components");
    }
    println!();
    let mut args = args.into_iter();
    let mut seeds = 16;
    while let Some(arg) = args.next() {
        if arg == "--spatial" {
            continue;
        }
        if arg == "--seeds" {
            seeds = args.next().expect("--seeds N").parse::<u32>().unwrap();
            assert!((1..=10000).contains(&seeds));
            continue;
        }
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
            spatial,
        );
    }
    for seed in 0..seeds {
        let config = WorldConfig {
            seed,
            ..WorldConfig::default()
        };
        let w = generate(config).unwrap();
        report(
            &format!("engine-v{ALGORITHM_VERSION}-{seed}"),
            config.width as usize,
            config.height as usize,
            &w.terrain,
            w.river.iter().filter(|&&r| r).count(),
            spatial,
        );
    }
}
