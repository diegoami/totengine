//! #329: `ADVANCE_IDS` is the executable's own table (skips without
//! CIVTOT_HOME). civ2.bak keeps 100 advance records of 20 bytes at VA
//! 0x5d0a18 (file offset 0x1cf018); each starts with its NUL-terminated
//! slot identifier.

#[test]
fn advance_ids_match_the_original_executable() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        eprintln!("skipping: CIVTOT_HOME not set");
        return;
    };
    let Ok(exe) = std::fs::read(std::path::Path::new(&home).join("civ2.bak")) else {
        eprintln!("skipping: civ2.bak not present");
        return;
    };
    let table = &exe[0x1cf018..0x1cf018 + 2000];
    for (k, id) in tot_kit::ADVANCE_IDS.iter().enumerate() {
        let rec = &table[20 * k..20 * k + 4];
        let end = rec.iter().position(|&b| b == 0).unwrap_or(4);
        assert_eq!(&rec[..end], id.as_bytes(), "slot {k}");
    }
}
