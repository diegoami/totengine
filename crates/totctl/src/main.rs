//! `totctl` — inspect Civilization II: Test of Time game data.
//!
//! Rust successor of `tools/totctl.py` (the Python probe stays as the fast
//! reverse-engineering loop; this crate absorbs features as they stabilize).

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let prog = args.first().map(String::as_str).unwrap_or("totctl");
    match args.get(1).map(String::as_str) {
        Some("mp") => cmd_mp(&args[2..]),
        Some("rules") => cmd_rules(&args[2..]),
        Some("--version") | Some("-V") => {
            println!("totctl {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!(
                "usage: {prog} mp <file.mp>   — decode an .MP map header and tiles\n\
                       {prog} --version"
            );
            ExitCode::from(2)
        }
    }
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
