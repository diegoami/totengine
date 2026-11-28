//! Headless experiment manifests, format 1 (#203, generated-world slice).
//! Design and usage: docs/experiment-manifests.md.
//!
//! A manifest declares one start (a generated world from a Rules file), the
//! policy, the number of rounds, a list of seeds (random variation) and a list
//! of named variants (deliberate parameter changes). `check` validates it and
//! expands the runs; `run` executes every run headlessly, writes one verifiable
//! `.totrec` recording per run and deterministic summaries, and never
//! overwrites existing output. Scenario starts wait for admitted scenario
//! entry (#197); mixed per-civilization policies are rejected because
//! campaigns run one policy for every civilization.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use engine::campaign::{AiPolicy, Campaign, Outcome, MAX_ROUNDS};
use engine::newgame::NewGameConfig;
use serde::Deserialize;
use serde_json::{json, Value};
use tot_record::{Bundle, Journal, Kind, StopReason, MAX_BYTES};

pub(crate) const FORMAT: &str = "totengine-experiment 1";
pub(crate) const SUMMARY_FORMAT: &str = "totengine-experiment-summary 1";
/// Runs one manifest may expand to.
const MAX_RUNS: usize = 256;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: String,
    name: String,
    /// Optional engine version pin; must equal this build when given.
    #[serde(default)]
    engine: Option<String>,
    start: Start,
    #[serde(default)]
    policies: Policies,
    rounds: u32,
    seeds: Vec<u32>,
    #[serde(default)]
    variants: Vec<Variant>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Start {
    Generated {
        /// Rules.txt, relative to the manifest's folder.
        rules: String,
        /// Optional `<len> <fnv>` pin of the canonical Rules.
        #[serde(default)]
        rules_identity: Option<String>,
        #[serde(default)]
        width: Option<u16>,
        #[serde(default)]
        height: Option<u16>,
        #[serde(default)]
        land_percent: Option<u8>,
        #[serde(default)]
        civilizations: Option<u8>,
        #[serde(default)]
        difficulty: Option<u8>,
    },
    Scenario {
        #[allow(dead_code)]
        path: String,
    },
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Policies {
    #[serde(default)]
    default: Option<String>,
    /// Per-civilization policies; only accepted when they all equal the
    /// default, since campaigns run one policy for every civilization.
    #[serde(default)]
    per_civ: BTreeMap<String, String>,
}

/// A deliberate change from the base start. Fields left out keep the base.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Variant {
    name: String,
    #[serde(default)]
    width: Option<u16>,
    #[serde(default)]
    height: Option<u16>,
    #[serde(default)]
    land_percent: Option<u8>,
    #[serde(default)]
    civilizations: Option<u8>,
    #[serde(default)]
    difficulty: Option<u8>,
    #[serde(default)]
    policy: Option<String>,
}

/// One expanded run.
pub(crate) struct Run {
    id: String,
    variant: String,
    seed: u32,
    /// Parameters this run's variant deliberately changed from the base.
    changed: BTreeMap<&'static str, Value>,
    config: NewGameConfig,
    policy: AiPolicy,
}

/// A validated manifest with its runs.
pub(crate) struct Plan {
    name: String,
    manifest_identity: String,
    rules_path: PathBuf,
    rules: tot_kit::Rules,
    rules_identity: String,
    rounds: u32,
    runs: Vec<Run>,
}

fn policy(id: &str) -> Result<AiPolicy, String> {
    AiPolicy::parse(id).map_err(|e| format!("policy {id:?}: {e}"))
}

