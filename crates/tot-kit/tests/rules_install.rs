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

    // T1-3: movement columns against classic Civ2 values in the GLOBAL
    // rules (the scenario files tune their own numbers).
    assert_eq!(rules.road_multiplier(), Some(3));
    let unit = |name: &str| {
        rules
            .unit_names()
            .iter()
            .position(|n| *n == name)
            .unwrap_or_else(|| panic!("unit {name} not in @UNITS"))
    };
    // classic movement rates: Settlers 1, Caravan 1, Knights 2, Trireme 3,
    // Ironclad 4, Cruise Missile 12, Nuclear Missile 16
    for (name, mv) in [
        ("Settlers", 1u32),
        ("Caravan", 1),
        ("Knights", 2),
        ("Trireme", 3),
        ("Ironclad", 4),
        ("Cruise Msl.", 12),
        ("Nuclear Msl.", 16),
    ] {
        let id = unit(name);
        assert_eq!(rules.unit_move(id), Some(mv), "{name} move");
    }
    // classic hold (transport capacity) and cost/10
    assert_eq!(rules.unit_hold(unit("Trireme")), Some(2));
    assert_eq!(rules.unit_hold(unit("Caravan")), Some(0));
    assert_eq!(rules.unit_cost(unit("Ironclad")), Some(6)); // 60 shields
    assert_eq!(rules.unit_cost(unit("Battleship")), Some(16));
    // classic terrain move costs (global file)
    let terr = |name: &str| {
        rules
            .base_terrain_names()
            .iter()
            .position(|n| *n == name)
            .unwrap_or_else(|| panic!("terrain {name} not in @TERRAIN"))
    };
    for (name, cost) in [
        ("Desert", 1u32),
        ("Plains", 1),
        ("Grassland", 1),
        ("Forest", 2),
        ("Hills", 2),
        ("Mountains", 3),
        ("Tundra", 1),
        ("Glacier", 2),
        ("Swamp", 2),
        ("Jungle", 2),
        ("Ocean", 1),
    ] {
        let id = terr(name);
        assert_eq!(rules.terrain_move_cost(id), Some(cost), "{name} move cost");
    }
    // classic defense bonuses: Forest 3 < Hills 4 < Mountains 6
    assert!(rules.terrain_defense(terr("Mountains")) > rules.terrain_defense(terr("Hills")));
    assert!(rules.terrain_defense(terr("Hills")) > rules.terrain_defense(terr("Plains")));
}
