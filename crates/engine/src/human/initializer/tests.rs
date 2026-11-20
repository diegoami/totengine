use super::*;
use crate::campaign::AiPolicy;
use crate::events::EventEngine;

const DEPENDENCIES: &str = "totengine-scenario-dependencies 1\nrules Original/Rules.txt 10 0000000000000001\nevents Original/Events.txt 20 0000000000000002\ntext-labels absent\ntext-describe absent\n";

fn rules() -> Rules {
    tot_kit::parse_rules(&format!(
        "{}\n{}",
        include_str!("../../../tests/fixtures/campaign-rules.txt"),
        include_str!("../../../tests/fixtures/builtin-settings.txt")
    ))
}

fn snapshot() -> Vec<u8> {
    let mut bytes = crate::tests_support::synthetic_save_bytes();
    bytes[678] = 0;
    bytes[680] = 1;
    bytes[681] = 1;
    bytes[687] = 2;
    bytes
}

fn events(text: &str) -> Option<EventEngine> {
    Some(EventEngine::new(tot_kit::parse_events(text).unwrap()))
}

fn initialized(save: &[u8], text: &str) -> HumanSession {
    HumanSession::scenario_initialized(
        ScenarioInput {
            path: "Original/Test.sav",
            snapshot: save,
            rules: rules(),
            events: events(text),
        },
        1,
        AiPolicy::AiV1,
        9,
        DEPENDENCIES,
    )
    .unwrap()
}

fn send(session: &mut HumanSession, command: Command) {
    session
        .submit(Request {
            civ: 1,
            revision: session.revision,
            id: session.next_request_id().unwrap(),
            command,
        })
        .unwrap();
}

fn choose(session: &mut HumanSession) {
    while let Some(Decision::Research { id, available, .. }) = session.decision.clone() {
        send(
            session,
            Command::Research {
                decision: id,
                advance: available[0],
            },
        );
    }
}

fn restore(
    save: &[u8],
    events_text: &str,
    checkpoint: &str,
    dependencies: &str,
) -> Result<HumanSession, String> {
    HumanSession::restore_with(
        rules(),
        dependencies,
        checkpoint,
        Some((save, events(events_text))),
    )
}

#[test]
fn format_three_binds_the_initial_state_and_replays_partial_opponent_work() {
    let save = snapshot();
    let mut s = initialized(&save, "");
    let initial = s.initializer().unwrap().clone();
    assert_eq!(initial.mode, ScenarioStartMode::SavedRound);
    assert_eq!(initial.phase, Phase::Human);
    assert_eq!(initial.cursor, 1);
    assert_eq!(initial.human, 1);
    assert_eq!(initial.dependencies, DEPENDENCIES);
    assert!(initial
        .descriptor()
        .contains("profile experimental-scenario-v1"));
    assert_eq!(
        HumanSession::checkpoint_scenario(&s.checkpoint(DEPENDENCIES)).unwrap(),
        initial.source
    );
    choose(&mut s);
    send(&mut s, Command::EndTurn);
    assert_eq!(s.advance(1), 1);
    let text = s.checkpoint(DEPENDENCIES);
    assert!(text.starts_with("totengine-human 3\n"));
    assert_eq!(
        s.checkpoint("replacement manifest"),
        text,
        "captured dependencies cannot be rebound by checkpoint()"
    );
    let back = restore(&save, "", &text, DEPENDENCIES).unwrap();
    assert_eq!(back.view(), s.view());
    assert_eq!(back.initializer(), Some(&initial));
    assert_eq!(back.checkpoint(DEPENDENCIES), text);
}

#[test]
fn startup_delay_and_latch_state_are_rebuilt_before_checkpoint_operations() {
    let save = snapshot();
    let event = "@IF\nSCENARIOLOADED\n@THEN\nJUSTONCE\nDELAY\ndelay=2\nCHANGEMONEY\nreceiver=Testers\namount=7\n@ENDIF\n";
    let mut s = initialized(&save, event);
    let ev = s.game.events.as_ref().unwrap();
    assert_eq!(ev.fired_count(), 1);
    assert_eq!(ev.pending.len(), 1);
    assert_eq!(ev.pending[0].1, 2);
    choose(&mut s);
    send(&mut s, Command::EndTurn);
    s.advance(1);
    let text = s.checkpoint(DEPENDENCIES);
    let back = restore(&save, event, &text, DEPENDENCIES).unwrap();
    assert_eq!(back.game.events.as_ref().unwrap().fired_count(), 1);
    assert_eq!(back.game.events.as_ref().unwrap().pending[0].1, 2);
    assert_eq!(back.checkpoint(DEPENDENCIES), text);
}

