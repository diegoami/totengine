use std::{path::Path, process::Command};

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
        std::fs::read(Path::new(&home).join(source)).unwrap(),
        before
    );
}