/// Parse and validate a manifest and expand its runs. Every run's setup is
/// constructed once, so an invalid world or policy fails here, not mid-batch.
pub(crate) fn plan(manifest_path: &Path) -> Result<Plan, String> {
    let text = std::fs::read_to_string(manifest_path)
        .map_err(|e| format!("read {}: {e}", manifest_path.display()))?;
    let m: Manifest = serde_json::from_str(&text).map_err(|e| format!("invalid manifest: {e}"))?;
    if m.format != FORMAT {
        return Err(format!(
            "unsupported manifest format {:?}; this build reads {FORMAT:?}",
            m.format
        ));
    }
    if m.name.is_empty() || m.name.len() > 80 || m.name.chars().any(char::is_control) {
        return Err("name must be a non-empty single line of at most 80 bytes".into());
    }
    if let Some(engine) = &m.engine {
        if engine != env!("CARGO_PKG_VERSION") {
            return Err(format!(
                "manifest pins engine {engine}, this build is {}",
                env!("CARGO_PKG_VERSION")
            ));
        }
    }
    let (rules_rel, pin, base) = match m.start {
        Start::Scenario { .. } => return Err(
            "scenario starts need admitted scenario entry (#197); only generated starts run now"
                .into(),
        ),
        Start::Generated {
            rules,
            rules_identity,
            width,
            height,
            land_percent,
            civilizations,
            difficulty,
        } => {
            let mut config = NewGameConfig::default();
            if let Some(v) = width {
                config.world.width = v;
            }
            if let Some(v) = height {
                config.world.height = v;
            }
            if let Some(v) = land_percent {
                config.world.land_percent = v;
            }
            if let Some(v) = civilizations {
                config.civilizations = v;
            }
            if let Some(v) = difficulty {
                config.difficulty = v;
            }
            (rules, rules_identity, config)
        }
    };
    let base_policy = policy(m.policies.default.as_deref().unwrap_or("builtin-1"))?;
    for (civ, id) in &m.policies.per_civ {
        if policy(id)? != base_policy {
            return Err(format!(
                "per-civilization policy for civ {civ} ({id}) differs from the default ({}); \
                 campaigns run one policy for every civilization, so mixed policies are not supported yet",
                base_policy.id()
            ));
        }
    }
    if m.rounds == 0 || m.rounds > MAX_ROUNDS {
        return Err(format!("rounds must be 1..={MAX_ROUNDS}"));
    }
    if m.seeds.is_empty() {
        return Err("seeds must list at least one seed".into());
    }
    let mut unique = m.seeds.clone();
    unique.sort_unstable();
    unique.dedup();
    if unique.len() != m.seeds.len() {
        return Err("seeds must not repeat".into());
    }
    let mut names: Vec<&str> = m.variants.iter().map(|v| v.name.as_str()).collect();
    names.sort_unstable();
    if names.windows(2).any(|w| w[0] == w[1]) || names.contains(&"base") {
        return Err("variant names must be unique and must not be \"base\"".into());
    }
    if m.variants.iter().any(|v| {
        v.name.is_empty()
            || !v
                .name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    }) {
        return Err("variant names use letters, digits, - and _ only".into());
    }
    let variants: Vec<Variant> = if m.variants.is_empty() {
        vec![Variant {
            name: "base".into(),
            width: None,
            height: None,
            land_percent: None,
            civilizations: None,
            difficulty: None,
            policy: None,
        }]
    } else {
        m.variants.clone()
    };
    if variants.len() * m.seeds.len() > MAX_RUNS {
        return Err(format!(
            "{} variants x {} seeds exceeds {MAX_RUNS} runs",
            variants.len(),
            m.seeds.len()
        ));
    }
    let dir = manifest_path.parent().unwrap_or(Path::new("."));
    let rules_path = dir.join(&rules_rel);
    let bytes =
        std::fs::read(&rules_path).map_err(|e| format!("read {}: {e}", rules_path.display()))?;
    let rules = tot_kit::parse_rules(&tot_kit::latin1(&bytes));
    let rules_identity = tot_record::rules_identity(&rules);
    if let Some(pin) = pin {
        if pin != rules_identity {
            return Err(format!(
                "Rules identity differs: manifest pins {pin}, {} is {rules_identity}",
                rules_path.display()
            ));
        }
    }
    let mut runs = Vec::new();
    for variant in &variants {
        let mut changed = BTreeMap::new();
        let mut config = base;
        if let Some(v) = variant.width {
            config.world.width = v;
            changed.insert("width", json!(v));
        }
        if let Some(v) = variant.height {
            config.world.height = v;
            changed.insert("height", json!(v));
        }
        if let Some(v) = variant.land_percent {
            config.world.land_percent = v;
            changed.insert("land_percent", json!(v));
        }
        if let Some(v) = variant.civilizations {
            config.civilizations = v;
            changed.insert("civilizations", json!(v));
        }
        if let Some(v) = variant.difficulty {
            config.difficulty = v;
            changed.insert("difficulty", json!(v));
        }
        let run_policy = match &variant.policy {
            Some(id) => {
                changed.insert("policy", json!(id));
                policy(id)?
            }
            None => base_policy,
        };
        for &seed in &m.seeds {
            let mut config = config;
            config.world.seed = seed;
            config.gameplay_seed = seed;
            let id = format!("{}-s{seed}", variant.name);
            // Construct the start now: invalid setups fail validation.
            Campaign::with_policy(config, rules.clone(), run_policy)
                .map_err(|e| format!("run {id}: {e}"))?;
            runs.push(Run {
                id,
                variant: variant.name.clone(),
                seed,
                changed: changed.clone(),
                config,
                policy: run_policy,
            });
        }
    }
    Ok(Plan {
        name: m.name,
        manifest_identity: tot_record::identity(&text),
        rules_path,
        rules,
        rules_identity,
        rounds: m.rounds,
        runs,
    })
}

