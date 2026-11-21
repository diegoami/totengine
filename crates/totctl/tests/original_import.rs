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
    assert_eq!(report["source"]["path"], source);
    assert_eq!(report["source"]["fnv64"], "0d8e264d74369c1f");
    assert_eq!(report["records"]["unit_slots"], 38);
    assert_eq!(report["records"]["live_units"], 37);
    assert_eq!(report["records"]["live_cities"], 10);
    assert_eq!(report["records"]["post_city_bytes_unchecked"], 3129);
    assert_eq!(report["saved_phase"]["boundary"], "human-preparation");
    assert_eq!(report["saved_phase"]["human_civ"], 4);
    assert_eq!(report["saved_view"]["explored_tiles"], 59);
    assert_eq!(report["saved_view"]["units"], 4);
    assert_eq!(report["saved_view"]["cities"], 1);
    assert_eq!(report["playable_import"], false);
    assert_eq!(
        std::fs::read(Path::new(&home).join(source)).unwrap(),
        before
    );
}
