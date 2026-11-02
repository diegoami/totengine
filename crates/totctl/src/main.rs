//! `totctl` — inspect Civilization II: Test of Time game data.
//!
//! Rust successor of `tools/totctl.py` (the Python probe stays as the fast
//! reverse-engineering loop; this crate absorbs features as they stabilize).

use std::process::ExitCode;

use engine::rng::GameRng as _;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let prog = args.first().map(String::as_str).unwrap_or("totctl");
    match args.get(1).map(String::as_str) {
        Some("mp") => cmd_mp(&args[2..]),
        Some("rules") => cmd_rules(&args[2..]),
        Some("boot") => cmd_boot(&args[2..]),
        Some("--version") | Some("-V") => {
            println!("totctl {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!(
                "usage: {prog} mp <file.mp>   — decode an .MP map header and tiles\n\
                        {prog} rules <Rules.txt>   — summarize Rules.txt sections\n\
                        {prog} boot <file.sav|scn> — headless boot into engine state + checksum\n\
                        {prog} --version"
            );
            ExitCode::from(2)
        }
    }
}

/// Headless boot of a scenario/save into the engine world state (T1-2):
/// parse → `engine::boot` → state summary + reproducible checksum.
/// `--turns N` advances the kernel N times before checksumming.
fn cmd_boot(args: &[String]) -> ExitCode {
    let mut path: Option<&String> = None;
    let mut turns: usize = 0;
    let mut seed: u64 = 1;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--turns" => match it.next().and_then(|v| v.parse().ok()) {
                Some(n) => turns = n,
                None => {
                    eprintln!("error: --turns needs a number");
                    return ExitCode::from(2);
                }
            },
            "--seed" => match it.next().and_then(|v| v.parse().ok()) {
                Some(s) => seed = s,
                None => {
                    eprintln!("error: --seed needs a number");
                    return ExitCode::from(2);
                }
            },
            _ if path.is_none() => path = Some(a),
            _ => {
                eprintln!("error: unexpected argument {a}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(path) = path else {
        eprintln!("usage: totctl boot <file.sav|scn> [--turns N] [--seed S]");
        return ExitCode::from(2);
    };
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: read {path}: {e}");
            return ExitCode::from(1);
        }
    };
    let sav = match tot_kit::parse_sav(&bytes) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    let mut world = engine::boot(&sav, seed);
    for _ in 0..turns {
        world.advance_turn();
    }
    println!("file          : {path}");
    println!("version       : 0x{:04x}", sav.version);
    println!("turn / year   : {} / {}", world.turn, world.year);
    println!(
        "difficulty    : {}  rivals: {}",
        world.difficulty, world.rivals
    );
    let named: Vec<&engine::Civ> = world.civs.iter().filter(|c| !c.names.is_empty()).collect();
    println!("civs (named)  : {}", named.len());
    for c in &named {
        println!(
            "  owner {} : {:<28} units {}",
            c.id,
            c.names.join(" / "),
            c.unit_count
        );
    }
    println!(
        "map           : {}x{} stored ({} tiles), blocks {}",
        world.map.stored_w,
        world.map.stored_h,
        world.map.terrain.len(),
        sav.maps.len()
    );
    let hist = sav.maps[0].terrain_histogram();
    let mut top: Vec<(usize, &usize)> = hist.iter().enumerate().collect();
    top.sort_by_key(|(_, &n)| std::cmp::Reverse(n));
    let top_s: Vec<String> = top
        .iter()
        .take(6)
        .map(|(id, n)| format!("{}={}", id, n))
        .collect();
    println!("terrain hist  : {}", top_s.join(" "));
    println!(
        "units / cities: {} / {}",
        world.units.len(),
        world.cities.len()
    );
    println!(
        "rng seed      : {} (draws so far: {})",
        seed,
        world.rng.draws()
    );
    println!("checksum      : {:016x}", world.checksum());
    ExitCode::SUCCESS
}

fn cmd_rules(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("usage: totctl rules <Rules.txt>");
        return ExitCode::from(2);
    };
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: read {path}: {e}");
            return ExitCode::from(1);
        }
    };
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let rules = tot_kit::parse_rules(&text);
    println!("sections      : {}", rules.section_order().join(","));
    println!("row counts    : {}", {
        let mut s = String::new();
        for (n, c) in rules.row_counts() {
            s.push_str(&format!("{n}={c} "));
        }
        s.trim_end().to_string()
    });
    println!("semantic ok   : {}", rules.terrain_semantic_ok());
    println!(
        "terrain 0/10  : {} / {}",
        {
            let n = rules.base_terrain_names();
            n.first().copied().unwrap_or("?")
        },
        {
            let n = rules.base_terrain_names();
            n.get(10).copied().unwrap_or("?")
        }
    );
    println!("units         : {}", rules.unit_names().len());
    let domains = {
        let mut counts = [0usize; 3];
        for i in 0..rules.unit_names().len() {
            if let Some(d) = rules.unit_domain(i) {
                counts[(d as usize).min(2)] += 1;
            }
        }
        format!("{}/{}/{}", counts[0], counts[1], counts[2])
    };
    println!("unit domains  : {domains}");
    println!("governments   : {}", rules.government_names().join(", "));
    println!("caravan       : {}", rules.caravan_commodities().len());
    println!("transport rel : {:?}", rules.transport_relationships());
    let road = rules
        .cosmic()
        .into_iter()
        .find(|(k, _)| k.contains("Road movement multiplier"))
        .map(|(_, v)| v.to_string());
    println!("cosmic road   : {}", road.unwrap_or_else(|| "?".into()));
    ExitCode::SUCCESS
}

fn cmd_mp(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("usage: totctl mp <file.mp>");
        return ExitCode::from(2);
    };
    match tot_kit::parse_mp_file(path) {
        Ok(map) => {
            println!("file          : {path}");
            println!("layout        : {:?}", map.format);
            println!("dimensions    : {} x {}", map.width, map.height);
            println!("declared real : {:?}", map.declared_real_tiles);
            println!("counted real  : {}", map.real_tile_count());
            println!("tail bytes    : {}", map.tail_bytes);
            let mut histogram: Vec<(u8, usize)> = [0usize; 128]
                .into_iter()
                .enumerate()
                .map(|(t, _)| (t as u8, 0))
                .collect();
            for i in 0..map.tiles.len() {
                if let Some(t) = map.terrain(i) {
                    histogram[t as usize].1 += 1;
                }
            }
            histogram.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
            println!("terrain top   :");
            for (t, n) in histogram.into_iter().take(6) {
                if n > 0 {
                    println!("  id {t:3} : {n}");
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
