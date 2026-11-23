//! Connect the captured input to the retained native resume trace by hash.

use serde_json::{json, Value};

const HANDOFF: &str = include_str!("../../../../../../docs/evidence/import-input-handoff.json");
const FIRST_ORDERS: &str = include_str!("../../../../../../docs/evidence/import-first-order.json");
const LOADER_EXIT: &str = include_str!("../../../../../../docs/evidence/import-loader-exit.json");

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
    let orders: Value = serde_json::from_str(FIRST_ORDERS)
        .map_err(|error| format!("first-order evidence: {error}"))?;
    if orders["source"] != report["source"]
        || orders["rules"] != report["rules"]
        || orders["binary_sha256"] != report["binary_sha256"]
    {
        return Err("retained first-order and input traces refer to different inputs".into());
    }
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
