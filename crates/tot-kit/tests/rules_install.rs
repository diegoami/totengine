//! Env-gated tests against the real install (skips without CIVTOT_HOME,
//! mirroring the golden harness — no game data is committed).

#![cfg(test)]

#[test]
fn real_rules_roundtrips_and_passes_semantics() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        eprintln!("rules_install: CIVTOT_HOME not set — skipping");
        return;
    };
    let path = std::path::Path::new(&home).join("Rules.txt");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let rules = tot_kit::parse_rules(&text);

    // lossless round trip (normalizing only the CRLF/EOL distinction)
    let emitted = tot_kit::emit_rules(&rules);
    let normalized = text.replace("\r\n", "\n");
    assert_eq!(
        emitted, normalized,
        "text -> structs -> text must be lossless"
    );

    // the #44 semantic invariant on real data
    assert!(rules.terrain_semantic_ok(), "Desert=0 / Ocean=10");

    // @COSMIC keys come from comments
    let cosmic = rules.cosmic();
    assert!(cosmic.len() > 30, "real COSMIC has dozens of entries");
    assert!(
        cosmic
            .iter()
            .any(|(k, v)| k.contains("Road movement multiplier") && *v == "3"),
        "cosmic road multiplier: {:?}",
        &cosmic[..3.min(cosmic.len())]
    );
}
