use engine::player::{Order, OrderResult};
use serde_json::{json, Value};
use std::{path::Path, process::Command};

const FIRST_ORDERS: &str = include_str!("../../../docs/evidence/import-first-order.json");

#[test]
fn unsupported_path_is_rejected_without_a_success_report() {
    let output = Command::new(env!("CARGO_BIN_EXE_totctl"))
        .args(["inspect-original-save", ".", "Other/file.scn"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("dependency capture:"),
        "{output:?}"
    );
}

#[test]
fn installed_save_reports_checked_records_phase_and_view() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        return;
    };
    let source = "Original/Ph_Auto2.SAV";
    let before = std::fs::read(Path::new(&home).join(source)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_totctl"))
        .arg("inspect-original-save")
        .arg(&home)
        .arg(source)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema"], 2);
    assert_eq!(report["source"]["path"], source);
    assert_eq!(report["source"]["fnv64"], "0d8e264d74369c1f");
    let native: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/evidence/import-input-handoff.json"
    ))
    .unwrap();
    assert_eq!(report["source"]["sha256"], native["source"]["sha256"]);
    let trace = &report["native_resume_trace"];
    assert_eq!(trace["matches_fixture"], true);
    assert_eq!(trace["binary_sha256"], native["binary_sha256"]);
    let witnessed = &native["cases"]["native_observation"]["observation_to_input"];
    assert_eq!(trace["first_poll"], witnessed["first_poll"]);
    assert_eq!(trace["idle_iteration"], witnessed["idle_iteration"]);
    let orders: Value = serde_json::from_str(FIRST_ORDERS).unwrap();
    assert_eq!(trace["first_orders"]["cases"], orders["cases"]);
    assert_eq!(trace["first_orders"]["scope"], orders["scope"]);
    let science: Value = serde_json::from_str(include_str!(
        "../../../docs/evidence/import-city-science-entry.json"
    ))
    .unwrap();
    assert_eq!(trace["city_science"]["cases"], science["cases"]);
    use engine::scenario::phase::{science_entry, ScienceBoundary};
    let save = tot_kit::parse_sav(&before).unwrap();
    for (case, boundary) in [
        ("pinned", ScienceBoundary::Return),
        ("one_science", ScienceBoundary::ChooseProject),
        (
            "one_science_with_advice",
            ScienceBoundary::AdvisorBeforeChoice,
        ),
    ] {
        let evidence = &science["cases"][case];
        let actual = science_entry(
            save.research[usize::from(save.player_civ)],
            evidence["accumulation"]["points"].as_i64().unwrap() as i32,
            true,
            evidence["advice"].as_bool().unwrap(),
        );
        assert_eq!(actual.boundary, boundary);
        assert_eq!(evidence["after"]["progress"], actual.progress);
    }
    let handoff: Value = serde_json::from_str(include_str!(
        "../../../docs/evidence/import-round-handoff.json"
    ))
    .unwrap();
    let handoff_check = &trace["round_handoff"];
    assert_eq!(handoff_check["saved_options"], handoff["saved_options"]);
    let advanced = &handoff["cleared_preference_bit_0x40"];
    assert_eq!(handoff_check["cleared_preference_bit_0x40"], *advanced);
    let civ = advanced["after"]["active_civ"].as_u64().unwrap() as u16;
    let first_city = save
        .cities
        .iter()
        .rposition(|city| city.city_id() != 0 && city.owner() == civ)
        .unwrap();
    assert_eq!(advanced["city_slot"], first_city);
    assert_eq!(advanced["before"]["turn"], advanced["after"]["turn"]);
    assert_eq!(advanced["after"]["resume_civ"], 0);
    let spent = &handoff["spend_saved_movement"];
    assert_eq!(handoff_check["spend_saved_movement"], *spent);
    assert_eq!(spent["after"], advanced["after"]);
    assert_eq!(spent["city_slot"], first_city);
    assert_eq!(
        spent["dispatched"]
            .as_array()
            .unwrap()
            .iter()
            .map(|input| (
                input["character"].as_u64().unwrap(),
                input["active_slot"].as_u64().unwrap()
            ))
            .collect::<Vec<_>>(),
        [(32, 6), (32, 9), (32, 26), (32, 18)]
    );
    let loader: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/evidence/import-loader-exit.json"
    ))
    .unwrap();
    assert_eq!(
        trace["loader_regions"]["after_event_read"],
        loader["after_event_read"]
    );
    assert_eq!(
        trace["loader_regions"]["same_mode_return"],
        loader["mode_restore"]["cases"]["same_mode"]
    );
    assert_eq!(
        trace["loader_regions"]["caller_options"],
        loader["caller_options"]
    );
    assert_eq!(report["records"]["unit_slots"], 38);
    assert_eq!(report["records"]["live_units"], 37);
    assert_eq!(report["records"]["live_cities"], 10);
    let native: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/evidence/import-event-tail.json"
    ))
    .unwrap();
    assert_eq!(
        report["records"]["post_city_bytes"],
        native["post_city_bytes"]
    );
    let trailer = &report["event_trailer"];
    assert_eq!(trailer["history_entries"], 0);
    assert_eq!(trailer["prefix_bytes"], native["event_offset_after_cities"]);
    assert_eq!(trailer["record_count"], native["record_count"]);
    assert_eq!(trailer["event_section_bytes"], native["native"]["consumed"]);
    let strings = native["native"]["strings"].as_array().unwrap();
    assert_eq!(trailer["string_count"], strings.len());
    assert_eq!(
        trailer["string_bytes"],
        strings
            .iter()
            .map(|row| row["bytes"].as_u64().unwrap())
            .sum::<u64>()
    );
    assert_eq!(report["saved_phase"]["boundary"], "human-preparation");
    assert_eq!(report["saved_phase"]["human_civ"], 4);
    assert_eq!(report["saved_view"]["explored_tiles"], 59);
    assert_eq!(report["saved_view"]["units"], 4);
    assert_eq!(report["saved_view"]["cities"], 1);
    assert_eq!(report["victory_event"]["matches"], true);
    assert_eq!(
        report["victory_event"]["saved_state"],
        serde_json::json!({"active": [false, false], "spent": [false, false]})
    );
    assert_eq!(report["playable_import"], false);
    assert_eq!(
        report["saved_opponent_combat_check"],
        json!({
            "verified": true,
            "initializer_schema": 7,
            "profile": "experimental-saved-opponent-combat-v1",
            "explored_tiles": 59,
            "combat_rng_matches": true,
            "field_stack_replays": true,
            "city_arrival_replays": true,
            "city_departure_replays": true,
            "partial_movement_replays": true,
            "singleton_movement_replays": true,
            "unconnected_road_replays": true,
            "round_close_replays": true,
            "fresh_human_view_replays": true,
            "fresh_orders_replay": true,
            "native_ready_slot": 18,
            "checkpoint_replays": true,
        })
    );
    assert_eq!(
        report["saved_opponent_stack_check"],
        json!({
            "verified": true,
            "initializer_schema": 6,
            "profile": "experimental-saved-opponent-stack-v1",
            "explored_tiles": 59,
            "checkpoint_replays": true,
            "moves_checked": 3,
        })
    );
    assert_eq!(
        report["saved_city_view_check"],
        json!({
            "verified": true,
            "initializer_schema": 5,
            "profile": "experimental-saved-city-view-v1",
            "explored_tiles": 59,
            "unit_knowledge_unchanged": true,
            "first_opponent_order_preserved_view": true,
            "checkpoint_replays": true,
        })
    );
    assert_eq!(
        report["saved_movement_check"],
        json!({
            "verified": true,
            "initializer_schema": 4,
            "profile": "experimental-saved-movement-v1",
            "unit_slot": 6,
            "position": [62, 4],
            "moves_left": 0,
            "explored_tiles": 60,
            "checkpoint_replays": true,
        })
    );
    assert_eq!(
        report["staged_round_check"],
        json!({
            "verified": true,
            "initializer_schema": 3,
            "profile": "experimental-staged-round-v1",
            "same_turn": true,
            "first_work_item": "city-pass",
            "second_work_item": "opponent-order",
            "checkpoints_replay": true,
        })
    );
    assert_eq!(
        report["saved_land_step"],
        json!({
            "matches": true,
            "unit_slot": 6,
            "position": [62, 4],
            "moves_left": 0,
            "explored_tiles": 60,
            "tile_bytes_match": true,
            "cache_bytes_match": true,
        })
    );
    assert_eq!(
        std::fs::read(Path::new(&home).join(source)).unwrap(),
        before
    );
}

