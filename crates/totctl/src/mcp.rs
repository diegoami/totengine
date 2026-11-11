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
    let server = Server { roots };
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
}

impl Server {
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
            other => Err(format!("unknown tool: {other}")),
        }
    }

    fn campaign_run(&self, a: CampaignArgs) -> std::result::Result<Vec<String>, String> {
        let mut config = engine::newgame::NewGameConfig::default();
        let mut changed_setup = false;
        let mut policy = engine::campaign::AiPolicy::BuiltIn1;
        if let Some(p) = &a.policy {
            policy = engine::campaign::AiPolicy::parse(p)
                .map_err(|e| format!("invalid value for --policy: {e}"))?;
            changed_setup = true;
        }
        if let Some(seed) = a.seed {
            config.world.seed = seed;
            config.gameplay_seed = seed;
            changed_setup = true;
        }
        if let Some(width) = a.width {
            config.world.width = width;
            changed_setup = true;
        }
        if let Some(height) = a.height {
            config.world.height = height;
            changed_setup = true;
        }
        if let Some(land) = a.land_percent {
            config.world.land_percent = land;
            changed_setup = true;
        }
        if let Some(civs) = a.civs {
            config.civilizations = civs;
            changed_setup = true;
        }
        if let Some(difficulty) = a.difficulty {
            config.difficulty = difficulty;
            changed_setup = true;
        }
        let report = crate::campaign_report(crate::CampaignRequest {
            rules_path: self.path(&a.rules)?.to_string(),
            policy,
            turns: a.turns,
            config,
            changed_setup,
            resume_checkpoint: a.resume_checkpoint,
        })?;
        let mut blocks = vec![report.text];
        if a.include_checkpoint {
            blocks.push(report.checkpoint);
        }
        if a.include_log {
            blocks.push(report.log);
        }
        Ok(blocks)
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
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../engine/tests/fixtures")
    }

    fn server() -> Server {
        Server {
            roots: vec![std::fs::canonicalize(fixtures()).unwrap()],
        }
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
    fn tools_list_declares_the_six_stage_one_tools() {
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
                "campaign_run"
            ]
        );
        for t in tools {
            assert_eq!(t["inputSchema"]["type"], "object");
            assert!(t["description"].as_str().unwrap().contains("totctl"));
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
        let no_roots = Server { roots: vec![] };
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
        // builtin-1 needs @ENDWONDER, which this fixture lacks: the tool surfaces
        // the same error the CLI prints (verified identical in the parity checks).
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
}
