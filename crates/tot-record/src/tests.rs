use super::*;
use engine::human::{Command, Decision, Request};

fn rules() -> Rules {
    tot_kit::parse_rules(&format!(
        "{}\n{}",
        include_str!("../../engine/tests/fixtures/campaign-rules.txt"),
        include_str!("../../engine/tests/fixtures/builtin-settings.txt")
    ))
}

fn config(seed: u32) -> NewGameConfig {
    NewGameConfig {
        gameplay_seed: seed,
        world: engine::worldgen::WorldConfig {
            seed,
            width: 40,
            height: 30,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn campaign(rounds: u32) -> Campaign {
    let mut c = Campaign::with_policy(config(5), rules(), AiPolicy::BuiltIn1).unwrap();
    c.advance(rounds).unwrap();
    c
}

fn bundle(rounds: u32) -> Bundle {
    let c = campaign(rounds);
    Bundle::campaign(&c, c.outcome().into(), "test", None)
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("tot-record-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir(&dir).unwrap();
    dir
}

#[test]
fn campaign_bundle_round_trips_and_verifies_by_replay() {
    let c = campaign(6);
    let b = Bundle::campaign(&c, c.outcome().into(), "test", None);
    let text = b.encode().unwrap();
    assert!(text.starts_with("totengine-recording 1\nengine "));
    assert!(text.contains("\ncontent reproducible\nprogress 6\nstop turn-limit\n"));
    let decoded = Bundle::decode(&text).unwrap();
    assert_eq!(decoded, b);
    assert_eq!(decoded.encode().unwrap(), text, "canonical encoding");
    let verified = decoded.verify(&rules()).unwrap();
    assert_eq!(verified.content, Content::Reproducible);
    assert_eq!(verified.fingerprint, Some(c.game.world.checksum()));
}

#[test]
fn corrupt_truncated_and_foreign_inputs_are_rejected() {
    let text = bundle(3).encode().unwrap();
    let cases = [
        (
            text.replace("totengine-recording 1", "totengine-recording 2"),
            "unsupported recording format",
        ),
        ("hello\n".to_string(), "not a ToTEngine recording"),
        (text[..text.len() - 4].to_string(), "truncated"),
        (text[..text.len() / 2].to_string(), "truncated"),
        (text.replace("\nend 1\n", "\nend 2\n"), "trailer"),
        (text.clone() + "junk\n", "trailer"),
        (text.replace("rounds 3", "rounds 4"), "corrupt"),
        (
            text.replace("content reproducible", "content view-only"),
            "declared content",
        ),
        (
            text.replace("stop turn-limit", "stop sideways"),
            "invalid stop reason",
        ),
        (
            text.replace("kind campaign", "kind tournament"),
            "unknown recording kind",
        ),
    ];
    for (bad, message) in cases {
        let err = Bundle::decode(&bad).unwrap_err();
        assert!(err.contains(message), "{message:?} not in {err:?}");
    }
}

#[test]
fn wrong_rules_engine_or_claims_fail_verification_with_diagnostics() {
    let b = bundle(4);
    let changed = tot_kit::parse_rules(&(tot_kit::emit_rules(&rules()) + "\n@EXTRA\nchanged\n"));
    assert!(b
        .verify(&changed)
        .unwrap_err()
        .contains("Rules identity differs"));
    let mut old = b.clone();
    old.manifest.engine = "0.0.1".into();
    assert!(old.verify(&rules()).unwrap_err().contains("same engine"));
    let mut wrong_progress = b.clone();
    wrong_progress.manifest.progress = 5;
    assert!(wrong_progress
        .verify(&rules())
        .unwrap_err()
        .contains("progress"));
    let mut wrong_stop = b.clone();
    wrong_stop.manifest.stop = StopReason::NoCivilizationsRemaining;
    assert!(wrong_stop
        .verify(&rules())
        .unwrap_err()
        .contains("stop reason"));
    let mut tampered = b;
    tampered.gameplay = Some(
        tampered
            .gameplay
            .unwrap()
            .replace("fingerprint ", "fingerprint 0"),
    );
    assert!(tampered.verify(&rules()).is_err());
}

#[test]
fn observer_frames_are_regenerated_and_labelled() {
    let c = campaign(4);
    for perspective in [
        Perspective::AllViews,
        Perspective::Spectator,
        Perspective::Civ(2),
    ] {
        let frames = campaign_frames(&rules(), c.config(), c.policy(), 1, 4, perspective).unwrap();
        assert_eq!(frames.lines().count(), 4, "rounds 1..=4");
        let b = Bundle::campaign(
            &c,
            StopReason::TurnLimit,
            "test",
            Some(Observer {
                perspective,
                from_round: 1,
                frames: frames.clone(),
            }),
        );
        let text = b.encode().unwrap();
        assert!(text.contains(&format!("section observer {}@1 ", perspective.encode())));
        let decoded = Bundle::decode(&text).unwrap();
        assert_eq!(decoded.verify(&rules()).unwrap().frames, 4);
        // A frame that does not come from this run is caught.
        let mut forged = decoded;
        let observer = forged.observer.as_mut().unwrap();
        observer.frames = observer.frames.replacen("\"turn\":", "\"turn\":1", 1);
        assert!(forged
            .verify(&rules())
            .unwrap_err()
            .contains("observer frames differ"));
    }
    // A view-only export: frames without gameplay.
    let frames =
        campaign_frames(&rules(), c.config(), c.policy(), 0, 2, Perspective::Civ(1)).unwrap();
    let mut view_only = Bundle::campaign(
        &c,
        StopReason::Stopped,
        "test",
        Some(Observer {
            perspective: Perspective::Civ(1),
            from_round: 0,
            frames,
        }),
    );
    view_only.gameplay = None;
    let text = view_only.encode().unwrap();
    assert!(text.contains("\ncontent view-only\n"));
    let verified = Bundle::decode(&text).unwrap().verify(&rules()).unwrap();
    assert_eq!(verified.content, Content::ViewOnly);
    assert_eq!(verified.fingerprint, None);
    assert!(!Perspective::Civ(1).full_state() && Perspective::Spectator.full_state());
}

#[test]
fn presentation_is_validated_but_inert() {
    let c = campaign(3);
    let mut b = Bundle::campaign(&c, StopReason::TurnLimit, "test", None);
    let before = b.verify(&rules()).unwrap();
    b.presentation = Some(
        "{\"round\":1,\"kind\":\"note\",\"text\":\"first contact\"}\n\
         {\"round\":3,\"kind\":\"camera\",\"civ\":2,\"x\":10,\"y\":12}\n"
            .into(),
    );
    let decoded = Bundle::decode(&b.encode().unwrap()).unwrap();
    assert_eq!(
        decoded.verify(&rules()).unwrap(),
        before,
        "notes change nothing"
    );
    for (bad, message) in [
        (
            "{\"round\":9,\"kind\":\"note\"}\n",
            "after the recording ends",
        ),
        ("{\"kind\":\"note\"}\n", "missing round"),
        ("{\"round\":1}\n", "missing kind"),
        ("not json\n", "presentation line 1"),
        ("{\"round\":1,\"kind\":\"note\"}", "newline"),
    ] {
        b.presentation = Some(bad.into());
        assert!(b.encode().unwrap_err().contains(message), "{bad}");
    }
}

#[test]
fn saving_is_atomic_and_never_overwrites() {
    let dir = temp_dir("save");
    let path = dir.join("run.totrec");
    let text = bundle(2).encode().unwrap();
    write_new(&path, &text).unwrap();
    let err = write_new(&path, "other").unwrap_err();
    assert!(err.contains("create"), "{err}");
    assert_eq!(
        read_limited(&path).unwrap(),
        text,
        "existing bundle untouched"
    );
    let leftovers: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(leftovers, ["run.totrec"], "no temporary files remain");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_interrupted_journal_recovers_its_last_complete_checkpoint() {
    let dir = temp_dir("journal");
    let path = dir.join("run.totrec.journal");
    let mut c = Campaign::with_policy(config(5), rules(), AiPolicy::BuiltIn1).unwrap();
    let manifest = Bundle::campaign(&c, StopReason::Stopped, "test", None).manifest;
    let mut journal = Journal::create(&path, &manifest).unwrap();
    assert!(Journal::create(&path, &manifest).is_err(), "never reused");
    for _ in 0..3 {
        c.advance(2).unwrap();
        journal.record(c.rounds(), &c.checkpoint()).unwrap();
    }
    drop(journal);
    let complete = read_limited(&path).unwrap();
    // Simulate a crash in the middle of a fourth record.
    c.advance(2).unwrap();
    let next = c.checkpoint();
    let torn = format!(
        "{complete}record 8 {}\n{}",
        identity(&next),
        &next[..next.len() / 2]
    );
    for (text, records, discarded) in [(&complete, 3, 0), (&torn, 3, torn.len() - complete.len())] {
        let recovered = recover(text).unwrap();
        assert_eq!(recovered.records, records);
        assert_eq!(recovered.discarded_bytes, discarded);
        let b = recovered.bundle;
        assert_eq!(b.manifest.stop, StopReason::Interrupted);
        assert_eq!(b.manifest.progress, 6);
        let verified = Bundle::decode(&b.encode().unwrap())
            .unwrap()
            .verify(&rules())
            .unwrap();
        assert_eq!(verified.content, Content::Reproducible);
        // The recovered prefix continues exactly like the uninterrupted run.
        let mut resumed = Campaign::restore(rules(), b.gameplay.as_ref().unwrap()).unwrap();
        resumed.advance(2).unwrap();
        assert_eq!(resumed.checkpoint(), next);
    }
    let header_only = complete.split("record ").next().unwrap();
    assert!(recover(header_only)
        .unwrap_err()
        .contains("no complete checkpoint"));
    assert!(recover("x\n").is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

fn human_session() -> HumanSession {
    let mut s = HumanSession::generated(config(7), rules(), 1).unwrap();
    for _ in 0..4 {
        let mut guard = 0;
        while s.phase() == engine::human::Phase::Opponents {
            assert!(s.advance(64) > 0);
            guard += 1;
            assert!(guard < 1000);
        }
        if let Some(Decision::Research { id, available, .. }) = s.view().decision {
            let request = Request {
                civ: 1,
                revision: s.view().revision,
                id: s.next_request_id().unwrap(),
                command: Command::Research {
                    decision: id,
                    advance: available[0],
                },
            };
            s.submit(request).unwrap();
            continue;
        }
        let request = Request {
            civ: 1,
            revision: s.view().revision,
            id: s.next_request_id().unwrap(),
            command: Command::EndTurn,
        };
        if s.submit(request).is_err() {
            break;
        }
    }
    s
}

#[test]
fn human_recordings_bind_resources_and_replay() {
    let s = human_session();
    let checkpoint = s.checkpoint("authored-test-resources-v1");
    let b = Bundle::human(
        &checkpoint,
        &rules(),
        "authored-test-resources-v1",
        StopReason::Stopped,
        "test",
    )
    .unwrap();
    assert_eq!(b.manifest.human, Some(1));
    assert!(
        b.manifest.progress >= 3,
        "turns played: {}",
        b.manifest.progress
    );
    assert!(
        checkpoint.contains("\ninput 1 "),
        "human inputs are recorded"
    );
    let text = b.encode().unwrap();
    assert!(text.contains("\nkind human\n") && text.contains("section resources identity "));
    let decoded = Bundle::decode(&text).unwrap();
    assert_eq!(decoded, b);
    assert_eq!(
        decoded.verify(&rules()).unwrap().content,
        Content::Reproducible
    );
    // A different resource identity is a diagnostic, not a different game.
    let mut moved = decoded.clone();
    moved.manifest.resources = Some("other-resources".into());
    assert!(moved
        .verify(&rules())
        .unwrap_err()
        .contains("resource identity"));
    assert!(Bundle::human(&checkpoint, &rules(), "wrong", StopReason::Stopped, "t").is_err());

    // Human journals carry the resource identity and recover too.
    let dir = temp_dir("human-journal");
    let path = dir.join("h.journal");
    let mut journal = Journal::create(&path, &b.manifest).unwrap();
    journal.record(b.manifest.progress, &checkpoint).unwrap();
    drop(journal);
    let recovered = recover(&read_limited(&path).unwrap()).unwrap().bundle;
    assert_eq!(recovered.manifest.resources, b.manifest.resources);
    assert!(recovered.verify(&rules()).is_ok());
    std::fs::remove_dir_all(dir).unwrap();
}
