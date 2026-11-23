//! Verify a staged checkpoint at both sides of the pinned handoff.
use super::OriginalCapture;
use engine::{
    campaign::AiPolicy,
    events::EventEngine,
    human::{Command, HumanSession, Phase, Request, ScenarioInput},
};
use serde_json::{json, Value};

pub(super) fn check(capture: &OriginalCapture, human: u8) -> Result<Value, String> {
    let files = capture.files();
    let rules = &capture.resources().rules;
    let events = files.events.clone().map(EventEngine::new);
    let resources = capture.identity();
    let mut session = HumanSession::scenario_initialized_staged_round(
        ScenarioInput {
            path: &files.path,
            snapshot: &files.bytes,
            rules: rules.clone(),
            events: events.clone(),
        },
        human,
        AiPolicy::BuiltIn1,
        7,
        resources,
    )?;
    let entry = session.view();
    if entry.phase != Phase::Human || entry.decision.is_some() {
        return Err("staged resume needs an open human order phase".into());
    }
    let initializer = session.initializer().ok_or("staged initializer missing")?;
    let schema = initializer.schema();
    let profile = initializer.profile();
    session.submit(Request {
        civ: human,
        revision: entry.revision,
        id: 1,
        command: Command::EndTurn,
    })?;
    let restore = |text: &str| {
        HumanSession::restore_with(
            rules.clone(),
            resources,
            text,
            Some((&files.bytes, events.clone())),
        )
    };
    let before_pass = session.checkpoint(resources);
    let mut replay = restore(&before_pass)?;
    if session.advance(1) != 1 || replay.advance(1) != 1 {
        return Err("staged city pass did not advance exactly once".into());
    }
    let after_pass = session.checkpoint(resources);
    let order_log = |text: &str| {
        text.lines()
            .find(|line| line.starts_with("log "))
            .map(str::to_owned)
    };
    if replay.checkpoint(resources) != after_pass
        || session.view().observation.turn != entry.observation.turn
        || order_log(&before_pass).is_none()
        || order_log(&after_pass) != order_log(&before_pass)
    {
        return Err("staged city pass changed turn or orders, or did not replay".into());
    }
    let mut replay = restore(&after_pass)?;
    if session.advance(1) != 1 || replay.advance(1) != 1 {
        return Err("staged opponent order did not advance exactly once".into());
    }
    let after_order = session.checkpoint(resources);
    if replay.checkpoint(resources) != after_order
        || session.view().observation.turn != entry.observation.turn
        || order_log(&after_order).is_none()
        || order_log(&after_order) == order_log(&after_pass)
    {
        return Err("staged opponent order did not reach a replayable same-turn log entry".into());
    }
    Ok(json!({
        "verified": true,
        "initializer_schema": schema,
        "profile": profile,
        "same_turn": true,
        "first_work_item": "city-pass",
        "second_work_item": "opponent-order",
        "checkpoints_replay": true,
    }))
}
