//! Verify a staged checkpoint at both sides of the pinned handoff.
use super::OriginalCapture;
use engine::{
    campaign::AiPolicy,
    events::EventEngine,
    human::{Command, HumanSession, Phase, Request, ScenarioInput},
    player::OrderResult,
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
    let order_log = checkpoint_line(&before_pass, "log ")?;
    let city_state = checkpoint_line(&after_pass, "world ")?;
    if replay.checkpoint(resources) != after_pass
        || session.view().observation.turn != entry.observation.turn
        || checkpoint_line(&after_pass, "log ")? != order_log
        || checkpoint_line(&before_pass, "world ")? == city_state
    {
        return Err(
            "staged city pass did not change the world on the same turn before orders".into(),
        );
    }
    let mut replay = restore(&after_pass)?;
    if session.advance(1) != 1 || replay.advance(1) != 1 {
        return Err("staged opponent order did not advance exactly once".into());
    }
    let after_order = session.checkpoint(resources);
    if replay.checkpoint(resources) != after_order
        || session.view().observation.turn != entry.observation.turn
        || checkpoint_line(&after_order, "log ")? == order_log
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

pub(super) fn check_saved_movement(
    capture: &OriginalCapture,
    human: u8,
    native: &Value,
) -> Result<Value, String> {
    let files = capture.files();
    let rules = &capture.resources().rules;
    let events = files.events.clone().map(EventEngine::new);
    let resources = capture.identity();
    let mut session = HumanSession::scenario_initialized_saved_movement(
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
    let slot = native["before"]["active_slot"]
        .as_u64()
        .and_then(|slot| usize::try_from(slot).ok())
        .ok_or("native saved movement evidence has no unit slot")?;
    let to: (u16, u16) = serde_json::from_value(native["unit_position_after"].clone())
        .map_err(|error| format!("native saved movement evidence: {error}"))?;
    let entry = session.view();
    let unit = *entry
        .units
        .iter()
        .find(|unit| unit.slot == slot)
        .ok_or("saved movement unit missing")?;
    let result = session.submit(Request {
        civ: human,
        revision: entry.revision,
        id: 1,
        command: Command::Move { unit, to },
    })?;
    let after = session.view();
    let checkpoint = session.checkpoint(resources);
    let replay = HumanSession::restore_with(
        rules.clone(),
        resources,
        &checkpoint,
        Some((&files.bytes, events)),
    )?;
    let explored = after.observation.seen.iter().flatten().count();
    let moved = after
        .observation
        .units
        .iter()
        .find(|unit| unit.index == slot)
        .ok_or("saved movement unit missing after accepted step")?;
    if result.result != Some(OrderResult::Moved)
        || (moved.x, moved.y) != to
        || json!(explored) != native["explored_tiles"]
        || replay.view() != after
        || replay.checkpoint(resources) != checkpoint
    {
        return Err("saved movement differs from the native view or checkpoint replay".into());
    }
    let initializer = session
        .initializer()
        .ok_or("saved movement initializer missing")?;
    Ok(json!({
        "verified": true,
        "initializer_schema": initializer.schema(),
        "profile": initializer.profile(),
        "unit_slot": slot,
        "position": to,
        "moves_left": moved.moves_left,
        "explored_tiles": explored,
        "checkpoint_replays": true,
    }))
}

fn checkpoint_line<'a>(checkpoint: &'a str, prefix: &str) -> Result<&'a str, String> {
    checkpoint
        .lines()
        .find(|line| line.starts_with(prefix))
        .ok_or_else(|| format!("staged checkpoint is missing {}", prefix.trim()))
}
