//! End-to-end check of `totctl mcp` over its stdio transport (#250): the
//! observer frames and worldgen summary an MCP client receives are the bytes
//! the equivalent CLI commands print or write.
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn totctl() -> Command {
    Command::new(env!("CARGO_BIN_EXE_totctl"))
}

/// Send requests to one `totctl mcp` process and collect one response each.
fn mcp_session(root: &std::path::Path, requests: &[Value]) -> Vec<Value> {
    let mut child = totctl()
        .arg("mcp")
        .arg("--root")
        .arg(root)
        .env_remove("CIVTOT_HOME")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut responses = Vec::new();
    for request in requests {
        writeln!(stdin, "{request}").unwrap();
        stdin.flush().unwrap();
        if request.get("id").is_none() {
            continue; // notifications get no response
        }
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        responses.push(serde_json::from_str(&line).unwrap());
    }
    drop(stdin);
    assert!(child.wait().unwrap().success());
    responses
}

fn tool_call(id: u64, name: &str, arguments: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
           "params": {"name": name, "arguments": arguments}})
}

fn blocks(response: &Value) -> Vec<String> {
    assert_eq!(response["result"]["isError"], false, "{response}");
    response["result"]["content"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["text"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn stdio_observer_frames_and_worldgen_match_the_cli() {
    let root = std::env::temp_dir().join(format!("totengine-mcp-stdio-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir(&root).unwrap();
    let rules = root.join("Rules.txt");
    std::fs::write(
        &rules,
        format!(
            "{}\n{}",
            include_str!("../../engine/tests/fixtures/campaign-rules.txt"),
            include_str!("../../engine/tests/fixtures/builtin-settings.txt")
        ),
    )
    .unwrap();
    let rules_arg = rules.to_str().unwrap();

    // CLI: a builtin-1 campaign with an observer stream and a generated map.
    let stream = root.join("run.ndjson");
    let cli = totctl()
        .args([
            "campaign",
            rules_arg,
            "--policy",
            "builtin-1",
            "--seed",
            "11",
        ])
        .args(["--width", "40", "--height", "30", "--turns", "3"])
        .arg("--stream")
        .arg(&stream)
        .output()
        .unwrap();
    assert!(
        cli.status.success(),
        "{}",
        String::from_utf8_lossy(&cli.stderr)
    );
    let stream_text = std::fs::read_to_string(&stream).unwrap();
    assert_eq!(stream_text.lines().count(), 4);
    let map = root.join("world.mp");
    let worldgen = totctl()
        .arg("worldgen")
        .arg(&map)
        .args(["--seed", "11", "--width", "40", "--height", "30"])
        .output()
        .unwrap();
    assert!(worldgen.status.success());

    // An existing stream file is never overwritten, and outputs must differ.
    let again = totctl()
        .args(["campaign", rules_arg, "--turns", "1", "--stream"])
        .arg(&stream)
        .output()
        .unwrap();
    assert!(!again.status.success());
    let same = totctl()
        .args([
            "campaign", rules_arg, "--turns", "1", "--stream", "x", "--log", "x",
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(!same.status.success());

    let responses = mcp_session(
        &root,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                   "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                              "clientInfo": {"name": "test", "version": "0"}}}),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            tool_call(
                2,
                "campaign_observe",
                json!({"rules": rules_arg, "policy": "builtin-1", "seed": 11, "width": 40,
                       "height": 30, "turns": 3, "from": 0, "count": 4}),
            ),
            tool_call(
                3,
                "worldgen_summary",
                json!({"seed": 11, "width": 40, "height": 30}),
            ),
        ],
    );
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "totctl");
    let observed = blocks(&responses[1]);
    assert_eq!(observed[0], String::from_utf8(cli.stdout).unwrap());
    let frames: String = observed[1..].iter().map(|f| format!("{f}\n")).collect();
    assert_eq!(frames, stream_text, "frames are the --stream file's lines");
    let summary = &blocks(&responses[2])[0];
    assert_eq!(
        format!("{summary}map           : {}\n", map.display()),
        String::from_utf8(worldgen.stdout).unwrap()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn stdio_sessions_keep_state_between_messages() {
    let root = std::env::temp_dir().join(format!("totengine-mcp-session-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir(&root).unwrap();
    let rules = root.join("Rules.txt");
    std::fs::write(
        &rules,
        include_str!("../../engine/tests/fixtures/campaign-rules.txt"),
    )
    .unwrap();
    let rules_arg = rules.to_str().unwrap();
    let setup = [
        "--policy", "ai-v1", "--seed", "4", "--width", "40", "--height", "30",
    ];
    let cli = totctl()
        .args(["campaign", rules_arg])
        .args(setup)
        .args(["--turns", "6"])
        .output()
        .unwrap();
    assert!(cli.status.success());
    let start =
        json!({"rules": rules_arg, "policy": "ai-v1", "seed": 4, "width": 40, "height": 30});
    let responses = mcp_session(
        &root,
        &[
            tool_call(1, "session_start", start),
            tool_call(2, "session_advance", json!({"session": 1, "rounds": 2})),
            tool_call(3, "session_advance", json!({"session": 1, "rounds": 4})),
            tool_call(4, "session_close", json!({"session": 1})),
            tool_call(5, "session_advance", json!({"session": 1, "rounds": 1})),
        ],
    );
    assert_eq!(blocks(&responses[0])[0], "session 1\n");
    assert_eq!(
        blocks(&responses[2])[0],
        String::from_utf8(cli.stdout).unwrap(),
        "2 + 4 session rounds == `totctl campaign --turns 6`"
    );
    assert_eq!(blocks(&responses[3])[0], "closed session 1\n");
    assert_eq!(responses[4]["result"]["isError"], true);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn stdio_agent_game_answers_a_decision_and_ends_a_turn() {
    let root = std::env::temp_dir().join(format!("totengine-mcp-agent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir(&root).unwrap();
    let rules = root.join("Rules.txt");
    std::fs::write(
        &rules,
        format!(
            "{}\n{}",
            include_str!("../../engine/tests/fixtures/campaign-rules.txt"),
            include_str!("../../engine/tests/fixtures/builtin-settings.txt")
        ),
    )
    .unwrap();
    let responses = mcp_session(
        &root,
        &[
            tool_call(
                1,
                "agent_start",
                json!({"rules": rules.to_str().unwrap(), "agent": "stdio-test", "seed": 5, "width": 40, "height": 30}),
            ),
            tool_call(
                2,
                "agent_submit",
                json!({"game": 1, "revision": 0, "request_id": 1,
                       "command": {"type": "research", "decision": 1, "advance": 0}}),
            ),
            tool_call(
                3,
                "agent_submit",
                json!({"game": 1, "revision": 1, "request_id": 2, "command": {"type": "end_turn"}}),
            ),
        ],
    );
    let first: Value = serde_json::from_str(&blocks(&responses[0])[1]).unwrap();
    assert_eq!(first["turn"], 1);
    assert_eq!(first["decision"]["type"], "research");
    let research: Value = serde_json::from_str(&blocks(&responses[1])[0]).unwrap();
    assert_eq!(research["accepted"], true);
    let ended: Value = serde_json::from_str(&blocks(&responses[2])[0]).unwrap();
    assert_eq!(
        ended["turn"], 2,
        "opponents ran and control returned: {ended}"
    );
    assert_eq!(ended["phase"], "your-turn");
    std::fs::remove_dir_all(root).unwrap();
}
