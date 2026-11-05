//! Asset-free command, decoder and overwrite-protection smoke test.
use std::{fs, process::Command};
#[test]
fn generated_maps_are_seeded_and_existing_files_are_preserved() {
    let dir = std::env::temp_dir().join(format!(
        "totengine-worldgen-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&dir).unwrap();
    let run = |name: &str, seed: &str, width: &str| {
        Command::new(env!("CARGO_BIN_EXE_totctl"))
            .arg("worldgen")
            .arg(dir.join(name))
            .args(["--seed", seed, "--width", width])
            .output()
            .unwrap()
    };
    assert!(run("a.mp", "7", "80").status.success());
    assert!(run("b.mp", "7", "80").status.success());
    assert!(run("c.mp", "8", "80").status.success());
    let a = fs::read(dir.join("a.mp")).unwrap();
    assert_eq!(a, fs::read(dir.join("b.mp")).unwrap());
    assert_ne!(a, fs::read(dir.join("c.mp")).unwrap());
    assert_eq!(tot_kit::parse_mp(&a).unwrap().real_tile_count(), 2000);
    let repeat = run("a.mp", "8", "80");
    assert!(!repeat.status.success());
    assert_eq!(a, fs::read(dir.join("a.mp")).unwrap());
    assert!(!run("bad.mp", "7", "81").status.success());
    assert!(!dir.join("bad.mp").exists());
    fs::remove_dir_all(dir).unwrap();
}
