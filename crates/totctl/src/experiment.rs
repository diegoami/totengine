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
use std::path::{Path, PathBuf};

use engine::campaign::{AiPolicy, Campaign, MAX_ROUNDS};
use engine::newgame::NewGameConfig;
use serde::Deserialize;
use serde_json::{json, Value};

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

    /// Execute every run into `out_dir`, which must not exist yet. Each run
    /// writes `<id>.totrec`; `summary.json` and `summary.txt` follow. A run
    /// that fails is reported as failed, never as a result.
    pub(crate) fn run(&self, out_dir: &Path) -> Result<String, String> {
        std::fs::create_dir(out_dir).map_err(|e| {
            format!(
                "create {}: {e} (outputs are never overwritten; choose a new folder)",
                out_dir.display()
            )
        })?;
        let mut results = Vec::new();
        for run in &self.runs {
            results.push(match self.execute(run, out_dir) {
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

    fn execute(&self, run: &Run, out_dir: &Path) -> Result<Value, String> {
        let mut campaign = Campaign::with_policy(run.config, self.rules.clone(), run.policy)?;
        let outcome = campaign.advance(self.rounds)?;
        let stop = tot_record::StopReason::from(outcome);
        let bundle = tot_record::Bundle::campaign(
            &campaign,
            stop,
            &format!("totctl experiment {}: run {}", self.name, run.id),
            None,
        );
        let text = bundle.encode()?;
        let file = format!("{}.totrec", run.id);
        tot_record::write_new(&out_dir.join(&file), &text)?;
        let world = &campaign.game.world;
        Ok(json!({
            "id": run.id,
            "variant": run.variant,
            "seed": run.seed,
            "changed": run.changed,
            "status": "completed",
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
        if r["status"] == "completed" {
            let _ = writeln!(
                out,
                "{:<24} {:<10} {:<9} {:>6} {:>6} {:>6}  {:<18} {}",
                r["id"].as_str().unwrap_or_default(),
                "completed",
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

/// `totctl experiment check|run`.
pub(crate) fn cmd_experiment(args: &[String]) -> std::process::ExitCode {
    let usage = "usage: totctl experiment check <manifest.json>\n       totctl experiment run <manifest.json> --out <new-folder>";
    let result = match args {
        [cmd, manifest] if cmd == "check" => plan(Path::new(manifest)).map(|p| p.describe()),
        [cmd, manifest, flag, out] if cmd == "run" && flag == "--out" => {
            plan(Path::new(manifest)).and_then(|p| p.run(Path::new(out)))
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