#[test]
fn initializer_assertions_are_verified_before_any_journal_entry() {
    let save = snapshot();
    let s = initialized(&save, "");
    let text = s.checkpoint(DEPENDENCIES);
    for (before, after) in [
        ("initializer 1 ", "initializer 99 "),
        (
            "profile experimental-scenario-v1",
            "profile classic-surface-032-v1",
        ),
        ("mode saved-round", "mode fresh-turn"),
        ("phase Human cursor 1", "phase Opponents cursor 1"),
        ("movement saved-thirds-clamp-v1", "movement unknown-v9"),
        (
            "knowledge saved-exploration-engine-sight-v1",
            "knowledge reveal-everything",
        ),
    ] {
        let bad = text
            .replacen(before, after, 1)
            .replace("end\n", "invalid-operation\nend\n");
        assert_ne!(text, bad);
        let error = restore(&save, "", &bad, DEPENDENCIES).err().unwrap();
        assert!(error.contains("initializer"), "{error}");
    }
    for field in ["world", "memory", "choice", "events", "rng"] {
        let line = text
            .lines()
            .find(|l| l.starts_with("initializer "))
            .unwrap();
        let mut fields: Vec<_> = line.split(' ').map(str::to_string).collect();
        let at = fields.iter().position(|p| p == field).unwrap() + 1;
        fields[at] = "ffffffffffffffff".into();
        let bad = text.replace(line, &fields.join(" "));
        assert!(restore(&save, "", &bad, DEPENDENCIES)
            .err()
            .unwrap()
            .contains("initializer"));
    }
    assert_eq!(
        s.checkpoint(DEPENDENCIES),
        text,
        "all failed restores leave the active session unchanged"
    );
}

#[test]
fn snapshot_dependencies_and_event_changes_fail_even_without_world_changes() {
    let save = snapshot();
    let event_a = "@IF\nTURN\nturn=Every\n@THEN\nTEXT\nA\nENDTEXT\n@ENDIF\n";
    let event_b = event_a.replace("\nA\n", "\nB\n");
    let s = initialized(&save, event_a);
    let text = s.checkpoint(DEPENDENCIES);
    assert_eq!(
        initialized(&save, &event_b).game.world.checksum(),
        s.game.world.checksum()
    );
    assert!(restore(&save, &event_b, &text, DEPENDENCIES)
        .err()
        .unwrap()
        .contains("initializer"));
    let different = DEPENDENCIES.replace(
        "text-labels absent",
        "text-labels Labels.txt 1 0000000000000000",
    );
    assert!(restore(&save, event_a, &text, &different)
        .err()
        .unwrap()
        .contains("resource identity"));
    let mut changed = save.clone();
    changed[25] ^= 1;
    assert!(restore(&changed, event_a, &text, DEPENDENCIES)
        .err()
        .unwrap()
        .contains("scenario file differs"));
    assert!(HumanSession::restore(rules(), DEPENDENCIES, &text)
        .err()
        .unwrap()
        .contains("scenario file"));
    let mut running = events(event_a).unwrap();
    running.flags[1] = 1;
    assert!(HumanSession::scenario_initialized(
        ScenarioInput {
            path: "Original/Test.sav",
            snapshot: &save,
            rules: rules(),
            events: Some(running),
        },
        1,
        AiPolicy::AiV1,
        9,
        DEPENDENCIES
    )
    .err()
    .unwrap()
    .contains("pristine"));
}

#[test]
fn initialized_terminal_state_replays_without_rebinding_the_initializer() {
    let mut save = snapshot();
    save[2944 + 3348 + 25] = 5;
    let event = "@IF\nRECEIVEDTECHNOLOGY\nreceiver=Anybody\nfuturetech=5\n@AND\nRANDOMTURN\ndenominator=1\n@THEN\nTEXT\nFinished\nENDTEXT\nENDGAME\nendscreens=yes\nPLAYAVIFILE\nTest.avi\n@ENDIF\n";
    let mut s = initialized(&save, event);
    let initial = s.initializer().unwrap().clone();
    choose(&mut s);
    send(&mut s, Command::EndTurn);
    for _ in 0..2000 {
        if s.phase() != Phase::Opponents {
            break;
        }
        s.advance(256);
    }
    assert_eq!(s.phase(), Phase::GameOver);
    let text = s.checkpoint(DEPENDENCIES);
    let mut back = restore(&save, event, &text, DEPENDENCIES).unwrap();
    assert_eq!(back.initializer(), Some(&initial));
    assert_eq!(back.view(), s.view());
    assert_eq!(back.advance(256), 0);
    assert_eq!(back.checkpoint(DEPENDENCIES), text);
    let rng_line = text.lines().find(|line| line.starts_with("rng ")).unwrap();
    let bad = text.replace(rng_line, "rng 00000000 0 00000000 0");
    assert!(restore(&save, event, &bad, DEPENDENCIES).is_err());
}

#[test]
fn legacy_scenario_format_two_is_not_silently_upgraded() {
    let save = snapshot();
    let s = HumanSession::scenario(
        "Original/Test.sav",
        &save,
        rules(),
        events(""),
        1,
        AiPolicy::AiV1,
        9,
    )
    .unwrap();
    let text = s.checkpoint("legacy resources");
    assert!(text.starts_with("totengine-human 2\n"));
    let back = restore(&save, "", &text, "legacy resources").unwrap();
    assert!(back.initializer().is_none());
    assert_eq!(back.checkpoint("legacy resources"), text);
    assert_eq!(back.view(), s.view());
}
