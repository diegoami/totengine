use super::*;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "tot-client-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(p.join("Original")).unwrap();
        fs::write(
            p.join("Rules.txt"),
            format!(
                "{}\n{}",
                include_str!("../../../engine/tests/fixtures/campaign-rules.txt"),
                include_str!("../../../engine/tests/fixtures/builtin-settings.txt")
            ),
        )
        .unwrap();
        let img = image::RgbaImage::from_pixel(640, 640, image::Rgba([40, 160, 70, 255]));
        for name in crate::resources::ART_NAMES {
            img.save(p.join(format!("{name}.bmp"))).unwrap();
        }
        Self(p)
    }
    fn client(&self) -> Client {
        Client::new(self.0.to_str().unwrap().into(), self.0.join("saves"))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap()
    }
}
fn input(c: &mut Client, i: Input) {
    let id = c.receipts.len() as u64 + 1;
    c.input(c.revision, id, i).unwrap();
}
fn key(c: &mut Client, k: &str) {
    input(c, Input::Key { key: k.into() });
}
fn button(c: &mut Client, label: &str) {
    let b = c
        .buttons()
        .into_iter()
        .find(|b| b.label.contains(label))
        .unwrap_or_else(|| panic!("missing button {label}"));
    input(
        c,
        Input::Click {
            x: b.x + 3.,
            y: b.y + 3.,
        },
    );
}
fn edit(c: &mut Client, value: &str) {
    let field = c.entry_meta().unwrap().id;
    input(
        c,
        Input::Edit {
            field,
            value: value.into(),
        },
    );
}
fn research(c: &mut Client) {
    if matches!(
        c.view().and_then(|v| v.decision),
        Some(Decision::Research { .. })
    ) {
        c.required();
        key(c, "ArrowDown");
        key(c, "Enter");
    }
}
fn ready(c: &mut Client) {
    for _ in 0..2000 {
        if c.view().unwrap().phase != Phase::Opponents {
            return;
        }
        input(c, Input::Tick);
    }
    panic!("no human turn")
}
fn start(c: &mut Client) {
    button(c, "Start Original");
    assert!(c.loaded.is_some(), "{}", c.status);
    research(c);
}
fn founding(c: &mut Client) {
    key(c, "f");
    assert!(matches!(c.modal, Some(Modal::Name)));
    edit(c, "Athena");
    key(c, "Enter");
    assert!(c.status.starts_with("founded"), "{}", c.status);
}