impl Plan {
    /// `totctl experiment check` output.
    pub(crate) fn describe(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "experiment  : {} ({FORMAT})", self.name);
        let _ = writeln!(out, "manifest    : {}", self.manifest_identity);
        let _ = writeln!(out, "engine      : {}", env!("CARGO_PKG_VERSION"));
        let _ = writeln!(
            out,
            "rules       : {} ({})",
            self.rules_identity,
            self.rules_path
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default()
        );
        let _ = writeln!(out, "rounds      : {}", self.rounds);
        let _ = writeln!(out, "runs        : {}", self.runs.len());
        for run in &self.runs {
            let _ = writeln!(
                out,
                "  {:<24} seed {:<10} policy {:<9} setup {}  changed {}",
                run.id,
                run.seed,
                run.policy.id(),
                run.config.descriptor(),
                changed_text(&run.changed)
            );
        }
        out
    }

    /// Create an independent output directory, optionally using verified
    /// recordings from a prior attempt. The identity is written first so
    /// recovery can work even when the summary has not been written yet.
    fn run(
        &self,
        out_dir: &Path,
        limit: u32,
        prior: Option<&Path>,
        every: Option<u32>,
    ) -> Result<String, String> {
        if !(1..=self.rounds).contains(&limit) {
            return Err(format!("round limit must be 1..={}", self.rounds));
        }
        let identity = format!(
            "totengine-experiment-batch 1\nengine {}\nmanifest {}\nrules {}\n",
            env!("CARGO_PKG_VERSION"),
            self.manifest_identity,
            self.rules_identity
        );
        if let Some(dir) = prior {
            let saved = read_batch_file(&dir.join("batch.identity"))?;
            if saved.as_deref() != Some(identity.as_str()) {
                return Err("prior batch identity is missing or differs from this engine, manifest or Rules".into());
            }
        }
        fs::create_dir(out_dir).map_err(|e| {
            format!(
                "create {}: {e} (outputs are never overwritten; choose a new folder)",
                out_dir.display()
            )
        })?;
        tot_record::write_new(&out_dir.join("batch.identity"), &identity)?;
        let mut results = Vec::new();
        for run in &self.runs {
            results.push(match self.execute(run, out_dir, limit, prior, every) {
                Ok(result) => result,
                Err(error) => json!({"id": run.id, "variant": run.variant, "seed": run.seed,
                                     "status": "failed", "error": error}),
            });
        }
        let summary = json!({
            "format": SUMMARY_FORMAT,
            "experiment": self.name,
            "manifest": self.manifest_identity,
            "engine": env!("CARGO_PKG_VERSION"),
            "rules": self.rules_identity,
            "rounds": self.rounds,
            "runs": results,
        });
        let json_text = serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())? + "\n";
        tot_record::write_new(&out_dir.join("summary.json"), &json_text)?;
        let text = summary_text(&summary);
        tot_record::write_new(&out_dir.join("summary.txt"), &text)?;
        Ok(text)
    }

    fn execute(
        &self,
        run: &Run,
        out_dir: &Path,
        limit: u32,
        prior: Option<&Path>,
        every: Option<u32>,
    ) -> Result<Value, String> {
        let file = format!("{}.totrec", run.id);
        let journal_name = format!("{}.totjournal", run.id);
        let producer = self.producer(run);
        let mut campaign = match prior
            .map(|dir| self.resume_run(run, dir, &file, &journal_name))
            .transpose()?
            .flatten()
        {
            Some(campaign) => campaign,
            None => Campaign::with_policy(run.config, self.rules.clone(), run.policy)?,
        };
        let journal_path = out_dir.join(journal_name);
        let journal_written = if let Some(every) =
            every.filter(|_| campaign.rounds() < limit && campaign.outcome() == Outcome::TurnLimit)
        {
            let manifest =
                Bundle::campaign(&campaign, StopReason::Interrupted, &producer, None).manifest;
            let mut journal = Journal::create(&journal_path, &manifest)?;
            record_progress(&mut journal, &journal_path, &campaign)?;
            while campaign.rounds() < limit {
                let outcome = campaign.advance(every.min(limit - campaign.rounds()))?;
                if outcome != Outcome::TurnLimit || campaign.rounds() == limit {
                    break;
                }
                record_progress(&mut journal, &journal_path, &campaign)?;
            }
            true
        } else {
            campaign.advance(limit - campaign.rounds())?;
            false
        };
        let stop = self.stop(&campaign);
        let text = Bundle::campaign(&campaign, stop, &producer, None).encode()?;
        tot_record::write_new(&out_dir.join(&file), &text)?;
        if journal_written {
            fs::remove_file(&journal_path)
                .map_err(|error| format!("remove published run's journal: {error}"))?;
        }
        let world = &campaign.game.world;
        Ok(json!({
            "id": run.id,
            "variant": run.variant,
            "seed": run.seed,
            "changed": run.changed,
            "status": if stop == StopReason::Stopped { "paused" } else { "completed" },
            "policy": run.policy.id(),
            "setup": campaign.config().descriptor(),
            "rounds": campaign.rounds(),
            "stop": format!("{stop:?}"),
            "fingerprint": format!("{:016x}", world.checksum()),
            "living_civs": world.living_civs(),
            "cities": world.cities.iter().filter(|c| c.city_id != 0).count(),
            "units": world.units.iter().filter(|u| u.id != 0).count(),
            "recording": file,
            "recording_identity": tot_record::identity(&text),
        }))
    }

    fn producer(&self, run: &Run) -> String {
        format!("totctl experiment {}: run {}", self.name, run.id)
    }

    fn resume_run(
        &self,
        run: &Run,
        dir: &Path,
        file: &str,
        journal: &str,
    ) -> Result<Option<Campaign>, String> {
        if let Some(text) = read_batch_file(&dir.join(file))? {
            return self
                .restore_run(run, Bundle::decode(&text)?, false)
                .map(Some);
        }
        if let Some(text) = read_batch_file(&dir.join(journal))? {
            let recovered = tot_record::recover(&text)?;
            return self.restore_run(run, recovered.bundle, true).map(Some);
        }
        Ok(None)
    }

    fn restore_run(&self, run: &Run, bundle: Bundle, recovered: bool) -> Result<Campaign, String> {
        let saved = &bundle.manifest;
        let producer = self.producer(run);
        if saved.kind != Kind::Campaign
            || saved.engine != env!("CARGO_PKG_VERSION")
            || saved.rules != self.rules_identity
            || saved.setup != run.config.descriptor()
            || saved.policy != run.policy.id()
            || saved.producer != producer
            || !(u32::from(!recovered)..=self.rounds).contains(&saved.progress)
        {
            return Err("recording does not match this batch run".into());
        }
        let checkpoint = bundle
            .gameplay
            .as_deref()
            .ok_or("batch recording has no checkpoint")?;
        let campaign = Campaign::restore(self.rules.clone(), checkpoint)?;
        let stop = if recovered {
            StopReason::Interrupted
        } else {
            self.stop(&campaign)
        };
        if Bundle::campaign(&campaign, stop, &producer, None) != bundle {
            return Err("recording metadata differs from the verified batch checkpoint".into());
        }
        Ok(campaign)
    }

    fn read_run(&self, path: &Path) -> Result<Campaign, String> {
        let id = path.file_stem().and_then(|stem| stem.to_str());
        let run = self
            .runs
            .iter()
            .find(|run| Some(run.id.as_str()) == id)
            .ok_or_else(|| format!("{} does not name a run in the manifest", path.display()))?;
        let text = read_batch_file(path)?
            .ok_or_else(|| format!("recording missing: {}", path.display()))?;
        self.restore_run(run, Bundle::decode(&text)?, false)
            .map_err(|e| format!("{}: {e}", path.display()))
    }

    fn compare(&self, left: &Path, right: &Path) -> Result<String, String> {
        let left = self.read_run(left)?;
        let right = self.read_run(right)?;
        let mut left_lines = left.game.log().lines();
        let mut right_lines = right.game.log().lines();
        let mut round = 0;
        let mut line = 0;
        loop {
            line += 1;
            match (left_lines.next(), right_lines.next()) {
                (Some(a), Some(b)) if a == b => {
                    if a.starts_with("R ") {
                        round += 1;
                    }
                }
                (Some(a), Some(b)) => return Ok(format!(
                    "First divergence at log line {line} after {round} completed rounds\nleft: {a}\nright: {b}\n"
                )),
                (None, None) => return Ok(format!("Identical gameplay logs through {round} rounds\n")),
                _ => return Ok(format!(
                    "Matching gameplay prefix through {round} rounds (left: {}, right: {})\n",
                    left.rounds(), right.rounds()
                )),
            }
        }
    }

    fn stop(&self, campaign: &Campaign) -> StopReason {
        let outcome = StopReason::from(campaign.outcome());
        if outcome == StopReason::TurnLimit && campaign.rounds() < self.rounds {
            StopReason::Stopped
        } else {
            outcome
        }
    }
}

