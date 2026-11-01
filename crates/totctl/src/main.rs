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
