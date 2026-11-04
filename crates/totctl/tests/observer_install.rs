//! #80/#81: real 100-turn all-AI stream is byte-identical to the stream
//! reconstructed from its canonical order log. No game data is committed.
use std::{path::PathBuf, process::Command};

#[test]
fn hundred_turn_observer_cli_replays_identically() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(PathBuf::from) else {
        eprintln!("skipping: CIVTOT_HOME not set");
        return;
    };
    let save = home.join("Original/Ph_Auto2.SAV");
    if !save.exists() {
        eprintln!("skipping: standard save not present");
        return;
    }
    let dir = std::env::temp_dir().join(format!("totengine-observer-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let live = dir.join("live.ndjson");
    let replay = dir.join("replay.ndjson");
    let log = dir.join("orders.log");
    let result = Command::new(env!("CARGO_BIN_EXE_totctl"))
        .arg("observe")
        .arg(&save)
        .args(["--turns", "100", "--seed", "3", "--stream"])
        .arg(&live)
        .arg("--log")
        .arg(&log)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result = Command::new(env!("CARGO_BIN_EXE_totctl"))
        .arg("observe")
        .arg(&save)
        .arg("--replay")
        .arg(&log)
        .arg("--stream")
        .arg(&replay)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let a = std::fs::read(&live).unwrap();
    assert_eq!(a, std::fs::read(&replay).unwrap());
    assert_eq!(a.iter().filter(|&&b| b == b'\n').count(), 101);
    // The CLI refuses to overwrite an existing archive.
    let result = Command::new(env!("CARGO_BIN_EXE_totctl"))
        .arg("observe")
        .arg(&save)
        .args(["--turns", "0", "--stream"])
        .arg(&live)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert_eq!(a, std::fs::read(&live).unwrap());
    std::fs::remove_dir_all(dir).unwrap();
}
