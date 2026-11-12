//! `totctl mcp` — a stdio MCP (Model Context Protocol) server over the
//! existing inspection and run commands (#250, headless stage 1).
//!
//! Scope of this slice (decision 0008, "MCP agent interface"):
//! read-only format/scenario inspection and runs with existing policies,
//! exposed as MCP tools whose text results are byte-identical to the
//! equivalent `totctl` invocation. No client attach, no observer/recording
//! queries, no acting agents, no original assets — all later, gated slices.
//!
//! Transport: newline-delimited JSON-RPC 2.0 on stdin/stdout (the MCP stdio
//! transport). Diagnostics go to stderr; stdout carries protocol messages
//! only. The server never writes to the filesystem: campaign checkpoints and
//! logs are returned inline instead of through `--checkpoint`/`--log`.
//!
//! Filesystem reads are bounded to configured roots: `--root PATH` (repeatable)
//! or, when no root is given, `CIVTOT_HOME` if it points at a directory.
//! Requested paths are canonicalized and must land inside a root.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Deserialize;
use serde_json::{json, Value};

/// Protocol versions this server speaks; the first is the preferred one.
const SUPPORTED_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

pub(crate) fn cmd_mcp(args: &[String]) -> ExitCode {
    let mut roots: Vec<PathBuf> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--root" => match it.next() {
                Some(p) => match std::fs::canonicalize(p) {
                    Ok(c) if c.is_dir() => roots.push(c),
                    Ok(_) => {
                        eprintln!("error: --root {p} is not a directory");
                        return ExitCode::from(2);
                    }
                    Err(e) => {
                        eprintln!("error: --root {p}: {e}");
                        return ExitCode::from(2);
                    }
                },
                None => {
                    eprintln!("error: --root needs a path");
                    return ExitCode::from(2);
                }
            },
            _ => {
                eprintln!("usage: totctl mcp [--root PATH]...");
                return ExitCode::from(2);
            }
        }
    }
    if roots.is_empty() {
        if let Some(home) = std::env::var_os("CIVTOT_HOME") {
            match std::fs::canonicalize(Path::new(&home)) {
                Ok(c) if c.is_dir() => roots.push(c),
                _ => {
                    eprintln!("warning: CIVTOT_HOME is not a directory; no roots configured");
                }
            }
        }
    }
    for r in &roots {
        eprintln!("totctl mcp: root {}", r.display());
    }
    if roots.is_empty() {
        eprintln!("totctl mcp: no roots configured; file tools will reject every path");
    }
    let server = Server::new(roots);
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    run(&server, &mut stdin.lock(), &mut stdout.lock())
}

/// The serve loop: one JSON-RPC message per line, responses on stdout.
fn run<R: BufRead, W: Write>(server: &Server, reader: &mut R, writer: &mut W) -> ExitCode {
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("error: read stdin: {e}");
                return ExitCode::FAILURE;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = server.handle(&line) {
            if writeln!(writer, "{response}")
                .and_then(|()| writer.flush())
                .is_err()
            {
                // Client went away (broken pipe); nothing left to do.
                return ExitCode::SUCCESS;
            }
        }
    }
    ExitCode::SUCCESS
}

struct Server {
    roots: Vec<PathBuf>,
    /// Live in-memory campaign sessions (`session_*` tools). Lost when the
    /// server exits; `session_checkpoint` is how a client keeps one.
    sessions: std::cell::RefCell<Sessions>,
}

#[derive(Default)]
struct Sessions {
    next: u32,
    open: std::collections::BTreeMap<u32, Session>,
}

struct Session {
    campaign: engine::campaign::Campaign,
    /// Captures for `session_observe`; events are those since the last call.
    recorder: engine::observer::Recorder,
}

/// Concurrently open sessions per server.
const MAX_SESSIONS: usize = 4;
/// Rounds one `session_advance` call may run.
const MAX_SESSION_ADVANCE: u32 = 1000;
/// Lines one `recording_log` call may return.
const MAX_LOG_LINES: usize = 5000;

impl Server {
    fn new(roots: Vec<PathBuf>) -> Self {
        Server {
            roots,
            sessions: Default::default(),
        }
    }

