//! T1-8 acceptance: the surgical writer round-trips real saves
//! byte-identically and typed edits survive a re-parse. Env-gated on
//! `CIVTOT_HOME` (self-skips in CI).

fn read(rel: &str) -> Option<Vec<u8>> {
    let home = std::env::var_os("CIVTOT_HOME")?;
    std::fs::read(std::path::PathBuf::from(home).join(rel)).ok()
}

#[test]
fn real_saves_roundtrip_byte_identically() {
    // Midgard is deliberately absent: it is a multi-map file the walker
    // cannot locate either (hdr[7] = 3 map blocks — known-open); the
    // parser must refuse it, not guess.
    for rel in [
        "FallRome/ROMFALL.scn",
        "FallRome/SCN/ROMFALL_WRE.scn",
        "Scenario/FallRome/Va_Auto.SAV",
    ] {
        let Some(raw) = read(rel) else {
            eprintln!("skipping {rel}: not present");
            continue;
        };
        let out = tot_kit::emit_sav(&raw).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert_eq!(out, raw, "{rel}: no-edit round trip must be identical");
    }
    // multi-map scenarios are correctly refused (no fabricated tables)
    if let Some(raw) = read("Midgard/Midgard.SCN") {
        assert!(
            tot_kit::parse_sav(&raw).is_err(),
            "multi-map file must not decode (known-open layout)"
        );
    }
}

#[test]
fn typed_edits_roundtrip_and_are_confined() {
    let Some(raw) = read("FallRome/ROMFALL.scn") else {
        eprintln!("skipping: install not present");
        return;
    };
    let before = tot_kit::parse_sav(&raw).expect("parses");

    let mut ed = tot_kit::SavEditor::new(&raw).expect("editor opens");
    ed.set_turn(7);
    ed.set_year(-3500);
    ed.set_difficulty(3);
    ed.set_rivals(2);
    ed.set_unit_pos(0, 40, 30).expect("unit edit");
    ed.set_city_name(0, "Testville").expect("city rename");
    let out = ed.into_bytes();

    let after = tot_kit::parse_sav(&out).expect("edited file parses");
    assert_eq!(after.turn, 7);
    assert_eq!(after.year, -3500);
    assert_eq!(after.difficulty, 3);
    assert_eq!(after.rivals, 2);
    assert_eq!((after.units[0].x(), after.units[0].y()), (40, 30));
    assert_eq!(after.cities[0].name(), "Testville");
    // structure untouched
    assert_eq!(after.units.len(), before.units.len());
    assert_eq!(after.cities.len(), before.cities.len());
    assert_eq!(after.maps[0].terrain_histogram(), before.maps[0].terrain_histogram());

    // every differing byte is an offset the edits touch
    let allowed: Vec<usize> = {
        let mut v = vec![668, 669, 670, 671, 684, 689];
        v.extend(iterator_offsets(&before, 0)); // unit 0 slot
        v.extend(before.city_offset + 34..before.city_offset + 50); // city 0 name
        v
    };
    let diffs: Vec<usize> = (0..raw.len()).filter(|&i| raw[i] != out[i]).collect();
    for d in &diffs {
        assert!(
            allowed.contains(d),
            "unexpected edited byte at offset {d}"
        );
    }
}

/// Byte offsets of unit 0's x/y slot (region start + 40*idx + 0..4)
/// plus type — the only bytes `set_unit_pos`/`set_unit_type` touch.
fn iterator_offsets(sav: &tot_kit::SaveFile, idx: usize) -> Vec<usize> {
    let base = sav.unit_region.0 + 40 * idx;
    vec![base, base + 1, base + 2, base + 3, base + 8]
}

#[test]
fn rejects_bad_edits() {
    let Some(raw) = read("FallRome/ROMFALL.scn") else {
        eprintln!("skipping: install not present");
        return;
    };
    let mut ed = tot_kit::SavEditor::new(&raw).expect("editor opens");
    assert!(ed.set_unit_pos(1_000_000, 0, 0).is_err());
    assert!(ed.set_city_name(0, "WayTooLongName123").is_err());
}