fn record_progress(journal: &mut Journal, path: &Path, campaign: &Campaign) -> Result<(), String> {
    let checkpoint = campaign.checkpoint();
    let size = fs::metadata(path)
        .map_err(|error| format!("read journal metadata: {error}"))?
        .len();
    // The journal line adds the round, payload length, checksum and delimiters.
    // Keep each published prefix within the reader's recovery budget.
    if size + checkpoint.len() as u64 + 128 > MAX_BYTES as u64 {
        return Err(
            "experiment journal exceeds its recovery budget; use a larger checkpoint interval"
                .into(),
        );
    }
    journal.record(campaign.rounds(), &checkpoint)
}

// A missing run is started from scratch. Unreadable or non-regular files
// are errors, so damaged evidence is never treated as an unstarted run.
fn read_batch_file(path: &Path) -> Result<Option<String>, String> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() => tot_record::read_limited(path).map(Some),
        Ok(_) => Err(format!("{} must be a regular file", path.display())),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("read {}: {e}", path.display())),
    }
}

fn changed_text(changed: &BTreeMap<&'static str, Value>) -> String {
    if changed.is_empty() {
        "-".into()
    } else {
        changed
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(",")
    }
}

fn summary_text(summary: &Value) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "experiment {} — engine {}, rules {}, {} rounds",
        summary["experiment"].as_str().unwrap_or_default(),
        summary["engine"].as_str().unwrap_or_default(),
        summary["rules"].as_str().unwrap_or_default(),
        summary["rounds"]
    );
    let _ = writeln!(
        out,
        "{:<24} {:<10} {:<9} {:>6} {:>6} {:>6}  {:<18} stop",
        "run", "status", "policy", "rounds", "cities", "units", "fingerprint"
    );
    for r in summary["runs"].as_array().into_iter().flatten() {
        if r["status"] != "failed" {
            let _ = writeln!(
                out,
                "{:<24} {:<10} {:<9} {:>6} {:>6} {:>6}  {:<18} {}",
                r["id"].as_str().unwrap_or_default(),
                r["status"].as_str().unwrap_or_default(),
                r["policy"].as_str().unwrap_or_default(),
                r["rounds"],
                r["cities"],
                r["units"],
                r["fingerprint"].as_str().unwrap_or_default(),
                r["stop"].as_str().unwrap_or_default()
            );
        } else {
            let _ = writeln!(
                out,
                "{:<24} FAILED     {}",
                r["id"].as_str().unwrap_or_default(),
                r["error"].as_str().unwrap_or_default()
            );
        }
    }
    out
}