#[test]
fn shared_ui_plays_ten_rounds_and_resumes_required_choices() {
    let f = Fixture::new();
    let mut c = f.client();
    button(&mut c, "Start Original");
    assert!(matches!(c.modal, Some(Modal::Research)));
    button(&mut c, "Save pending");
    edit(&mut c, "initial-research");
    key(&mut c, "Enter");
    assert!(f.0.join("saves/initial-research.cth").exists());
    research(&mut c);
    key(&mut c, "f");
    button(&mut c, "Save pending");
    edit(&mut c, "pending-name");
    key(&mut c, "Enter");
    assert!(matches!(c.modal, Some(Modal::Name)));
    edit(&mut c, "Athena");
    key(&mut c, "Enter");
    let city = c.view().unwrap().cities[0];
    button(&mut c, "Choose production");
    key(&mut c, "ArrowDown");
    key(&mut c, "Enter");
    assert_eq!(c.status, "prod-set");
    let saved = c.checkpoint().unwrap();
    key(&mut c, "s");
    edit(&mut c, "normal");
    key(&mut c, "Enter");
    assert_eq!(storage::load(&c.saves, "normal").unwrap(), saved);
    for _ in 0..10 {
        research(&mut c);
        key(&mut c, "e");
        if matches!(c.modal, Some(Modal::Confirm { .. })) {
            key(&mut c, "Enter");
        }
        ready(&mut c);
        assert_eq!(c.view().unwrap().phase, Phase::Human);
    }
    research(&mut c);
    let advanced = c.checkpoint().unwrap();
    assert_ne!(advanced, saved);
    key(&mut c, "l");
    edit(&mut c, "normal");
    key(&mut c, "Enter");
    assert_eq!(c.checkpoint().unwrap(), saved);
    key(&mut c, "l");
    edit(&mut c, "pending-name");
    key(&mut c, "Enter");
    assert!(matches!(c.modal, Some(Modal::Name)));
    assert!(c.view().unwrap().cities.is_empty());
    edit(&mut c, "Athena");
    key(&mut c, "Enter");
    assert_eq!(c.view().unwrap().cities[0], city);
    key(&mut c, "l");
    edit(&mut c, "initial-research");
    key(&mut c, "Enter");
    assert!(matches!(c.modal, Some(Modal::Research)));
}
#[test]
fn stale_duplicate_focus_and_cancellation_cannot_retarget_a_unit() {
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    let before = c.checkpoint().unwrap();
    let revision = c.revision;
    let id = 9000;
    let event = Input::Key { key: "m".into() };
    c.input(revision, id, event.clone()).unwrap();
    let mode = c.moving;
    c.input(revision, id, event).unwrap();
    assert_eq!(mode, c.moving);
    assert!(c
        .input(revision, 9001, Input::Key { key: "6".into() })
        .is_err());
    assert_eq!(c.checkpoint().unwrap(), before);
    input(&mut c, Input::Focus { focused: false });
    key(&mut c, "6");
    assert_eq!(c.checkpoint().unwrap(), before);
    input(&mut c, Input::Focus { focused: true });
    key(&mut c, "f");
    key(&mut c, "Escape");
    assert_eq!(
        c.checkpoint().unwrap().lines().nth(7),
        before.lines().nth(7)
    );
    let old = c.entry_meta();
    key(&mut c, "f");
    let field = c.entry_meta().unwrap().id;
    key(&mut c, "Escape");
    input(
        &mut c,
        Input::Edit {
            field,
            value: "Late name".into(),
        },
    );
    assert!(c.entry_meta().is_none());
    assert!(old.is_none());
    assert_eq!(
        c.checkpoint().unwrap().lines().nth(7),
        before.lines().nth(7)
    );
}
#[test]
fn invalid_load_and_existing_slot_cannot_destroy_the_running_game() {
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    founding(&mut c);
    let before = c.checkpoint().unwrap();
    storage::save(&c.saves, "keep", &before).unwrap();
    assert!(storage::save(&c.saves, "keep", "bad").is_err());
    assert_eq!(storage::load(&c.saves, "keep").unwrap(), before);
    fs::write(c.saves.join("broken.cth"), "truncated").unwrap();
    key(&mut c, "l");
    edit(&mut c, "broken");
    key(&mut c, "Enter");
    assert_eq!(c.checkpoint().unwrap(), before);
    assert!(c.status.contains("unsupported"));
    assert!(storage::load(&c.saves, "../keep").is_err());
    let mut fresh = f.client();
    button(&mut fresh, "Load engine checkpoint");
    edit(&mut fresh, "keep");
    key(&mut fresh, "Enter");
    assert_eq!(fresh.checkpoint().unwrap(), before);
}
#[test]
fn frame_serialization_never_contains_spectator_state_or_replay() {
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    let v = c.view().unwrap();
    let json = serde_json::to_string(&c.frame()).unwrap();
    for forbidden in [
        "totengine-human-orderlog",
        "fingerprint",
        "policy builtin-1",
        "log ",
        "checkpoint",
    ] {
        assert!(!json.contains(forbidden), "{forbidden}");
    }
    assert!(!v.observation.seen.iter().all(Option::is_some));
    let foreign = c
        .loaded
        .as_ref()
        .unwrap()
        .session
        .view()
        .observation
        .units
        .iter()
        .filter(|u| u.owner != v.observation.civ)
        .count();
    assert_eq!(foreign, 0);
    assert!(c.loaded.as_ref().unwrap().atlas.sprites.len() > 50);
}

#[test]
fn viewing_movement_skip_and_partial_checkpoint_resume_use_validated_orders() {
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    let before = c.checkpoint().unwrap();
    key(&mut c, "6");
    assert_eq!(c.checkpoint().unwrap(), before);
    key(&mut c, "m");
    let v = c.view().unwrap();
    let unit = c.active.unwrap();
    let u = v
        .observation
        .units
        .iter()
        .find(|u| u.index == unit.slot)
        .unwrap();
    let directions = [
        ("6", 2, 0),
        ("4", -2, 0),
        ("8", 0, -2),
        ("2", 0, 2),
        ("9", 1, -1),
        ("7", -1, -1),
        ("3", 1, 1),
        ("1", -1, 1),
    ];
    let direction = directions
        .iter()
        .find(|(_, dx, dy)| {
            let (x, y) = (i32::from(u.x) + dx, i32::from(u.y) + dy);
            x >= 0
                && y >= 0
                && x < i32::from(v.observation.w2)
                && y < i32::from(v.observation.h)
                && v.observation.seen
                    [(y as usize) * (usize::from(v.observation.w2) / 2) + x as usize / 2]
                    .is_some_and(|t| t.terrain != 10)
        })
        .unwrap()
        .0;
    key(&mut c, direction);
    assert_ne!(c.checkpoint().unwrap(), before);
    assert_eq!(c.status, "moved");
    key(&mut c, " ");
    assert_eq!(c.status, "skipped");
    assert_eq!(
        c.view()
            .unwrap()
            .observation
            .units
            .iter()
            .find(|u| u.index == unit.slot)
            .unwrap()
            .moves_left,
        0
    );
    key(&mut c, "e");
    if matches!(c.modal, Some(Modal::Confirm { .. })) {
        key(&mut c, "Enter");
    }
    assert_eq!(c.view().unwrap().phase, Phase::Opponents);
    let l = c.loaded.as_ref().unwrap();
    let text = l.session.checkpoint(&l.identity);
    storage::save(&c.saves, "partial", &text).unwrap();
    key(&mut c, "l");
    edit(&mut c, "partial");
    key(&mut c, "Enter");
    assert!(!c.advancing);
    let paused = c.checkpoint().unwrap();
    input(&mut c, Input::Tick);
    assert_eq!(c.checkpoint().unwrap(), paused);
    button(&mut c, "Continue opponents");
    ready(&mut c);
    assert_eq!(c.view().unwrap().phase, Phase::Human);
}

