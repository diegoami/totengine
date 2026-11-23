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