fn batch_options(args: &[String], staged: bool) -> Result<(Option<u32>, Option<u32>), String> {
    let mut limit = None;
    let mut every = None;
    for option in args.chunks(2) {
        let [flag, value] = option else {
            return Err(format!("missing value for {}", option[0]));
        };
        let target = match flag.as_str() {
            "--round-limit" if staged => &mut limit,
            "--checkpoint-every" => &mut every,
            _ => return Err(format!("unknown batch option {flag}")),
        };
        if target.is_some() {
            return Err(format!("duplicate batch option {flag}"));
        }
        let value = value
            .parse::<u32>()
            .ok()
            .filter(|&n| n > 0)
            .ok_or_else(|| format!("{flag} requires a positive integer"))?;
        *target = Some(value);
    }
    Ok((limit, every))
}

/// `totctl experiment check|run|resume|compare`.
pub(crate) fn cmd_experiment(args: &[String]) -> std::process::ExitCode {
    let usage = "usage: totctl experiment check <manifest.json>\n       totctl experiment run <manifest.json> --out <new-folder> [--round-limit N] [--checkpoint-every N]\n       totctl experiment resume <manifest.json> --from <prior-folder> --out <new-folder> [--checkpoint-every N]\n       totctl experiment compare <manifest.json> <left.totrec> <right.totrec>";
    let result = match args {
        [cmd, manifest] if cmd == "check" => plan(Path::new(manifest)).map(|p| p.describe()),
        [cmd, manifest, flag, out, options @ ..] if cmd == "run" && flag == "--out" => {
            batch_options(options, true).and_then(|(limit, every)| {
                plan(Path::new(manifest))
                    .and_then(|p| p.run(Path::new(out), limit.unwrap_or(p.rounds), None, every))
            })
        }
        [cmd, manifest, from_flag, from, out_flag, out, options @ ..]
            if cmd == "resume" && from_flag == "--from" && out_flag == "--out" =>
        {
            batch_options(options, false).and_then(|(_, every)| {
                plan(Path::new(manifest))
                    .and_then(|p| p.run(Path::new(out), p.rounds, Some(Path::new(from)), every))
            })
        }
        [cmd, manifest, left, right] if cmd == "compare" => {
            plan(Path::new(manifest)).and_then(|p| p.compare(Path::new(left), Path::new(right)))
        }
        _ => Err(usage.into()),
    };
    match result {
        Ok(text) => {
            print!("{text}");
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
