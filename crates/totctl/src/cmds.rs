//! Shared command implementations used by both the `totctl` CLI and the
//! MCP server (`mcp.rs`). Both surfaces call these functions, so a tool call
//! and the equivalent CLI invocation produce identical bytes. Side effects
//! (file writes) happen here, after validation, with `create_new` semantics.

use std::fmt::Write as _;

/// `totctl worldgen <out.mp> [--seed S] [--width W] [--height H] [--land-percent P]`
pub fn worldgen(path: &str, config: engine::worldgen::WorldConfig) -> Result<String, String> {
    let world = engine::worldgen::generate(config)?;
    // create_new prevents a typo from overwriting a user's original map.
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("create {path}: {e}"))?;
    file.write_all(&world.mp_bytes())
        .map_err(|e| format!("write {path}: {e}"))?;
    let mut out = String::new();
    writeln!(
        out,
        "generator     : engine-designed v{} (not an original-algorithm port)",
        engine::worldgen::ALGORITHM_VERSION
    )
    .unwrap();
    writeln!(out, "seed          : {}", config.seed).unwrap();
    writeln!(
        out,
        "dimensions    : {} x {} logical",
        config.width, config.height
    )
    .unwrap();
    writeln!(out, "terrain counts: {:?}", world.terrain_histogram()).unwrap();
    writeln!(
        out,
        "river tiles   : {}",
        world.river.iter().filter(|&&r| r).count()
    )
    .unwrap();
    writeln!(out, "map           : {path}").unwrap();
    Ok(out)
}

/// Parsed `totctl campaign` invocation, shared with the `campaign` MCP tool.
pub struct CampaignArgs {
    pub rules_path: String,
    pub config: engine::newgame::NewGameConfig,
    /// True when the caller passed any setup option (policy/seed/size/...).
    pub changed_setup: bool,
    pub policy: engine::campaign::AiPolicy,
    pub turns: u32,
    pub checkpoint: Option<String>,
    pub resume: Option<String>,
    pub log: Option<String>,
}

/// `totctl campaign <Rules.txt> [options]`: run or resume a generated-world
/// campaign with an existing policy, optionally writing a checkpoint and the
/// canonical order log. `--resume` re-drives the recorded transcript and
/// requires byte-identical playback before any new round is played, so
/// `--resume X --turns 0` is also the generated-campaign replay path.
pub fn campaign(args: &CampaignArgs) -> Result<String, String> {
    use std::io::Write as _;
    if args.resume.is_some() && args.changed_setup {
        return Err("resume uses the checkpoint setup; omit setup options".into());
    }
    if args.checkpoint.is_some() && args.checkpoint == args.log {
        return Err("checkpoint and log output paths must differ".into());
    }
    for path in [args.checkpoint.as_ref(), args.log.as_ref()]
        .into_iter()
        .flatten()
    {
        if std::path::Path::new(path).exists() {
            return Err(format!("output already exists: {path}"));
        }
    }
    let rules_path = &args.rules_path;
    let bytes = std::fs::read(rules_path).map_err(|e| format!("read {rules_path}: {e}"))?;
    let rules = tot_kit::parse_rules(&tot_kit::latin1(&bytes));
    let mut campaign = if let Some(path) = &args.resume {
        let saved = std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
        engine::campaign::Campaign::restore(rules, &saved)?
    } else {
        engine::campaign::Campaign::with_policy(args.config, rules, args.policy)?
    };
    let outcome = campaign.advance(args.turns)?;
    for (path, contents) in [
        (&args.checkpoint, campaign.checkpoint()),
        (&args.log, campaign.game.log().to_string()),
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
    let world = &campaign.game.world;
    let mut out = String::new();
    writeln!(
        out,
        "generated campaign: engine-designed world v{}, initialization policy v{}",
        engine::worldgen::ALGORITHM_VERSION,
        engine::newgame::INITIALIZATION_VERSION
    )
    .unwrap();
    writeln!(out, "setup       : {}", campaign.config().descriptor()).unwrap();
    writeln!(out, "rounds      : {}", campaign.rounds()).unwrap();
    writeln!(out, "turn / year : {} / {}", world.turn, world.year).unwrap();
    writeln!(out, "living civs : {:?}", world.living_civs()).unwrap();
    writeln!(
        out,
        "cities/units: {} / {}",
        world.cities.iter().filter(|c| c.city_id != 0).count(),
        world.units.iter().filter(|u| u.id != 0).count()
    )
    .unwrap();
    writeln!(out, "fingerprint : {:016x}", world.checksum()).unwrap();
    writeln!(
        out,
        "outcome     : {outcome:?} (observed state; original victory rules not claimed)"
    )
    .unwrap();
    Ok(out)
}
