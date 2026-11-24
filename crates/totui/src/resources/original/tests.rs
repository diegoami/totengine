use super::*;

struct Fixture {
    root: tempfile::TempDir,
    pins: Vec<Pin>,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("Original");
        fs::create_dir(&dir).unwrap();
        fs::write(
            dir.join("Authored.sav"),
            b"authored snapshot, not native save bytes",
        )
        .unwrap();
        let mut pins = Vec::new();
        for pin in &PINS {
            let data = match pin.role {
                "rules" => b"@COSMIC\n3\n".to_vec(),
                "events" => b"@IF\nTURN\nturn=1\n@THEN\nTEXT\nAuthored text.\nENDTEXT\n@ENDIF\n".to_vec(),
                "labels" => b"@LABELS\n2\nBefore\nAfter\n".to_vec(),
                "describe" => b"@CONCEPT_DESCRIPTIONS\n@@Authored\nAuthored description.\n@This must be here to terminate search!!!\n".to_vec(),
                _ => {
                    let image = image::DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 4, image::Rgba([40, 80, 120, 255])));
                    let mut out = Cursor::new(Vec::new());
                    image.write_to(&mut out, ImageFormat::Bmp).unwrap();
                    out.into_inner()
                },
            };
            fs::write(dir.join(pin.file), &data).unwrap();
            let mut pin = pin.clone();
            pin.bytes = data.len();
            pin.sha256 = Cow::Owned(sha256(&data));
            pins.push(pin);
        }
        Self { root, pins }
    }

    fn capture(&self) -> Result<OriginalCapture, String> {
        OriginalCapture::capture_with_pins(
            self.root.path(),
            "Original/Authored.sav",
            &self.pins,
            None,
        )
    }

    fn path(&self, file: &str) -> PathBuf {
        self.root.path().join("Original").join(file)
    }
}

#[test]
fn capture_binds_owned_buffers_and_has_no_public_arbitrary_pin_override() {
    let f = Fixture::new();
    let capture = f.capture().unwrap();
    assert!(OriginalCapture::capture(f.root.path(), "Original/Authored.sav").is_err());
    assert_eq!(capture.bytes.len(), 13);
    assert_eq!(
        capture.files().bytes,
        capture.captured_bytes("scenario").unwrap()
    );
    assert_eq!(
        capture.resources().rules.section("COSMIC").unwrap().rows[0][0],
        "3"
    );
    assert_eq!(capture.files().events.as_ref().unwrap().events.len(), 1);
    assert_eq!(
        capture.resources().labels.as_ref().unwrap().get(0),
        Some("Before")
    );
    assert!(!capture.files().lua);
    let before = capture.identity().to_owned();
    assert!(before.contains(OriginalCapture::SET));
    assert!(before.contains(OriginalCapture::PRESENTATION));
    for (role, bytes) in &capture.bytes {
        assert!(before.contains(&format!("sha256 {role} {}\n", sha256(bytes))));
    }
    let raw = capture.captured_bytes("rules").unwrap().to_vec();
    fs::write(f.path("Rules.txt"), b"@COSMIC\n9\n").unwrap();
    assert_eq!(capture.identity(), before);
    assert_eq!(capture.captured_bytes("rules").unwrap(), raw);
    assert_eq!(
        capture.resources().rules.section("COSMIC").unwrap().rows[0][0],
        "3"
    );
    assert!(capture
        .revalidate_with_pins(f.root.path(), &f.pins)
        .unwrap_err()
        .contains("Rules.txt"));
    assert_eq!(capture.identity(), before);
}

#[test]
fn every_required_dependency_rejects_missing_and_changed_bytes_without_root_fallback() {
    let f = Fixture::new();
    for pin in &f.pins {
        let path = f.path(pin.file);
        let original = fs::read(&path).unwrap();
        fs::write(f.root.path().join(pin.file), &original).unwrap();
        fs::remove_file(&path).unwrap();
        let err = f.capture().err().unwrap();
        assert!(err.contains(pin.file) && err.contains("missing"), "{err}");
        let mut changed = original.clone();
        changed[0] ^= 1; // same length: digest must change
        fs::write(&path, changed).unwrap();
        let err = f.capture().err().unwrap();
        assert!(err.contains(pin.file) && err.contains("reference"), "{err}");
        let mut changed = original.clone();
        changed.push(b' ');
        fs::write(&path, changed).unwrap();
        assert!(f.capture().is_err());
        fs::write(path, original).unwrap();
    }
    assert!(f.capture().is_ok());
}

#[test]
fn snapshot_change_and_missing_file_fail_revalidation_without_mutating_capture() {
    let f = Fixture::new();
    let capture = f.capture().unwrap();
    let before = capture.identity().to_owned();
    fs::write(f.path("Authored.sav"), b"other authored snapshot").unwrap();
    assert!(capture
        .revalidate_with_pins(f.root.path(), &f.pins)
        .unwrap_err()
        .contains("changed since capture"));
    fs::remove_file(f.path("Authored.sav")).unwrap();
    assert!(capture
        .revalidate_with_pins(f.root.path(), &f.pins)
        .is_err());
    assert_eq!(capture.identity(), before);
    assert_eq!(
        capture.captured_bytes("scenario").unwrap(),
        b"authored snapshot, not native save bytes"
    );
}

#[test]
fn explicit_bmp_policy_ignores_unused_alternates_and_relocation_preserves_identity() {
    let a = Fixture::new();
    let capture = a.capture().unwrap();
    fs::write(a.path("Units.gif"), b"not selected by this policy").unwrap();
    let b = Fixture::new();
    capture
        .revalidate_with_pins(a.root.path(), &a.pins)
        .unwrap();
    capture
        .revalidate_with_pins(b.root.path(), &b.pins)
        .unwrap();
    assert_eq!(capture.identity(), b.capture().unwrap().identity());
    assert!(!capture
        .identity()
        .contains(&a.root.path().display().to_string()));
}

