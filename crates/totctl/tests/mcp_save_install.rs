//! #254: the MCP save tools over stdio equal the CLI for a corpus save.
//! `save_observe` frames are the bytes of `totctl observe --stream`, and
//! `save_replay_verify` returns the `totctl replay` stdout line. Env-gated on
//! `CIVTOT_HOME` (self-skips in CI and on machines without the install); no
//! game data is committed.
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
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

fn error_text(response: &Value) -> String {
    assert_eq!(response["result"]["isError"], true, "{response}");
    response["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .into()
}

#[test]
fn mcp_save_tools_match_the_cli_for_a_corpus_save() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(PathBuf::from) else {
        eprintln!("skipping: CIVTOT_HOME not set");
        return;
    };
    let save = home.join("Original/Ph_Auto2.SAV");
    if !save.exists() {
        eprintln!("skipping: standard save not present");
        return;
    }
    let save = save.to_string_lossy().into_owned();
    let dir = std::env::temp_dir().join(format!("totengine-mcp-save-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let stream = dir.join("live.ndjson");
    let log = dir.join("orders.log");

    // CLI reference: default builtin AI, 25 turns, seed 7, with the order log.
    let cli = totctl()
        .arg("observe")
        .arg(&save)
        .args(["--turns", "25", "--seed", "7", "--stream"])
        .arg(&stream)
        .arg("--log")
        .arg(&log)
        .output()
        .unwrap();
    assert!(
        cli.status.success(),
        "{}",
        String::from_utf8_lossy(&cli.stderr)
    );
    let stream_text = std::fs::read_to_string(&stream).unwrap();
    let lines: Vec<&str> = stream_text.lines().collect();
    assert_eq!(lines.len(), 26, "frame 0 plus one per turn");
    let cli_replay = totctl()
        .arg("replay")
        .arg(&save)
        .arg(&log)
        .output()
        .unwrap();
    assert!(
        cli_replay.status.success(),
        "{}",
        String::from_utf8_lossy(&cli_replay.stderr)
    );

    let responses = mcp_session(
        &home,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                   "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                              "clientInfo": {"name": "test", "version": "0"}}}),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            // First window: frames 0..8 are exactly the stream's first lines.
            tool_call(
                2,
                "save_observe",
                json!({"path": save, "turns": 25, "seed": 7, "from": 0, "count": 8}),
            ),
            // Default window: the last `count` frames.
            tool_call(
                3,
                "save_observe",
                json!({"path": save, "turns": 25, "seed": 7, "count": 8}),
            ),
            // A filtered view keeps only the requested civilization.
            tool_call(
                4,
                "save_observe",
                json!({"path": save, "turns": 2, "seed": 7, "view": "2", "count": 1}),
            ),
            // Replay verification equals the CLI's stdout.
            tool_call(
                5,
                "save_replay_verify",
                json!({"path": save, "log": std::fs::read_to_string(&log).unwrap()}),
            ),
        ],
    );
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "totctl");

    let observed = blocks(&responses[1]);
    assert!(observed[0].contains("Ph_Auto2.SAV"), "{}", observed[0]);
    assert!(observed[0].contains("frames      : 26"), "{}", observed[0]);
    assert_eq!(observed.len(), 9, "header + eight frames");
    for (block, line) in observed[1..].iter().zip(&lines[..8]) {
        assert_eq!(block, line, "frame blocks are the --stream file's lines");
    }

    let tail = blocks(&responses[2]);
    for (block, line) in tail[1..].iter().zip(&lines[18..]) {
        assert_eq!(block, line, "the default window is the last frames");
    }

    let filtered = blocks(&responses[3]);
    let frame: Value = serde_json::from_str(&filtered[1]).unwrap();
    let views = frame["views"].as_array().unwrap();
    assert!(!views.is_empty(), "civ 2 has a view in this fixture");
    assert!(views.iter().all(|v| v["civ"] == json!(2)), "{frame}");

    assert_eq!(
        blocks(&responses[4])[0],
        String::from_utf8(cli_replay.stdout).unwrap(),
        "save_replay_verify returns the CLI replay line"
    );

    // Out-of-range windows are refused, and paths outside the roots never run.
    let out_of_range = mcp_session(
        &home,
        &[tool_call(
            1,
            "save_observe",
            json!({"path": save, "turns": 2, "seed": 7, "from": 3, "count": 1}),
        )],
    );
    assert!(
        error_text(&out_of_range[0]).contains("out of range"),
        "{}",
        out_of_range[0]
    );
    let outside = mcp_session(
        &dir,
        &[tool_call(
            1,
            "save_observe",
            json!({"path": save, "turns": 2, "seed": 7}),
        )],
    );
    assert!(
        error_text(&outside[0]).contains("outside the configured roots"),
        "{}",
        outside[0]
    );
    // #275: an outside-root symlink to an in-root save, with an authored
    // sibling Rules.txt beside the link, cannot smuggle that read past the
    // root boundary (first review round reproduction).
    let link_dir = dir.join("outside-link");
    std::fs::create_dir_all(&link_dir).unwrap();
    std::fs::write(link_dir.join("Rules.txt"), "@CIVILIZE\n").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        home.join("Original/Ph_Auto2.SAV"),
        link_dir.join("Linked.SAV"),
    )
    .unwrap();
    #[cfg(unix)]
    {
        let smuggle = mcp_session(
            &home,
            &[tool_call(
                1,
                "save_summary",
                json!({"path": link_dir.join("Linked.SAV")}),
            )],
        );
        assert!(
            error_text(&smuggle[0]).contains("outside the configured roots"),
            "{}",
            smuggle[0]
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
}
