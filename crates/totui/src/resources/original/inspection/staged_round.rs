//! Verify a staged checkpoint at both sides of the pinned handoff.
use super::OriginalCapture;
use engine::{
    campaign::AiPolicy,
    events::EventEngine,
    human::{Command, HumanSession, HumanView, Phase, Request, ScenarioInput},
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
    let slots = if session
        .initializer()
        .is_some_and(|initializer| initializer.schema() == 7)
    {
        trace["saved_viewport"]["cases"]["saved_anchor"]["skip_order"]
            .as_array()
            .filter(|slots| slots.len() == 4)
            .ok_or("native saved-viewport evidence is missing its skips")?
            .iter()
            .map(|slot| slot.as_u64().ok_or("native Skip slot missing"))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        trace["round_handoff"]["spend_saved_movement"]["dispatched"]
            .as_array()
            .filter(|messages| messages.len() == 4)
            .ok_or("native city-view evidence is missing its skips")?
            .iter()
            .map(|input| {
                if input["character"] != 32 {
                    return Err("native city-view evidence contains a different order");
                }
                input["active_slot"]
                    .as_u64()
                    .ok_or("native Skip slot missing")
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    for raw_slot in slots {
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

pub(super) fn check_pending_choice(
    capture: &OriginalCapture,
    human: u8,
    trace: &Value,
) -> Result<Value, String> {
    let native = &trace["cases"]["pinned"];
    let poll = &native["input"];
    let guard = &native["round_start_controls"][0];
    if guard != &json!({"resume_civ": human, "restart": 0, "dispatches": false})
        || poll["stop_at_call"] != "00576ee4"
        || native["idle"]["next_general_poll"] != poll["stop_at_call"]
        || native["idle"]["civ_unit_city_records_unchanged"] != true
        || native["caller"]["human_advice_gate"] != "00473e6b"
    {
        return Err("prepared resume did not reach quiet human input".into());
    }
    let files = capture.files();
    let session = HumanSession::scenario_initialized_saved_opponent_combat(
        ScenarioInput {
            path: &files.path,
            snapshot: &files.bytes,
            rules: capture.resources().rules.clone(),
            events: files.events.clone().map(EventEngine::new),
        },
        human,
        AiPolicy::BuiltIn1,
        7,
        capture.identity(),
    )?;
    let view = session.view();
    if view.phase != Phase::Human
        || view.decision.is_some()
        || view.observation.research.target.is_some()
        || view.observation.research.free_advances != 0
        || poll["research_target"] != -1
        || json!(view.observation.research.progress) != poll["research_progress"]
        || json!(view.observation.government) != native["government_at_poll"]
    {
        return Err("engine entry decision differs from the prepared native input boundary".into());
    }
    Ok(json!({
        "verified": true,
        "boundary": "prepared-resume-input",
        "research_target": poll["research_target"],
        "research_progress": poll["research_progress"],
        "government": native["government_at_poll"],
        "pending_decision": false,
    }))
}

pub(super) fn check_saved_viewport(
    capture: &OriginalCapture,
    human: u8,
    viewport: (i16, i16),
    trace: &Value,
) -> Result<Value, String> {
    let anchor = (
        u16::try_from(viewport.0).map_err(|_| "saved viewport x is negative")?,
        u16::try_from(viewport.1).map_err(|_| "saved viewport y is negative")?,
    );
    if json!(anchor) != trace["cases"]["saved_anchor"]["anchor"] {
        return Err("the captured viewport differs from the native resume input".into());
    }
    let files = capture.files();
    let session = HumanSession::scenario_initialized_saved_opponent_combat(
        ScenarioInput {
            path: &files.path,
            snapshot: &files.bytes,
            rules: capture.resources().rules.clone(),
            events: files.events.clone().map(EventEngine::new),
        },
        human,
        AiPolicy::BuiltIn1,
        7,
        capture.identity(),
    )?;
    let view = session.view();
    for label in ["saved_anchor", "zero_anchor"] {
        let native = &trace["cases"][label];
        let anchor: (u16, u16) = serde_json::from_value(native["anchor"].clone())
            .map_err(|error| format!("invalid native viewport anchor: {error}"))?;
        let selected = session
            .saved_initial_focus(anchor)
            .ok_or("the saved viewport has no checked ready unit")?;
        if json!(selected.slot) != native["handoff"]["active_slot"]
            || json!(view.observation.seen.iter().flatten().count())
                != native["handoff"]["explored_tiles_after"]
            || view.decision.is_some()
        {
            return Err("initial unit focus differs from the prepared native resume".into());
        }
    }
    Ok(json!({
        "verified": true,
        "anchor": anchor,
        "selected_slot": trace["cases"]["saved_anchor"]["handoff"]["active_slot"],
        "zero_anchor_slot": trace["cases"]["zero_anchor"]["handoff"]["active_slot"],
    }))
}

pub(super) fn check_saved_rates(
    capture: &OriginalCapture,
    human: u8,
    trace: &Value,
) -> Result<Value, String> {
    let files = capture.files();
    let rules = &capture.resources().rules;
    let events = files.events.clone().map(EventEngine::new);
    let identity = capture.identity();
    let initial = HumanSession::scenario_initialized_saved_opponent_combat(
        ScenarioInput {
            path: &files.path,
            snapshot: &files.bytes,
            rules: rules.clone(),
            events: events.clone(),
        },
        human,
        AiPolicy::BuiltIn1,
        7,
        identity,
    )?;
    let view = initial.view();
    let first = initial.checkpoint(identity);
    let restore = |text: &str| {
        HumanSession::restore_with(
            rules.clone(),
            identity,
            text,
            Some((&files.bytes, events.clone())),
        )
    };
    let cases = trace["cases"]
        .as_array()
        .filter(|cases| !cases.is_empty())
        .ok_or("prepared rates evidence has no cases")?;
    for native in cases {
        let science: u8 = serde_json::from_value(native["science"].clone())
            .map_err(|error| format!("invalid native science rate: {error}"))?;
        let tax: u8 = serde_json::from_value(native["tax"].clone())
            .map_err(|error| format!("invalid native tax rate: {error}"))?;
        let mut session = restore(&first)?;
        let response = session.submit(Request {
            civ: human,
            revision: view.revision,
            id: view.revision + 1,
            command: Command::Rates { science, tax },
        })?;
        let after = session.view();
        if response.result != Some(OrderResult::RatesSet)
            || after.observation.seen != view.observation.seen
            || after.observation.units != view.observation.units
            || json!(after.observation.seen.iter().flatten().count()) != native["explored_tiles"]
            || !after
                .rates
                .as_ref()
                .is_some_and(|rates| rates.science == science && rates.tax == tax)
        {
            return Err("the checked rate change differs from the prepared native view".into());
        }
        let checkpoint = session.checkpoint(identity);
        let replay = restore(&checkpoint)?;
        if replay.view() != after || replay.checkpoint(identity) != checkpoint {
            return Err("the checked rate change did not replay".into());
        }
    }
    Ok(json!({"verified": true, "cases": cases.len(), "checkpoint_replays": true}))
}

pub(super) fn check_saved_citizens(
    capture: &OriginalCapture,
    human: u8,
    trace: &Value,
) -> Result<Value, String> {
    use engine::human::HumanView;

    let files = capture.files();
    let rules = &capture.resources().rules;
    let events = files.events.clone().map(EventEngine::new);
    let identity = capture.identity();
    let mut session = HumanSession::scenario_initialized_saved_opponent_combat(
        ScenarioInput {
            path: &files.path,
            snapshot: &files.bytes,
            rules: rules.clone(),
            events: events.clone(),
        },
        human,
        AiPolicy::BuiltIn1,
        7,
        identity,
    )?;
    let initial = session.view();
    let cases = trace["cases"]
        .as_array()
        .filter(|cases| !cases.is_empty())
        .ok_or("prepared worker evidence has no cases")?;
    let packed_city = |view: &HumanView, index| {
        let city = view
            .city_citizens
            .iter()
            .find(|city| city.index == index)
            .ok_or("prepared worker city has no filtered citizen view")?;
        let worked = city.radius.iter().enumerate().fold(
            (city.specialists.len() as u32) << 26,
            |bits, (slot, tile)| bits | (u32::from(tile.worked) << slot),
        );
        let specialists = city
            .specialists
            .iter()
            .take(16)
            .enumerate()
            .fold(0u32, |bits, (slot, role)| {
                bits | (u32::from(*role) << (2 * slot))
            });
        Ok::<_, String>(json!({"worked": worked, "specialists": specialists}))
    };
    for native in cases {
        let slot = native["city"]
            .as_u64()
            .ok_or("prepared worker city missing")?;
        let city = *initial
            .cities
            .iter()
            .find(|city| city.slot as u64 == slot)
            .ok_or("prepared worker city is absent")?;
        let tile: u8 = serde_json::from_value(native["tile"].clone())
            .map_err(|error| format!("invalid prepared worker radius: {error}"))?;
        let before = session.view();
        if packed_city(&before, city.slot)? != native["before"] {
            return Err("the worker edit starts from different citizen records".into());
        }
        let response = session.submit(Request {
            civ: human,
            revision: before.revision,
            id: before.revision + 1,
            command: Command::Worker { city, tile },
        })?;
        let after = session.view();
        if response.result != Some(OrderResult::CitizensSet)
            || after.observation.seen != initial.observation.seen
            || after.observation.units != initial.observation.units
            || json!(after.observation.seen.iter().flatten().count()) != native["explored_tiles"]
            || packed_city(&after, city.slot)? != native["after"]
        {
            return Err("the checked worker edit differs from the prepared native view".into());
        }
        let checkpoint = session.checkpoint(identity);
        let replay = HumanSession::restore_with(
            rules.clone(),
            identity,
            &checkpoint,
            Some((&files.bytes, events.clone())),
        )?;
        if replay.view() != after || replay.checkpoint(identity) != checkpoint {
            return Err("the checked worker edit did not replay".into());
        }
    }
    Ok(json!({"verified": true, "cases": cases.len(), "checkpoint_replays": true}))
}

pub(super) fn check_saved_human_production(
    capture: &OriginalCapture,
    human: u8,
    trace: &Value,
    orders: &Value,
) -> Result<Value, String> {
    let native = &trace["human_city_birth"];
    let files = capture.files();
    let rules = &capture.resources().rules;
    let events = files.events.clone().map(EventEngine::new);
    let identity = capture.identity();
    let mut session = HumanSession::scenario_initialized_saved_naval_arrival(
        ScenarioInput {
            path: &files.path,
            snapshot: &files.bytes,
            rules: rules.clone(),
            events: events.clone(),
        },
        human,
        AiPolicy::BuiltIn1,
        7,
        identity,
    )?;
    let initial = session.view();
    let city = *initial
        .cities
        .iter()
        .find(|city| json!(city.slot) == native["city"])
        .ok_or("the prepared production city is absent")?;
    let submit = |session: &mut HumanSession, command| {
        let revision = session.view().revision;
        session.submit(Request {
            civ: human,
            revision,
            id: revision + 1,
            command,
        })
    };
    submit(&mut session, Command::Worker { city, tile: 4 })?;
    let quote = session
        .view()
        .city_production
        .iter()
        .find(|p| p.index == city.slot)
        .ok_or("the prepared city has no purchase view")?
        .hurry?;
    submit(&mut session, Command::Buy { city, quote })?;
    submit(&mut session, Command::EndTurn)?;
    session.advance(128);
    let after = session.view();
    let mut new_units = after.observation.units.iter().filter(|u| {
        !initial
            .observation
            .units
            .iter()
            .any(|old| old.index == u.index)
    });
    let unit = new_units
        .next()
        .ok_or("the prepared city produced no visible unit")?;
    if after.phase != Phase::Human
        || after.decision.is_some()
        || json!(after.observation.turn) != native["turn"]
        || after.observation.seen != initial.observation.seen
        || json!(after.observation.seen.iter().flatten().count()) != native["explored_at_poll"]
        || json!([unit.x, unit.y]) != native["unit"]["position"]
        || json!(unit.owner) != native["unit"]["owner"]
        || json!(unit.type_id) != native["unit"]["type"]
        || new_units.next().is_some()
    {
        return Err("the checked city production differs from the measured saved view".into());
    }
    let focus = &native["focus"];
    let anchors: Vec<(u16, u16)> = serde_json::from_value(focus["anchors"].clone())
        .map_err(|error| format!("invalid prepared focus anchors: {error}"))?;
    let choices: Vec<usize> = serde_json::from_value(focus["choices"].clone())
        .map_err(|error| format!("invalid prepared focus choices: {error}"))?;
    let stack: [usize; 2] = serde_json::from_value(focus["original_stack"].clone())
        .map_err(|error| format!("invalid prepared production stack: {error}"))?;
    if anchors.len() != choices.len()
        || anchors.is_empty()
        || focus["choices"] != focus["reversed_head_choices"]
        || json!(stack[0]) != native["unit"]["slot"]
    {
        return Err("the prepared city focus evidence is inconsistent".into());
    }
    let expected: Vec<_> = choices
        .into_iter()
        .map(|slot| {
            if slot == stack[0] {
                unit.index.min(stack[1])
            } else {
                slot
            }
        })
        .collect();
    let verify_focus = |session: &HumanSession| {
        anchors.iter().zip(&expected).all(|(&anchor, &slot)| {
            session
                .saved_initial_focus(anchor)
                .is_some_and(|unit| unit.slot == slot)
        })
    };
    if !verify_focus(&session) {
        return Err("the checked city stack focus differs from the native selection rule".into());
    }
    let checkpoint = session.checkpoint(identity);
    let restore = |text: &str| {
        HumanSession::restore_with(
            rules.clone(),
            identity,
            text,
            Some((&files.bytes, events.clone())),
        )
    };
    let replay = restore(&checkpoint)?;
    if replay.view() != after || replay.checkpoint(identity) != checkpoint || !verify_focus(&replay)
    {
        return Err("the checked human production did not replay".into());
    }
    let shields = after
        .observation
        .cities
        .iter()
        .find(|c| c.index == city.slot)
        .and_then(|c| c.shield_box)
        .ok_or("the production city shield box is absent")?;
    if json!(shields) != native["shields_after"] {
        return Err("the human production shield box differs from the native completion".into());
    }
    let order_cases = check_human_stack_orders(
        &after,
        &checkpoint,
        &restore,
        [(stack[0], unit.index), (stack[1], stack[1])],
        &orders["cases"],
        human,
        identity,
    )?;
    let mut contact = restore(&checkpoint)?;
    let entity = |slot| {
        after
            .units
            .iter()
            .find(|u| u.slot == slot)
            .copied()
            .ok_or("prepared contact actor is absent")
    };
    submit(
        &mut contact,
        Command::Wait {
            unit: entity(stack[1])?,
        },
    )?;
    submit(
        &mut contact,
        Command::Move {
            unit: entity(unit.index)?,
            to: (52, 8),
        },
    )?;
    submit(&mut contact, Command::EndTurn)?;
    if contact.advance(8) != 8 {
        return Err("saved contact approach did not reach the prepared boundary".into());
    }
    let before = contact.view();
    if contact.advance(1) != 1 || before.observation.seen != contact.view().observation.seen {
        return Err("saved opponent contact changed the human view".into());
    }
    let contact_checkpoint = contact.checkpoint(identity);
    let reloaded = restore(&contact_checkpoint)?;
    if reloaded.view() != contact.view() || reloaded.checkpoint(identity) != contact_checkpoint {
        return Err("saved opponent contact did not replay".into());
    }
    if contact.advance(1) != 1 || before.observation.seen != contact.view().observation.seen {
        return Err("saved occupied-city attack changed the human view".into());
    }
    let attack_checkpoint = contact.checkpoint(identity);
    let reloaded = restore(&attack_checkpoint)?;
    if reloaded.view() != contact.view() || reloaded.checkpoint(identity) != attack_checkpoint {
        return Err("saved occupied-city attack did not replay".into());
    }
    if contact.advance(8) != 8 {
        return Err("saved hut approach did not reach the prepared boundary".into());
    }
    let before = contact.view();
    if contact.advance(1) != 1 || before.observation.seen != contact.view().observation.seen {
        return Err("saved opponent hut entry changed the human view".into());
    }
    let hut_checkpoint = contact.checkpoint(identity);
    let reloaded = restore(&hut_checkpoint)?;
    if reloaded.view() != contact.view() || reloaded.checkpoint(identity) != hut_checkpoint {
        return Err("saved opponent hut entry did not replay".into());
    }
    if contact.advance(4) != 4 {
        return Err("armed stack approach did not reach the prepared boundary".into());
    }
    let before = contact.view();
    if contact.advance(1) != 1 || before.observation.seen != contact.view().observation.seen {
        return Err("saved armed stack arrival changed the human view".into());
    }
    contact.advance(128);
    let fresh = contact.view();
    if fresh.phase != Phase::Human
        || fresh.observation.turn != 45
        || fresh.observation.seen != initial.observation.seen
    {
        return Err("checked continuation did not retain the saved view into turn 45".into());
    }
    let actor = entity(unit.index)?;
    let returned = submit(
        &mut contact,
        Command::Move {
            unit: actor,
            to: (53, 9),
        },
    )?;
    let final_view = contact.view();
    if returned.result != Some(OrderResult::Moved)
        || final_view.observation.seen != fresh.observation.seen
        || !final_view
            .observation
            .units
            .iter()
            .any(|u| u.index == actor.slot && (u.x, u.y, u.moves_left) == (53, 9, 0))
    {
        return Err("checked turn-45 city return differs from the witnessed movement".into());
    }
    let arrival_checkpoint = contact.checkpoint(identity);
    let replay = restore(&arrival_checkpoint)?;
    if replay.view() != final_view || replay.checkpoint(identity) != arrival_checkpoint {
        return Err("checked turn-45 city return did not replay".into());
    }
    submit(&mut contact, Command::EndTurn)?;
    if contact.advance(23) != 23 || final_view.observation.seen != contact.view().observation.seen {
        return Err("saved second armed arrival changed the human view".into());
    }
    contact.advance(128);
    let fresh = contact.view();
    if fresh.phase != Phase::Human
        || fresh.observation.turn != 46
        || fresh.observation.seen != final_view.observation.seen
    {
        return Err("checked continuation did not retain saved observation into turn 46".into());
    }
    let departed = submit(
        &mut contact,
        Command::Move {
            unit: actor,
            to: (52, 8),
        },
    )?;
    let post_departure = contact.view();
    if departed.result != Some(OrderResult::Moved)
        || post_departure.observation.seen != fresh.observation.seen
        || !post_departure
            .observation
            .units
            .iter()
            .any(|u| u.index == actor.slot && (u.x, u.y, u.moves_left) == (52, 8, 0))
    {
        return Err("checked turn-46 city departure differs from the saved movement".into());
    }
    let departure_checkpoint = contact.checkpoint(identity);
    let replay = restore(&departure_checkpoint)?;
    if replay.view() != post_departure || replay.checkpoint(identity) != departure_checkpoint {
        return Err("checked turn-46 city departure did not replay".into());
    }
    submit(&mut contact, Command::EndTurn)?;
    contact.advance(256);
    let fresh = contact.view();
    if fresh.phase != Phase::Human
        || fresh.observation.turn != 47
        || fresh.observation.seen != post_departure.observation.seen
    {
        return Err("checked continuation did not retain saved observation into turn 47".into());
    }
    let returned = submit(
        &mut contact,
        Command::Move {
            unit: actor,
            to: (53, 9),
        },
    )?;
    let post_return = contact.view();
    if returned.result != Some(OrderResult::Moved)
        || post_return.observation.seen != fresh.observation.seen
        || !post_return
            .observation
            .units
            .iter()
            .any(|u| u.index == actor.slot && (u.x, u.y, u.moves_left) == (53, 9, 0))
    {
        return Err("checked turn-47 city return differs from the saved movement".into());
    }
    let return_checkpoint = contact.checkpoint(identity);
    let restored = restore(&return_checkpoint)?;
    if restored.view() != post_return || restored.checkpoint(identity) != return_checkpoint {
        return Err("checked turn-47 city return did not replay".into());
    }
    submit(&mut contact, Command::EndTurn)?;
    if contact.advance(2) != 2 {
        return Err("checked turn-47 opponent founding did not advance".into());
    }
    for steps in [0, 16] {
        if steps != 0 && contact.advance(steps) != steps {
            return Err("the second checked turn-47 founding did not advance".into());
        }
        let founded = contact.view();
        let founding_checkpoint = contact.checkpoint(identity);
        let replay = restore(&founding_checkpoint)?;
        if founded.observation.seen != post_return.observation.seen
            || founded.observation.cities != post_return.observation.cities
            || replay.view() != founded
            || replay.checkpoint(identity) != founding_checkpoint
        {
            return Err(
                "checked turn-47 opponent founding did not retain the human view on replay".into(),
            );
        }
    }
    if contact.advance(6) != 6 {
        return Err("checked turn-47 westward arrival did not advance".into());
    }
    let stacked = contact.view();
    let arrival_checkpoint = contact.checkpoint(identity);
    let replay = restore(&arrival_checkpoint)?;
    if stacked.observation.seen != post_return.observation.seen
        || stacked.observation.cities != post_return.observation.cities
        || replay.view() != stacked
        || replay.checkpoint(identity) != arrival_checkpoint
    {
        return Err(
            "checked turn-47 westward arrival did not retain the human view on replay".into(),
        );
    }
    if contact.advance(3) != 3 {
        return Err("the checked turn-48 naval arrival did not advance".into());
    }
    let arrived = contact.view();
    let arrival_checkpoint = contact.checkpoint(identity);
    let replay = restore(&arrival_checkpoint)?;
    if arrived.observation.turn != 48
        || arrived.observation.seen != post_return.observation.seen
        || arrived.observation.cities != post_return.observation.cities
        || replay.view() != arrived
        || replay.checkpoint(identity) != arrival_checkpoint
    {
        return Err("the checked naval arrival did not retain the human view on replay".into());
    }
    Ok(json!({
        "verified": true,
        "explored_tiles": native["explored_at_poll"],
        "engine_unit_slot": unit.index,
        "focus_cases": anchors.len(),
        "stack_order_cases": order_cases,
        "contact_checkpoint_replays": true,
        "city_attack_checkpoint_replays": true,
        "hut_checkpoint_replays": true,
        "turn_45_city_return_replays": true,
        "turn_46_city_departure_replays": true,
        "turn_47_city_return_replays": true,
        "turn_47_hidden_founding_replays": 2,
        "turn_47_opponent_stack_replays": true,
        "turn_48_naval_arrival_replays": true,
        "native_unit_slot": native["unit"]["slot"],
        "engine_shields": shields,
        "native_shields": native["shields_after"],
        "checkpoint_replays": true,
    }))
}

fn check_human_stack_orders(
    before: &HumanView,
    checkpoint: &str,
    restore: &impl Fn(&str) -> Result<HumanSession, String>,
    slots: [(usize, usize); 2],
    cases: &Value,
    human: u8,
    identity: &str,
) -> Result<usize, String> {
    let mut checked = 0;
    for [(native, slot), (native_mate, mate)] in [slots, [slots[1], slots[0]]] {
        let actor = *before
            .units
            .iter()
            .find(|u| u.slot == slot)
            .ok_or("the checked city-stack actor is absent")?;
        let case = &cases[native.to_string()];
        let wait = &case["wait"];
        let movement = &case["move_northwest"];
        let skip = &case["skip"];
        let to: (u16, u16) = serde_json::from_value(movement["unit_position_after"].clone())
            .map_err(|error| format!("invalid prepared stack move destination: {error}"))?;
        if json!(native_mate) != wait["after"]["active_slot"] {
            return Err("the native stack Wait witness specifies a different partner".into());
        }
        for (command, result, trace) in [
            (Command::Wait { unit: actor }, OrderResult::Waited, wait),
            (Command::Skip { unit: actor }, OrderResult::Skipped, skip),
            (
                Command::Move { unit: actor, to },
                OrderResult::Moved,
                movement,
            ),
        ] {
            let mut session = restore(checkpoint)?;
            let response = session.submit(Request {
                civ: human,
                revision: before.revision,
                id: before.revision + 1,
                command,
            })?;
            let after = session.view();
            let actor_view = after
                .observation
                .units
                .iter()
                .find(|u| u.index == slot)
                .ok_or("the ordered city-stack unit is absent")?;
            let observations_match = match command {
                Command::Wait { .. } => {
                    session.saved_order_focus().is_some_and(|e| e.slot == mate)
                        && after.observation.seen == before.observation.seen
                        && after.observation.units == before.observation.units
                }
                Command::Skip { .. } => {
                    let mut units = before.observation.units.clone();
                    units
                        .iter_mut()
                        .find(|u| u.index == slot)
                        .ok_or("the ready city-stack unit is absent")?
                        .moves_left = 0;
                    session.saved_order_focus().is_some_and(|e| e.slot == mate)
                        && after.observation.seen == before.observation.seen
                        && after.observation.units == units
                }
                _ => {
                    (actor_view.x, actor_view.y, actor_view.moves_left) == (to.0, to.1, 0)
                        && after.observation.units.iter().any(|u| {
                            u.index == mate && json!([u.x, u.y]) == movement["unit_position_before"]
                        })
                }
            };
            if response.result != Some(result)
                || !observations_match
                || json!(after.observation.seen.iter().flatten().count()) != trace["explored_tiles"]
            {
                return Err("the checked city-stack order differs from the native witness".into());
            }
            let text = session.checkpoint(identity);
            let replay = restore(&text)?;
            if replay.view() != after
                || replay.checkpoint(identity) != text
                || replay.saved_order_focus() != session.saved_order_focus()
            {
                return Err("the checked city-stack order did not replay".into());
            }
            checked += 1;
        }
    }
    Ok(checked)
}

pub(super) fn check_saved_purchase(
    capture: &OriginalCapture,
    human: u8,
    trace: &Value,
) -> Result<Value, String> {
    let files = capture.files();
    let rules = &capture.resources().rules;
    let events = files.events.clone().map(EventEngine::new);
    let identity = capture.identity();
    let mut session = HumanSession::scenario_initialized_saved_opponent_combat(
        ScenarioInput {
            path: &files.path,
            snapshot: &files.bytes,
            rules: rules.clone(),
            events: events.clone(),
        },
        human,
        AiPolicy::BuiltIn1,
        7,
        identity,
    )?;
    let initial = session.view();
    let native = &trace["pinned"];
    let slot = native["city"]
        .as_u64()
        .ok_or("prepared purchase city missing")?;
    let city = *initial
        .cities
        .iter()
        .find(|city| city.slot as u64 == slot)
        .ok_or("prepared purchase city is absent")?;
    let quote = initial
        .city_production
        .iter()
        .find(|p| p.index == city.slot)
        .ok_or("prepared purchase city has no production view")?
        .hurry?;
    if native["admitted"] != true
        || json!(quote.item) != native["item"]
        || json!(quote.price) != native["price"]
        || json!(quote.stored) != native["shields_before"]
        || json!(quote.treasury) != native["treasury_before"]
        || json!(quote.required) != native["shields_after"]
    {
        return Err("the checked purchase quote differs from the native price".into());
    }
    let response = session.submit(Request {
        civ: human,
        revision: initial.revision,
        id: initial.revision + 1,
        command: Command::Buy { city, quote },
    })?;
    let after = session.view();
    let changed = after
        .observation
        .cities
        .iter()
        .find(|c| c.index == city.slot)
        .ok_or("the funded city is absent from the view")?;
    let rebased = session.preview_production(city, quote.item)?;
    if response.result != Some(OrderResult::ProductionBought)
        || after.observation.seen != initial.observation.seen
        || after.observation.units != initial.observation.units
        || json!(after.observation.seen.iter().flatten().count()) != native["explored_tiles"]
        || json!(after.observation.treasury) != native["treasury_after"]
        || json!(changed.shield_box) != native["shields_after"]
        || json!(rebased.initial_shields) != native["rebased_shields"]
    {
        return Err("the checked purchase differs from the prepared native commit".into());
    }
    let checkpoint = session.checkpoint(identity);
    let replay = HumanSession::restore_with(
        rules.clone(),
        identity,
        &checkpoint,
        Some((&files.bytes, events)),
    )?;
    if replay.view() != after || replay.checkpoint(identity) != checkpoint {
        return Err("the checked purchase did not replay".into());
    }
    Ok(json!({"verified": true, "price": quote.price, "checkpoint_replays": true}))
}

pub(super) fn check_saved_production(
    capture: &OriginalCapture,
    human: u8,
    trace: &Value,
) -> Result<Value, String> {
    let files = capture.files();
    let rules = &capture.resources().rules;
    let events = files.events.clone().map(EventEngine::new);
    let identity = capture.identity();
    let initial = HumanSession::scenario_initialized_saved_opponent_combat(
        ScenarioInput {
            path: &files.path,
            snapshot: &files.bytes,
            rules: rules.clone(),
            events: events.clone(),
        },
        human,
        AiPolicy::BuiltIn1,
        7,
        identity,
    )?;
    let view = initial.view();
    let first = initial.checkpoint(identity);
    let restore = |text: &str| {
        HumanSession::restore_with(
            rules.clone(),
            identity,
            text,
            Some((&files.bytes, events.clone())),
        )
    };
    let cases = trace["cases"]
        .as_array()
        .filter(|cases| !cases.is_empty())
        .ok_or("prepared production evidence has no cases")?;
    for native in cases {
        let slot = native["city"]
            .as_u64()
            .ok_or("prepared production city missing")?;
        let city = *view
            .cities
            .iter()
            .find(|city| city.slot as u64 == slot)
            .ok_or("prepared production city is absent from the saved view")?;
        let item: i8 = serde_json::from_value(native["item"].clone())
            .map_err(|error| format!("invalid prepared production item: {error}"))?;
        let mut session = restore(&first)?;
        let quote = session.preview_production(city, item)?;
        if json!(quote.current_item) != native["before_item"]
            || json!(quote.current_shields) != native["before_shields"]
            || json!(quote.retained) != native["shields"]
        {
            return Err("the checked production budget differs from the native commit".into());
        }
        let response = session.submit(Request {
            civ: human,
            revision: view.revision,
            id: view.revision + 1,
            command: Command::QuotedProduction { city, quote },
        })?;
        let after = session.view();
        let changed = after
            .observation
            .cities
            .iter()
            .find(|c| c.index == city.slot)
            .ok_or("the changed production city is absent from the view")?;
        if response.result != Some(OrderResult::ProductionSet)
            || after.observation.seen != view.observation.seen
            || after.observation.units != view.observation.units
            || json!(after.observation.seen.iter().flatten().count()) != native["explored_tiles"]
            || changed.building != Some(item)
            || json!(changed.shield_box) != native["shields"]
        {
            return Err(
                "the checked production commit differs from the prepared native view".into(),
            );
        }
        let checkpoint = session.checkpoint(identity);
        let replay = restore(&checkpoint)?;
        if replay.view() != after || replay.checkpoint(identity) != checkpoint {
            return Err("the checked production change did not replay".into());
        }
    }
    Ok(json!({"verified": true, "cases": cases.len(), "checkpoint_replays": true}))
}

pub(super) fn check_saved_wait(
    capture: &OriginalCapture,
    human: u8,
    trace: &Value,
) -> Result<Value, String> {
    let files = capture.files();
    let rules = &capture.resources().rules;
    let events = files.events.clone().map(EventEngine::new);
    let identity = capture.identity();
    let mut session = HumanSession::scenario_initialized_saved_opponent_combat(
        ScenarioInput {
            path: &files.path,
            snapshot: &files.bytes,
            rules: rules.clone(),
            events: events.clone(),
        },
        human,
        AiPolicy::BuiltIn1,
        7,
        identity,
    )?;
    let before = session.view();
    let anchor: (u16, u16) = serde_json::from_value(trace["anchor"].clone())
        .map_err(|error| format!("invalid native Wait viewport: {error}"))?;
    let mut actor = session
        .saved_initial_focus(anchor)
        .ok_or("saved Wait has no initial unit")?;
    let steps = trace["waits"]
        .as_array()
        .filter(|steps| !steps.is_empty())
        .ok_or("native Wait evidence has no orders")?;
    let mut first_wait = None;
    for step in steps {
        if json!(actor.slot) != step["actor"] {
            return Err("saved Wait actor differs from native focus".into());
        }
        let revision = session.view().revision;
        let response = session.submit(Request {
            civ: human,
            id: revision + 1,
            revision,
            command: Command::Wait { unit: actor },
        })?;
        let view = session.view();
        actor = session
            .saved_order_focus()
            .ok_or("saved Wait did not select a ready unit")?;
        if json!(actor.slot) != step["selected"]
            || response.result != Some(OrderResult::Waited)
            || view.observation.units != before.observation.units
            || view.observation.seen != before.observation.seen
            || view.decision.is_some()
        {
            return Err("saved Wait differs from the prepared native transition".into());
        }
        if first_wait.is_none() {
            first_wait = Some(session.checkpoint(identity));
        }
    }
    let checkpoint = session.checkpoint(identity);
    let restore = |checkpoint: &str| {
        HumanSession::restore_with(
            rules.clone(),
            identity,
            checkpoint,
            Some((&files.bytes, events.clone())),
        )
    };
    let replay = restore(&checkpoint)?;
    if replay.checkpoint(identity) != checkpoint || replay.saved_order_focus() != Some(actor) {
        return Err("saved Wait did not restore its focus".into());
    }
    let mut moving = restore(&first_wait.ok_or("saved Wait has no first checkpoint")?)?;
    let mover = moving
        .saved_order_focus()
        .ok_or("the checked move has no ready actor")?;
    let native = &trace["move_after_wait"];
    let to: (u16, u16) = serde_json::from_value(native["position"].clone())
        .map_err(|error| format!("invalid saved Wait move destination: {error}"))?;
    let revision = moving.view().revision;
    let result = moving.submit(Request {
        civ: human,
        revision,
        id: revision + 1,
        command: Command::Move { unit: mover, to },
    })?;
    let view = moving.view();
    let focus = moving
        .saved_order_focus()
        .ok_or("exhausted movement did not select another unit")?;
    if json!(mover.slot) != native["actor"]
        || json!(focus.slot) != native["selected"]
        || result.result != Some(OrderResult::Moved)
        || json!(view.observation.seen.iter().flatten().count()) != native["explored_tiles"]
        || !view
            .observation
            .units
            .iter()
            .any(|unit| unit.index == mover.slot && unit.moves_left == 0 && (unit.x, unit.y) == to)
    {
        return Err(
            "the checked move after Wait differs from the prepared native transition".into(),
        );
    }
    let moved = moving.checkpoint(identity);
    let replay = restore(&moved)?;
    if replay.checkpoint(identity) != moved || replay.saved_order_focus() != Some(focus) {
        return Err("the checked move after Wait did not restore its focus".into());
    }
    Ok(
        json!({"verified": true, "waits": steps.len(), "selected_slot": actor.slot,
             "move_after_wait_slot": focus.slot, "checkpoint_replays": true}),
    )
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
    let round = &trace["round_close"];
    for (label, step) in [
        ("field stack", &native["following_field_stack"]),
        ("city arrival", &native["following_city_arrival"]),
        ("city departure", &native["following_city_departure"]),
        ("partial move", &native["following_partial_move"]),
        ("singleton move", &native["following_singleton_move"]),
        ("unconnected road", &native["following_unconnected_road"]),
        ("last round move", &round["last_move"]),
    ] {
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
    let civ = round["last_move"]["after"][0]["owner"]
        .as_u64()
        .ok_or("prepared round close owner missing")?;
    let order = format!("T {} C {civ} end = ended\n", initial.observation.turn);
    let (_, before_close) = reach_opponent_order(&mut session, resources, &order)?;
    let native_close = &round["annual_update"];
    if json!(session.view().observation.turn) != native_close["before"]["turn"]
        || session.advance(1) != 1
        || json!(session.view().observation.turn) != native_close["after"]["turn"]
        || session.view().observation.seen != initial.observation.seen
        || json!(session.view().observation.seen.iter().flatten().count())
            != native_close["after"]["explored"]
    {
        return Err("the staged round boundary changed the saved player view".into());
    }
    let after_close = session.checkpoint(resources);
    for (checkpoint, phase) in [(&before_close, "before"), (&after_close, "after")] {
        let crt = native_close[phase]["crt"]
            .as_u64()
            .ok_or("prepared round CRT state missing")?;
        if checkpoint_line(checkpoint, "rng ")?
            .split_whitespace()
            .nth(3)
            != Some(format!("{crt:08x}").as_str())
        {
            return Err(format!(
                "the staged round boundary diverged from the prepared CRT state {phase} close"
            ));
        }
    }
    let mut replay = HumanSession::restore_with(
        rules.clone(),
        resources,
        &before_close,
        Some((&files.bytes, replay_events.clone())),
    )?;
    if replay.advance(1) != 1 || replay.checkpoint(resources) != after_close {
        return Err("the checked round close did not replay".into());
    }
    for _ in 0..64 {
        if session.view().phase != Phase::Opponents {
            break;
        }
        if session.advance(1) != 1
            || replay.advance(1) != 1
            || replay.checkpoint(resources) != session.checkpoint(resources)
        {
            return Err("the checked next human turn did not replay".into());
        }
    }
    let view = session.view();
    let native_handoff = &round["fresh_human_input"]["handoff"];
    let active = native_handoff["active_slot"]
        .as_u64()
        .ok_or("the native fresh handoff has no selected unit")?;
    let selected_ready = view
        .observation
        .units
        .iter()
        .any(|unit| unit.index as u64 == active && unit.owner == human && unit.moves_left > 0);
    if view.phase != Phase::Human
        || view.observation.turn != initial.observation.turn + 1
        || view.observation.seen != initial.observation.seen
        || json!(view.observation.seen.iter().flatten().count())
            != native_handoff["explored_tiles_after"]
        || view.decision.is_some()
        || view.observation.research.target.is_some()
        || json!(view.observation.research.progress) != native_handoff["research_progress"]
        || !selected_ready
    {
        return Err(
            "the staged next human turn differs from the prepared view or research state".into(),
        );
    }
    let focus = &round["fresh_human_input"]["focus"];
    let anchor: (u16, u16) = serde_json::from_value(focus["anchor"].clone())
        .map_err(|error| format!("invalid prepared unit focus anchor: {error}"))?;
    if session
        .saved_initial_focus(anchor)
        .map(|unit| unit.slot as u64)
        != Some(active)
        || focus["selected"] != native_handoff["active_slot"]
    {
        return Err("the staged initial focus differs from the native selection".into());
    }
    let width: u16 = serde_json::from_value(focus["width"].clone())
        .map_err(|error| format!("invalid prepared focus width: {error}"))?;
    let height: u16 = serde_json::from_value(focus["height"].clone())
        .map_err(|error| format!("invalid prepared focus height: {error}"))?;
    let mut raster = 0xcbf2_9ce4_8422_2325u64;
    for y in 0..height {
        for x in (y % 2..width).step_by(2) {
            let selected = session
                .saved_initial_focus((x, y))
                .ok_or("a prepared focus anchor has no staged unit")?;
            raster = (raster ^ selected.slot as u64).wrapping_mul(0x100_0000_01b3);
        }
    }
    if focus["raster_fnv64"] != json!(format!("{raster:016x}")) {
        return Err("the staged focus differs across the prepared map anchors".into());
    }
    let actor = view
        .units
        .iter()
        .find(|unit| unit.slot as u64 == active)
        .copied()
        .ok_or("the native fresh actor is missing from the staged view")?;
    let controls = &round["fresh_orders"];
    let to: (u16, u16) =
        serde_json::from_value(controls["move_northwest"]["unit_position_after"].clone())
            .map_err(|error| format!("invalid prepared fresh move destination: {error}"))?;
    let ready_checkpoint = session.checkpoint(resources);
    for (name, command, expected) in [
        ("skip", Command::Skip { unit: actor }, OrderResult::Skipped),
        (
            "move_northwest",
            Command::Move { unit: actor, to },
            OrderResult::Moved,
        ),
    ] {
        let native_order = &controls[name];
        let mut control = HumanSession::restore_with(
            rules.clone(),
            resources,
            &ready_checkpoint,
            Some((&files.bytes, replay_events.clone())),
        )?;
        let revision = control.view().revision;
        let result = control
            .submit(Request {
                civ: human,
                id: revision + 1,
                revision,
                command,
            })
            .map_err(|error| format!("prepared fresh {name}: {error:?}"))?;
        let view = control.view();
        let unit = view
            .observation
            .units
            .iter()
            .find(|unit| unit.index == actor.slot)
            .ok_or_else(|| format!("prepared fresh {name} lost its actor"))?;
        if result.result != Some(expected)
            || view.decision.is_some()
            || unit.moves_left != 0
            || json!([unit.x, unit.y]) != native_order["unit_position_after"]
            || json!(view.observation.seen.iter().flatten().count())
                != native_order["explored_tiles"]
        {
            return Err(format!(
                "the staged fresh {name} differs from the prepared order"
            ));
        }
        let after = control.checkpoint(resources);
        let restored = HumanSession::restore_with(
            rules.clone(),
            resources,
            &after,
            Some((&files.bytes, replay_events.clone())),
        )?;
        if restored.view() != view || restored.checkpoint(resources) != after {
            return Err(format!("the staged fresh {name} did not replay"));
        }
    }
    Ok(json!({
        "verified": true, "initializer_schema": 7,
        "profile": engine::human::ScenarioInitializer::SAVED_OPPONENT_COMBAT_PROFILE,
        "explored_tiles": explored, "combat_rng_matches": true, "field_stack_replays": true,
        "city_arrival_replays": true, "city_departure_replays": true,
        "partial_movement_replays": true, "singleton_movement_replays": true,
        "unconnected_road_replays": true, "round_close_replays": true,
        "fresh_human_view_replays": true, "fresh_orders_replay": true,
        "native_ready_slot": active, "native_focus_matches": true, "checkpoint_replays": true,
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