#[test]
fn invalid_selection_nonregular_inputs_and_lua_reject() {
    let f = Fixture::new();
    for path in [
        "Authored.sav",
        "../Original/Authored.sav",
        "Original/../Authored.sav",
        "Other/Authored.sav",
        "Original/Authored.scn",
        "Original/C:x.sav",
        "Original/x\ns.sav",
    ] {
        assert!(
            OriginalCapture::capture_with_pins(f.root.path(), path, &f.pins, None).is_err(),
            "{path}"
        );
    }
    assert!(OriginalCapture::capture_with_pins(
        f.root.path(),
        "original\\AUTHORED.SAV",
        &f.pins,
        None
    )
    .is_ok());
    for name in ["events.LuA", "other.luac", "LuA"] {
        fs::write(f.path(name), b"not executed").unwrap();
        assert!(f.capture().err().unwrap().contains("Lua"));
        fs::remove_file(f.path(name)).unwrap();
    }
    fs::create_dir(f.path("Lua")).unwrap();
    assert!(f.capture().err().unwrap().contains("Lua"));
    fs::remove_dir(f.path("Lua")).unwrap();
    fs::remove_file(f.path("Events.txt")).unwrap();
    fs::create_dir(f.path("Events.txt")).unwrap();
    assert!(f.capture().err().unwrap().contains("regular file"));
}

#[cfg(target_os = "linux")]
#[test]
fn case_ambiguity_rejects_required_files_and_profile_directories() {
    let f = Fixture::new();
    fs::copy(f.path("Rules.txt"), f.path("rULES.TXT")).unwrap();
    assert!(f.capture().err().unwrap().contains("ambiguous"));
    fs::remove_file(f.path("rULES.TXT")).unwrap();
    fs::create_dir(f.root.path().join("ORIGINAL")).unwrap();
    assert!(f.capture().err().unwrap().contains("ambiguous"));
}

#[cfg(unix)]
#[test]
fn dependency_and_snapshot_symlinks_cannot_escape_installation_or_profile() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let other = Fixture::new();
    fs::remove_file(f.path("Rules.txt")).unwrap();
    symlink(other.path("Rules.txt"), f.path("Rules.txt")).unwrap();
    assert!(f.capture().err().unwrap().contains("escapes"));
    fs::remove_file(f.path("Rules.txt")).unwrap();
    fs::write(f.root.path().join("Rules.txt"), b"@COSMIC\n3\n").unwrap();
    symlink(f.root.path().join("Rules.txt"), f.path("Rules.txt")).unwrap();
    assert!(f
        .capture()
        .err()
        .unwrap()
        .contains("leaves the selected profile"));
    fs::remove_file(f.path("Rules.txt")).unwrap();
    fs::write(f.path("Rules.txt"), b"@COSMIC\n3\n").unwrap();
    fs::remove_file(f.path("Authored.sav")).unwrap();
    symlink(other.path("Authored.sav"), f.path("Authored.sav")).unwrap();
    assert!(f.capture().err().unwrap().contains("escapes"));
}

#[test]
fn installed_original_capture_retains_the_complete_reference_dependency_set() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        return;
    };
    let home = PathBuf::from(home);
    let capture = OriginalCapture::capture(&home, "Original/Ph_Auto2.SAV").unwrap();
    capture.revalidate(&home).unwrap();
    assert_eq!(capture.bytes.len(), 13);
    let resources = capture.resources();
    let legacy = Resources::load(&home, "Original").unwrap();
    assert_eq!(resources.sheets, legacy.sheets);
    assert_eq!(
        tot_kit::emit_rules(&resources.rules),
        tot_kit::emit_rules(&legacy.rules)
    );
    assert_eq!(resources.labels, legacy.labels);
    assert_eq!(resources.pedia, legacy.pedia);
    assert_eq!(resources.text_sources, legacy.text_sources);
    assert_eq!(resources.sheets.len(), 8);
    assert!(resources.missing.is_empty());
    assert!(capture
        .files()
        .events
        .as_ref()
        .unwrap()
        .unknown_keywords
        .is_empty());
    if let Ok(path) = std::env::var("TOTENGINE_DEPENDENCIES_EVIDENCE") {
        let inputs = capture.bytes.iter().map(|(role, bytes)| {
            let source = resources.sources.get(role).or_else(||resources.text_sources.get(role)).unwrap();
            assert_eq!(source.fingerprint, fingerprint(bytes));
            assert_eq!(source.bytes, bytes.len());
            serde_json::json!({"role":role,"path":source.relative,"length":bytes.len(),"fnv64":format!("{:016x}",source.fingerprint),"sha256":sha256(bytes),"dimensions":resources.sheets.get(role).map(|s|[s.width(),s.height()])})
        }).collect::<Vec<_>>();
        let report = serde_json::json!({"schema":1,"dependency_set":OriginalCapture::SET,"presentation":OriginalCapture::PRESENTATION,"runtime_admitted":false,"inputs":inputs,"identity":capture.identity(),"identity_sha256":sha256(capture.identity().as_bytes()),"art_roles":resources.sheets.len(),"event_blocks":capture.files().events.as_ref().unwrap().events.len(),"source_length":capture.files().bytes.len(),"source_fnv64":format!("{:016x}",fingerprint(&capture.files().bytes))});
        fs::write(path, serde_json::to_string_pretty(&report).unwrap() + "\n").unwrap();
    }
}
