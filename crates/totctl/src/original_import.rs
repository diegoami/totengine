use std::{path::Path, process::ExitCode};

use engine::{
    saved_knowledge::SavedMapKnowledge,
    scenario::{
        phase::{saved_human_entry, RoundEntry},
        snapshot::CheckedSnapshot,
    },
};
use serde_json::{json, Value};
use totui::resources::original::OriginalCapture;

mod event_tail;

pub fn run(args: &[String]) -> ExitCode {
    let [install, relative] = args else {
        eprintln!("usage: totctl inspect-original-save <install> <Original/file.sav>");
        return ExitCode::from(2);
    };
    match inspect(Path::new(install), relative) {
        Ok(report) => {
            println!("{report:#}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("inspect-original-save: {error}");
            ExitCode::FAILURE
        }
    }
}

fn inspect(install: &Path, relative: &str) -> Result<Value, String> {
    let capture = OriginalCapture::capture(install, relative)
        .map_err(|error| format!("dependency capture: {error}"))?;
    let files = capture.files();
    let rules = &capture.resources().rules;
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
    Ok(json!({
        "schema": 2,
        "source": {
            "path": files.path,
            "bytes": snapshot.source_length(),
            "fnv64": format!("{:016x}", snapshot.source_fingerprint())
        },
        "dependency_set": OriginalCapture::SET,
        "records": {
            "unit_slots": save.units.len(),
            "live_units": snapshot.stack_positions().iter().flatten().count(),
            "live_cities": save.cities.iter().filter(|city| city.city_id() != 0).count(),
            "post_city_bytes": snapshot.trailing_bytes()
        },
        "event_trailer": tail,
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
