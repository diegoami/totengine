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
        Some("tech") => cmd_tech(&args[2..]),
        Some("run") => cmd_run(&args[2..]),
        Some("edit") => cmd_edit(&args[2..]),
        Some("events") => cmd_events(&args[2..]),
        Some("--version") | Some("-V") => {
            println!("totctl {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!(
                "usage: {prog} mp <file.mp>   — decode an .MP map header and tiles\n\
                        {prog} rules <Rules.txt>   — summarize Rules.txt sections\n\
                        {prog} boot <file.sav|scn> [--rules PATH] [--turns N] [--seed S]\n\
                        \x20                        — headless boot into engine state + checksum\n\
                        {prog} tech <Rules.txt>    — advance tree + wonders (reachability run)\n\
                        {prog} run <file.sav|scn> --turns N [--seed S] [--rules P]\n\
                        \x20                        — deterministic N-turn headless run (per-turn checksums)\n\
                        {prog} edit <in> <out> [--turn N] [--year N] [--difficulty N]\n\
                        \x20      [--rivals N] [--unit-pos IDX X Y] [--city-name IDX NAME]\n\
                        \x20                        — surgical field edits (T1-8 writer)\n\
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
    let mut rules_path: Option<String> = None;
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
            "--rules" => match it.next() {
                Some(p) => rules_path = Some(p.clone()),
                None => {
                    eprintln!("error: --rules needs a path");
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
    // rules: explicit --rules, else a sibling Rules.txt (scenario dir)
    let rules = rules_path.or_else(|| {
        let sib = std::path::Path::new(path).with_file_name("Rules.txt");
        sib.exists().then(|| sib.to_string_lossy().into_owned())
    });
    let rules = rules.and_then(|rp| {
        std::fs::read_to_string(&rp).ok().map(|text| {
            println!("rules         : {rp}");
            tot_kit::parse_rules(&text)
        })
    });
    let mut world = engine::boot(&sav, seed);
    if let Some(r) = &rules {
        world.start_turn(r);
    }
    for _ in 0..turns {
        world.advance_turn();
        if let Some(r) = &rules {
            world.start_turn(r);
        }
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
    if let Some(c) = world.cities.first() {
        println!(
            "city[0]       : {} size{} owner{} F{}/S{}/T{} sci{} tax{} box {}/{}",
            c.name,
            c.size,
            c.owner,
            c.gross_food,
            c.gross_shields,
            c.gross_trade,
            c.net_science,
            c.net_taxes,
            c.food_box,
            c.shield_box
        );
    }
    if let Some(r) = &rules {
        let movable = world
            .units
            .iter()
            .filter(|u| r.unit_move(u.type_id as usize).unwrap_or(0) > 0)
            .count();
        println!(
            "moves         : {movable}/{} units movable at turn start",
            world.units.len()
        );
    }
    println!(
        "rng seed      : {} (draws so far: {})",
        seed,
        world.rng.draws()
    );
    println!("checksum      : {:016x}", world.checksum());
    ExitCode::SUCCESS
}

/// Surgical save edits (T1-8): copy the input applying typed field edits
/// to the proven offsets; every other byte stays untouched.
fn cmd_edit(args: &[String]) -> ExitCode {
    let mut positional: Vec<&String> = Vec::new();
    let mut turn: Option<i16> = None;
    let mut year: Option<i16> = None;
    let mut difficulty: Option<u8> = None;
    let mut rivals: Option<u8> = None;
    let mut unit_pos: Option<(usize, u16, u16)> = None;
    let mut city_name: Option<(usize, String)> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--turn" => turn = it.next().and_then(|v| v.parse().ok()),
            "--year" => year = it.next().and_then(|v| v.parse().ok()),
            "--difficulty" => difficulty = it.next().and_then(|v| v.parse().ok()),
            "--rivals" => rivals = it.next().and_then(|v| v.parse().ok()),
            "--unit-pos" => {
                let idx = it.next().and_then(|v| v.parse().ok());
                let x = it.next().and_then(|v| v.parse().ok());
                let y = it.next().and_then(|v| v.parse().ok());
                match (idx, x, y) {
                    (Some(i), Some(x), Some(y)) => unit_pos = Some((i, x, y)),
                    _ => {
                        eprintln!("error: --unit-pos needs IDX X Y");
                        return ExitCode::from(2);
                    }
                }
            }
            "--city-name" => {
                let idx = it.next().and_then(|v| v.parse().ok());
                let name = it.next().cloned();
                match (idx, name) {
                    (Some(i), Some(n)) => city_name = Some((i, n)),
                    _ => {
                        eprintln!("error: --city-name needs IDX NAME");
                        return ExitCode::from(2);
                    }
                }
            }
            _ => positional.push(a),
        }
    }
    if positional.len() != 2 {
        eprintln!("usage: totctl edit <in> <out> [edits]");
        return ExitCode::from(2);
    }
    let (src, dst) = (positional[0], positional[1]);
    let raw = match std::fs::read(src) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: read {src}: {e}");
            return ExitCode::from(1);
        }
    };
    let mut ed = match tot_kit::SavEditor::new(&raw) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    if let Some(t) = turn {
        ed.set_turn(t);
    }
    if let Some(y) = year {
        ed.set_year(y);
    }
    if let Some(d) = difficulty {
        ed.set_difficulty(d);
    }
    if let Some(r) = rivals {
        ed.set_rivals(r);
    }
    if let Some((i, x, y)) = unit_pos {
        if let Err(e) = ed.set_unit_pos(i, x, y) {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    }
    if let Some((i, n)) = city_name {
        if let Err(e) = ed.set_city_name(i, &n) {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    }
    let out = ed.into_bytes();
    if let Err(e) = std::fs::write(dst, &out) {
        eprintln!("error: write {dst}: {e}");
        return ExitCode::from(1);
    }
    let changed = raw.iter().zip(out.iter()).filter(|(a, b)| a != b).count();
    println!("wrote {dst} ({} bytes, {changed} bytes edited)", out.len());
    ExitCode::SUCCESS
}

/// Deterministic headless run (T1-9): boot, then advance N turns,
/// printing one fingerprint line per turn. The golden suite diffs this
/// against `docs/evidence/fallrome-50turn.txt`.
fn cmd_run(args: &[String]) -> ExitCode {
    let mut path: Option<&String> = None;
    let mut turns: usize = 50;
    let mut seed: u64 = 1;
    let mut rules_path: Option<String> = None;
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
            "--rules" => match it.next() {
                Some(p) => rules_path = Some(p.clone()),
                None => {
                    eprintln!("error: --rules needs a path");
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
        eprintln!("usage: totctl run <file.sav|scn> --turns N [--seed S] [--rules P]");
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
    // Scenario files are not strictly UTF-8 (FallRome): read lossily.
    let read_lossy = |p: &std::path::Path| {
        std::fs::read(p)
            .ok()
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    };
    let rules_path = rules_path.map(std::path::PathBuf::from).or_else(|| {
        let sib = std::path::Path::new(path).with_file_name("Rules.txt");
        sib.exists().then_some(sib)
    });
    let rules = rules_path
        .as_deref()
        .and_then(read_lossy)
        .map(|t| tot_kit::parse_rules(&t));
    let mut events = read_lossy(&std::path::Path::new(path).with_file_name("Events.txt"))
        .and_then(|t| tot_kit::parse_events(&t).ok())
        .map(engine::events::EventEngine::new);
    let mut world = engine::boot(&sav, seed);
    if let Some(r) = &rules {
        world.start_turn(r);
    }
    println!("file {}", path);
    println!("seed {seed}");
    println!(
        "rules {} events {}",
        if rules.is_some() { "yes" } else { "no" },
        events.as_ref().map_or(0, |e| e.file.events.len())
    );
    for t in 0..=turns {
        let line = format!("{t:04} {:016x}", world.checksum());
        if t < turns {
            match &rules {
                Some(r) => {
                    let s = world.run_turn(r, events.as_mut());
                    println!("{line} grew {} events {}", s.grew.len(), s.events.len());
                }
                None => {
                    world.advance_turn();
                    println!("{line}");
                }
            }
        } else {
            println!("{line}");
        }
    }
    ExitCode::SUCCESS
}

/// Advance tree + wonder table (T1-5): resolves prerequisites, runs the
/// deterministic scripted research pass and reports reachability.
fn cmd_tech(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("usage: totctl tech <Rules.txt>");
        return ExitCode::from(2);
    };
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: read {path}: {e}");
            return ExitCode::from(1);
        }
    };
    let rules = tot_kit::parse_rules(&text);
    let tree = engine::research::TechTree::from_rules(&rules);
    let wonders = engine::research::WonderTable::from_rules(&rules, &tree);
    println!("advances      : {}", tree.techs.len());
    let mut epochs = [0usize; 4];
    for t in &tree.techs {
        if let Some(e) = t.epoch {
            if let Some(slot) = epochs.get_mut(e as usize) {
                *slot += 1;
            }
        }
    }
    println!(
        "epochs        : Ancient {} / Renaissance {} / Industrial {} / Modern {}",
        epochs[0], epochs[1], epochs[2], epochs[3]
    );
    if tree.unresolved_prereqs.is_empty() {
        println!("prereqs       : all resolved");
    } else {
        println!("prereqs       : UNRESOLVED {:?}", tree.unresolved_prereqs);
    }
    let show = |name: &str| {
        let Some(id) = tree.techs.iter().position(|t| t.name == name) else {
            return format!("{name}: ?");
        };
        let pre: Vec<String> = tree.techs[id]
            .prereqs
            .iter()
            .map(|p| {
                p.and_then(|pid| tree.techs[pid].name.clone().into())
                    .unwrap_or_else(|| "—".into())
            })
            .collect();
        format!("{name} <- {}", pre.join(" + "))
    };
    for name in ["Monarchy", "Iron Working", "Trade", "Writing"] {
        println!("  {}", show(name));
    }
    let (order, unreached) = tree.scripted_research();
    println!(
        "scripted run  : researched {} advances in {} steps",
        order.len(),
        order.len()
    );
    if unreached.is_empty() {
        println!("reachability  : every advance reachable from zero");
    } else {
        let names: Vec<&str> = unreached
            .iter()
            .map(|&i| tree.techs[i].name.as_str())
            .collect();
        println!("reachability  : UNREACHABLE {names:?}");
    }
    println!("wonders       : {}", wonders.wonders.len());
    for w in wonders.wonders.iter().take(4) {
        let prereq = w
            .prereq
            .map(|id| tree.techs[id].name.as_str())
            .unwrap_or("—");
        let obs = w
            .obsolete_by
            .map(|id| tree.techs[id].name.as_str())
            .unwrap_or("never");
        println!(
            "  {:<28} cost {:>3} prereq {:<14} obsolete {obs}",
            w.name,
            w.cost.unwrap_or(0),
            prereq
        );
    }
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

fn cmd_events(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("usage: totctl events <Events.txt>");
        return ExitCode::FAILURE;
    };
    let Ok(src) = std::fs::read_to_string(path) else {
        eprintln!("cannot read {path}");
        return ExitCode::FAILURE;
    };
    let f = match tot_kit::parse_events(&src) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("parse error: {e:?}");
            return ExitCode::FAILURE;
        }
    };
    use std::collections::BTreeMap;
    let mut triggers: BTreeMap<&str, usize> = BTreeMap::new();
    let mut actions: BTreeMap<&str, usize> = BTreeMap::new();
    for e in &f.events {
        *triggers.entry(e.trigger.keyword.as_str()).or_default() += 1;
        for a in &e.actions {
            *actions.entry(a.keyword.as_str()).or_default() += 1;
        }
    }
    println!("events       : {}", f.events.len());
    println!("triggers     :");
    for (k, n) in &triggers {
        println!("  {k:<18} {n}");
    }
    println!("actions      :");
    for (k, n) in &actions {
        println!("  {k:<18} {n}");
    }
    ExitCode::SUCCESS
}
