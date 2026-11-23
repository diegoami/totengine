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

pub(super) fn check_saved_city_view(
    capture: &OriginalCapture,
    human: u8,
    trace: &Value,
) -> Result<Value, String> {
    let files = capture.files();
    let rules = &capture.resources().rules;
    let events = files.events.clone().map(EventEngine::new);
    let resources = capture.identity();
    let mut session = HumanSession::scenario_initialized_saved_city_view(
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
    issue_saved_handoff(&mut session, human, trace)?;
    let before = session.view();
    let before_pass = session.checkpoint(resources);
    let report = &trace["city_view"]["observed"];
    if session.advance(1) != 1 {
        return Err("saved city-view work item did not advance".into());
    }
    let after = session.view();
    let explored = after.observation.seen.iter().flatten().count();
    if after.observation.seen != before.observation.seen
        || after.observation.units != before.observation.units
        || json!(explored) != report["viewer_explored_tiles"]
    {
        return Err("city work did not preserve the measured view".into());
    }
    let checkpoint = session.checkpoint(resources);
    if before.observation.turn != after.observation.turn
        || checkpoint_line(&before_pass, "log ")? != checkpoint_line(&checkpoint, "log ")?
        || checkpoint_line(&before_pass, "world ")? == checkpoint_line(&checkpoint, "world ")?
    {
        return Err("saved city-view check did not reach same-turn city work before orders".into());
    }
    let restore = |text: &str| {
        HumanSession::restore_with(
            rules.clone(),
            resources,
            text,
            Some((&files.bytes, events.clone())),
        )
    };
    let mut replay = restore(&checkpoint)?;
    if replay.view() != after || replay.checkpoint(resources) != checkpoint {
        return Err("saved city work did not replay".into());
    }
    if session.advance(1) != 1
        || replay.advance(1) != 1
        || session.view().observation.seen != after.observation.seen
        || session.view().observation.units != after.observation.units
    {
        return Err("first opponent order did not preserve the saved view".into());
    }
    let next = session.checkpoint(resources);
    if next != replay.checkpoint(resources)
        || checkpoint_line(&checkpoint, "log ")? == checkpoint_line(&next, "log ")?
    {
        return Err("first opponent order did not extend the replayable order log".into());
    }
    let initializer = session
        .initializer()
        .ok_or("saved city-view initializer missing")?;
    Ok(json!({
        "verified": true, "initializer_schema": initializer.schema(),
        "profile": initializer.profile(), "explored_tiles": explored,
        "unit_knowledge_unchanged": true, "first_opponent_order_preserved_view": true,
        "checkpoint_replays": true,
    }))
}

fn issue_saved_handoff(session: &mut HumanSession, human: u8, trace: &Value) -> Result<(), String> {
    let skips = trace["round_handoff"]["spend_saved_movement"]["dispatched"]
        .as_array()
        .filter(|messages| messages.len() == 4)
        .ok_or("native city-view evidence is missing its skips")?;
    for input in skips {
        if input["character"] != 32 {
            return Err("native city-view evidence contains a different order".into());
        }
        let raw_slot = input["active_slot"]
            .as_u64()
            .ok_or("native Skip slot missing")?;
        let view = session.view();
        let unit = *view
            .units
            .iter()
            .find(|unit| unit.slot as u64 == raw_slot)
            .ok_or("saved city-view Skip unit missing")?;
        session.submit(Request {
            civ: human,
            id: view.revision + 1,
            revision: view.revision,
            command: Command::Skip { unit },
        })?;
    }
    let before_end = session.view();
    session.submit(Request {
        civ: human,
        id: before_end.revision + 1,
        revision: before_end.revision,
        command: Command::EndTurn,
    })?;
    Ok(())
}

/// Find the pinned moves in the canonical log identity without publishing
/// the opponent order log into the player's interface.
pub(super) fn check_saved_opponent_stack(
    capture: &OriginalCapture,
    human: u8,
    trace: &Value,
) -> Result<Value, String> {
    let files = capture.files();
    let rules = &capture.resources().rules;
    let events = files.events.clone().map(EventEngine::new);
    let resources = capture.identity();
    let mut session = HumanSession::scenario_initialized_saved_opponent_stack(
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
    issue_saved_handoff(&mut session, human, trace)?;
    let initial = session.view();
    let observed = &trace["city_view"]["observed"];
    let mut moves_checked = 0;
    for key in [
        "prepared_opponent_move",
        "prepared_opponent_empty_move",
        "prepared_opponent_departure",
    ] {
        let native = &observed[key];
        let mover = native["after"]
            .as_array()
            .and_then(|units| units.first())
            .ok_or("prepared opponent move evidence is missing the mover")?;
        let (x, y): (u16, u16) = serde_json::from_value(mover["position"].clone())
            .map_err(|error| format!("invalid prepared opponent position: {error}"))?;
        let slot = mover["slot"]
            .as_u64()
            .ok_or("prepared opponent slot missing")?;
        let civ = mover["owner"]
            .as_u64()
            .ok_or("prepared opponent owner missing")?;
        let order = format!(
            "T {} C {civ} move {slot} {x} {y} = moved\n",
            initial.observation.turn
        );
        let (prior, after) = reach_opponent_order(&mut session, resources, &order)?;
        let view = session.view();
        let explored = view.observation.seen.iter().flatten().count();
        if json!(explored) != native["explored_tiles"]
            || view.observation.seen != initial.observation.seen
            || view.observation.units != initial.observation.units
        {
            return Err("the opponent move did not preserve the measured player view".into());
        }
        let mut replay = HumanSession::restore_with(
            rules.clone(),
            resources,
            &prior,
            Some((&files.bytes, events.clone())),
        )?;
        if replay.advance(1) != 1 || replay.checkpoint(resources) != after {
            return Err("the checked opponent move did not replay".into());
        }
        moves_checked += 1;
    }
    Ok(json!({
        "verified": true, "initializer_schema": 6,
        "profile": engine::human::ScenarioInitializer::SAVED_OPPONENT_STACK_PROFILE,
        "explored_tiles": session.view().observation.seen.iter().flatten().count(),
        "moves_checked": moves_checked, "checkpoint_replays": true,
    }))
}

pub(super) fn check_saved_opponent_combat(
    capture: &OriginalCapture,
    human: u8,
    trace: &Value,
) -> Result<Value, String> {
    let files = capture.files();
    let rules = &capture.resources().rules;
    let events = files.events.clone().map(EventEngine::new);
    let resources = capture.identity();
    let native = &trace["opponent_combat"]["cases"]["7"];
    let seed = native["crt_before"]
        .as_u64()
        .ok_or("prepared combat seed missing")?;
    let mut session = HumanSession::scenario_initialized_saved_opponent_combat(
        ScenarioInput {
            path: &files.path,
            snapshot: &files.bytes,
            rules: rules.clone(),
            events: events.clone(),
        },
        human,
        AiPolicy::BuiltIn1,
        seed,
        resources,
    )?;
    issue_saved_handoff(&mut session, human, trace)?;
    let initial = session.view();
    let participants = native["before"]
        .as_array()
        .filter(|values| values.len() == 2)
        .ok_or("prepared combat participants missing")?;
    let (x, y): (u16, u16) = serde_json::from_value(participants[1]["position"].clone())
        .map_err(|error| format!("invalid prepared combat destination: {error}"))?;
    let attacker = participants[0]["slot"]
        .as_u64()
        .ok_or("prepared combat slot missing")?;
    let civ = participants[0]["owner"]
        .as_u64()
        .ok_or("prepared combat owner missing")?;
    let order = format!(
        "T {} C {civ} move {attacker} {x} {y} = attack won\n",
        initial.observation.turn
    );
    let (prior, after) = reach_opponent_order(&mut session, resources, &order)?;
    let actual = session.view();
    let explored = actual.observation.seen.iter().flatten().count();
    if actual.observation.seen != initial.observation.seen
        || actual.observation.units != initial.observation.units
        || json!(explored) != native["viewer_explored_tiles"]
    {
        return Err("the checked combat changed the saved player view".into());
    }
    let crt = native["crt_after"]
        .as_u64()
        .ok_or("prepared combat final CRT state missing")?;
    let recorded_crt = checkpoint_line(&after, "rng ")?.split_whitespace().nth(3);
    if recorded_crt != Some(format!("{crt:08x}").as_str()) {
        return Err("the checked combat diverged from the prepared CRT state".into());
    }
    let replay_events = events.clone();
    let mut replay = HumanSession::restore_with(
        rules.clone(),
        resources,
        &prior,
        Some((&files.bytes, events)),
    )?;
    if replay.advance(1) != 1 || replay.checkpoint(resources) != after {
        return Err("the checked combat did not replay".into());
    }
    for (label, key) in [
        ("field stack", "following_field_stack"),
        ("city arrival", "following_city_arrival"),
        ("city departure", "following_city_departure"),
        ("partial move", "following_partial_move"),
        ("singleton move", "following_singleton_move"),
        ("unconnected road", "following_unconnected_road"),
    ] {
        let step = &native[key];
        let mover = step["after"]
            .as_array()
            .and_then(|units| units.first())
            .ok_or_else(|| format!("prepared {label} mover missing"))?;
        let destination = step.get("destination").unwrap_or(&mover["position"]);
        let (x, y): (u16, u16) = serde_json::from_value(destination.clone())
            .map_err(|error| format!("invalid prepared {label} position: {error}"))?;
        let slot = mover["slot"]
            .as_u64()
            .ok_or_else(|| format!("prepared {label} slot missing"))?;
        let civ = mover["owner"]
            .as_u64()
            .ok_or_else(|| format!("prepared {label} owner missing"))?;
        let outcome = if step["moved"] == false {
            "movement exhausted"
        } else {
            "moved"
        };
        let order = format!(
            "T {} C {civ} move {slot} {x} {y} = {outcome}\n",
            initial.observation.turn
        );
        let (prior, after) = reach_opponent_order(&mut session, resources, &order)?;
        if let Some(crt) = step["crt_after"].as_u64() {
            let recorded = checkpoint_line(&after, "rng ")?.split_whitespace().nth(3);
            if recorded != Some(format!("{crt:08x}").as_str()) {
                return Err(format!(
                    "the checked {label} diverged from the prepared CRT state"
                ));
            }
        }
        let view = session.view();
        if view.observation.seen != initial.observation.seen
            || json!(view.observation.seen.iter().flatten().count()) != step["explored_tiles"]
        {
            return Err(format!("the checked {label} changed the saved player view"));
        }
        let mut replay = HumanSession::restore_with(
            rules.clone(),
            resources,
            &prior,
            Some((&files.bytes, replay_events.clone())),
        )?;
        if replay.advance(1) != 1 || replay.checkpoint(resources) != after {
            return Err(format!("the checked {label} did not replay"));
        }
    }
    Ok(json!({
        "verified": true, "initializer_schema": 7,
        "profile": engine::human::ScenarioInitializer::SAVED_OPPONENT_COMBAT_PROFILE,
        "explored_tiles": explored, "combat_rng_matches": true, "field_stack_replays": true,
        "city_arrival_replays": true, "city_departure_replays": true,
        "partial_movement_replays": true, "singleton_movement_replays": true,
        "unconnected_road_replays": true, "checkpoint_replays": true,
    }))
}

fn reach_opponent_order(
    session: &mut HumanSession,
    resources: &str,
    order: &str,
) -> Result<(String, String), String> {
    for _ in 0..64 {
        let prior = session.checkpoint(resources);
        let log = checkpoint_line(&prior, "log ")?;
        let (bytes, digest) = log[4..]
            .split_once(' ')
            .ok_or("invalid checkpoint log identity")?;
        let bytes: usize = bytes.parse().map_err(|_| "invalid checkpoint log length")?;
        let digest =
            u64::from_str_radix(digest, 16).map_err(|_| "invalid checkpoint log fingerprint")?;
        let extended = order.as_bytes().iter().fold(digest, |h, &byte| {
            (h ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3)
        });
        let expected_log = format!("log {} {extended:016x}", bytes + order.len());
        if session.advance(1) != 1 {
            break;
        }
        let after = session.checkpoint(resources);
        if checkpoint_line(&after, "log ")? == expected_log {
            return Ok((prior, after));
        }
    }
    Err("staged opponent did not reach the measured move".into())
}

fn checkpoint_line<'a>(checkpoint: &'a str, prefix: &str) -> Result<&'a str, String> {
    checkpoint
        .lines()
        .find(|line| line.starts_with(prefix))
        .ok_or_else(|| format!("staged checkpoint is missing {}", prefix.trim()))
}