    /// One line in, an optional JSON-RPC response out (notifications get none).
    fn handle(&self, line: &str) -> Option<String> {
        let msg: Value = match serde_json::from_str(line) {
            Ok(m) => m,
            Err(e) => {
                return Some(
                    json!({"jsonrpc": "2.0", "id": null,
                           "error": {"code": -32700, "message": format!("parse error: {e}")}})
                    .to_string(),
                );
            }
        };
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let result = match method {
            "initialize" => Ok(self.initialize(msg.get("params"))),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tools()})),
            "tools/call" => self.call_tool(msg.get("params")),
            // Notifications (initialized, cancelled, ...) are never answered.
            _ if id.is_none() => return None,
            _ => Err((-32601, format!("method not found: {method}"))),
        };
        let id = id?; // notifications get no response
        let response = match result {
            Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
            Err((code, message)) => {
                json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
            }
        };
        Some(response.to_string())
    }

    fn initialize(&self, params: Option<&Value>) -> Value {
        let requested = params
            .and_then(|p| p.get("protocolVersion"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let protocol = if SUPPORTED_VERSIONS.contains(&requested) {
            requested
        } else {
            SUPPORTED_VERSIONS[0]
        };
        json!({
            "protocolVersion": protocol,
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "totctl", "version": env!("CARGO_PKG_VERSION")},
            "instructions": "ToTEngine read-only inspection and existing-policy run tools. \
                             Results are byte-identical to the equivalent totctl invocation. \
                             Paths must resolve inside the server's configured roots. \
                             campaign_observe returns observer frames for generated campaigns. \
                             This stage-1 slice has no client attach, no acting tools and \
                             returns no original game assets."
        })
    }

    fn call_tool(&self, params: Option<&Value>) -> std::result::Result<Value, (i64, String)> {
        let params = params.ok_or((-32602, "tools/call needs params".to_string()))?;
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or((-32602, "tools/call needs a tool name".to_string()))?;
        let empty = json!({});
        let args = params.get("arguments").unwrap_or(&empty);
        match self.run_tool(name, args) {
            Ok(blocks) => Ok(json!({
                "content": blocks
                    .iter()
                    .map(|text| json!({"type": "text", "text": text}))
                    .collect::<Vec<_>>(),
                "isError": false,
            })),
            Err(message) => Ok(json!({
                "content": [{"type": "text", "text": message}],
                "isError": true,
            })),
        }
    }

    /// Runs a tool; the returned blocks are the result content in order.
    /// Every tool's first block is byte-identical to the matching CLI stdout.
    fn run_tool(&self, name: &str, args: &Value) -> std::result::Result<Vec<String>, String> {
        match name {
            "rules_summary" => {
                let a: PathArgs = parse_args(args)?;
                Ok(vec![crate::rules_report(self.path(&a.path)?)?])
            }
            "map_summary" => {
                let a: PathArgs = parse_args(args)?;
                Ok(vec![crate::mp_report(self.path(&a.path)?)?])
            }
            "events_summary" => {
                let a: PathArgs = parse_args(args)?;
                Ok(vec![crate::events_report(self.path(&a.path)?)?])
            }
            "save_summary" => {
                let a: SaveArgs = parse_args(args)?;
                let rules = self.optional_path(a.rules)?;
                Ok(vec![crate::boot_report(
                    self.path(&a.path)?,
                    rules,
                    a.turns,
                    a.seed,
                )?])
            }
            "run_turns" => {
                let a: RunArgs = parse_args(args)?;
                let rules = self.optional_path(a.rules)?;
                Ok(vec![crate::run_report(
                    self.path(&a.path)?,
                    a.turns,
                    a.seed,
                    rules,
                    a.no_rules,
                )?])
            }
            "campaign_run" => {
                let a: CampaignArgs = parse_args(args)?;
                self.campaign_run(a)
            }
            "campaign_observe" => {
                let a: ObserveArgs = parse_args(args)?;
                self.campaign_observe(a)
            }
            "recording_info" => {
                let a: PathArgs = parse_args(args)?;
                Ok(vec![crate::record_info_report(self.path(&a.path)?)?])
            }
            "recording_verify" => {
                let a: VerifyArgs = parse_args(args)?;
                let rules = self.path(&a.rules)?.to_string();
                Ok(vec![crate::record_verify_report(
                    self.path(&a.path)?,
                    &rules,
                )?])
            }
            "recording_log" => {
                let a: LogArgs = parse_args(args)?;
                self.recording_log(a)
            }
            "session_start" => {
                let a: SessionStartArgs = parse_args(args)?;
                self.session_start(a)
            }
            "session_advance" => {
                let a: SessionAdvanceArgs = parse_args(args)?;
                self.with_session(a.session, |s| {
                    if a.rounds == 0 || a.rounds > MAX_SESSION_ADVANCE {
                        return Err(format!("rounds must be 1..={MAX_SESSION_ADVANCE}"));
                    }
                    let outcome = s.campaign.advance(a.rounds)?;
                    Ok(vec![crate::campaign_summary(&s.campaign, &outcome)])
                })
            }
            "session_observe" => {
                let a: SessionObserveArgs = parse_args(args)?;
                let keep = parse_view(a.view.as_deref())?;
                self.with_session(a.session, |s| {
                    let mut frame = s.recorder.capture(&s.campaign.game);
                    if let Some(civ) = keep {
                        frame.views.retain(|v| v.civ == civ);
                    }
                    Ok(vec![frame.to_json()])
                })
            }
            "session_checkpoint" => {
                let a: SessionCheckpointArgs = parse_args(args)?;
                self.with_session(a.session, |s| {
                    match a.format.as_deref().unwrap_or("checkpoint") {
                        "checkpoint" => Ok(vec![s.campaign.checkpoint()]),
                        "recording" => Ok(vec![tot_record::Bundle::campaign(
                            &s.campaign,
                            tot_record::StopReason::Stopped,
                            "totctl mcp session",
                            None,
                        )
                        .encode()?]),
                        other => Err(format!(
                            "invalid format {other}: expected \"checkpoint\" or \"recording\""
                        )),
                    }
                })
            }
            "session_close" => {
                let a: SessionArgs = parse_args(args)?;
                match self.sessions.borrow_mut().open.remove(&a.session) {
                    Some(_) => Ok(vec![format!("closed session {}\n", a.session)]),
                    None => Err(format!("no open session {}", a.session)),
                }
            }
            "worldgen_summary" => {
                let a: WorldgenArgs = parse_args(args)?;
                let mut config = engine::worldgen::WorldConfig::default();
                if let Some(seed) = a.seed {
                    config.seed = seed;
                }
                if let Some(width) = a.width {
                    config.width = width;
                }
                if let Some(height) = a.height {
                    config.height = height;
                }
                if let Some(land) = a.land_percent {
                    config.land_percent = land;
                }
                Ok(vec![crate::worldgen_report(config)?.0])
            }
            other => Err(format!("unknown tool: {other}")),
        }
    }

    /// Build the shared campaign request exactly as `totctl campaign` would
    /// from the equivalent options.
    fn campaign_request(
        &self,
        rules: &str,
        turns: u32,
        setup: CampaignSetup,
        resume_checkpoint: Option<String>,
        observe: bool,
    ) -> std::result::Result<crate::CampaignRequest, String> {
        let mut config = engine::newgame::NewGameConfig::default();
        let mut changed_setup = false;
        let mut policy = engine::campaign::AiPolicy::BuiltIn1;
        if let Some(p) = &setup.policy {
            policy = engine::campaign::AiPolicy::parse(p)
                .map_err(|e| format!("invalid value for --policy: {e}"))?;
            changed_setup = true;
        }
        if let Some(seed) = setup.seed {
            config.world.seed = seed;
            config.gameplay_seed = seed;
            changed_setup = true;
        }
        if let Some(width) = setup.width {
            config.world.width = width;
            changed_setup = true;
        }
        if let Some(height) = setup.height {
            config.world.height = height;
            changed_setup = true;
        }
        if let Some(land) = setup.land_percent {
            config.world.land_percent = land;
            changed_setup = true;
        }
        if let Some(civs) = setup.civs {
            config.civilizations = civs;
            changed_setup = true;
        }
        if let Some(difficulty) = setup.difficulty {
            config.difficulty = difficulty;
            changed_setup = true;
        }
        Ok(crate::CampaignRequest {
            rules_path: self.path(rules)?.to_string(),
            policy,
            turns,
            config,
            changed_setup,
            resume_checkpoint,
            observe,
            record: None,
        })
    }

    fn campaign_run(&self, a: CampaignArgs) -> std::result::Result<Vec<String>, String> {
        let setup = CampaignSetup {
            policy: a.policy,
            seed: a.seed,
            width: a.width,
            height: a.height,
            land_percent: a.land_percent,
            civs: a.civs,
            difficulty: a.difficulty,
        };
        let request =
            self.campaign_request(&a.rules, a.turns, setup, a.resume_checkpoint, false)?;
        let report = crate::campaign_report(request)?;
        let mut blocks = vec![report.text];
        if a.include_checkpoint {
            blocks.push(report.checkpoint);
        }
        if a.include_log {
            blocks.push(report.log);
        }
        Ok(blocks)
    }

    /// Run (or resume) a campaign and return observer v1 frames for a window
    /// of it. With `view: "all"` each frame block is exactly one line of the
    /// `totctl campaign --stream` file; otherwise the frame keeps only the
    /// requested view (spectator = full state, or one civ's fogged view).
    fn campaign_observe(&self, a: ObserveArgs) -> std::result::Result<Vec<String>, String> {
        let keep = parse_view(a.view.as_deref())?;
        if a.count == 0 || a.count > MAX_OBSERVE_FRAMES {
            return Err(format!("count must be 1..={MAX_OBSERVE_FRAMES}"));
        }
        let setup = CampaignSetup {
            policy: a.policy,
            seed: a.seed,
            width: a.width,
            height: a.height,
            land_percent: a.land_percent,
            civs: a.civs,
            difficulty: a.difficulty,
        };
        let request = self.campaign_request(&a.rules, a.turns, setup, a.resume_checkpoint, true)?;
        let report = crate::campaign_report(request)?;
        let total = report.frames.len();
        let from = match a.from {
            Some(from) if from >= total => {
                return Err(format!(
                    "frame {from} is out of range: this run has frames 0..={}",
                    total - 1
                ))
            }
            Some(from) => from,
            None => total.saturating_sub(a.count),
        };
        let mut blocks = vec![report.text];
        for frame in report.frames.iter().skip(from).take(a.count) {
            blocks.push(match keep {
                None => frame.to_json(),
                Some(civ) => {
                    let mut filtered = frame.clone();
                    filtered.views.retain(|v| v.civ == civ);
                    filtered.to_json()
                }
            });
        }
        Ok(blocks)
    }

    fn session_start(&self, a: SessionStartArgs) -> std::result::Result<Vec<String>, String> {
        if self.sessions.borrow().open.len() >= MAX_SESSIONS {
            return Err(format!(
                "at most {MAX_SESSIONS} sessions may be open; close one with session_close"
            ));
        }
        let setup = CampaignSetup {
            policy: a.policy,
            seed: a.seed,
            width: a.width,
            height: a.height,
            land_percent: a.land_percent,
            civs: a.civs,
            difficulty: a.difficulty,
        };
        let request = self.campaign_request(&a.rules, 0, setup, a.resume_checkpoint, false)?;
        let campaign = crate::campaign_open(&request)?;
        let summary = crate::campaign_summary(&campaign, &campaign.outcome());
        let mut sessions = self.sessions.borrow_mut();
        sessions.next += 1;
        let id = sessions.next;
        sessions.open.insert(
            id,
            Session {
                campaign,
                recorder: Default::default(),
            },
        );
        Ok(vec![format!("session {id}\n"), summary])
    }

    fn with_session(
        &self,
        id: u32,
        f: impl FnOnce(&mut Session) -> std::result::Result<Vec<String>, String>,
    ) -> std::result::Result<Vec<String>, String> {
        let mut sessions = self.sessions.borrow_mut();
        let session = sessions
            .open
            .get_mut(&id)
            .ok_or_else(|| format!("no open session {id}"))?;
        f(session)
    }

    /// Canonical order-log lines of a recorded campaign, sliced by turn and
    /// optionally by civilization. Read from the bundle without replay.
    fn recording_log(&self, a: LogArgs) -> std::result::Result<Vec<String>, String> {
        let text = tot_record::read_limited(Path::new(self.path(&a.path)?))?;
        let bundle = tot_record::Bundle::decode(&text)?;
        if bundle.manifest.kind != tot_record::Kind::Campaign {
            return Err("only campaign recordings carry an order log; use recording_verify".into());
        }
        let gameplay = bundle
            .gameplay
            .as_deref()
            .ok_or("view-only recording has no order log")?;
        let log = gameplay
            .split_once("\nlog ")
            .and_then(|(_, rest)| rest.split_once('\n'))
            .map(|(_, log)| log)
            .ok_or("campaign checkpoint has no order log")?;
        let from = a.from_turn.unwrap_or(0);
        let to = a.to_turn.unwrap_or(u32::MAX);
        if from > to {
            return Err("from_turn is after to_turn".into());
        }
        let mut out = String::new();
        let mut count = 0;
        for (n, line) in log.lines().enumerate() {
            let keep = if n < 3 {
                true // format, setup and Rules identity header
            } else if let Some(rest) = line.strip_prefix("T ") {
                let mut fields = rest.split(' ');
                let turn: Option<u32> = fields.next().and_then(|t| t.parse().ok());
                let civ: Option<u8> = match (fields.next(), fields.next()) {
                    (Some("C"), Some(c)) => c.parse().ok(),
                    _ => None,
                };
                turn.is_some_and(|t| (from..=to).contains(&t))
                    && a.civ.is_none_or(|c| civ == Some(c))
            } else if let Some(rest) = line.strip_prefix("R ") {
                // `R n` closes turn n-1.
                rest.split(' ')
                    .next()
                    .and_then(|n| n.parse::<u32>().ok())
                    .is_some_and(|n| (from..=to).contains(&n.saturating_sub(1)))
            } else {
                true
            };
            if keep {
                count += 1;
                if count > MAX_LOG_LINES {
                    return Err(format!(
                        "more than {MAX_LOG_LINES} lines; narrow from_turn/to_turn or civ"
                    ));
                }
                out.push_str(line);
                out.push('\n');
            }
        }
        Ok(vec![out])
    }

    /// Canonicalize a requested path and require it inside a configured root.
    fn path<'p>(&self, requested: &'p str) -> std::result::Result<&'p str, String> {
        if self.roots.is_empty() {
            return Err("no filesystem roots configured".into());
        }
        let canonical = std::fs::canonicalize(requested)
            .map_err(|e| format!("cannot resolve {requested}: {e}"))?;
        if self.roots.iter().any(|root| canonical.starts_with(root)) {
            Ok(requested)
        } else {
            Err(format!(
                "{requested} is outside the configured roots ({})",
                self.roots
                    .iter()
                    .map(|r| r.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        }
    }

    fn optional_path(&self, path: Option<String>) -> std::result::Result<Option<String>, String> {
        path.map(|p| self.path(&p).map(str::to_string)).transpose()
    }
}

fn parse_args<T: for<'de> Deserialize<'de>>(args: &Value) -> std::result::Result<T, String> {
    serde_json::from_value(args.clone()).map_err(|e| format!("invalid arguments: {e}"))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PathArgs {
    path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveArgs {
    path: String,
    #[serde(default)]
    rules: Option<String>,
    #[serde(default)]
    turns: usize,
    #[serde(default = "default_seed")]
    seed: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RunArgs {
    path: String,
    #[serde(default = "default_run_turns")]
    turns: usize,
    #[serde(default = "default_seed")]
    seed: u64,
    #[serde(default)]
    rules: Option<String>,
    #[serde(default)]
    no_rules: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CampaignArgs {
    /// Path to a Rules.txt (required, also with `resume_checkpoint`).
    rules: String,
    /// "ai-v1" or "builtin-1" (default).
    #[serde(default)]
    policy: Option<String>,
    #[serde(default = "default_campaign_turns")]
    turns: u32,
    #[serde(default)]
    seed: Option<u32>,
    #[serde(default)]
    width: Option<u16>,
    #[serde(default)]
    height: Option<u16>,
    #[serde(default)]
    land_percent: Option<u8>,
    #[serde(default)]
    civs: Option<u8>,
    #[serde(default)]
    difficulty: Option<u8>,
    /// Checkpoint text from a previous run's `include_checkpoint` result.
    /// Setup options are rejected when resuming, like the CLI.
    #[serde(default)]
    resume_checkpoint: Option<String>,
    /// Add the checkpoint text as a second result block (what the CLI's
    /// `--checkpoint` would write), e.g. to resume the run later.
    #[serde(default)]
    include_checkpoint: bool,
    /// Add the order log as a final result block (the CLI's `--log`).
    #[serde(default)]
    include_log: bool,
}

/// Campaign setup options shared by `campaign_run` and `campaign_observe`.
struct CampaignSetup {
    policy: Option<String>,
    seed: Option<u32>,
    width: Option<u16>,
    height: Option<u16>,
    land_percent: Option<u8>,
    civs: Option<u8>,
    difficulty: Option<u8>,
}

/// Frames per `campaign_observe` call; a full all-views frame of the default
/// 80x50 world is about 130 KB.
const MAX_OBSERVE_FRAMES: usize = 8;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObserveArgs {
    rules: String,
    #[serde(default)]
    policy: Option<String>,
    #[serde(default = "default_campaign_turns")]
    turns: u32,
    #[serde(default)]
    seed: Option<u32>,
    #[serde(default)]
    width: Option<u16>,
    #[serde(default)]
    height: Option<u16>,
    #[serde(default)]
    land_percent: Option<u8>,
    #[serde(default)]
    civs: Option<u8>,
    #[serde(default)]
    difficulty: Option<u8>,
    #[serde(default)]
    resume_checkpoint: Option<String>,
    /// "all" (default), "spectator" or a civ number "1".."7".
    #[serde(default)]
    view: Option<String>,
    /// First frame index (0 = starting state); default: the last `count`.
    #[serde(default)]
    from: Option<usize>,
    #[serde(default = "default_observe_count")]
    count: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorldgenArgs {
    #[serde(default)]
    seed: Option<u32>,
    #[serde(default)]
    width: Option<u16>,
    #[serde(default)]
    height: Option<u16>,
    #[serde(default)]
    land_percent: Option<u8>,
}

/// `None` = all views; `Some(None)` = spectator; `Some(Some(c))` = civ c.
fn parse_view(view: Option<&str>) -> std::result::Result<Option<Option<u8>>, String> {
    match view.unwrap_or("all") {
        "all" => Ok(None),
        "spectator" => Ok(Some(None)),
        civ => match civ.parse::<u8>() {
            Ok(c @ 1..=7) => Ok(Some(Some(c))),
            _ => Err(format!(
                "invalid view {civ}: expected \"all\", \"spectator\" or a civ 1..7"
            )),
        },
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifyArgs {
    path: String,
    rules: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LogArgs {
    path: String,
    #[serde(default)]
    from_turn: Option<u32>,
    #[serde(default)]
    to_turn: Option<u32>,
    #[serde(default)]
    civ: Option<u8>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionStartArgs {
    rules: String,
    #[serde(default)]
    policy: Option<String>,
    #[serde(default)]
    seed: Option<u32>,
    #[serde(default)]
    width: Option<u16>,
    #[serde(default)]
    height: Option<u16>,
    #[serde(default)]
    land_percent: Option<u8>,
    #[serde(default)]
    civs: Option<u8>,
    #[serde(default)]
    difficulty: Option<u8>,
    #[serde(default)]
    resume_checkpoint: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionArgs {
    session: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionAdvanceArgs {
    session: u32,
    rounds: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionObserveArgs {
    session: u32,
    #[serde(default)]
    view: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionCheckpointArgs {
    session: u32,
    #[serde(default)]
    format: Option<String>,
}

fn default_observe_count() -> usize {
    1
}

fn default_seed() -> u64 {
    1
}
fn default_run_turns() -> usize {
    50
}
fn default_campaign_turns() -> u32 {
    100
}

fn tools() -> Vec<Value> {
    let path_prop = || json!({"type": "string", "description": "file path; must resolve inside a configured root"});
    vec![
        json!({
            "name": "rules_summary",
            "description": "Summarize a Rules.txt (sections, row counts, units, governments, cosmic). Byte-identical to `totctl rules`.",
            "inputSchema": {
                "type": "object",
                "properties": {"path": path_prop()},
                "required": ["path"],
                "additionalProperties": false,
            }
        }),
        json!({
            "name": "map_summary",
            "description": "Decode an .MP map (layout, dimensions, terrain histogram). Byte-identical to `totctl mp`.",
            "inputSchema": {
                "type": "object",
                "properties": {"path": path_prop()},
                "required": ["path"],
                "additionalProperties": false,
            }
        }),
        json!({
            "name": "events_summary",
            "description": "Summarize an Events.txt (trigger/action counts). Byte-identical to `totctl events`.",
            "inputSchema": {
                "type": "object",
                "properties": {"path": path_prop()},
                "required": ["path"],
                "additionalProperties": false,
            }
        }),
        json!({
            "name": "save_summary",
            "description": "Boot a .SAV/.SCN into engine state and summarize it with a reproducible checksum. Byte-identical to `totctl boot`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": path_prop(),
                    "rules": {"type": "string", "description": "explicit Rules.txt path; default: sibling Rules.txt if present"},
                    "turns": {"type": "integer", "minimum": 0, "default": 0},
                    "seed": {"type": "integer", "minimum": 0, "default": 1},
                },
                "required": ["path"],
                "additionalProperties": false,
            }
        }),
        json!({
            "name": "run_turns",
            "description": "Deterministic N-turn headless run of a .SAV/.SCN with per-turn checksums. Byte-identical to `totctl run`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": path_prop(),
                    "turns": {"type": "integer", "minimum": 0, "default": 50},
                    "seed": {"type": "integer", "minimum": 0, "default": 1},
                    "rules": {"type": "string", "description": "explicit Rules.txt path; default: sibling Rules.txt"},
                    "no_rules": {"type": "boolean", "default": false, "description": "advance only the turn counter"},
                },
                "required": ["path"],
                "additionalProperties": false,
            }
        }),
        json!({
            "name": "campaign_run",
            "description": "Start or resume a seeded generated campaign with an existing policy. The first result block is byte-identical to `totctl campaign` stdout; on request the checkpoint text and order log follow as extra blocks (the server writes no files).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "rules": path_prop(),
                    "policy": {"type": "string", "enum": ["ai-v1", "builtin-1"], "default": "builtin-1"},
                    "turns": {"type": "integer", "minimum": 0, "default": 100},
                    "seed": {"type": "integer", "minimum": 0},
                    "width": {"type": "integer", "minimum": 20, "maximum": 65535},
                    "height": {"type": "integer", "minimum": 20, "maximum": 65535},
                    "land_percent": {"type": "integer", "minimum": 1, "maximum": 100},
                    "civs": {"type": "integer", "minimum": 2, "maximum": 255},
                    "difficulty": {"type": "integer", "minimum": 0, "maximum": 5},
                    "resume_checkpoint": {"type": "string", "description": "checkpoint text from a previous run; rejects setup options like the CLI"},
                    "include_checkpoint": {"type": "boolean", "default": false},
                    "include_log": {"type": "boolean", "default": false},
                },
                "required": ["rules"],
                "additionalProperties": false,
            }
        }),
        json!({
            "name": "campaign_observe",
            "description": "Run or resume a seeded generated campaign and return observer v1 frames (schema 1, as served by the browser observer). The first block is the `totctl campaign` report; each further block is one frame: frame 0 is the starting state, frame k the state after round k. With view \"all\" a frame is byte-identical to a line of `totctl campaign --stream`; \"spectator\" keeps only the full-state view; a civ number keeps only that civ's fogged view.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "rules": path_prop(),
                    "policy": {"type": "string", "enum": ["ai-v1", "builtin-1"], "default": "builtin-1"},
                    "turns": {"type": "integer", "minimum": 0, "default": 100},
                    "seed": {"type": "integer", "minimum": 0},
                    "width": {"type": "integer", "minimum": 20, "maximum": 65535},
                    "height": {"type": "integer", "minimum": 20, "maximum": 65535},
                    "land_percent": {"type": "integer", "minimum": 1, "maximum": 100},
                    "civs": {"type": "integer", "minimum": 2, "maximum": 255},
                    "difficulty": {"type": "integer", "minimum": 0, "maximum": 5},
                    "resume_checkpoint": {"type": "string", "description": "checkpoint text from campaign_run; frame 0 is then the resumed state"},
                    "view": {"type": "string", "description": "\"all\" (default), \"spectator\", or a civ number \"1\"..\"7\"", "default": "all"},
                    "from": {"type": "integer", "minimum": 0, "description": "first frame index; default: the last `count` frames"},
                    "count": {"type": "integer", "minimum": 1, "maximum": MAX_OBSERVE_FRAMES, "default": 1},
                },
                "required": ["rules"],
                "additionalProperties": false,
            }
        }),
        json!({
            "name": "recording_info",
            "description": "Describe a .totrec session recording without replaying it: kind, content class (reproducible or view-only), identities it needs, progress, stop reason and labelled sections. Byte-identical to `totctl record info`.",
            "inputSchema": {
                "type": "object",
                "properties": {"path": path_prop()},
                "required": ["path"],
                "additionalProperties": false,
            }
        }),
        json!({
            "name": "recording_verify",
            "description": "Replay a .totrec recording against a Rules.txt and check every claim (engine, Rules/resources identity, progress, stop reason, observer frames). Byte-identical to `totctl record verify`.",
            "inputSchema": {
                "type": "object",
                "properties": {"path": path_prop(), "rules": path_prop()},
                "required": ["path", "rules"],
                "additionalProperties": false,
            }
        }),
        json!({
            "name": "recording_log",
            "description": "Canonical order-log lines of a recorded campaign (`T <turn> C <civ> <order> = <result>`, and `R <n> <checksum>` closing turn n-1), sliced by turn range and optionally one civ; the 3 header lines are always included. Read from the recording without replay. Full-state content (every civ's orders); the lines are those of the run's `totctl campaign --log` file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": path_prop(),
                    "from_turn": {"type": "integer", "minimum": 0},
                    "to_turn": {"type": "integer", "minimum": 0},
                    "civ": {"type": "integer", "minimum": 1, "maximum": 7},
                },
                "required": ["path"],
                "additionalProperties": false,
            }
        }),
        json!({
            "name": "session_start",
            "description": "Open a live in-memory generated campaign (no rounds run yet) and return its id and the `totctl campaign` report of its starting state. Sessions avoid re-simulating on every call; at most 4 are open; they end with the server or session_close.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "rules": path_prop(),
                    "policy": {"type": "string", "enum": ["ai-v1", "builtin-1"], "default": "builtin-1"},
                    "seed": {"type": "integer", "minimum": 0},
                    "width": {"type": "integer", "minimum": 20, "maximum": 65535},
                    "height": {"type": "integer", "minimum": 20, "maximum": 65535},
                    "land_percent": {"type": "integer", "minimum": 1, "maximum": 100},
                    "civs": {"type": "integer", "minimum": 2, "maximum": 255},
                    "difficulty": {"type": "integer", "minimum": 0, "maximum": 5},
                    "resume_checkpoint": {"type": "string", "description": "checkpoint text; rejects setup options like the CLI"},
                },
                "required": ["rules"],
                "additionalProperties": false,
            }
        }),
        json!({
            "name": "session_advance",
            "description": "Run more rounds of an open session and return the `totctl campaign` report. After k rounds in total the report is byte-identical to `totctl campaign --turns k` with the same setup.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session": {"type": "integer", "minimum": 1},
                    "rounds": {"type": "integer", "minimum": 1, "maximum": MAX_SESSION_ADVANCE},
                },
                "required": ["session", "rounds"],
                "additionalProperties": false,
            }
        }),
        json!({
            "name": "session_observe",
            "description": "Observer v1 frame of an open session's current state, with the events since the previous session_observe. view: \"all\" (default), \"spectator\" (full state) or a civ number (that civ's fogged view).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session": {"type": "integer", "minimum": 1},
                    "view": {"type": "string", "default": "all"},
                },
                "required": ["session"],
                "additionalProperties": false,
            }
        }),
        json!({
            "name": "session_checkpoint",
            "description": "Return an open session as text: a campaign checkpoint (default; resumable with campaign_run or session_start) or a .totrec recording (format \"recording\") that recording_verify and `totctl record verify` accept. Nothing is written.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session": {"type": "integer", "minimum": 1},
                    "format": {"type": "string", "enum": ["checkpoint", "recording"], "default": "checkpoint"},
                },
                "required": ["session"],
                "additionalProperties": false,
            }
        }),
        json!({
            "name": "session_close",
            "description": "Close an open session and free it.",
            "inputSchema": {
                "type": "object",
                "properties": {"session": {"type": "integer", "minimum": 1}},
                "required": ["session"],
                "additionalProperties": false,
            }
        }),
        json!({
            "name": "worldgen_summary",
            "description": "Generate an engine-designed world in memory (not an original-algorithm port) and summarize it. Byte-identical to `totctl worldgen` stdout without its final `map` line; no map file is written.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "seed": {"type": "integer", "minimum": 0, "default": 1},
                    "width": {"type": "integer", "minimum": 20, "maximum": 65535, "default": 80},
                    "height": {"type": "integer", "minimum": 20, "maximum": 65535, "default": 50},
                    "land_percent": {"type": "integer", "minimum": 1, "maximum": 100, "default": 47},
                },
                "additionalProperties": false,
            }
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../engine/tests/fixtures")
    }

    fn server() -> Server {
        Server::new(vec![std::fs::canonicalize(fixtures()).unwrap()])
    }

    fn call(server: &Server, name: &str, arguments: Value) -> Value {
        let request = json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call",
            "params": {"name": name, "arguments": arguments}});
        let response = server.handle(&request.to_string()).unwrap();
        serde_json::from_str(&response).unwrap()
    }

    fn result_text(response: &Value) -> &str {
        assert_eq!(response["result"]["isError"], false, "{response}");
        response["result"]["content"][0]["text"].as_str().unwrap()
    }

    fn error_text(response: &Value) -> &str {
        assert_eq!(response["result"]["isError"], true, "{response}");
        response["result"]["content"][0]["text"].as_str().unwrap()
    }

    #[test]
    fn initialize_negotiates_the_protocol_version() {
        let s = server();
        let answer = |v: &str| {
            let request = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                "params": {"protocolVersion": v, "capabilities": {}, "clientInfo": {"name": "t", "version": "0"}}});
            let response = s.handle(&request.to_string()).unwrap();
            let response: Value = serde_json::from_str(&response).unwrap();
            response["result"]["protocolVersion"]
                .as_str()
                .unwrap()
                .to_string()
        };
        assert_eq!(answer("2024-11-05"), "2024-11-05");
        assert_eq!(answer("2025-06-18"), "2025-06-18");
        assert_eq!(
            answer("1999-01-01"),
            "2025-06-18",
            "unsupported → preferred"
        );
        let request = json!({"jsonrpc": "2.0", "id": "x", "method": "initialize", "params": {}});
        let response: Value =
            serde_json::from_str(&s.handle(&request.to_string()).unwrap()).unwrap();
        assert_eq!(response["id"], json!("x"), "string ids are preserved");
        assert_eq!(response["result"]["serverInfo"]["name"], "totctl");
        assert!(response["result"]["capabilities"]["tools"].is_object());
    }

    #[test]
    fn notifications_and_ping() {
        let s = server();
        let initialized = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
        assert!(s.handle(&initialized.to_string()).is_none());
        let unknown_notification = json!({"jsonrpc": "2.0", "method": "notifications/cancelled"});
        assert!(s.handle(&unknown_notification.to_string()).is_none());
        let ping = json!({"jsonrpc": "2.0", "id": 2, "method": "ping"});
        let response: Value = serde_json::from_str(&s.handle(&ping.to_string()).unwrap()).unwrap();
        assert_eq!(response["result"], json!({}));
        let unknown = json!({"jsonrpc": "2.0", "id": 3, "method": "resources/list"});
        let response: Value =
            serde_json::from_str(&s.handle(&unknown.to_string()).unwrap()).unwrap();
        assert_eq!(response["error"]["code"], -32601);
        let response: Value = serde_json::from_str(&s.handle("not json").unwrap()).unwrap();
        assert_eq!(response["error"]["code"], -32700);
    }

    #[test]
    fn tools_list_declares_the_stage_one_tools() {
        let s = server();
        let request = json!({"jsonrpc": "2.0", "id": 4, "method": "tools/list"});
        let response: Value =
            serde_json::from_str(&s.handle(&request.to_string()).unwrap()).unwrap();
        let tools = response["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(
            names,
            [
                "rules_summary",
                "map_summary",
                "events_summary",
                "save_summary",
                "run_turns",
                "campaign_run",
                "campaign_observe",
                "recording_info",
                "recording_verify",
                "recording_log",
                "session_start",
                "session_advance",
                "session_observe",
                "session_checkpoint",
                "session_close",
                "worldgen_summary"
            ]
        );
        for t in tools {
            assert_eq!(t["inputSchema"]["type"], "object");
            let name = t["name"].as_str().unwrap();
            let description = t["description"].as_str().unwrap();
            assert!(
                description.contains("totctl") || name.starts_with("session_"),
                "{name}"
            );
        }
    }

    #[test]
    fn rules_summary_matches_the_cli_report_byte_for_byte() {
        let path = fixtures().join("campaign-rules.txt");
        let path = path.to_string_lossy().into_owned();
        let response = call(&server(), "rules_summary", json!({"path": path}));
        assert_eq!(result_text(&response), crate::rules_report(&path).unwrap());
        assert!(result_text(&response).contains("sections"));
    }

    #[test]
    fn paths_outside_the_roots_are_rejected() {
        let s = server();
        let response = call(&s, "rules_summary", json!({"path": "/etc/hostname"}));
        assert!(
            error_text(&response).contains("outside the configured roots"),
            "{response}"
        );
        let no_roots = Server::new(vec![]);
        let path = fixtures().join("campaign-rules.txt");
        let response = call(&no_roots, "rules_summary", json!({"path": path}));
        assert!(error_text(&response).contains("no filesystem roots"));
    }

    #[test]
    fn bad_arguments_are_tool_errors_not_protocol_errors() {
        let s = server();
        assert!(error_text(&call(&s, "no_such_tool", json!({}))).contains("unknown tool"));
        let response = call(&s, "rules_summary", json!({"path": "x", "bogus": 1}));
        assert!(error_text(&response).contains("invalid arguments"));
        let request = json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call", "params": {}});
        let response: Value =
            serde_json::from_str(&s.handle(&request.to_string()).unwrap()).unwrap();
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn campaign_run_is_byte_identical_and_resumes_from_inline_checkpoints() {
        let rules = fixtures()
            .join("campaign-rules.txt")
            .to_string_lossy()
            .into_owned();
        let s = server();
        // One 8-turn run, checkpoint requested.
        let first = call(
            &s,
            "campaign_run",
            json!({"rules": rules, "turns": 3, "seed": 7, "policy": "ai-v1", "include_checkpoint": true}),
        );
        let first_text = result_text(&first).to_string();
        let checkpoint = first["result"]["content"][1]["text"]
            .as_str()
            .unwrap()
            .to_string();
        // Resume for 5 more turns from the inline checkpoint.
        let resumed = call(
            &s,
            "campaign_run",
            json!({"rules": rules, "turns": 5, "resume_checkpoint": checkpoint}),
        );
        // A single 8-turn run must end in the same state.
        let full = call(
            &s,
            "campaign_run",
            json!({"rules": rules, "turns": 8, "seed": 7, "policy": "ai-v1"}),
        );
        let fingerprint = |text: &str| {
            text.lines()
                .find(|l| l.starts_with("fingerprint"))
                .unwrap()
                .to_string()
        };
        assert_eq!(
            fingerprint(result_text(&resumed)),
            fingerprint(result_text(&full))
        );
        // The tool text equals the shared report the CLI prints.
        let cli = crate::campaign_report(crate::CampaignRequest {
            rules_path: rules.clone(),
            policy: engine::campaign::AiPolicy::parse("ai-v1").unwrap(),
            turns: 8,
            config: engine::newgame::NewGameConfig {
                world: engine::worldgen::WorldConfig {
                    seed: 7,
                    ..Default::default()
                },
                gameplay_seed: 7,
                ..Default::default()
            },
            changed_setup: true,
            resume_checkpoint: None,
            observe: false,
            record: None,
        })
        .unwrap();
        assert_eq!(result_text(&full), cli.text);
        assert!(first_text.contains("setup"));
        // Resume rejects setup options, like the CLI.
        let conflict = call(
            &s,
            "campaign_run",
            json!({"rules": rules, "turns": 1, "resume_checkpoint": "x", "seed": 3}),
        );
        assert!(error_text(&conflict).contains("omit setup options"));
        // builtin-1 needs @ENDWONDER, which this fixture alone lacks: the tool
        // surfaces the same error the CLI prints. `builtin_rules_root` below
        // adds the authored settings and runs builtin-1 for real.
        let unsupported = call(
            &s,
            "campaign_run",
            json!({"rules": rules, "turns": 1, "policy": "builtin-1"}),
        );
        assert!(error_text(&unsupported).contains("missing ENDWONDER"));
    }

    #[test]
    fn save_summary_and_run_turns_validate_their_inputs() {
        let s = server();
        let missing = fixtures()
            .join("no-such.sav")
            .to_string_lossy()
            .into_owned();
        let response = call(&s, "save_summary", json!({"path": missing.clone()}));
        assert!(error_text(&response).contains("cannot resolve"));
        let response = call(&s, "run_turns", json!({"path": missing, "turns": 2}));
        assert!(error_text(&response).contains("cannot resolve"));
    }

    #[test]
    fn serve_loop_reads_ndjson_and_skips_blank_lines() {
        let s = server();
        let mut input = b"\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n\
                          {\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n\
                          {\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\n"
            .as_slice();
        let mut output = Vec::new();
        let code = run(&s, &mut input, &mut output);
        assert_eq!(code, ExitCode::SUCCESS);
        let text = String::from_utf8(output).unwrap();
        let responses: Vec<Value> = text
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(responses.len(), 2, "notifications get no response: {text}");
        assert!(responses.iter().all(|r| r["result"] == json!({})));
        // Each response is a single line, as the stdio transport requires.
        assert_eq!(text.lines().count(), 2);
    }

    /// A temporary root holding campaign-rules.txt + the authored built-in AI
    /// settings, the same concatenation `tests/campaign.rs` uses for builtin-1.
    fn builtin_rules_root(tag: &str) -> (PathBuf, String) {
        let dir = std::env::temp_dir().join(format!("totctl-mcp-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).unwrap();
        let rules = dir.join("Rules.txt");
        std::fs::write(
            &rules,
            format!(
                "{}\n{}",
                include_str!("../../engine/tests/fixtures/campaign-rules.txt"),
                include_str!("../../engine/tests/fixtures/builtin-settings.txt")
            ),
        )
        .unwrap();
        (dir, rules.to_string_lossy().into_owned())
    }

    fn texts(response: &Value) -> Vec<String> {
        assert_eq!(response["result"]["isError"], false, "{response}");
        response["result"]["content"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["text"].as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn builtin_policy_runs_and_resumes_through_the_tool() {
        let (dir, rules) = builtin_rules_root("builtin");
        let s = Server::new(vec![std::fs::canonicalize(&dir).unwrap()]);
        let small =
            json!({"rules": rules, "seed": 5, "width": 40, "height": 30, "policy": "builtin-1"});
        let mut first_args = small.clone();
        first_args["turns"] = json!(3);
        first_args["include_checkpoint"] = json!(true);
        let first = texts(&call(&s, "campaign_run", first_args));
        let resumed = call(
            &s,
            "campaign_run",
            json!({"rules": rules, "turns": 4, "resume_checkpoint": first[1]}),
        );
        let mut full_args = small;
        full_args["turns"] = json!(7);
        let full = call(&s, "campaign_run", full_args);
        let tail = |text: &str| text.lines().skip(2).collect::<Vec<_>>().join("\n");
        // Same rounds, state and fingerprint whether resumed or run straight.
        assert_eq!(tail(result_text(&resumed)), tail(result_text(&full)));
        assert!(first[1].contains("\npolicy builtin-1\n"), "{}", first[1]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn campaign_observe_returns_stream_frames_and_filtered_views() {
        let rules = fixtures()
            .join("campaign-rules.txt")
            .to_string_lossy()
            .into_owned();
        let s = server();
        let setup = json!({"rules": rules, "seed": 9, "width": 40, "height": 30,
                           "policy": "ai-v1", "turns": 4});
        let with = |extra: Value| {
            let mut args = setup.clone();
            for (k, v) in extra.as_object().unwrap() {
                args[k] = v.clone();
            }
            args
        };
        // The shared report with observation is what `--stream` writes.
        let cli = crate::campaign_report(crate::CampaignRequest {
            rules_path: rules.clone(),
            policy: engine::campaign::AiPolicy::AiV1,
            turns: 4,
            config: engine::newgame::NewGameConfig {
                world: engine::worldgen::WorldConfig {
                    seed: 9,
                    width: 40,
                    height: 30,
                    ..Default::default()
                },
                gameplay_seed: 9,
                ..Default::default()
            },
            changed_setup: true,
            resume_checkpoint: None,
            observe: true,
            record: None,
        })
        .unwrap();
        let stream: Vec<String> = cli.stream().lines().map(str::to_string).collect();
        assert_eq!(stream.len(), 5, "starting state + one frame per round");

        let all = texts(&call(
            &s,
            "campaign_observe",
            with(json!({"from": 0, "count": 8})),
        ));
        assert_eq!(all[0], cli.text, "first block is the campaign report");
        assert_eq!(all[1..], stream[..], "view all = the --stream lines");
        // The report is unchanged by observation.
        let plain = call(&s, "campaign_run", setup.clone());
        assert_eq!(result_text(&plain), cli.text);

        // Default window: the last frame.
        let last = texts(&call(&s, "campaign_observe", setup.clone()));
        assert_eq!(last[1..], stream[4..]);

        // One civ's fogged view, or the spectator view only.
        let full: Value = serde_json::from_str(&stream[2]).unwrap();
        for (view, index) in [("3", 3usize), ("spectator", 0)] {
            let one = texts(&call(
                &s,
                "campaign_observe",
                with(json!({"from": 2, "view": view})),
            ));
            let frame: Value = serde_json::from_str(&one[1]).unwrap();
            assert_eq!(frame["views"].as_array().unwrap().len(), 1);
            assert_eq!(frame["views"][0], full["views"][index]);
            let (mut a, mut b) = (frame.clone(), full.clone());
            a.as_object_mut().unwrap().remove("views");
            b.as_object_mut().unwrap().remove("views");
            assert_eq!(a, b, "all other frame fields are unchanged");
        }

        // Resuming observes the continuation: same checksums as the straight run.
        let mut first_args = with(json!({"turns": 2}));
        first_args.as_object_mut().unwrap().remove("view");
        let first = texts(&call(&s, "campaign_run", {
            let mut a = first_args.clone();
            a["include_checkpoint"] = json!(true);
            a
        }));
        let resumed = texts(&call(
            &s,
            "campaign_observe",
            json!({"rules": rules, "turns": 2, "resume_checkpoint": first[1], "from": 0, "count": 3}),
        ));
        let checksum =
            |line: &str| serde_json::from_str::<Value>(line).unwrap()["checksum"].clone();
        let got: Vec<Value> = resumed[1..].iter().map(|l| checksum(l)).collect();
        let want: Vec<Value> = stream[2..].iter().map(|l| checksum(l)).collect();
        assert_eq!(got, want);

        for (extra, message) in [
            (json!({"count": 0}), "count must be"),
            (json!({"count": 9}), "count must be"),
            (json!({"from": 5}), "out of range"),
            (json!({"view": "8"}), "invalid view"),
            (json!({"view": "barbarians"}), "invalid view"),
            (json!({"include_log": true}), "invalid arguments"),
        ] {
            let response = call(&s, "campaign_observe", with(extra));
            assert!(error_text(&response).contains(message), "{response}");
        }
    }

    #[test]
    fn worldgen_summary_matches_the_shared_report() {
        let s = server();
        let response = call(
            &s,
            "worldgen_summary",
            json!({"seed": 4, "width": 40, "height": 30}),
        );
        let (report, _) = crate::worldgen_report(engine::worldgen::WorldConfig {
            seed: 4,
            width: 40,
            height: 30,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(result_text(&response), report);
        assert!(report.contains("seed          : 4"));
        let response = call(&s, "worldgen_summary", json!({"path": "x.mp"}));
        assert!(error_text(&response).contains("invalid arguments"));
        // Generation needs no filesystem root.
        let rootless = Server::new(vec![]);
        let response = call(&rootless, "worldgen_summary", json!({}));
        assert!(result_text(&response).contains("generator"));
    }

    #[test]
    fn sessions_advance_incrementally_and_match_straight_runs() {
        let (dir, rules) = builtin_rules_root("session");
        let s = Server::new(vec![std::fs::canonicalize(&dir).unwrap()]);
        let setup = json!({"rules": rules, "seed": 6, "width": 40, "height": 30});
        let started = texts(&call(&s, "session_start", setup.clone()));
        assert_eq!(started[0], "session 1\n");
        assert!(started[1].contains("rounds      : 0\n"));
        let id = json!(1);

        // First observation: no events yet; civ view only.
        let first = texts(&call(
            &s,
            "session_observe",
            json!({"session": id, "view": "2"}),
        ));
        let frame: Value = serde_json::from_str(&first[0]).unwrap();
        assert_eq!(frame["views"].as_array().unwrap().len(), 1);
        assert_eq!(frame["views"][0]["civ"], 2);
        assert_eq!(frame["events"], json!([]));

        call(&s, "session_advance", json!({"session": id, "rounds": 3}));
        let after = texts(&call(
            &s,
            "session_advance",
            json!({"session": id, "rounds": 4}),
        ));
        // 3 + 4 rounds in a session == one 7-round run, byte for byte.
        let mut straight = setup.clone();
        straight["turns"] = json!(7);
        straight["include_checkpoint"] = json!(true);
        let straight = texts(&call(&s, "campaign_run", straight));
        assert_eq!(after[0], straight[0]);
        let checkpoint = texts(&call(&s, "session_checkpoint", json!({"session": id})));
        assert_eq!(checkpoint[0], straight[1]);

        // Events since the previous observation are reported once.
        let second = texts(&call(&s, "session_observe", json!({"session": id})));
        let frame: Value = serde_json::from_str(&second[0]).unwrap();
        assert!(!frame["events"].as_array().unwrap().is_empty());
        let third = texts(&call(
            &s,
            "session_observe",
            json!({"session": id, "view": "spectator"}),
        ));
        let frame: Value = serde_json::from_str(&third[0]).unwrap();
        assert_eq!(frame["events"], json!([]));
        assert_eq!(frame["views"][0]["civ"], Value::Null);

        // A session exported as a recording verifies through the recording tools.
        let recording = texts(&call(
            &s,
            "session_checkpoint",
            json!({"session": id, "format": "recording"}),
        ));
        let path = dir.join("session.totrec");
        std::fs::write(&path, &recording[0]).unwrap();
        let path = path.to_string_lossy().into_owned();
        let info = texts(&call(&s, "recording_info", json!({"path": path})));
        assert_eq!(info[0], crate::record_info_report(&path).unwrap());
        assert!(info[0].contains("progress    : 7\n"));
        let verified = texts(&call(
            &s,
            "recording_verify",
            json!({"path": path, "rules": rules}),
        ));
        assert_eq!(
            verified[0],
            crate::record_verify_report(&path, &rules).unwrap()
        );
        assert!(verified[0].contains("replay reproduces the recording"));

        // Resuming a session from its checkpoint continues identically.
        let resumed = texts(&call(
            &s,
            "session_start",
            json!({"rules": rules, "resume_checkpoint": checkpoint[0]}),
        ));
        assert_eq!(resumed[0], "session 2\n");
        assert_eq!(resumed[1], after[0]);
        let conflict = call(
            &s,
            "session_start",
            json!({"rules": rules, "resume_checkpoint": "x", "seed": 1}),
        );
        assert!(error_text(&conflict).contains("omit setup options"));

        // Limits and errors.
        for n in 3..=4 {
            assert_eq!(
                texts(&call(&s, "session_start", setup.clone()))[0],
                format!("session {n}\n")
            );
        }
        assert!(error_text(&call(&s, "session_start", setup.clone())).contains("at most 4"));
        assert_eq!(
            texts(&call(&s, "session_close", json!({"session": 3})))[0],
            "closed session 3\n"
        );
        assert_eq!(
            texts(&call(&s, "session_start", setup.clone()))[0],
            "session 5\n"
        );
        for (tool, args, message) in [
            (
                "session_advance",
                json!({"session": 3, "rounds": 1}),
                "no open session 3",
            ),
            ("session_close", json!({"session": 3}), "no open session 3"),
            (
                "session_advance",
                json!({"session": 1, "rounds": 0}),
                "rounds must be",
            ),
            (
                "session_advance",
                json!({"session": 1, "rounds": 1001}),
                "rounds must be",
            ),
            (
                "session_observe",
                json!({"session": 1, "view": "9"}),
                "invalid view",
            ),
            (
                "session_checkpoint",
                json!({"session": 1, "format": "sav"}),
                "invalid format",
            ),
        ] {
            let response = call(&s, tool, args);
            assert!(
                error_text(&response).contains(message),
                "{tool}: {response}"
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn recording_log_slices_the_canonical_order_log() {
        let (dir, rules) = builtin_rules_root("log");
        let s = Server::new(vec![std::fs::canonicalize(&dir).unwrap()]);
        let started = texts(&call(
            &s,
            "session_start",
            json!({"rules": rules, "seed": 2, "width": 40, "height": 30}),
        ));
        assert_eq!(started[0], "session 1\n");
        call(&s, "session_advance", json!({"session": 1, "rounds": 5}));
        let recording = texts(&call(
            &s,
            "session_checkpoint",
            json!({"session": 1, "format": "recording"}),
        ));
        let path = dir.join("log.totrec").to_string_lossy().into_owned();
        std::fs::write(&path, &recording[0]).unwrap();
        // The unfiltered slice is exactly the run's --log file.
        let full = texts(&call(&s, "recording_log", json!({"path": path})));
        let mut run = json!({"rules": rules, "seed": 2, "width": 40, "height": 30, "turns": 5, "include_log": true});
        let log = texts(&call(&s, "campaign_run", run.take()));
        assert_eq!(full[0], log[1]);
        // A turn and civ window.
        let window = texts(&call(
            &s,
            "recording_log",
            json!({"path": path, "from_turn": 2, "to_turn": 3, "civ": 1}),
        ));
        let lines: Vec<&str> = window[0].lines().collect();
        assert_eq!(lines[..3], full[0].lines().take(3).collect::<Vec<_>>()[..]);
        let body = &lines[3..];
        assert!(body.iter().any(|l| l.starts_with("T 2 C 1 ")));
        assert!(body.iter().any(|l| l.starts_with("T 3 C 1 ")));
        for line in body {
            assert!(
                line.starts_with("T 2 C 1 ")
                    || line.starts_with("T 3 C 1 ")
                    || line.starts_with("R 3 ")
                    || line.starts_with("R 4 "),
                "{line}"
            );
        }
        for (args, message) in [
            (
                json!({"path": path, "from_turn": 4, "to_turn": 2}),
                "after to_turn",
            ),
            (
                json!({"path": path, "civ": 1, "extra": 1}),
                "invalid arguments",
            ),
        ] {
            assert!(error_text(&call(&s, "recording_log", args)).contains(message));
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
