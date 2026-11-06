use std::process::Command;

#[test]
fn generated_cli_checkpoints_resume_and_refuse_overwrites() {
    let root = std::env::temp_dir().join(format!(
        "totengine-campaign-cli-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let rules = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../engine/tests/fixtures/campaign-rules.txt");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_totctl"))
            .arg("campaign")
            .arg(&rules)
            .args(args)
            .output()
            .unwrap()
    };
    let first = root.join("first.ctc");
    let resumed = root.join("resumed.ctc");
    let full = root.join("full.ctc");
    let first = first.to_str().unwrap();
    let resumed = resumed.to_str().unwrap();
    let full = full.to_str().unwrap();
    for args in [
        vec![
            "--policy",
            "ai-v1",
            "--seed",
            "7",
            "--turns",
            "17",
            "--checkpoint",
            first,
        ],
        vec!["--resume", first, "--turns", "23", "--checkpoint", resumed],
        vec![
            "--policy",
            "ai-v1",
            "--seed",
            "7",
            "--turns",
            "40",
            "--checkpoint",
            full,
        ],
    ] {
        let output = run(&args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("TurnLimit"));
    }
    assert_eq!(
        std::fs::read(resumed).unwrap(),
        std::fs::read(full).unwrap()
    );
    let before = std::fs::read(first).unwrap();
    assert!(!run(&["--policy", "ai-v1", "--checkpoint", first])
        .status
        .success());
    assert_eq!(before, std::fs::read(first).unwrap());
    assert!(!run(&["--resume", first, "--seed", "3"]).status.success());
    assert!(!run(&["--width", "33"]).status.success());
    assert!(!run(&["--turns", "30001"]).status.success());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn builtin_policy_cli_is_checkpointed_and_replayable() {
    let root = std::env::temp_dir().join(format!("totengine-builtin-cli-{}", std::process::id()));
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
    let checkpoint = root.join("prefix.ctc");
    let resumed = root.join("resumed.ctc");
    let full = root.join("full.ctc");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_totctl"))
            .arg("campaign")
            .arg(&rules)
            .args(args)
            .output()
            .unwrap()
    };
    for args in [
        vec![
            "--policy",
            "builtin-1",
            "--seed",
            "7",
            "--turns",
            "17",
            "--checkpoint",
            checkpoint.to_str().unwrap(),
        ],
        vec![
            "--resume",
            checkpoint.to_str().unwrap(),
            "--turns",
            "23",
            "--checkpoint",
            resumed.to_str().unwrap(),
        ],
        vec![
            "--seed",
            "7",
            "--turns",
            "40",
            "--checkpoint",
            full.to_str().unwrap(),
        ],
    ] {
        let result = run(&args);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    assert_eq!(
        std::fs::read(&resumed).unwrap(),
        std::fs::read(&full).unwrap()
    );
    assert!(!run(&[
        "--resume",
        checkpoint.to_str().unwrap(),
        "--policy",
        "ai-v1"
    ])
    .status
    .success());
    assert!(!run(&["--policy", "unknown"]).status.success());
    std::fs::remove_dir_all(root).unwrap();
}
