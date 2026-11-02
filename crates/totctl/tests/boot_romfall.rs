//! T1-2 acceptance: headless boot of ROMFALL.scn into engine state with a
//! reproducible checksum. Env-gated on `CIVTOT_HOME` (self-skips in CI),
//! mirroring `rules_install.rs`.

use std::process::Command;

use engine::rng::GameRng as _;

fn romfall() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("CIVTOT_HOME")?;
    let p = std::path::PathBuf::from(home).join("FallRome/ROMFALL.scn");
    p.exists().then_some(p)
}

#[test]
fn romfall_boots_headless_with_reproducible_checksum() {
    let Some(path) = romfall() else {
        eprintln!("skipping: CIVTOT_HOME/FallRome/ROMFALL.scn not present");
        return;
    };
    let sav = tot_kit::parse_sav_file(&path).expect("ROMFALL parses");

    // Walker-verified structure (docs/evidence/README.md §T0-6):
    // 34 + 710*40 unit region, 146-city grid, 9000-tile surface map.
    assert_eq!(sav.units.len(), 710, "unit count must match the walker");
    assert_eq!(sav.cities.len(), 146, "city count must match the walker");
    assert_eq!(sav.map_header.surface(), 9000);
    assert_eq!(sav.map_header.stored_w(), 75);
    assert_eq!(sav.map_header.stored_h(), 120);
    // ROMFALL's civ 1 name block starts with the West Roman leader.
    assert_eq!(
        sav.civ_names[0].strings.first().map(String::as_str),
        Some("Valentianus III")
    );

    // Boot is pure: same file + seed => identical checksum, twice.
    let a = engine::boot(&sav, 1);
    let b = engine::boot(&sav, 1);
    assert_eq!(a.checksum(), b.checksum());

    // Turn kernel is deterministic given the seed: two runs advance
    // identically (checksums stay equal), turn counters move together.
    let (mut x, mut y) = (engine::boot(&sav, 42), engine::boot(&sav, 42));
    for _ in 0..25 {
        assert_eq!(x.advance_turn(), y.advance_turn());
    }
    assert_eq!(x.turn, sav.turn + 25);
    assert_eq!(x.checksum(), y.checksum());

    // ...and the kernel never touches randomness at boot (skeleton).
    assert_eq!(x.rng.draws(), 0);
}

#[test]
fn totctl_boot_cli_reports_stable_checksum() {
    let Some(path) = romfall() else {
        eprintln!("skipping: CIVTOT_HOME/FallRome/ROMFALL.scn not present");
        return;
    };
    let run = || {
        let out = Command::new(env!("CARGO_BIN_EXE_totctl"))
            .arg("boot")
            .arg(&path)
            .output()
            .expect("totctl runs");
        assert!(
            out.status.success(),
            "totctl boot failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    let first = run();
    let second = run();
    let sum = |s: &str| {
        s.lines()
            .find(|l| l.starts_with("checksum"))
            .unwrap_or_else(|| panic!("no checksum line in output:\n{s}"))
            .to_string()
    };
    assert_eq!(sum(&first), sum(&second), "checksum must be reproducible");
    assert!(first.contains("units / cities: 710 / 146"));
}
