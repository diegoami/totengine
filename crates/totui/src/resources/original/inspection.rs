//! Checks and report for one immutable capture of Original save inputs.

use super::{sha256, OriginalCapture};
use engine::{
    saved_knowledge::SavedMapKnowledge,
    scenario::{
        phase::{saved_human_entry, RoundEntry},
        snapshot::CheckedSnapshot,
    },
};
use serde_json::{json, Value};

mod event_tail;
mod evidence;
mod staged_round;

impl OriginalCapture {
    /// Check the captured save and report the supported inspection results.
    pub fn inspect_save(&self) -> Result<Value, String> {
        let files = self.files();
        let rules = &self.resources().rules;
        let source_sha256 = sha256(&files.bytes);
        let rules_sha256 = sha256(
            self.captured_bytes("rules")
                .ok_or("reference Rules.txt bytes are unavailable")?,
        );
        let snapshot = CheckedSnapshot::read(&files.bytes, rules)?;
        let save = snapshot.save();
        let human = save.player_civ;
        let phase = saved_human_entry(save, rules, human)?;
        let phase = match phase {
            RoundEntry::HumanPreparation => "human-preparation",
            _ => return Err("saved phase: expected human preparation".into()),
        };
        let view =
            SavedMapKnowledge::read(save, human).map_err(|error| format!("saved view: {error}"))?;
        // The snapshot checker has already verified the city table ends within
        // these captured bytes.
        let tail_start = files.bytes.len() - snapshot.trailing_bytes();
        let tail = event_tail::inspect(&files.bytes[tail_start..])?;
        let victory = files
            .events
            .as_ref()
            .ok_or("reference Events.txt was not parsed")
            .map_err(str::to_owned)
            .and_then(|events| tail.victory_state(events));
        let victory = match victory {
            Ok(state) => json!({"matches": true, "saved_state": state}),
            Err(reason) => json!({"matches": false, "reason": reason}),
        };
        let trace = evidence::resume_trace(&source_sha256, &rules_sha256)?;
        let land_step = if trace["matches_fixture"] == true {
            compare_land_step(
                &snapshot,
                rules,
                &trace["first_orders"]["cases"]["move_northwest"],
            )?
        } else {
            Value::Null
        };
        let staged = if trace["matches_fixture"] == true {
            staged_round::check(self, human)
                .unwrap_or_else(|reason| json!({"verified": false, "reason": reason}))
        } else {
            Value::Null
        };
        let movement = if land_step["matches"] == true {
            staged_round::check_saved_movement(
                self,
                human,
                &trace["first_orders"]["cases"]["move_northwest"],
            )
            .unwrap_or_else(|reason| json!({"verified": false, "reason": reason}))
        } else {
            Value::Null
        };
        let city_view = if trace["matches_fixture"] == true {
            staged_round::check_saved_city_view(self, human, &trace)
                .unwrap_or_else(|reason| json!({"verified": false, "reason": reason}))
        } else {
            Value::Null
        };
        Ok(json!({
            "schema": 2,
            "source": {
                "path": files.path,
                "bytes": snapshot.source_length(),
                "fnv64": format!("{:016x}", snapshot.source_fingerprint()),
                "sha256": source_sha256
            },
            "dependency_set": OriginalCapture::SET,
            "native_resume_trace": trace,
            "saved_land_step": land_step,
            "staged_round_check": staged,
            "saved_movement_check": movement,
            "saved_city_view_check": city_view,
            "records": {
                "unit_slots": save.units.len(),
                "live_units": snapshot.stack_positions().iter().flatten().count(),
                "live_cities": save.cities.iter().filter(|city| city.city_id() != 0).count(),
                "post_city_bytes": snapshot.trailing_bytes()
            },
            "event_trailer": tail,
            "victory_event": victory,
            "saved_phase": {
                "boundary": phase,
                "human_civ": human,
                "turn": save.turn
            },
            "saved_view": {
                "explored_tiles": view.tiles.iter().flatten().count(),
                "units": view.units.len(),
                "cities": view.cities.len()
            },
            "playable_import": false
        }))
    }
}

fn byte_changes<'a>(
    before: &'a [u8],
    after: &'a [u8],
) -> impl Iterator<Item = (usize, u8, u8)> + 'a {
    before
        .iter()
        .zip(after)
        .enumerate()
        .filter_map(|(index, (&old, &new))| (old != new).then_some((index, old, new)))
}

fn compare_land_step(
    snapshot: &CheckedSnapshot,
    rules: &tot_kit::Rules,
    native: &Value,
) -> Result<Value, String> {
    let slot = native["before"]["active_slot"]
        .as_u64()
        .and_then(|slot| usize::try_from(slot).ok())
        .ok_or("native land step evidence has no unit slot")?;
    let to: (u16, u16) = serde_json::from_value(native["unit_position_after"].clone())
        .map_err(|error| format!("native land step evidence: {error}"))?;
    let step = match snapshot.project_land_step(rules, slot, to) {
        Ok(step) => step,
        Err(reason) => return Ok(json!({"matches": false, "reason": reason})),
    };
    let before = &snapshot.save().maps[0];
    let tiles: Vec<_> = byte_changes(&before.tile_fields, &step.map.tile_fields).collect();
    let area = snapshot.save().map_header.surface();
    let cache: Vec<_> = byte_changes(&before.visibility, &step.map.visibility)
        .map(|(index, old, new)| (index / area + 1, index % area, old, new))
        .collect();
    let explored_tiles = step.view.tiles.iter().flatten().count();
    let tile_bytes_match = json!(tiles) == native["tile_byte_changes"];
    let cache_bytes_match = json!(cache) == native["cache_byte_changes"];
    let allowance = native["active_unit_type_allowance"]
        .as_u64()
        .ok_or("native land step evidence has no movement allowance")?;
    let spent = native["unit_byte_changes"]
        .as_array()
        .and_then(|bytes| {
            bytes
                .iter()
                .find(|row| row["byte"] == 10 && row["slot"] == slot)
        })
        .and_then(|row| row["after"].as_u64())
        .ok_or("native land step evidence has no resulting movement cost")?;
    Ok(json!({
        "matches": tile_bytes_match && cache_bytes_match && step.position == to
            && u64::from(step.moves_left) == allowance.saturating_sub(spent)
            && json!(explored_tiles) == native["explored_tiles"],
        "unit_slot": slot,
        "position": step.position,
        "moves_left": step.moves_left,
        "explored_tiles": explored_tiles,
        "tile_bytes_match": tile_bytes_match,
        "cache_bytes_match": cache_bytes_match,
    }))
}
