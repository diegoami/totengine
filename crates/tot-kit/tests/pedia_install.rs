//! Env-gated check of the Rust Civilopedia decoder against the install's real
//! `Describe.txt` files, using the structure summary produced by the independent
//! Python inventory (`tools/pedia_inventory.py`, counts and hashes only).

#![cfg(test)]

#[test]
fn real_describe_files_match_the_independent_inventory() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        eprintln!("pedia_install: CIVTOT_HOME not set — skipping");
        return;
    };
    let home = std::path::Path::new(&home);
    let summary: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/evidence/pedia-structure-v1.json"
    ))
    .unwrap();
    let mut checked = 0;
    for (relative, expected) in summary["files"].as_object().unwrap() {
        let path = home.join(relative);
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("pedia_install: {relative} not installed — skipped");
            continue;
        };
        assert_eq!(
            bytes.len() as u64,
            expected["bytes"].as_u64().unwrap(),
            "{relative}"
        );
        let pedia = tot_kit::Pedia::parse(&tot_kit::latin1(&bytes))
            .unwrap_or_else(|e| panic!("{relative}: {e}"));
        let sections = expected["sections"].as_array().unwrap();
        assert_eq!(pedia.sections.len(), sections.len(), "{relative}");
        for (got, want) in pedia.sections.iter().zip(sections) {
            let name = want["name"].as_str().unwrap();
            assert_eq!(got.name, name, "{relative}");
            assert_eq!(
                got.entries.len() as u64,
                want["entries"].as_u64().unwrap(),
                "{relative} {name}"
            );
            assert_eq!(
                got.index.as_ref().map(|i| i.len() as u64),
                want["index_len"].as_u64(),
                "{relative} {name} index length"
            );
            assert_eq!(
                got.index
                    .as_ref()
                    .map(|i| i.iter().filter(|&&v| v >= 0).count() as u64),
                want["index_described"].as_u64(),
                "{relative} {name} described entries"
            );
            assert_eq!(
                got.index
                    .as_ref()
                    .map(|_| got.unresolved_index_values() as u64),
                want["index_unresolved"].as_u64(),
                "{relative} {name} unresolved"
            );
            assert!(
                got.entries.iter().all(|e| !e.title.is_empty()),
                "{relative} {name}"
            );
        }
        checked += 1;
    }
    assert!(checked > 0, "no Describe.txt found");
}