/// Recorded UI input from a generated Original game; no injected world state.
/// It is automation evidence, not owner-play acceptance.
#[test]
#[ignore = "install-backed generated land encounter input route"]
fn generated_land_encounter_through_shared_ui() {
    use engine::player::OrderResult;
    let home = std::env::var("CIVTOT_HOME").expect("set CIVTOT_HOME");
    let out =
        PathBuf::from(std::env::var("TOTENGINE_ENCOUNTER_OUT").expect("set private output folder"));
    fs::create_dir_all(&out).unwrap();
    let inputs: Vec<Input> = serde_json::from_str(include_str!(
        "../../../../docs/evidence/client-input-route-v1.json"
    ))
    .unwrap();
    let mut c = Client::new(home, out.clone());
    let mut battle = false;
    let mut capture = false;
    let mut direction = String::new();
    for mut i in inputs {
        // Field tokens are presentation epochs; bind typing to the current field.
        if let Input::Edit { field, .. } = &mut i {
            *field = c.entry_meta().expect("recorded edit needs a field").id;
        }
        let before = c.checkpoint();
        if let Input::Key { key } = &i {
            if key.chars().all(|v| v.is_ascii_digit()) {
                direction = key.clone();
            }
        }
        let pending = if let Some(Modal::Confirm { command, .. }) = &c.modal {
            Some(*command)
        } else {
            None
        };
        let active = c.active;
        input(&mut c, i);
        let after = c.checkpoint();
        if before != after {
            let Some(before) = before else { continue };
            let result = c.view().unwrap().observation.last_result;
            let name = match result {
                Some(OrderResult::Attacked { .. }) if !battle => {
                    battle = true;
                    Some("battle")
                }
                Some(OrderResult::Occupied { .. }) if !capture => {
                    capture = true;
                    Some("capture")
                }
                _ => None,
            };
            if let Some(name) = name {
                fs::write(out.join(format!("before-{name}.cth")), before).unwrap();
                fs::write(out.join(format!("after-{name}.cth")), after.unwrap()).unwrap();
                let actor = active.unwrap();
                fs::write(out.join(format!("{name}.json")),serde_json::to_string_pretty(&serde_json::json!({"actor":actor.slot,"direction":direction,"command":format!("{pending:?}"),"result":format!("{result:?}")})).unwrap()).unwrap();
            }
        }
    }
    eprintln!("route final: {}", c.status);
    fs::write(out.join("last.cth"), c.checkpoint().unwrap()).unwrap();
    assert!(battle, "no ground battle in recorded input route");
    assert!(capture, "no city occupation in recorded input route");
    fs::write(out.join("last.cth"), c.checkpoint().unwrap()).unwrap();
    println!(
        "Generated route completed at turn {}: {out:?}",
        c.view().unwrap().observation.turn
    );
}

#[test]
fn installed_atlas_matches_retained_numeric_manifest() {
    let Ok(home) = std::env::var("CIVTOT_HOME") else {
        return;
    };
    let resources = Resources::load(Path::new(&home), "Original").unwrap();
    let atlas = Atlas::load(&resources).unwrap();
    let sprites = atlas.sprites.iter().map(|(name,s)| {
        let opaque=s.rgba.as_chunks::<4>().0.iter().filter(|p| p[3] != 0).count();
        let hash=s.rgba.iter().fold(0xcbf2_9ce4_8422_2325u64,|h,b|(h^u64::from(*b)).wrapping_mul(0x100_0000_01b3));
        serde_json::json!({"name":name,"width":s.w,"height":s.h,"nontransparent":opaque,"rgba_fnv64":format!("{hash:016x}")})
    }).collect::<Vec<_>>();
    let measured = serde_json::json!({"resource_identity":resources.identity(),"sprites":sprites});
    if let Ok(out) = std::env::var("TOTENGINE_ATLAS_MANIFEST_OUT") {
        fs::write(out, serde_json::to_string_pretty(&measured).unwrap() + "\n").unwrap();
    }
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/evidence/client-atlas-v1.json"
    ))
    .unwrap();
    // Different owned installs remain supported; this evidence binds the reference install.
    if measured["resource_identity"] == expected["resource_identity"] {
        assert_eq!(measured, expected);
    }
    assert_eq!(
        atlas.sprites.len(),
        61 + resources.rules.unit_names().len().min(80)
    );
}
