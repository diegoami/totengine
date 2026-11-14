//! `totctl experiment check|run` (#203, generated-world slice).
use std::path::Path;
use std::process::{Command, Output};

fn totctl(args: &[&str], dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_totctl"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn ok(out: &Output) -> String {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn err(out: &Output) -> String {
    assert!(
        !out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    String::from_utf8(out.stderr.clone()).unwrap()
}

#[test]
fn manifests_validate_expand_run_reproducibly_and_never_overwrite() {
    let dir = std::env::temp_dir().join(format!("totengine-experiment-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(
        dir.join("Rules.txt"),
        format!(
            "{}\n{}",
            include_str!("../../engine/tests/fixtures/campaign-rules.txt"),
            include_str!("../../engine/tests/fixtures/builtin-settings.txt")
        ),
    )
    .unwrap();
    let manifest = serde_json::json!({
        "format": "totengine-experiment 1",
        "name": "difficulty-sweep",
        "start": {"kind": "generated", "rules": "Rules.txt", "width": 40, "height": 30},
        "policies": {"default": "builtin-1"},
        "rounds": 6,
        "seeds": [3, 5],
        "variants": [
            {"name": "easy", "difficulty": 0},
            {"name": "hard", "difficulty": 4, "policy": "ai-v1"}
        ]
    });
    let write = |name: &str, value: &serde_json::Value| {
        std::fs::write(dir.join(name), serde_json::to_string_pretty(value).unwrap()).unwrap();
    };
    write("m.json", &manifest);

    // check: four runs, seed variation separate from deliberate changes.
    let check = ok(&totctl(&["experiment", "check", "m.json"], &dir));
    assert!(check.contains("runs        : 4\n"), "{check}");
    assert!(check.contains("easy-s3"), "{check}");
    assert!(check.contains("changed difficulty=0"), "{check}");
    assert!(
        check.contains("changed difficulty=4,policy=\"ai-v1\""),
        "{check}"
    );

    // run: every run completes and writes a verifiable recording.
    let text = ok(&totctl(
        &["experiment", "run", "m.json", "--out", "out1"],
        &dir,
    ));
    assert_eq!(
        text,
        std::fs::read_to_string(dir.join("out1/summary.txt")).unwrap()
    );
    let summary: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("out1/summary.json")).unwrap())
            .unwrap();
    assert_eq!(summary["format"], "totengine-experiment-summary 1");
    let runs = summary["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 4);
    for run in runs {
        assert_eq!(run["status"], "completed", "{run}");
        let recording = format!("out1/{}", run["recording"].as_str().unwrap());
        let verified = ok(&totctl(
            &["record", "verify", &recording, "Rules.txt"],
            &dir,
        ));
        assert!(
            verified.contains(&format!(
                "fingerprint {}",
                run["fingerprint"].as_str().unwrap()
            )),
            "{verified}"
        );
    }

    // A run equals the plain `totctl campaign` with the same options.
    let hard = runs.iter().find(|r| r["id"] == "hard-s3").unwrap();
    let campaign = ok(&totctl(
        &[
            "campaign",
            "Rules.txt",
            "--policy",
            "ai-v1",
            "--seed",
            "3",
            "--difficulty",
            "4",
            "--width",
            "40",
            "--height",
            "30",
            "--turns",
            "6",
        ],
        &dir,
    ));
    assert!(
        campaign.contains(&format!(
            "fingerprint : {}",
            hard["fingerprint"].as_str().unwrap()
        )),
        "{campaign}"
    );

    // The same manifest reproduces byte-identical outputs.
    ok(&totctl(
        &["experiment", "run", "m.json", "--out", "out2"],
        &dir,
    ));
    for file in [
        "summary.json",
        "summary.txt",
        "easy-s3.totrec",
        "hard-s5.totrec",
    ] {
        assert_eq!(
            std::fs::read(dir.join("out1").join(file)).unwrap(),
            std::fs::read(dir.join("out2").join(file)).unwrap(),
            "{file}"
        );
    }

    // Existing outputs are never overwritten.
    let before = std::fs::read(dir.join("out1/summary.json")).unwrap();
    let again = err(&totctl(
        &["experiment", "run", "m.json", "--out", "out1"],
        &dir,
    ));
    assert!(again.contains("never overwritten"), "{again}");
    assert_eq!(
        std::fs::read(dir.join("out1/summary.json")).unwrap(),
        before
    );

    // Invalid manifests are rejected before anything runs.
    let with = |patch: serde_json::Value| {
        let mut m = manifest.clone();
        for (k, v) in patch.as_object().unwrap() {
            m[k] = v.clone();
        }
        m
    };
    let mut start_bad = manifest.clone();
    start_bad["start"]["width"] = serde_json::json!(5);
    let mut pinned = manifest.clone();
    pinned["start"]["rules_identity"] = serde_json::json!("1 0000000000000000");
    let mut extra = manifest.clone();
    extra["start"]["sea"] = serde_json::json!(true);
    let cases = [
        (
            with(serde_json::json!({"format": "totengine-experiment 2"})),
            "unsupported manifest format",
        ),
        (
            with(serde_json::json!({"start": {"kind": "scenario", "path": "X.SCN"}})),
            "admitted scenario entry",
        ),
        (
            with(
                serde_json::json!({"policies": {"default": "builtin-1", "per_civ": {"3": "ai-v1"}}}),
            ),
            "mixed policies are not supported",
        ),
        (
            with(serde_json::json!({"policies": {"default": "random"}})),
            "policy",
        ),
        (with(serde_json::json!({"seeds": []})), "at least one seed"),
        (
            with(serde_json::json!({"seeds": [1, 1]})),
            "must not repeat",
        ),
        (with(serde_json::json!({"rounds": 0})), "rounds must be"),
        (with(serde_json::json!({"engine": "0.0.1"})), "pins engine"),
        (
            with(serde_json::json!({"variants": [{"name": "a"}, {"name": "a"}]})),
            "unique",
        ),
        (
            with(serde_json::json!({"variants": [{"name": "a b"}]})),
            "letters, digits",
        ),
        (
            with(serde_json::json!({"variants": [{"name": "x", "colour": 1}]})),
            "invalid manifest",
        ),
        (
            with(serde_json::json!({"seeds": (0..300).collect::<Vec<u32>>()})),
            "exceeds 256 runs",
        ),
        (start_bad, "run easy-s3"),
        (pinned, "Rules identity differs"),
        (extra, "invalid manifest"),
    ];
    for (i, (bad, message)) in cases.into_iter().enumerate() {
        let name = format!("bad{i}.json");
        write(&name, &bad);
        let e = err(&totctl(&["experiment", "check", &name], &dir));
        assert!(e.contains(message), "case {i}: {message:?} not in {e}");
        let out = format!("never{i}");
        err(&totctl(&["experiment", "run", &name, "--out", &out], &dir));
        assert!(
            !dir.join(&out).exists(),
            "a rejected manifest creates no output"
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
}
