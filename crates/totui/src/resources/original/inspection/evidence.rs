//! Connect the captured input to the retained native resume trace by hash.

use serde_json::{json, Value};

const HANDOFF: &str = include_str!("../../../../../../docs/evidence/import-input-handoff.json");
const FIRST_ORDERS: &str = include_str!("../../../../../../docs/evidence/import-first-order.json");
const LOADER_EXIT: &str = include_str!("../../../../../../docs/evidence/import-loader-exit.json");

const CITY_SCIENCE: &str =
    include_str!("../../../../../../docs/evidence/import-city-science-entry.json");

const CITY_VIEW: &str = include_str!("../../../../../../docs/evidence/import-city-view.json");

const ROUND_HANDOFF: &str =
    include_str!("../../../../../../docs/evidence/import-round-handoff.json");

const OPPONENT_COMBAT: &str =
    include_str!("../../../../../../docs/evidence/import-opponent-combat-view.json");

const ROUND_CLOSE: &str = include_str!("../../../../../../docs/evidence/import-round-close.json");

const SAVED_VIEWPORT: &str =
    include_str!("../../../../../../docs/evidence/import-saved-viewport.json");

fn same_input_trace(text: &str, report: &Value, name: &str) -> Result<Value, String> {
    let trace: Value = serde_json::from_str(text).map_err(|error| format!("{name}: {error}"))?;
    for key in ["source", "rules", "binary_sha256"] {
        if trace[key] != report[key] {
            return Err(format!("{name} and input trace refer to different inputs"));
        }
    }
    Ok(trace)
}

pub(super) fn resume_trace(save_sha256: &str, rules_sha256: &str) -> Result<Value, String> {
    let report: Value =
        serde_json::from_str(HANDOFF).map_err(|error| format!("native evidence: {error}"))?;
    if report["source"]["sha256"] != save_sha256 {
        return Ok(json!({"matches_fixture": false, "reason": "save bytes differ"}));
    }
    if report["rules"]["sha256"] != rules_sha256 {
        return Ok(json!({"matches_fixture": false, "reason": "Rules.txt bytes differ"}));
    }
    let observed = &report["cases"]["native_observation"];
    let resumed = &observed["observation_to_input"];
    let loader: Value =
        serde_json::from_str(LOADER_EXIT).map_err(|error| format!("loader evidence: {error}"))?;
    if loader["source"]["sha256"] != save_sha256
        || loader["rules_sha256"] != rules_sha256
        || loader["binary_sha256"] != report["binary_sha256"]
    {
        return Err("retained loader and input traces refer to different inputs".into());
    }
    let orders = same_input_trace(FIRST_ORDERS, &report, "first-order evidence")?;
    let science = same_input_trace(CITY_SCIENCE, &report, "city science evidence")?;
    let round = same_input_trace(ROUND_HANDOFF, &report, "round handoff evidence")?;
    let city_view = same_input_trace(CITY_VIEW, &report, "first opponent city-view evidence")?;
    let combat = same_input_trace(OPPONENT_COMBAT, &report, "opponent combat view evidence")?;
    let close = same_input_trace(ROUND_CLOSE, &report, "round close evidence")?;
    let viewport = same_input_trace(SAVED_VIEWPORT, &report, "saved viewport evidence")?;
    Ok(json!({
        "matches_fixture": true,
        "scope": "prepared native resume with display and empty-queue substitutes",
        "report": "docs/evidence/import-input-handoff.json",
        "binary_sha256": report["binary_sha256"],
        "explored_tiles_at_observation_boundary": observed["explored_tiles_after"],
        "first_poll": resumed["first_poll"],
        "idle_iteration": resumed["idle_iteration"],
        "first_orders": {
            "report": "docs/evidence/import-first-order.json",
            "scope": orders["scope"],
            "cases": orders["cases"],
        },
        "city_science": {
            "report": "docs/evidence/import-city-science-entry.json",
            "scope": science["scope"],
            "cases": science["cases"],
        },
        "round_handoff": {
            "report": "docs/evidence/import-round-handoff.json",
            "scope": round["scope"],
            "saved_options": round["saved_options"],
            "cleared_preference_bit_0x40": round["cleared_preference_bit_0x40"],
            "spend_saved_movement": round["spend_saved_movement"],
        },
        "city_view": {
            "report": "docs/evidence/import-city-view.json",
            "scope": city_view["scope"],
            "observed": city_view["observed"],
        },
        "opponent_combat": {
            "report": "docs/evidence/import-opponent-combat-view.json",
            "scope": combat["scope"],
            "cases": combat["cases"],
        },
        "round_close": close,
        "saved_viewport": viewport,
        "loader_regions": {
            "report": "docs/evidence/import-loader-exit.json",
            "after_event_read": loader["after_event_read"],
            "same_mode_return": loader["mode_restore"]["cases"]["same_mode"],
            "caller_options": loader["caller_options"]
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::original::sha256;

    #[test]
    fn either_input_change_excludes_native_observations() {
        let report: Value = serde_json::from_str(HANDOFF).unwrap();
        let saved = report["source"]["sha256"].as_str().unwrap();
        let rules = report["rules"]["sha256"].as_str().unwrap();
        let changed = sha256(b"an authored different input");
        for (save_digest, rules_digest, reason) in [
            (changed.as_str(), rules, "save bytes differ"),
            (saved, changed.as_str(), "Rules.txt bytes differ"),
        ] {
            assert_eq!(
                resume_trace(save_digest, rules_digest).unwrap(),
                json!({"matches_fixture": false, "reason": reason})
            );
        }
    }
}
