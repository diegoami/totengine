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
        Some("worldgen") => cmd_worldgen(&args[2..]),
        Some("rules") => cmd_rules(&args[2..]),
        Some("boot") => cmd_boot(&args[2..]),
        Some("tech") => cmd_tech(&args[2..]),
        Some("run") => cmd_run(&args[2..]),
        Some("play") => cmd_play(&args[2..]),
        Some("replay") => cmd_replay(&args[2..]),
        Some("observe") => cmd_observe(&args[2..]),
        Some("edit") => cmd_edit(&args[2..]),
        Some("events") => cmd_events(&args[2..]),
        Some("--version") | Some("-V") => {
            println!("totctl {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!(
                "usage: {prog} worldgen <out.mp> [--seed S] [--width W] [--height H] [--land-percent P]\n\
                        {prog} mp <file.mp>   — decode an .MP map header and tiles\n\
                        {prog} rules <Rules.txt>   — summarize Rules.txt sections\n\
                        {prog} play <file.sav|scn> [--turns N] [--seed S] [--log OUT] [--ai | --ai-v1]\n\
                        \x20                        — random players, AI v0 (--ai), or research-capable AI (--ai-v1)\n\
                        {prog} replay <file.sav|scn> <order.log>\n\
                        \x20                        — re-drive a game from its log; exit 1 at the first divergence\n\
                        {prog} observe <file.sav|scn> [--turns N] [--seed S] [--port P] [--replay LOG] [--stream OUT] [--log OUT] [--ai-v1]\n\
                        \x20                        — watch all-AI play or verified replay in a local browser\n\
                        {prog} boot <file.sav|scn> [--rules PATH] [--turns N] [--seed S]\n\
                        \x20                        — headless boot into engine state + checksum\n\
                        {prog} tech <Rules.txt>    — advance tree + wonders (reachability run)\n\
                        {prog} run <file.sav|scn> --turns N [--seed S] [--rules P | --no-rules]\n\
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
        world.cities.iter().filter(|c| c.city_id != 0).count()
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
    let mut no_rules = false;
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
            "--no-rules" => no_rules = true,
            _ if path.is_none() => path = Some(a),
            _ => {
                eprintln!("error: unexpected argument {a}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(path) = path else {
        eprintln!("usage: totctl run <file.sav|scn> --turns N [--seed S] [--rules P | --no-rules]");
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
    // Scenario files are Latin-1 (FallRome is not valid UTF-8).
    let read_lossy = |p: &std::path::Path| std::fs::read(p).ok().map(|b| tot_kit::latin1(&b));
    // Reviewer finding #66: an explicit --rules that cannot be read is an
    // error; running without rules needs an explicit --no-rules.
    let rules = if no_rules {
        None
    } else {
        let rp = rules_path
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::Path::new(path).with_file_name("Rules.txt"));
        match read_lossy(&rp) {
            Some(t) => Some(tot_kit::parse_rules(&t)),
            None => {
                eprintln!(
                    "error: cannot read rules {} (pass --rules PATH, or --no-rules to only advance the turn counter)",
                    rp.display()
                );
                return ExitCode::from(1);
            }
        }
    };
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
                    println!(
                        "{line} grew {} built {} events {}",
                        s.grew.len(),
                        s.built.len(),
                        s.events.len()
                    );
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
    let text = tot_kit::latin1(&bytes);
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

fn cmd_worldgen(args: &[String]) -> ExitCode {
    let result = (|| -> Result<(), String> {
        let Some(path) = args.first().filter(|p| !p.starts_with('-')) else {
            return Err("usage: totctl worldgen <out.mp> [--seed S] [--width W] [--height H] [--land-percent P]".into());
        };
        let mut config = engine::worldgen::WorldConfig::default();
        let mut options = args[1..].iter();
        while let Some(key) = options.next() {
            let value = options
                .next()
                .ok_or_else(|| format!("{key} needs a value"))?;
            let invalid = || format!("invalid {key}: {value}");
            match key.as_str() {
                "--seed" => config.seed = value.parse().map_err(|_| invalid())?,
                "--width" => config.width = value.parse().map_err(|_| invalid())?,
                "--height" => config.height = value.parse().map_err(|_| invalid())?,
                "--land-percent" => config.land_percent = value.parse().map_err(|_| invalid())?,
                _ => return Err(format!("unknown worldgen option {key}")),
            }
        }
        let world = engine::worldgen::generate(config)?;
        // create_new prevents a typo from overwriting a user's original map.
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| format!("create {path}: {e}"))?;
        file.write_all(&world.mp_bytes())
            .map_err(|e| format!("write {path}: {e}"))?;
        println!(
            "generator     : engine-designed v{} (original statistical comparison pending)",
            engine::worldgen::ALGORITHM_VERSION
        );
        println!("seed          : {}", config.seed);
        println!(
            "dimensions    : {} x {} logical",
            config.width, config.height
        );
        println!("terrain counts: {:?}", world.terrain_histogram());
        println!(
            "river tiles   : {}",
            world.river.iter().filter(|&&r| r).count()
        );
        println!("map           : {path}");
        Ok(())
    })();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
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

/// Rules (required) and Events (optional) next to a save, read as Latin-1.
fn sibling_rules_events(
    path: &str,
) -> Result<(tot_kit::Rules, Option<engine::events::EventEngine>), String> {
    let p = std::path::Path::new(path);
    let read = |n: &str| {
        std::fs::read(p.with_file_name(n))
            .ok()
            .map(|b| tot_kit::latin1(&b))
    };
    let rules = read("Rules.txt")
        .map(|t| tot_kit::parse_rules(&t))
        .ok_or_else(|| format!("cannot read {}", p.with_file_name("Rules.txt").display()))?;
    let events = read("Events.txt")
        .and_then(|t| tot_kit::parse_events(&t).ok())
        .map(engine::events::EventEngine::new);
    Ok((rules, events))
}

/// `totctl play`: every civ is a RandomPlayer (decision 0003); prints one
/// line per turn and writes the order log.
fn cmd_play(args: &[String]) -> ExitCode {
    let (mut path, mut turns, mut seed, mut out, mut ai) = (None, 10usize, 1u64, None, false);
    let mut ai_v1 = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--turns" => turns = it.next().and_then(|v| v.parse().ok()).unwrap_or(turns),
            "--seed" => seed = it.next().and_then(|v| v.parse().ok()).unwrap_or(seed),
            "--log" => out = it.next().cloned(),
            "--ai" => ai = true,
            "--ai-v1" => ai_v1 = true,
            _ if path.is_none() => path = Some(a.clone()),
            _ => {
                eprintln!("error: unexpected argument {a}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(path) = path else {
        eprintln!(
            "usage: totctl play <file.sav|scn> [--turns N] [--seed S] [--log OUT] [--ai | --ai-v1]"
        );
        return ExitCode::from(2);
    };
    let run = || -> Result<engine::player::Game, String> {
        let bytes = std::fs::read(&path).map_err(|e| format!("read {path}: {e}"))?;
        let (rules, events) = sibling_rules_events(&path)?;
        let name = std::path::Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut g = engine::player::Game::new(&name, &bytes, seed, rules, events)?;
        for civ in 1..8 {
            g.players[civ] = if ai_v1 {
                Some(Box::new(engine::ai::AiV1::new(&g.rules, seed, civ as u8)))
            } else if ai {
                Some(Box::new(engine::ai::AiV0::new(&g.rules, seed, civ as u8)))
            } else {
                Some(Box::new(engine::player::RandomPlayer::new(seed, civ as u8)))
            };
        }
        for _ in 0..turns {
            g.play_turn();
            println!(
                "turn {:>4} units {:>4} cities {:>3} checksum {:016x}",
                g.world.turn,
                g.world.units.iter().filter(|u| u.id != 0).count(),
                g.world.cities.iter().filter(|c| c.city_id != 0).count(),
                g.world.checksum()
            );
        }
        Ok(g)
    };
    match run() {
        Ok(g) => {
            if let Some(o) = out {
                if let Err(e) = std::fs::write(&o, g.log()) {
                    eprintln!("error: write {o}: {e}");
                    return ExitCode::from(1);
                }
                eprintln!("wrote {o}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

/// `totctl replay`: re-drive a game from its order log.
fn cmd_replay(args: &[String]) -> ExitCode {
    let [path, log] = args else {
        eprintln!("usage: totctl replay <file.sav|scn> <order.log>");
        return ExitCode::from(2);
    };
    let run = || -> Result<engine::player::Game, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("read {path}: {e}"))?;
        let text = std::fs::read_to_string(log).map_err(|e| format!("read {log}: {e}"))?;
        let (rules, events) = sibling_rules_events(path)?;
        let name = std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        engine::player::Game::replay(&name, &bytes, rules, events, &text)
    };
    match run() {
        Ok(g) => {
            println!(
                "replay ok: turn {} checksum {:016x}",
                g.world.turn,
                g.world.checksum()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("replay FAILED: {e}");
            ExitCode::from(1)
        }
    }
}

/// Browser observer: live AI by default, or frames reconstructed from a
/// verified order log. --stream exports without starting an HTTP server.
fn cmd_observe(args: &[String]) -> ExitCode {
    let run = || -> Result<(), String> {
        let mut path = None;
        let (mut turns, mut seed, mut port) = (100usize, 1u64, 8000u16);
        let (mut replay, mut stream, mut log) = (None, None, None);
        let mut ai_v1 = false;
        let mut it = args.iter();
        while let Some(a) = it.next() {
            match a.as_str() {
                "--ai-v1" => ai_v1 = true,
                "--turns" => {
                    turns = it
                        .next()
                        .and_then(|v| v.parse().ok())
                        .ok_or("--turns needs a nonnegative integer")?
                }
                "--seed" => {
                    seed = it
                        .next()
                        .and_then(|v| v.parse().ok())
                        .ok_or("--seed needs a nonnegative integer")?
                }
                "--port" => {
                    port = it
                        .next()
                        .and_then(|v| v.parse().ok())
                        .ok_or("--port needs an integer 0..65535")?
                }
                "--replay" => replay = Some(it.next().ok_or("--replay needs a path")?),
                "--stream" => stream = Some(it.next().ok_or("--stream needs a path")?),
                "--log" => log = Some(it.next().ok_or("--log needs a path")?),
                _ if !a.starts_with('-') && path.is_none() => path = Some(a),
                _ => return Err(format!("unexpected argument {a}")),
            }
        }
        let path = path.ok_or("usage: totctl observe <save> [--turns N] [--seed S] [--port P] [--replay LOG] [--stream OUT] [--log OUT] [--ai-v1]")?;
        if log.is_some() && stream.is_none() {
            return Err(
                "--log requires --stream; the live viewer offers an order-log download".into(),
            );
        }
        let bytes = std::fs::read(path).map_err(|e| format!("read {path}: {e}"))?;
        let (rules, events) = sibling_rules_events(path)?;
        let name = std::path::Path::new(path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let mut observer = if let Some(replay) = replay {
            let text =
                std::fs::read_to_string(replay).map_err(|e| format!("read {replay}: {e}"))?;
            totui::Observer::replay(&name, &bytes, rules, events, &text)?
        } else {
            let mut game = engine::player::Game::new(&name, &bytes, seed, rules, events)?;
            for civ in 1..8 {
                game.players[civ] = if ai_v1 {
                    Some(Box::new(engine::ai::AiV1::new(
                        &game.rules,
                        seed,
                        civ as u8,
                    )))
                } else {
                    Some(Box::new(engine::ai::AiV0::new(
                        &game.rules,
                        seed,
                        civ as u8,
                    )))
                };
            }
            totui::Observer::live(game, turns)
        };
        if let Some(path) = stream {
            // Refuse accidental overwrites, including the user's input save.
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|e| format!("create {path}: {e}"))?;
            let mut out = std::io::BufWriter::new(file);
            observer.export(&mut out).map_err(|e| e.to_string())?;
            use std::io::Write as _;
            out.flush().map_err(|e| e.to_string())?;
            if let Some(path) = log {
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)
                    .map_err(|e| format!("create {path}: {e}"))?;
                file.write_all(observer.log().as_bytes())
                    .map_err(|e| e.to_string())?;
            }
            eprintln!("wrote {path}");
        } else {
            observer.serve(port).map_err(|e| e.to_string())?;
        }
        Ok(())
    };
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