#[test]
fn installed_native_first_move_matches_saved_knowledge_kernel() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        return;
    };
    let install = Path::new(&home).join("Original");
    let bytes = std::fs::read(install.join("Ph_Auto2.SAV")).unwrap();
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(install.join("Rules.txt")).unwrap(),
    ));
    let snapshot = engine::scenario::snapshot::CheckedSnapshot::read(&bytes, &rules).unwrap();
    let save = snapshot.save();
    let native: Value = serde_json::from_str(FIRST_ORDERS).unwrap();
    let moved = &native["cases"]["move_northwest"];
    assert_eq!(moved["observation_arguments"], json!([[6, 1]]));
    let slot = moved["before"]["active_slot"].as_u64().unwrap() as usize;
    let coordinate = |position: &Value| {
        (
            position[0].as_u64().unwrap() as u16,
            position[1].as_u64().unwrap() as u16,
        )
    };
    let old = coordinate(&moved["unit_position_before"]);
    let destination = coordinate(&moved["unit_position_after"]);
    assert_eq!((save.units[slot].x(), save.units[slot].y()), old);
    assert!(
        snapshot.project_land_step(&rules, slot, (61, 5)).is_err(),
        "a stack move requires separate continuation"
    );
    let measured = snapshot
        .project_land_step(&rules, slot, destination)
        .unwrap();
    assert_eq!(measured.position, destination);
    assert_eq!(measured.moves_left, 0);
    assert_eq!(measured.view.tiles.iter().flatten().count(), 60);
    assert_eq!((save.units[slot].x(), save.units[slot].y()), old);
    let delta = |before: &[u8], after: &[u8]| {
        before
            .iter()
            .zip(after)
            .enumerate()
            .filter_map(|(i, (old, new))| (old != new).then_some((i, *old, *new)))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        json!(delta(&save.maps[0].tile_fields, &measured.map.tile_fields)),
        moved["tile_byte_changes"]
    );
    let area = save.maps[0].tile_fields.len() / 6;
    let cache_delta = delta(&save.maps[0].visibility, &measured.map.visibility)
        .into_iter()
        .map(|(i, old, new)| (i / area + 1, i % area, old, new))
        .collect::<Vec<_>>();
    assert_eq!(json!(cache_delta), moved["cache_byte_changes"]);

    let mut world = engine::boot(save, 7);
    world.start_turn(&rules);
    let owner = save.player_civ;
    assert_eq!(world.units[slot].moves_left, 3);
    assert_eq!(
        world.apply_order(
            &rules,
            None,
            owner,
            Order::Move {
                unit: slot,
                to: destination
            }
        ),
        OrderResult::Moved
    );
    assert_eq!((world.units[slot].x, world.units[slot].y), destination);
    assert_eq!(world.units[slot].moves_left, 0);
    let visible = |bytes: Vec<u8>| {
        bytes
            .into_iter()
            .map(|byte| byte & (1 << owner) != 0)
            .collect::<Vec<_>>()
    };
    let native_visible = visible(
        measured
            .map
            .tile_fields
            .iter()
            .skip(4)
            .step_by(6)
            .copied()
            .collect(),
    );
    let engine_visible = visible(world.map.visible);
    let excess = native_visible
        .iter()
        .zip(engine_visible.iter())
        .enumerate()
        .filter_map(|(i, (original, current))| (original != current).then_some(i))
        .collect::<Vec<_>>();
    assert_eq!(
        excess,
        [12 * world.map.stored_w + 27],
        "legacy observation divergence"
    );
    assert!(engine_visible[excess[0]]);
}
