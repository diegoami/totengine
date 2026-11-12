//! `totctl` — inspect Civilization II: Test of Time game data.
//!
//! Rust successor of `tools/totctl.py` (the Python probe stays as the fast
//! reverse-engineering loop; this crate absorbs features as they stabilize).

use std::process::ExitCode;

use engine::rng::GameRng as _;

mod mcp;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let prog = args.first().map(String::as_str).unwrap_or("totctl");
    match args.get(1).map(String::as_str) {
        Some("mp") => cmd_mp(&args[2..]),
        Some("worldgen") => cmd_worldgen(&args[2..]),
        Some("campaign") => cmd_campaign(&args[2..]),
        Some("rules") => cmd_rules(&args[2..]),
        Some("boot") => cmd_boot(&args[2..]),
        Some("tech") => cmd_tech(&args[2..]),
        Some("run") => cmd_run(&args[2..]),
        Some("play") => cmd_play(&args[2..]),
        Some("replay") => cmd_replay(&args[2..]),
        Some("observe") => cmd_observe(&args[2..]),
        Some("edit") => cmd_edit(&args[2..]),
        Some("events") => cmd_events(&args[2..]),
        Some("mcp") => mcp::cmd_mcp(&args[2..]),
        Some("record") => cmd_record(&args[2..]),
        Some("--version") | Some("-V") => {
            println!("totctl {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!(
                "usage: {prog} worldgen <out.mp> [--seed S] [--width W] [--height H] [--land-percent P]\n\
                        {prog} campaign <Rules.txt> [--policy ai-v1|builtin-1] [--turns N] [--seed S] [--civs C] [--difficulty D] [--width W] [--height H] [--land-percent P] [--checkpoint OUT] [--resume FILE] [--log OUT] [--stream OUT] [--record OUT.totrec [--record-every K]]\n\
                        {prog} record info <bundle.totrec> | verify <bundle.totrec> <Rules.txt> | recover <journal> <out.totrec>\n\
                        \x20                        — inspect, replay-verify or recover session recordings (#206)\n\
                        {prog} mp <file.mp>   — decode an .MP map header and tiles\n\
                        {prog} rules <Rules.txt>   — summarize Rules.txt sections\n\
                        {prog} play <file.sav|scn> [--turns N] [--seed S] [--log OUT] [--ai | --ai-v1 | --builtin]\n\
                        \x20                        — random players, AI v0 (--ai), AI v1 (--ai-v1), or built-in AI (--builtin)\n\
                        {prog} replay <file.sav|scn> <order.log>\n\
                        \x20                        — re-drive a game from its log; exit 1 at the first divergence\n\
                        {prog} observe <file.sav|scn> [--turns N] [--seed S] [--port P] [--replay LOG] [--stream OUT] [--log OUT] [--ai | --ai-v1 | --builtin]\n\
                        \x20                        — watch all-AI play or verified replay in a local browser\n\
                        {prog} boot <file.sav|scn> [--rules PATH] [--turns N] [--seed S]\n\
                        \x20                        — headless boot into engine state + checksum\n\
                        {prog} tech <Rules.txt>    — advance tree + wonders (reachability run)\n\
                        {prog} run <file.sav|scn> --turns N [--seed S] [--rules P | --no-rules]\n\
                        \x20                        — deterministic N-turn headless run (per-turn checksums)\n\
                        {prog} mcp [--root PATH]...   — stdio MCP server: read-only inspection/run/observer tools (#250)
\
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
    match boot_report(path, rules_path, turns, seed) {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

/// Boot summary shared by `totctl boot` and the MCP `save_summary` tool (#250);
/// the text is byte-identical to the CLI output.
fn boot_report(
    path: &str,
    rules_path: Option<String>,
    turns: usize,
    seed: u64,
) -> Result<String, String> {
    use std::fmt::Write as _;
    let mut out = String::new();
    let bytes = std::fs::read(path).map_err(|e| format!("read {path}: {e}"))?;
    let sav = tot_kit::parse_sav(&bytes).map_err(|e| e.to_string())?;
    // rules: explicit --rules, else a sibling Rules.txt (scenario dir)
    let rules = rules_path.or_else(|| {
        let sib = std::path::Path::new(path).with_file_name("Rules.txt");
        sib.exists().then(|| sib.to_string_lossy().into_owned())
    });
    let rules = rules.and_then(|rp| {
        std::fs::read_to_string(&rp).ok().map(|text| {
            let _ = writeln!(out, "rules         : {rp}");
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
    let _ = writeln!(out, "file          : {path}");
    let _ = writeln!(out, "version       : 0x{:04x}", sav.version);
    let _ = writeln!(out, "turn / year   : {} / {}", world.turn, world.year);
    let _ = writeln!(
        out,
        "difficulty    : {}  rivals: {}",
        world.difficulty, world.rivals
    );
    let named: Vec<&engine::Civ> = world.civs.iter().filter(|c| !c.names.is_empty()).collect();
    let _ = writeln!(out, "civs (named)  : {}", named.len());
    for c in &named {
        let _ = writeln!(
            out,
            "  owner {} : {:<28} units {}",
            c.id,
            c.names.join(" / "),
            c.unit_count
        );
    }
    let _ = writeln!(
        out,
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
    let _ = writeln!(out, "terrain hist  : {}", top_s.join(" "));
    let _ = writeln!(
        out,
        "units / cities: {} / {}",
        world.units.len(),
        world.cities.iter().filter(|c| c.city_id != 0).count()
    );
    if let Some(c) = world.cities.first() {
        let _ = writeln!(
            out,
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
        let _ = writeln!(
            out,
            "moves         : {movable}/{} units movable at turn start",
            world.units.len()
        );
    }
    let _ = writeln!(
        out,
        "rng seed      : {} (draws so far: {})",
        seed,
        world.rng.draws()
    );
    let _ = writeln!(out, "checksum      : {:016x}", world.checksum());
    Ok(out)
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
    match run_report(path, turns, seed, rules_path, no_rules) {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

/// Deterministic N-turn run report shared by `totctl run` and the MCP
/// `run_turns` tool (#250); the text is byte-identical to the CLI output.
fn run_report(
    path: &str,
    turns: usize,
    seed: u64,
    rules_path: Option<String>,
    no_rules: bool,
) -> Result<String, String> {
    use std::fmt::Write as _;
    let mut out = String::new();
    let bytes = std::fs::read(path).map_err(|e| format!("read {path}: {e}"))?;
    let sav = tot_kit::parse_sav(&bytes).map_err(|e| e.to_string())?;
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
                return Err(format!(
                    "cannot read rules {} (pass --rules PATH, or --no-rules to only advance the turn counter)",
                    rp.display()
                ));
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
    let _ = writeln!(out, "file {}", path);
    let _ = writeln!(out, "seed {seed}");
    let _ = writeln!(
        out,
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
                    let _ = writeln!(
                        out,
                        "{line} grew {} built {} events {}",
                        s.grew.len(),
                        s.built.len(),
                        s.events.len()
                    );
                }
                None => {
                    world.advance_turn();
                    let _ = writeln!(out, "{line}");
                }
            }
        } else {
            let _ = writeln!(out, "{line}");
        }
    }
    Ok(out)
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
    match rules_report(path) {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

/// Rules summary shared by `totctl rules` and the MCP `rules_summary` tool
/// (#250); the text is byte-identical to the CLI output.
fn rules_report(path: &str) -> Result<String, String> {
    use std::fmt::Write as _;
    let mut out = String::new();
    let bytes = std::fs::read(path).map_err(|e| format!("read {path}: {e}"))?;
    let text = tot_kit::latin1(&bytes);
    let rules = tot_kit::parse_rules(&text);
    let _ = writeln!(out, "sections      : {}", rules.section_order().join(","));
    let _ = writeln!(out, "row counts    : {}", {
        let mut s = String::new();
        for (n, c) in rules.row_counts() {
            s.push_str(&format!("{n}={c} "));
        }
        s.trim_end().to_string()
    });
    let _ = writeln!(out, "semantic ok   : {}", rules.terrain_semantic_ok());
    let _ = writeln!(
        out,
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
    let _ = writeln!(out, "units         : {}", rules.unit_names().len());
    let domains = {
        let mut counts = [0usize; 3];
        for i in 0..rules.unit_names().len() {
            if let Some(d) = rules.unit_domain(i) {
                counts[(d as usize).min(2)] += 1;
            }
        }
        format!("{}/{}/{}", counts[0], counts[1], counts[2])
    };
    let _ = writeln!(out, "unit domains  : {domains}");
    let _ = writeln!(
        out,
        "governments   : {}",
        rules.government_names().join(", ")
    );
    let _ = writeln!(out, "caravan       : {}", rules.caravan_commodities().len());
    let _ = writeln!(out, "transport rel : {:?}", rules.transport_relationships());
    let road = rules
        .cosmic()
        .into_iter()
        .find(|(k, _)| k.contains("Road movement multiplier"))
        .map(|(_, v)| v.to_string());
    let _ = writeln!(
        out,
        "cosmic road   : {}",
        road.unwrap_or_else(|| "?".into())
    );
    Ok(out)
}

fn cmd_campaign(args: &[String]) -> ExitCode {
    use std::io::Write;
    let run = || -> Result<(), String> {
        let rules_path = args.first().ok_or("usage: totctl campaign <Rules.txt> [--policy ai-v1|builtin-1] [--turns N] [--seed S] [--checkpoint OUT] [--resume FILE]")?;
        let mut config = engine::newgame::NewGameConfig::default();
        let mut changed_setup = false;
        let mut policy = engine::campaign::AiPolicy::BuiltIn1;
        let mut turns = 100u32;
        let mut checkpoint = None;
        let mut resume = None;
        let mut log_path = None;
        let mut stream_path = None;
        let mut record_path = None;
        let mut record_every = 10u32;
        let (options, remainder) = args[1..].as_chunks::<2>();
        for option in options {
            let key = option[0].as_str();
            let value = &option[1];
            let bad = || format!("invalid value for {key}: {value}");
            match key {
                "--policy" => {
                    policy = engine::campaign::AiPolicy::parse(value)?;
                    changed_setup = true;
                }
                "--turns" => turns = value.parse().map_err(|_| bad())?,
                "--checkpoint" => checkpoint = Some(value),
                "--resume" => resume = Some(value),
                "--log" => log_path = Some(value),
                "--stream" => stream_path = Some(value),
                "--record" => record_path = Some(value),
                "--record-every" => {
                    record_every = value.parse().map_err(|_| bad())?;
                    if record_every == 0 {
                        return Err(bad());
                    }
                }
                "--seed" => {
                    config.world.seed = value.parse().map_err(|_| bad())?;
                    config.gameplay_seed = config.world.seed;
                    changed_setup = true;
                }
                "--width" => {
                    config.world.width = value.parse().map_err(|_| bad())?;
                    changed_setup = true;
                }
                "--height" => {
                    config.world.height = value.parse().map_err(|_| bad())?;
                    changed_setup = true;
                }
                "--land-percent" => {
                    config.world.land_percent = value.parse().map_err(|_| bad())?;
                    changed_setup = true;
                }
                "--civs" => {
                    config.civilizations = value.parse().map_err(|_| bad())?;
                    changed_setup = true;
                }
                "--difficulty" => {
                    config.difficulty = value.parse().map_err(|_| bad())?;
                    changed_setup = true;
                }
                _ => return Err(format!("unknown campaign option {key}")),
            }
        }
        if !remainder.is_empty() {
            return Err("campaign options need values".into());
        }
        if checkpoint.is_some() && checkpoint == log_path {
            return Err("checkpoint and log output paths must differ".into());
        }
        if stream_path.is_some() && (stream_path == checkpoint || stream_path == log_path) {
            return Err("stream output path must differ from checkpoint and log".into());
        }
        let journal_path = record_path.map(|p| format!("{p}.journal"));
        if record_path.is_some() && [checkpoint, log_path, stream_path].contains(&record_path) {
            return Err("record output path must differ from the other outputs".into());
        }
        for path in [checkpoint, log_path, stream_path, record_path]
            .into_iter()
            .flatten()
            .chain(journal_path.as_ref())
        {
            if std::path::Path::new(path).exists() {
                return Err(format!("output already exists: {path}"));
            }
        }
        let resume_checkpoint = match resume {
            Some(path) => {
                Some(std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?)
            }
            None => None,
        };
        let report = campaign_report(CampaignRequest {
            rules_path: rules_path.to_string(),
            policy,
            turns,
            config,
            changed_setup,
            resume_checkpoint,
            observe: stream_path.is_some(),
            record: journal_path.as_ref().map(|journal| RecordOptions {
                journal: journal.into(),
                every: record_every,
            }),
        })?;
        let stream = report.stream();
        for (path, contents) in [
            (checkpoint, &report.checkpoint),
            (log_path, &report.log),
            (stream_path, &stream),
        ] {
            if let Some(path) = path {
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)
                    .map_err(|e| format!("create {path}: {e}"))?;
                file.write_all(contents.as_bytes())
                    .map_err(|e| format!("write {path}: {e}"))?;
            }
        }
        if let (Some(path), Some(bundle), Some(journal)) =
            (record_path, &report.bundle, &journal_path)
        {
            tot_record::write_new(std::path::Path::new(path), &bundle.encode()?)?;
            // The bundle is durable; the journal is no longer needed.
            std::fs::remove_file(journal).map_err(|e| format!("remove {journal}: {e}"))?;
        }
        print!("{}", report.text);
        Ok(())
    };
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// Campaign run inputs shared by `totctl campaign` and the MCP `campaign_run`
/// tool (#250). `resume_checkpoint` is the checkpoint *text*: the CLI reads it
/// from `--resume FILE`; the MCP tool receives it inline from a previous run.
struct CampaignRequest {
    rules_path: String,
    policy: engine::campaign::AiPolicy,
    turns: u32,
    config: engine::newgame::NewGameConfig,
    changed_setup: bool,
    resume_checkpoint: Option<String>,
    /// Capture an observer v1 frame before the first round and after each
    /// completed round (`--stream`, MCP `campaign_observe`).
    observe: bool,
    /// Keep a crash-recoverable journal while running and build a recording
    /// bundle at the end (`--record`). The MCP tools never record.
    record: Option<RecordOptions>,
}

struct RecordOptions {
    journal: std::path::PathBuf,
    /// Journal a checkpoint every this many rounds (and at the end).
    every: u32,
}

/// Campaign run outputs. `text` is byte-identical to the CLI stdout;
/// `checkpoint` and `log` are exactly what `--checkpoint`/`--log` would write.
struct CampaignReport {
    text: String,
    checkpoint: String,
    log: String,
    /// Observer frames when requested: the starting state of this
    /// invocation, then one per completed round. Empty otherwise.
    frames: Vec<engine::observer::Snapshot>,
    /// The recording bundle when `record` was requested.
    bundle: Option<tot_record::Bundle>,
}

impl CampaignReport {
    /// Observer v1 NDJSON, exactly what `--stream` writes.
    fn stream(&self) -> String {
        self.frames
            .iter()
            .map(|frame| frame.to_json() + "\n")
            .collect()
    }
}

/// Create or restore the campaign a request describes, without running it.
/// Shared by `campaign_report` and the MCP session tools.
fn campaign_open(req: &CampaignRequest) -> Result<engine::campaign::Campaign, String> {
    if req.resume_checkpoint.is_some() && req.changed_setup {
        return Err("resume uses the checkpoint setup; omit setup options".into());
    }
    let bytes =
        std::fs::read(&req.rules_path).map_err(|e| format!("read {}: {e}", req.rules_path))?;
    let rules = tot_kit::parse_rules(&tot_kit::latin1(&bytes));
    match &req.resume_checkpoint {
        Some(saved) => engine::campaign::Campaign::restore(rules, saved),
        None => engine::campaign::Campaign::with_policy(req.config, rules, req.policy),
    }
}

fn campaign_report(req: CampaignRequest) -> Result<CampaignReport, String> {
    let mut campaign = campaign_open(&req)?;
    let mut frames = Vec::new();
    let start_round = campaign.rounds();
    let mut journal = match &req.record {
        Some(options) => {
            let manifest = tot_record::Bundle::campaign(
                &campaign,
                tot_record::StopReason::Stopped,
                "totctl campaign",
                None,
            )
            .manifest;
            let mut journal = tot_record::Journal::create(&options.journal, &manifest)?;
            // The starting state, so an early interruption is recoverable too.
            journal.record(campaign.rounds(), &campaign.checkpoint())?;
            Some(journal)
        }
        None => None,
    };
    let outcome = if req.observe || journal.is_some() {
        // Same limit check as `advance`, before any round runs.
        if req.turns > engine::campaign::MAX_ROUNDS - campaign.rounds() {
            return Err(format!(
                "campaign supports at most {} rounds",
                engine::campaign::MAX_ROUNDS
            ));
        }
        let mut recorder = engine::observer::Recorder::default();
        if req.observe {
            frames.push(recorder.capture(&campaign.game));
        }
        let every = req.record.as_ref().map_or(0, |r| r.every);
        let mut outcome = campaign.outcome();
        for _ in 0..req.turns {
            let before = campaign.rounds();
            outcome = campaign.advance(1)?;
            if campaign.rounds() == before {
                break; // fewer than two civilizations remain
            }
            if req.observe {
                frames.push(recorder.capture(&campaign.game));
            }
            if let Some(journal) = journal.as_mut() {
                if (campaign.rounds() - start_round) % every == 0 {
                    journal.record(campaign.rounds(), &campaign.checkpoint())?;
                }
            }
        }
        if let Some(journal) = journal.as_mut() {
            journal.record(campaign.rounds(), &campaign.checkpoint())?;
        }
        outcome
    } else {
        campaign.advance(req.turns)?
    };
    let out = campaign_summary(&campaign, &outcome);
    let bundle = req.record.as_ref().map(|_| {
        let observer = req.observe.then(|| tot_record::Observer {
            perspective: tot_record::Perspective::AllViews,
            from_round: start_round,
            frames: frames.iter().map(|f| f.to_json() + "\n").collect(),
        });
        tot_record::Bundle::campaign(&campaign, outcome.into(), "totctl campaign", observer)
    });
    Ok(CampaignReport {
        text: out,
        checkpoint: campaign.checkpoint(),
        log: campaign.game.log().to_string(),
        frames,
        bundle,
    })
}

/// The `totctl campaign` stdout for a campaign's current state. Shared with
/// the MCP campaign and session tools.
fn campaign_summary(
    campaign: &engine::campaign::Campaign,
    outcome: &engine::campaign::Outcome,
) -> String {
    use std::fmt::Write as _;
    let world = &campaign.game.world;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "generated campaign: engine-designed world v{}, initialization policy v{}",
        engine::worldgen::ALGORITHM_VERSION,
        engine::newgame::INITIALIZATION_VERSION
    );
    let _ = writeln!(out, "setup       : {}", campaign.config().descriptor());
    let _ = writeln!(out, "rounds      : {}", campaign.rounds());
    let _ = writeln!(out, "turn / year : {} / {}", world.turn, world.year);
    let _ = writeln!(out, "living civs : {:?}", world.living_civs());
    let _ = writeln!(
        out,
        "cities/units: {} / {}",
        world.cities.iter().filter(|c| c.city_id != 0).count(),
        world.units.iter().filter(|u| u.id != 0).count()
    );
    let _ = writeln!(out, "fingerprint : {:016x}", world.checksum());
    let _ = writeln!(
        out,
        "outcome     : {outcome:?} (observed state; original victory rules not claimed)"
    );
    out
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
        let (report, world) = worldgen_report(config)?;
        // create_new prevents a typo from overwriting a user's original map.
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| format!("create {path}: {e}"))?;
        file.write_all(&world.mp_bytes())
            .map_err(|e| format!("write {path}: {e}"))?;
        print!("{report}");
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

/// `totctl record`: inspect, verify or recover recording bundles (#206).
fn cmd_record(args: &[String]) -> ExitCode {
    use std::path::Path;
    let usage = "usage: totctl record info <bundle.totrec>\n       totctl record verify <bundle.totrec> <Rules.txt>\n       totctl record recover <journal> <out.totrec>";
    let run = || -> Result<(), String> {
        match args {
            [cmd, bundle] if cmd == "info" => {
                print!("{}", record_info_report(bundle)?);
                Ok(())
            }
            [cmd, bundle, rules] if cmd == "verify" => {
                print!("{}", record_verify_report(bundle, rules)?);
                Ok(())
            }
            [cmd, journal, out] if cmd == "recover" => {
                let recovered =
                    tot_record::recover(&tot_record::read_limited(Path::new(journal))?)?;
                tot_record::write_new(Path::new(out), &recovered.bundle.encode()?)?;
                print!("{}", record_summary(&recovered.bundle));
                println!(
                    "recovered   : {} complete record(s); {} trailing byte(s) discarded; journal kept",
                    recovered.records, recovered.discarded_bytes
                );
                Ok(())
            }
            _ => Err(usage.into()),
        }
    };
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// `totctl record info` stdout; shared with the MCP `recording_info` tool.
fn record_info_report(path: &str) -> Result<String, String> {
    let b = tot_record::Bundle::decode(&tot_record::read_limited(std::path::Path::new(path))?)?;
    Ok(record_summary(&b))
}

/// `totctl record verify` stdout; shared with the MCP `recording_verify` tool.
fn record_verify_report(path: &str, rules_path: &str) -> Result<String, String> {
    use std::fmt::Write as _;
    let b = tot_record::Bundle::decode(&tot_record::read_limited(std::path::Path::new(path))?)?;
    let bytes = std::fs::read(rules_path).map_err(|e| format!("read {rules_path}: {e}"))?;
    let rules = tot_kit::parse_rules(&tot_kit::latin1(&bytes));
    let verified = b.verify(&rules)?;
    let mut out = record_summary(&b);
    let _ = match verified.fingerprint {
        Some(f) => writeln!(
            out,
            "verified    : replay reproduces the recording; fingerprint {f:016x}"
        ),
        None => writeln!(
            out,
            "verified    : view-only; identities match, nothing to replay"
        ),
    };
    Ok(out)
}

fn record_summary(b: &tot_record::Bundle) -> String {
    use std::fmt::Write as _;
    let m = &b.manifest;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "recording   : {} ({:?})",
        tot_record::FORMAT,
        b.content()
    );
    let _ = writeln!(out, "kind        : {:?}", m.kind);
    let _ = writeln!(out, "engine      : {}", m.engine);
    let _ = writeln!(out, "setup       : {}", m.setup);
    let _ = writeln!(out, "policy      : {}", m.policy);
    if let Some(human) = m.human {
        let _ = writeln!(out, "human civ   : {human}");
    }
    let _ = writeln!(out, "rules       : {}", m.rules);
    if let Some(resources) = &m.resources {
        let _ = writeln!(out, "resources   : {}", tot_record::identity(resources));
    }
    let _ = writeln!(out, "progress    : {}", m.progress);
    let _ = writeln!(out, "stop        : {:?}", m.stop);
    let _ = writeln!(out, "producer    : {}", m.producer);
    if let Some(o) = &b.observer {
        let _ = writeln!(
            out,
            "observer    : {} frame(s) from round {}, {} ({})",
            o.frames.lines().count(),
            o.from_round,
            o.perspective.encode(),
            if o.perspective.full_state() {
                "full state"
            } else {
                "one civ's fogged view"
            }
        );
    }
    if let Some(p) = &b.presentation {
        let _ = writeln!(out, "presentation: {} inert entr(ies)", p.lines().count());
    }
    out
}

/// Generate a world in memory and describe it. Shared by `totctl worldgen`
/// (which then writes the map and adds a `map` line) and the MCP
/// `worldgen_summary` tool (which writes nothing).
fn worldgen_report(
    config: engine::worldgen::WorldConfig,
) -> Result<(String, engine::worldgen::GeneratedWorld), String> {
    use std::fmt::Write as _;
    let world = engine::worldgen::generate(config)?;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "generator     : engine-designed v{} (not an original-algorithm port)",
        engine::worldgen::ALGORITHM_VERSION
    );
    let _ = writeln!(out, "seed          : {}", config.seed);
    let _ = writeln!(
        out,
        "dimensions    : {} x {} logical",
        config.width, config.height
    );
    let _ = writeln!(out, "terrain counts: {:?}", world.terrain_histogram());
    let _ = writeln!(
        out,
        "river tiles   : {}",
        world.river.iter().filter(|&&r| r).count()
    );
    Ok((out, world))
}

fn cmd_mp(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("usage: totctl mp <file.mp>");
        return ExitCode::from(2);
    };
    match mp_report(path) {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// MP map summary shared by `totctl mp` and the MCP `map_summary` tool (#250);
/// the text is byte-identical to the CLI output.
fn mp_report(path: &str) -> Result<String, String> {
    use std::fmt::Write as _;
    let mut out = String::new();
    let map = tot_kit::parse_mp_file(path).map_err(|e| e.to_string())?;
    let _ = writeln!(out, "file          : {path}");
    let _ = writeln!(out, "layout        : {:?}", map.format);
    let _ = writeln!(out, "dimensions    : {} x {}", map.width, map.height);
    let _ = writeln!(out, "declared real : {:?}", map.declared_real_tiles);
    let _ = writeln!(out, "counted real  : {}", map.real_tile_count());
    let _ = writeln!(out, "tail bytes    : {}", map.tail_bytes);
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
    let _ = writeln!(out, "terrain top   :");
    for (t, n) in histogram.into_iter().take(6) {
        if n > 0 {
            let _ = writeln!(out, "  id {t:3} : {n}");
        }
    }
    Ok(out)
}

fn cmd_events(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("usage: totctl events <Events.txt>");
        return ExitCode::FAILURE;
    };
    match events_report(path) {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// Events summary shared by `totctl events` and the MCP `events_summary` tool
/// (#250); the text is byte-identical to the CLI output.
fn events_report(path: &str) -> Result<String, String> {
    use std::fmt::Write as _;
    let mut out = String::new();
    let src = std::fs::read_to_string(path).map_err(|_| format!("cannot read {path}"))?;
    let f = tot_kit::parse_events(&src).map_err(|e| format!("parse error: {e:?}"))?;
    use std::collections::BTreeMap;
    let mut triggers: BTreeMap<&str, usize> = BTreeMap::new();
    let mut actions: BTreeMap<&str, usize> = BTreeMap::new();
    for e in &f.events {
        *triggers.entry(e.trigger.keyword.as_str()).or_default() += 1;
        for a in &e.actions {
            *actions.entry(a.keyword.as_str()).or_default() += 1;
        }
    }
    let _ = writeln!(out, "events       : {}", f.events.len());
    let _ = writeln!(out, "triggers     :");
    for (k, n) in &triggers {
        let _ = writeln!(out, "  {k:<18} {n}");
    }
    let _ = writeln!(out, "actions      :");
    for (k, n) in &actions {
        let _ = writeln!(out, "  {k:<18} {n}");
    }
    Ok(out)
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
    let mut builtin = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--turns" => turns = it.next().and_then(|v| v.parse().ok()).unwrap_or(turns),
            "--seed" => seed = it.next().and_then(|v| v.parse().ok()).unwrap_or(seed),
            "--log" => out = it.next().cloned(),
            "--ai" => ai = true,
            "--ai-v1" => ai_v1 = true,
            "--builtin" => builtin = true,
            _ if path.is_none() => path = Some(a.clone()),
            _ => {
                eprintln!("error: unexpected argument {a}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(path) = path else {
        eprintln!(
            "usage: totctl play <file.sav|scn> [--turns N] [--seed S] [--log OUT] [--ai | --ai-v1 | --builtin]"
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
            g.players[civ] = if builtin {
                Some(Box::new(engine::ai::BuiltInAi::new(
                    &g.rules, seed, civ as u8,
                )?))
            } else if ai_v1 {
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
        let mut builtin = true;
        let mut it = args.iter();
        while let Some(a) = it.next() {
            match a.as_str() {
                "--ai-v1" => {
                    ai_v1 = true;
                    builtin = false;
                }
                "--ai" => {
                    ai_v1 = false;
                    builtin = false;
                }
                "--builtin" => builtin = true,
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
        let path = path.ok_or("usage: totctl observe <save> [--turns N] [--seed S] [--port P] [--replay LOG] [--stream OUT] [--log OUT] [--ai | --ai-v1 | --builtin]")?;
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
                game.players[civ] = if builtin {
                    Some(Box::new(engine::ai::BuiltInAi::new(
                        &game.rules,
                        seed,
                        civ as u8,
                    )?))
                } else if ai_v1 {
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
