//! `totctl campaign --record` and `totctl record info|verify|recover` (#206).
use std::process::{Command, Output};

fn totctl(args: &[&str], dir: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_totctl"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn text(out: &Output) -> String {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn err(out: &Output) -> String {
    assert!(!out.status.success());
    String::from_utf8(out.stderr.clone()).unwrap()
}

#[test]
fn recorded_campaigns_verify_and_interrupted_journals_recover() {
    let dir = std::env::temp_dir().join(format!("totengine-record-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir(&dir).unwrap();
    let base = include_str!("../../engine/tests/fixtures/campaign-rules.txt");
    std::fs::write(
        dir.join("Rules.txt"),
        format!(
            "{base}\n{}",
            include_str!("../../engine/tests/fixtures/builtin-settings.txt")
        ),
    )
    .unwrap();
    std::fs::write(dir.join("Other.txt"), base).unwrap();
    let setup = [
        "--seed", "3", "--width", "40", "--height", "30", "--turns", "5",
    ];

    // Recording does not change the run's output.
    let plain = text(&totctl(
        &[&["campaign", "Rules.txt"][..], &setup].concat(),
        &dir,
    ));
    let recorded = text(&totctl(
        &[
            &["campaign", "Rules.txt"][..],
            &setup,
            &[
                "--record",
                "run.totrec",
                "--record-every",
                "2",
                "--stream",
                "s.ndjson",
            ],
        ]
        .concat(),
        &dir,
    ));
    assert_eq!(recorded, plain);
    assert!(dir.join("run.totrec").exists());
    assert!(!dir.join("run.totrec.journal").exists(), "journal removed");

    let info = text(&totctl(&["record", "info", "run.totrec"], &dir));
    assert!(info.contains("(Reproducible)"), "{info}");
    assert!(info.contains("progress    : 5\n"), "{info}");
    assert!(
        info.contains("observer    : 6 frame(s) from round 0, all-views (full state)"),
        "{info}"
    );
    let verified = text(&totctl(
        &["record", "verify", "run.totrec", "Rules.txt"],
        &dir,
    ));
    let fingerprint = plain
        .lines()
        .find_map(|l| l.strip_prefix("fingerprint : "))
        .unwrap();
    assert!(
        verified.contains(&format!(
            "replay reproduces the recording; fingerprint {fingerprint}"
        )),
        "{verified}"
    );
    // The bundle's frames are the --stream file.
    let bundle = std::fs::read_to_string(dir.join("run.totrec")).unwrap();
    assert!(bundle.contains(&std::fs::read_to_string(dir.join("s.ndjson")).unwrap()));

    // Wrong Rules give a diagnostic; outputs are never overwritten.
    let wrong = err(&totctl(
        &["record", "verify", "run.totrec", "Other.txt"],
        &dir,
    ));
    assert!(wrong.contains("Rules identity differs"), "{wrong}");
    let again = err(&totctl(
        &[
            &["campaign", "Rules.txt"][..],
            &setup,
            &["--record", "run.totrec"],
        ]
        .concat(),
        &dir,
    ));
    assert!(again.contains("already exists"), "{again}");
    assert_eq!(
        std::fs::read_to_string(dir.join("run.totrec")).unwrap(),
        bundle
    );
    let same = err(&totctl(
        &["campaign", "Rules.txt", "--record", "x", "--log", "x"],
        &dir,
    ));
    assert!(same.contains("must differ"), "{same}");

    // An interrupted run: its journal survives and blocks a new recording
    // to the same name until it is recovered.
    let mut campaign = engine::campaign::Campaign::with_policy(
        engine::newgame::NewGameConfig::default(),
        tot_kit::parse_rules(&std::fs::read_to_string(dir.join("Rules.txt")).unwrap()),
        engine::campaign::AiPolicy::BuiltIn1,
    )
    .unwrap();
    let manifest =
        tot_record::Bundle::campaign(&campaign, tot_record::StopReason::Stopped, "test", None)
            .manifest;
    let journal_path = dir.join("crash.totrec.journal");
    let mut journal = tot_record::Journal::create(&journal_path, &manifest).unwrap();
    journal.record(0, &campaign.checkpoint()).unwrap();
    campaign.advance(3).unwrap();
    journal.record(3, &campaign.checkpoint()).unwrap();
    drop(journal);
    // A torn final write.
    let mut torn = std::fs::read_to_string(&journal_path).unwrap();
    torn.push_str("record 9 123 0000\npartial");
    std::fs::write(&journal_path, torn).unwrap();
    let blocked = err(&totctl(
        &[
            "campaign",
            "Rules.txt",
            "--turns",
            "1",
            "--record",
            "crash.totrec",
        ],
        &dir,
    ));
    assert!(blocked.contains("crash.totrec.journal"), "{blocked}");
    let recovered = text(&totctl(
        &["record", "recover", "crash.totrec.journal", "crash.totrec"],
        &dir,
    ));
    assert!(
        recovered.contains("stop        : Interrupted"),
        "{recovered}"
    );
    assert!(recovered.contains("2 complete record(s)"), "{recovered}");
    assert!(
        recovered.contains("trailing byte(s) discarded"),
        "{recovered}"
    );
    let verified = text(&totctl(
        &["record", "verify", "crash.totrec", "Rules.txt"],
        &dir,
    ));
    assert!(verified.contains("progress    : 3\n"), "{verified}");
    assert!(verified.contains(&format!("{:016x}", campaign.game.world.checksum())));
    let exists = err(&totctl(
        &["record", "recover", "crash.totrec.journal", "crash.totrec"],
        &dir,
    ));
    assert!(exists.contains("crash.totrec"), "{exists}");
    assert!(err(&totctl(
        &["record", "verify", "missing.totrec", "Rules.txt"],
        &dir
    ))
    .contains("missing.totrec"));
    std::fs::remove_dir_all(dir).unwrap();
}
