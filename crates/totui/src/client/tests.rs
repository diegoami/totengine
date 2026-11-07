use super::*;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
pub(super) struct Fixture(PathBuf);
impl Fixture {
    pub(super) fn new() -> Self {
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
    pub(super) fn client(&self) -> Client {
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
    let buttons = c.buttons();
    let b = buttons
        .iter()
        .find(|b| b.label == label)
        .or_else(|| buttons.iter().find(|b| b.label.contains(label)))
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
pub(super) fn start(c: &mut Client) {
    button(c, "Start Original");
    assert!(c.loaded.is_some(), "{}", c.status);
    research(c);
}
pub(super) fn founding(c: &mut Client) {
    key(c, "b");
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
    key(&mut c, "b");
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
    assert_eq!(c.view().unwrap().production_context_city, None);
    let saved = c.checkpoint().unwrap();
    key(&mut c, "Ctrl+s");
    edit(&mut c, "normal");
    key(&mut c, "Enter");
    assert_eq!(storage::load(&c.saves, "normal").unwrap(), saved);
    for _ in 0..10 {
        research(&mut c);
        key(&mut c, "Ctrl+n");
        if matches!(c.modal, Some(Modal::Confirm { .. })) {
            key(&mut c, "Enter");
        }
        ready(&mut c);
        assert_eq!(c.view().unwrap().phase, Phase::Human);
    }
    research(&mut c);
    let advanced = c.checkpoint().unwrap();
    assert_ne!(advanced, saved);
    key(&mut c, "Ctrl+l");
    edit(&mut c, "normal");
    key(&mut c, "Enter");
    assert_eq!(c.checkpoint().unwrap(), saved);
    assert!(!c.city_open, "map picker reload must remain on the map");
    key(&mut c, "Ctrl+l");
    edit(&mut c, "pending-name");
    key(&mut c, "Enter");
    assert!(matches!(c.modal, Some(Modal::Name)));
    assert!(c.view().unwrap().cities.is_empty());
    edit(&mut c, "Athena");
    key(&mut c, "Enter");
    assert_eq!(c.view().unwrap().cities[0], city);
    key(&mut c, "Ctrl+l");
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
    let event = Input::Key { key: "v".into() };
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
    key(&mut c, "b");
    key(&mut c, "Escape");
    assert_eq!(
        c.checkpoint().unwrap().lines().nth(7),
        before.lines().nth(7)
    );
    let old = c.entry_meta();
    key(&mut c, "b");
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
    key(&mut c, "Ctrl+l");
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
fn city_workspace_edits_commit_close_and_resume_without_retargeting() {
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    founding(&mut c);
    let original = c.checkpoint().unwrap();
    button(&mut c, "Manage city");
    assert!(c.city_open);
    assert_eq!(original, c.checkpoint().unwrap(), "opening is inert");
    assert!(c
        .frame()
        .draws
        .iter()
        .any(|d| matches!(d, Draw::Text{text,..} if text == "Currently producing:")));
    let current = c.view().unwrap().observation.cities[0].building.unwrap();
    assert!(c
        .frame()
        .draws
        .iter()
        .any(|d| matches!(d, Draw::Text{text,..} if text == &c.item_name(current))));
    let v = c.view().unwrap();
    let k = v.city_citizens[0]
        .radius
        .iter()
        .take(20)
        .position(|t| t.worked)
        .unwrap() as u8;
    for _ in 0..21 {
        if c.city_tile == k {
            break;
        }
        key(&mut c, "ArrowRight");
    }
    let stale = c.revision;
    key(&mut c, " ");
    assert_eq!(c.status, "citizens-set");
    assert_eq!(c.view().unwrap().city_citizens[0].specialists, vec![1]);
    let edited = c.checkpoint().unwrap();
    assert!(c
        .input(stale, 999, Input::Click { x: 500., y: 300. })
        .is_err());
    assert_eq!(edited, c.checkpoint().unwrap());
    button(&mut c, "1: Entertainer");
    assert!(c.status.contains("population five"));
    assert_eq!(edited, c.checkpoint().unwrap());
    button(&mut c, "Choose production");
    key(&mut c, "ArrowDown");
    key(&mut c, "Escape");
    assert_eq!(
        edited,
        c.checkpoint().unwrap(),
        "production cancel is inert"
    );
    key(&mut c, "Escape");
    assert!(!c.city_open);
    assert_eq!(
        edited,
        c.checkpoint().unwrap(),
        "closing keeps immediate edits"
    );
    key(&mut c, "Ctrl+s");
    edit(&mut c, "citizens");
    key(&mut c, "Enter");
    button(&mut c, "Manage city");
    button(&mut c, "Auto workers");
    assert_ne!(edited, c.checkpoint().unwrap());
    key(&mut c, "Ctrl+l");
    edit(&mut c, "citizens");
    key(&mut c, "Enter");
    assert_eq!(edited, c.checkpoint().unwrap());
    c.city_tile = k;
    key(&mut c, " ");
    assert!(c.view().unwrap().city_citizens[0].specialists.is_empty());
    assert!(c.view().unwrap().city_citizens[0].radius[k as usize].worked);
}

#[test]
#[ignore = "install-backed genuine generated population-five city fixture"]
fn generated_population_five_city_fixture() {
    let home = std::env::var("CIVTOT_HOME").expect("set CIVTOT_HOME");
    let out = PathBuf::from(std::env::var("TOTENGINE_CITIZENS_OUT").expect("set private output"));
    fs::create_dir_all(&out).unwrap();
    let mut c = Client::new(home, out.clone());
    for _ in 0..3 {
        button(&mut c, "Civilizations");
    }
    for _ in 0..4 {
        button(&mut c, "Difficulty");
    }
    start(&mut c);
    founding(&mut c);
    button(&mut c, "Choose production");
    let id = c
        .current_choices()
        .iter()
        .position(|(id, _)| {
            *id >= 0
                && c.loaded
                    .as_ref()
                    .unwrap()
                    .resources
                    .rules
                    .unit_role(*id as usize)
                    != Some(5)
        })
        .unwrap();
    for _ in 0..id / 12 {
        button(&mut c, "Next page");
    }
    input(
        &mut c,
        Input::Click {
            x: 220.,
            y: 177. + (id % 12) as f64 * 29.,
        },
    );
    key(&mut c, "Enter");
    for _ in 0..250 {
        research(&mut c);
        let v = c.view().unwrap();
        if v.observation
            .cities
            .iter()
            .any(|city| city.owner == v.observation.civ && city.size >= 5)
        {
            fs::write(out.join("mature.cth"), c.checkpoint().unwrap()).unwrap();
            let inputs = c.receipts.values().map(|(_, i)| i).collect::<Vec<_>>();
            fs::write(
                out.join("inputs.json"),
                serde_json::to_string(&inputs).unwrap(),
            )
            .unwrap();
            println!(
                "Generated mature city at turn {}: {:?}",
                v.observation.turn,
                v.observation
                    .cities
                    .iter()
                    .map(|c| (&c.name, c.size))
                    .collect::<Vec<_>>()
            );
            let tile = v.city_citizens[0]
                .radius
                .iter()
                .take(20)
                .position(|t| t.worked)
                .unwrap() as u8;
            button(&mut c, "Manage city");
            for _ in 0..21 {
                if c.city_tile == tile {
                    break;
                }
                key(&mut c, "ArrowRight");
            }
            key(&mut c, " ");
            assert_eq!(c.view().unwrap().city_citizens[0].specialists, vec![1]);
            fs::write(out.join("after-entertainer.cth"), c.checkpoint().unwrap()).unwrap();
            button(&mut c, "1: Entertainer");
            assert_eq!(c.view().unwrap().city_citizens[0].specialists, vec![2]);
            fs::write(out.join("after-tax.cth"), c.checkpoint().unwrap()).unwrap();
            button(&mut c, "1: Tax collector");
            assert_eq!(c.view().unwrap().city_citizens[0].specialists, vec![3]);
            fs::write(out.join("after-science.cth"), c.checkpoint().unwrap()).unwrap();
            button(&mut c, "1: Scientist");
            let edited = c.checkpoint().unwrap();
            fs::write(out.join("after-cycle.cth"), &edited).unwrap();
            let save_name = format!("edited{}", storage::nonce().unwrap());
            key(&mut c, "Ctrl+s");
            edit(&mut c, &save_name);
            key(&mut c, "Enter");
            button(&mut c, "Auto workers");
            key(&mut c, "Ctrl+l");
            edit(&mut c, &save_name);
            key(&mut c, "Enter");
            assert_eq!(edited, c.checkpoint().unwrap());
            c.city_tile = tile;
            key(&mut c, " ");
            fs::write(out.join("after-resume.cth"), c.checkpoint().unwrap()).unwrap();
            assert!(c.view().unwrap().city_citizens[0].specialists.is_empty());
            fs::write(out.join("route.json"),serde_json::to_string_pretty(&serde_json::json!({"worker_tile":tile,"city_slot":v.cities[0].slot,"mature_turn":v.observation.turn,"population":5})).unwrap()).unwrap();
            return;
        }
        assert_eq!(v.phase, Phase::Human, "{}", c.status);
        key(&mut c, "Ctrl+n");
        if matches!(c.modal, Some(Modal::Confirm { .. })) {
            key(&mut c, "Enter");
        }
        ready(&mut c);
    }
    panic!(
        "generated city did not reach population five: {:?}",
        c.view().unwrap().observation.city_economy
    );
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
    assert!(c.moving);
    key(&mut c, "v");
    key(&mut c, "6");
    assert_eq!(c.checkpoint().unwrap(), before);
    key(&mut c, "v");
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
    key(&mut c, "Ctrl+n");
    if matches!(c.modal, Some(Modal::Confirm { .. })) {
        key(&mut c, "Enter");
    }
    assert_eq!(c.view().unwrap().phase, Phase::Opponents);
    let l = c.loaded.as_ref().unwrap();
    let text = l.session.checkpoint(&l.identity);
    storage::save(&c.saves, "partial", &text).unwrap();
    key(&mut c, "Ctrl+l");
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
    #[derive(Deserialize)]
    struct RecordedInput {
        input: Input,
        actor: Option<(usize, u32)>,
    }
    let inputs: Vec<RecordedInput> = serde_json::from_str(include_str!(
        "../../../../docs/evidence/client-input-route-v3.json"
    ))
    .unwrap();
    let mut c = Client::new(home, out.clone());
    let mut battle = false;
    let mut capture = false;
    let mut direction = String::new();
    for record in inputs {
        let mut i = record.input;
        if let Some((slot, id)) = record.actor {
            let actor = Entity { slot, id };
            assert!(
                c.view().unwrap().units.contains(&actor),
                "recorded actor must still exist"
            );
            for _ in 0..c.view().unwrap().units.len() + 1 {
                if c.active == Some(actor) {
                    break;
                }
                key(&mut c, "w");
            }
            assert_eq!(c.active, Some(actor));
            if !c.moving {
                key(&mut c, "v");
            }
        }
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
    let v = c.view().unwrap();
    println!(
        "Own cities: {:?}",
        v.observation
            .cities
            .iter()
            .filter(|city| city.owner == v.observation.civ)
            .map(|city| (&city.name, city.size))
            .collect::<Vec<_>>()
    );
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
        "../../../../docs/evidence/client-atlas-v2.json"
    ))
    .unwrap();
    // Different owned installs remain supported; this evidence binds the reference install.
    if measured["resource_identity"] == expected["resource_identity"] {
        assert_eq!(measured, expected);
    }
    assert_eq!(
        atlas.sprites.len(),
        65 + resources.rules.unit_names().len().min(80)
    );
}

#[test]
fn production_draft_budget_and_city_lifecycle_resume_without_double_penalty() {
    let f = Fixture::new();
    let file = f.0.join("Rules.txt");
    let mut rules = tot_kit::parse_rules(&fs::read_to_string(&file).unwrap());
    let cosmic = rules.sections.get_mut("COSMIC").unwrap();
    cosmic.rows[18] = vec!["50".into()];
    cosmic.raw_lines[18] = "50 ; authored switching penalty".into();
    fs::write(&file, tot_kit::emit_rules(&rules)).unwrap();
    let mut c = f.client();
    start(&mut c);
    founding(&mut c);
    for _ in 0..20 {
        if c.view().unwrap().city_production[0].stored >= 10 {
            break;
        }
        end_round(&mut c);
    }
    let initial = c.view().unwrap().city_production[0].stored;
    assert!(initial >= 10);
    assert_eq!(c.view().unwrap().production_context_city, None);
    button(&mut c, "Manage city");
    let before = c.checkpoint().unwrap();
    fn select(c: &mut Client, item: i8) {
        button(c, "Choose production");
        let at = c
            .current_choices()
            .iter()
            .position(|(id, _)| *id == i32::from(item))
            .unwrap();
        for _ in 0..=at {
            key(c, "ArrowDown");
        }
    }
    select(&mut c, -4);
    let retained = initial - initial / 2;
    assert!(c.frame().draws.iter().any(|d| matches!(d,Draw::Text{text,..} if text.contains(&format!("Shields: {initial} → {retained}")))));
    assert_eq!(
        before,
        c.checkpoint().unwrap(),
        "draft selection only reads its quote"
    );
    key(&mut c, "Escape");
    assert_eq!(before, c.checkpoint().unwrap());
    select(&mut c, -4);
    key(&mut c, "Enter");
    assert_eq!(c.view().unwrap().city_production[0].stored, retained);
    let active = c.checkpoint().unwrap();
    key(&mut c, "Ctrl+s");
    edit(&mut c, "budget-active");
    key(&mut c, "Enter");
    select(&mut c, 0);
    assert!(c
        .frame()
        .draws
        .iter()
        .any(|d| matches!(d,Draw::Text{text,..} if text.contains("restored from city budget"))));
    key(&mut c, "Escape");
    assert_eq!(active, c.checkpoint().unwrap());
    key(&mut c, "Escape");
    assert_eq!(c.view().unwrap().production_context_city, None);
    button(&mut c, "Manage city");
    select(&mut c, 0);
    let new_retained = retained - retained / 2;
    assert!(c.frame().draws.iter().any(|d| matches!(d,Draw::Text{text,..} if text.contains(&format!("Shields: {retained} → {new_retained}")))));
    key(&mut c, "Escape");
    key(&mut c, "Ctrl+l");
    edit(&mut c, "budget-active");
    key(&mut c, "Enter");
    assert!(
        c.city_open,
        "active production context restores its city workspace"
    );
    assert_eq!(active, c.checkpoint().unwrap());
    select(&mut c, 0);
    key(&mut c, "Enter");
    assert_eq!(c.view().unwrap().city_production[0].stored, initial);
}

#[test]
fn quoted_city_purchase_and_sale_confirm_cancel_and_resume() {
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    founding(&mut c);
    button(&mut c, "Manage city");
    button(&mut c, "Choose production");
    let at = c
        .current_choices()
        .iter()
        .position(|(id, _)| *id == -4)
        .unwrap();
    for _ in 0..=at {
        key(&mut c, "ArrowDown");
    }
    key(&mut c, "Enter");
    let original = c.checkpoint().unwrap();
    button(&mut c, "Buy production");
    assert!(matches!(c.modal, Some(Modal::Purchase { .. })));
    assert!(
        !c.buttons()
            .iter()
            .any(|b| matches!(b.action, Action::Accept)),
        "poor purchase has no confirm action"
    );
    key(&mut c, "Escape");
    assert_eq!(original, c.checkpoint().unwrap());
    let mut expected = None;
    for _ in 0..80 {
        let v = c.view().unwrap();
        assert!(
            v.observation.city_economy[0].buildings[0] & (1 << 4) == 0,
            "buy before natural completion"
        );
        if let Ok(q) = v.city_production[0].hurry {
            if q.price <= q.treasury {
                expected = Some(q);
                break;
            }
        }
        key(&mut c, "Escape");
        end_round(&mut c);
        button(&mut c, "Manage city");
    }
    let quote = expected.expect("recorded play must fund the quote");
    let before = c.checkpoint().unwrap();
    button(&mut c, "Buy production");
    key(&mut c, "Escape");
    assert_eq!(before, c.checkpoint().unwrap(), "cancel is inert");
    button(&mut c, "Buy production");
    let old = c.revision;
    key(&mut c, "Enter");
    let after = c.checkpoint().unwrap();
    assert_ne!(after, before);
    assert_eq!(c.view().unwrap().city_production[0].stored, quote.required);
    assert_eq!(
        c.view().unwrap().observation.treasury,
        quote.treasury - quote.price
    );
    assert!(c
        .input(
            old,
            99999,
            Input::Key {
                key: "Enter".into()
            }
        )
        .is_err());
    assert_eq!(after, c.checkpoint().unwrap());
    key(&mut c, "Ctrl+s");
    edit(&mut c, "bought");
    key(&mut c, "Enter");
    key(&mut c, "Escape");
    end_round(&mut c);
    button(&mut c, "Manage city");
    assert!(c.view().unwrap().observation.city_economy[0].buildings[0] & (1 << 4) != 0);
    fn open_sale(c: &mut Client) {
        button(c, "Improvements / sell");
        let at = c
            .current_choices()
            .iter()
            .position(|(id, _)| *id == 4)
            .unwrap();
        for _ in 0..=at {
            key(c, "ArrowDown");
        }
        key(c, "Enter");
        assert!(matches!(c.modal, Some(Modal::Sale { .. })));
    }
    let before = c.checkpoint().unwrap();
    open_sale(&mut c);
    assert_eq!(before, c.checkpoint().unwrap(), "preview reads a clone");
    assert!(c
        .frame()
        .draws
        .iter()
        .any(|d| matches!(d,Draw::Text{text,..} if text.starts_with("Happy:"))));
    key(&mut c, "Escape");
    assert_eq!(before, c.checkpoint().unwrap());
    button(&mut c, "Improvements");
    let sell = c
        .buttons()
        .into_iter()
        .find(|b| matches!(b.action, Action::InspectSale(_, 4)))
        .expect("panel sale row");
    input(
        &mut c,
        Input::Click {
            x: sell.x + 3.,
            y: sell.y + 3.,
        },
    );
    assert!(matches!(c.modal, Some(Modal::Sale { .. })));
    key(&mut c, "Escape");
    assert_eq!(
        before,
        c.checkpoint().unwrap(),
        "panel sale inspection/cancel remains inert"
    );
    open_sale(&mut c);
    key(&mut c, "Enter");
    assert!(c.view().unwrap().observation.city_economy[0].buildings[0] & (1 << 4) == 0);
    key(&mut c, "Ctrl+l");
    edit(&mut c, "bought");
    key(&mut c, "Enter");
    assert_eq!(
        after,
        c.checkpoint().unwrap(),
        "reload retains purchased shields before creation"
    );
}

#[test]
fn original_bindings_activation_mouse_stacking_and_blink_are_inert_or_validated() {
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    let before = c.checkpoint().unwrap();
    for reserved in ["f", "r", "m", "o", "e", "s", "l"] {
        key(&mut c, reserved);
        assert!(
            c.modal.is_none(),
            "{reserved} must not open an unrelated action"
        );
        assert_eq!(c.checkpoint().unwrap(), before);
    }
    let v = c.view().unwrap();
    let entity = c.active.unwrap();
    let u = v
        .observation
        .units
        .iter()
        .find(|u| u.index == entity.slot)
        .unwrap();
    let origin = (u.x, u.y);
    // Click the visible upper half of the unit, above its ground diamond.
    input(&mut c, Input::Click { x: 380., y: 350. });
    assert_eq!(c.inspect, Some(origin));
    button(&mut c, &format!("Activate unit {}", entity.slot));
    assert!(
        c.moving,
        "the owner's Activate button route must enter Moving"
    );
    assert_eq!(c.active, Some(entity));
    assert_eq!(c.checkpoint().unwrap(), before);
    let frame = serde_json::to_string(&c.frame()).unwrap();
    assert!(frame.contains("\"kind\":\"unit_sprite\""));
    assert!(frame.contains("\"blink\":true"));
    assert_eq!(frame, serde_json::to_string(&c.frame()).unwrap());
    assert_eq!(
        c.checkpoint().unwrap(),
        before,
        "rendering never advances game state"
    );
    key(&mut c, "v");
    assert!(!c.moving);
    let cursor = |c: &Client| {
        c.frame()
            .draws
            .iter()
            .find_map(|d| match d {
                Draw::Cursor { x, y, .. } => Some((*x, *y)),
                _ => None,
            })
            .unwrap()
    };
    let before_cursor = cursor(&c);
    key(&mut c, "6");
    assert_eq!(
        cursor(&c),
        (before_cursor.0 + 64. * c.zoom, before_cursor.1)
    );
    assert_eq!(c.inspect, Some((origin.0 + 2, origin.1)));
    assert_eq!(
        c.checkpoint().unwrap(),
        before,
        "Viewing keypad only moves its cursor"
    );
    // A activates the inspected stack without requiring a mode toggle.
    c.inspect = Some(origin);
    key(&mut c, "a");
    if matches!(c.modal, Some(Modal::Units { .. })) {
        key(&mut c, "ArrowDown");
        key(&mut c, "Enter");
    }
    assert!(c.moving);
    assert!(c.modal.is_none());
    key(&mut c, "c");
    let v = c.view().unwrap();
    let selected = c.active.unwrap();
    let u = v
        .observation
        .units
        .iter()
        .find(|u| u.index == selected.slot)
        .unwrap();
    let (dx, dy) = [
        (2, 0),
        (-2, 0),
        (0, -2),
        (0, 2),
        (1, -1),
        (-1, -1),
        (1, 1),
        (-1, 1),
    ]
    .into_iter()
    .find(|(dx, dy)| {
        let (x, y) = (i32::from(u.x) + dx, i32::from(u.y) + dy);
        x >= 0
            && y >= 0
            && x < i32::from(v.observation.w2)
            && y < i32::from(v.observation.h)
            && v.observation.seen[y as usize * (v.observation.w2 as usize / 2) + x as usize / 2]
                .is_some_and(|t| t.terrain != 10)
    })
    .unwrap();
    input(
        &mut c,
        Input::Click {
            x: 380. + f64::from(dx) * 32.,
            y: 390. + f64::from(dy) * 16.,
        },
    );
    assert_eq!(c.status, "moved");
    let u = c
        .view()
        .unwrap()
        .observation
        .units
        .into_iter()
        .find(|u| u.index == selected.slot)
        .unwrap();
    assert_eq!(
        (u.x, u.y),
        (
            (i32::from(origin.0) + dx) as u16,
            (i32::from(origin.1) + dy) as u16
        )
    );
    assert_ne!(c.checkpoint().unwrap(), before);
}

#[test]
#[ignore = "install-backed mouse movement fixture from the generated warrior encounter"]
fn generated_warrior_mouse_fixture() {
    let home = std::env::var("CIVTOT_HOME").unwrap();
    let before = fs::read_to_string(std::env::var("TOTENGINE_MOVEMENT_SOURCE").unwrap()).unwrap();
    let out = PathBuf::from(std::env::var("TOTENGINE_MOVEMENT_OUT").unwrap());
    fs::create_dir_all(&out).unwrap();
    let mut c = Client::new(home, out.clone());
    storage::save(&out, "source", &before).unwrap();
    button(&mut c, "Load engine checkpoint");
    edit(&mut c, "source");
    key(&mut c, "Enter");
    // Warrior #1 is reached through ordinary selection, never a world mutation.
    for _ in 0..100 {
        if c.active.is_some_and(|e| e.slot == 1) {
            break;
        }
        key(&mut c, "w");
    }
    let v = c.view().unwrap();
    let actor = c.active.unwrap();
    assert_eq!(actor.slot, 1);
    let u = v
        .observation
        .units
        .iter()
        .find(|u| u.index == actor.slot)
        .unwrap();
    let (x, y) = (u.x, u.y);
    assert!(u.moves_left > 0);
    let (dx, dy) = [
        (2, 0),
        (-2, 0),
        (0, -2),
        (0, 2),
        (1, -1),
        (-1, -1),
        (1, 1),
        (-1, 1),
    ]
    .into_iter()
    .find(|(dx, dy)| {
        let (tx, ty) = (i32::from(x) + dx, i32::from(y) + dy);
        tx >= 0
            && ty >= 0
            && tx < i32::from(v.observation.w2)
            && ty < i32::from(v.observation.h)
            && v.observation.seen[ty as usize * (v.observation.w2 as usize / 2) + tx as usize / 2]
                .is_some_and(|t| t.terrain != 10)
            && !v
                .observation
                .units
                .iter()
                .any(|u| i32::from(u.x) == tx && i32::from(u.y) == ty)
            && !v
                .observation
                .cities
                .iter()
                .any(|u| i32::from(u.x) == tx && i32::from(u.y) == ty)
    })
    .unwrap();
    fs::write(out.join("before.cth"), c.checkpoint().unwrap()).unwrap();
    c.moving = false;
    c.center();
    input(&mut c, Input::Click { x: 380., y: 390. });
    button(&mut c, &format!("Activate unit {}", actor.slot));
    assert!(c.moving);
    input(
        &mut c,
        Input::Click {
            x: 380. + f64::from(dx) * 32.,
            y: 390. + f64::from(dy) * 16.,
        },
    );
    assert_eq!(c.status, "moved");
    fs::write(out.join("after.cth"), c.checkpoint().unwrap()).unwrap();
    fs::write(
        out.join("route.json"),
        serde_json::to_string_pretty(
            &serde_json::json!({"actor":actor.slot,"x":x,"y":y,"dx":dx,"dy":dy}),
        )
        .unwrap(),
    )
    .unwrap();
}

fn end_round(c: &mut Client) {
    research(c);
    key(c, "Ctrl+n");
    if matches!(c.modal, Some(Modal::Confirm { .. })) {
        key(c, "Enter");
    }
    ready(c);
    research(c);
}
fn build_pair(c: &mut Client) {
    for _ in 0..3 {
        button(c, "Civilizations");
    }
    for _ in 0..4 {
        button(c, "Difficulty");
    }
    start(c);
    founding(c);
    button(c, "Choose production");
    let item = c
        .current_choices()
        .iter()
        .find(|(id, _)| {
            *id >= 0
                && c.loaded
                    .as_ref()
                    .unwrap()
                    .resources
                    .rules
                    .unit_role(*id as usize)
                    != Some(5)
        })
        .unwrap()
        .0;
    let index = c
        .current_choices()
        .iter()
        .position(|(id, _)| *id == item)
        .unwrap();
    for _ in 0..index / 12 {
        button(c, "Next page");
    }
    click_point(c, (220., 177. + (index % 12) as f64 * 29.));
    key(c, "Enter");
    for _ in 0..100 {
        if c.view().unwrap().units.len() >= 2 {
            return;
        }
        end_round(c);
    }
    panic!("no pair built: {}", c.status);
}
fn unit_at(c: &Client, e: Entity) -> engine::player::UnitView {
    c.view()
        .unwrap()
        .observation
        .units
        .into_iter()
        .find(|u| u.index == e.slot)
        .unwrap()
}
fn point(c: &Client, e: Entity, sy: f64) -> (f64, f64) {
    let u = unit_at(c, e);
    (
        c.pan.0 + f64::from(u.x) * 32. * c.zoom,
        c.pan.1 + f64::from(u.y) * 16. * c.zoom + sy * c.zoom,
    )
}
fn click_point(c: &mut Client, p: (f64, f64)) {
    input(c, Input::Click { x: p.0, y: p.1 });
}
fn land_step(c: &Client, e: Entity) -> (i32, i32) {
    let v = c.view().unwrap();
    let u = unit_at(c, e);
    [
        (2, 0),
        (-2, 0),
        (0, -2),
        (0, 2),
        (1, -1),
        (-1, -1),
        (1, 1),
        (-1, 1),
    ]
    .into_iter()
    .find(|(dx, dy)| {
        let (x, y) = (i32::from(u.x) + dx, i32::from(u.y) + dy);
        x >= 0
            && y >= 0
            && x < i32::from(v.observation.w2)
            && y < i32::from(v.observation.h)
            && v.observation.seen[y as usize * (v.observation.w2 as usize / 2) + x as usize / 2]
                .is_some_and(|t| t.terrain != 10)
            && !v
                .observation
                .units
                .iter()
                .any(|u| i32::from(u.x) == x && i32::from(u.y) == y)
            && !v
                .observation
                .cities
                .iter()
                .any(|u| i32::from(u.x) == x && i32::from(u.y) == y)
    })
    .unwrap()
}

#[test]
fn mouse_switches_different_units_and_stack_selection_never_spends_moves() {
    let f = Fixture::new();
    let mut c = f.client();
    build_pair(&mut c);
    let units = c.view().unwrap().units;
    let a = units[0];
    let b = units[1];
    let before = c.checkpoint().unwrap();
    let p = point(&c, a, -20.);
    click_point(&mut c, p);
    assert!(
        matches!(c.modal, Some(Modal::Units { .. })),
        "a stack needs explicit selection"
    );
    c.choice = Some(b.slot as i32);
    key(&mut c, "Enter");
    assert_eq!(c.active, Some(b));
    assert!(c.moving);
    assert_eq!(
        c.checkpoint().unwrap(),
        before,
        "stack selection is presentation only"
    );
    let (dx, dy) = land_step(&c, b);
    c.step(dx, dy).unwrap();
    end_round(&mut c);
    c.action(Action::Unit(a)).unwrap();
    let baseline = c.checkpoint().unwrap();
    for viewing in [false, true] {
        for zoom in [1., 1.25, 0.8] {
            c.zoom = zoom;
            c.action(Action::Unit(a)).unwrap();
            if viewing {
                key(&mut c, "v");
            }
            let p = point(&c, b, -20.);
            click_point(&mut c, p);
            assert_eq!(c.active, Some(b), "click another unit in either mode");
            assert!(c.moving);
            assert_eq!(
                c.checkpoint().unwrap(),
                baseline,
                "click did not move the previous actor"
            );
            assert!(c.frame().draws.iter().any(
                |d| matches!(d,Draw::UnitSprite{unit,selected:true,blink:true,..} if *unit==b.slot)
            ));
            assert!(
                !c.frame()
                    .draws
                    .iter()
                    .any(|d| matches!(d, Draw::Cursor { .. })),
                "Moving blinks the sprite, not a tile"
            );
        }
    }
    // Snapshot identities, not just slots, survive across a stack chooser.
    c.action(Action::Unit(a)).unwrap();
    let p = point(&c, a, -20.);
    click_point(&mut c, p);
    assert_eq!(c.active, Some(a));
    assert_eq!(c.checkpoint().unwrap(), baseline);
}

#[test]
fn exhaustion_selects_next_then_end_turn_without_retargeting_queued_input() {
    let f = Fixture::new();
    let mut c = f.client();
    build_pair(&mut c);
    let units = c.view().unwrap().units;
    let a = units[0];
    let b = units[1];
    c.action(Action::Unit(a)).unwrap();
    let turn = c.view().unwrap().observation.turn;
    let revision = c.revision;
    key(&mut c, " ");
    assert_eq!(c.active, Some(b));
    assert!(c.moving);
    assert_eq!(unit_at(&c, a).moves_left, 0);
    let checkpoint = c.checkpoint().unwrap();
    assert!(c
        .input(revision, 99999, Input::Key { key: "6".into() })
        .is_err());
    assert_eq!(
        c.checkpoint().unwrap(),
        checkpoint,
        "queued move must not act on new selection"
    );
    c.action(Action::Unit(a)).unwrap();
    assert!(!c.moving);
    assert!(c.status.contains("no moves"));
    assert!(!c
        .frame()
        .draws
        .iter()
        .any(|d| matches!(d, Draw::UnitSprite { blink: true, .. })));
    key(&mut c, "v");
    assert_eq!(c.active, Some(b));
    assert!(c.moving);
    key(&mut c, " ");
    assert!(!c.moving);
    assert!(c
        .frame()
        .draws
        .iter()
        .any(|d| matches!(d,Draw::Text{text,..} if text=="End of turn")));
    input(&mut c, Input::Tick);
    assert_eq!(c.view().unwrap().observation.turn, turn);
    assert!(matches!(c.buttons()[c.focus_button].action, Action::End));
    key(&mut c, "Tab");
    assert!(!matches!(c.buttons()[c.focus_button].action, Action::End));
    key(&mut c, "ShiftTab");
    assert!(matches!(c.buttons()[c.focus_button].action, Action::End));
    // Returning from city inspection restores the visible end-turn default.
    button(&mut c, "Manage city");
    key(&mut c, "Escape");
    assert!(matches!(c.buttons()[c.focus_button].action, Action::End));
    let old_revision = c.revision;
    key(&mut c, "Enter");
    let ended = c.checkpoint().unwrap();
    assert!(c
        .input(
            old_revision,
            99998,
            Input::Key {
                key: "Enter".into()
            }
        )
        .is_err());
    assert_eq!(ended, c.checkpoint().unwrap());
    ready(&mut c);
    research(&mut c);
    assert_eq!(c.view().unwrap().observation.turn, turn + 1);
    assert!(c.moving);
    assert!(unit_at(&c, c.active.unwrap()).moves_left > 0);
}

#[test]
#[ignore = "install-backed normal-play two-unit selection fixture"]
fn generated_selection_fixture() {
    let home = std::env::var("CIVTOT_HOME").unwrap();
    let out = PathBuf::from(std::env::var("TOTENGINE_SELECTION_OUT").unwrap());
    fs::create_dir_all(&out).unwrap();
    let mut c = Client::new(home, out.clone());
    build_pair(&mut c);
    let units = c.view().unwrap().units;
    assert_eq!(units.len(), 2);
    let a = units[0];
    let b = units[1];
    fs::write(out.join("stack.cth"), c.checkpoint().unwrap()).unwrap();
    let p = point(&c, a, 0.);
    click_point(&mut c, p);
    assert!(matches!(c.modal, Some(Modal::Units { .. })));
    button(&mut c, &format!("#{} ·", b.slot));
    key(&mut c, "Enter");
    let (dx, dy) = land_step(&c, b);
    c.step(dx, dy).unwrap();
    end_round(&mut c);
    assert_eq!(
        c.view().unwrap().units.len(),
        2,
        "fixture needs exactly two own units"
    );
    assert!(unit_at(&c, a).moves_left > 0 && unit_at(&c, b).moves_left > 0);
    c.action(Action::Unit(a)).unwrap();
    let before = c.checkpoint().unwrap();
    fs::write(out.join("before.cth"), &before).unwrap();
    let source = unit_at(&c, a);
    let target = unit_at(&c, b);
    let sprite = c
        .atlas()
        .unwrap()
        .sprites
        .get(&format!("unit-{}", target.type_id))
        .unwrap();
    let opaque: Vec<_> = (0..sprite.h)
        .filter_map(|row| {
            let cols: Vec<_> = (0..sprite.w)
                .filter(|&col| sprite.rgba[((row * sprite.w + col) * 4 + 3) as usize] == 255)
                .collect();
            cols.get(cols.len() / 2).copied().map(|col| (col, row))
        })
        .collect();
    let hit_points:Vec<_>=[0,opaque.len()/2,opaque.len()-1].into_iter().map(|k|{
        let(col,row)=opaque[k];let at=((row*sprite.w+col)*4) as usize;
        serde_json::json!({"x":(f64::from(col)+0.5)/f64::from(sprite.w),"y":(f64::from(row)+0.5)/f64::from(sprite.h),"rgb":format!("#{:02x}{:02x}{:02x}",sprite.rgba[at],sprite.rgba[at+1],sprite.rgba[at+2])})
    }).collect();
    let delta = land_step(&c, b);
    // Directly click a DIFFERENT own unit while already Moving. No W or
    // Activate button preselects it; selection must leave the checkpoint intact.
    let (px, py) = point(&c, b, 0.);
    let hx = hit_points[1]["x"].as_f64().unwrap();
    let hy = hit_points[1]["y"].as_f64().unwrap();
    click_point(&mut c, (px - 32. + hx * 64., py - 48. + hy * 64.));
    assert_eq!(c.active, Some(b));
    assert!(c.moving);
    assert_eq!(c.checkpoint().unwrap(), before);
    c.step(delta.0, delta.1).unwrap();
    assert_eq!(c.status, "moved");
    assert_eq!(unit_at(&c, b).moves_left, 0, "choose a one-turn empty move");
    assert_eq!(c.active, Some(a));
    assert!(c.moving);
    fs::write(out.join("after-move.cth"), c.checkpoint().unwrap()).unwrap();
    key(&mut c, " ");
    assert!(!c.moving);
    fs::write(out.join("after-skip.cth"), c.checkpoint().unwrap()).unwrap();
    end_round(&mut c);
    assert!(c.moving);
    fs::write(out.join("after-turn.cth"), c.checkpoint().unwrap()).unwrap();
    fs::write(
        out.join("route.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "source":{"slot":a.slot,"id":a.id,"x":source.x,"y":source.y,"moves":source.moves_left},
            "target":{"slot":b.slot,"id":b.id,"x":target.x,"y":target.y,"moves":target.moves_left},
            "hit_points":hit_points,"move_delta":delta,"turn":c.view().unwrap().observation.turn-1
        }))
        .unwrap(),
    )
    .unwrap();
    println!("Two-unit selection fixture: {}", out.display());
}

#[test]
#[ignore = "install-backed recorded city production input route"]
fn generated_city_production_fixture() {
    let home = std::env::var("CIVTOT_HOME").expect("set CIVTOT_HOME");
    let out = PathBuf::from(std::env::var("TOTENGINE_PRODUCTION_OUT").expect("set private output"));
    fs::create_dir_all(&out).unwrap();
    let mut c = Client::new(home, out.clone());
    for _ in 0..3 {
        button(&mut c, "Civilizations");
    }
    for _ in 0..2 {
        button(&mut c, "Difficulty");
    }
    start(&mut c);
    founding(&mut c);
    fn choose(c: &mut Client, item: i8) {
        button(c, "Choose production");
        let at = c
            .current_choices()
            .iter()
            .position(|(id, _)| *id == i32::from(item))
            .expect("legal target");
        for _ in 0..=at {
            key(c, "ArrowDown");
        }
        key(c, "Enter");
    }
    button(&mut c, "Choose production");
    let target = c
        .current_choices()
        .iter()
        .find_map(|(id, _)| (-35..=-2).contains(id).then_some(*id as i8))
        .expect("initial saleable improvement");
    key(&mut c, "Escape");
    choose(&mut c, target);
    let building = (-target) as usize;
    let mut routes = Vec::new();
    // Each scenario starts from a genuine recorded checkpoint. Route inputs
    // are also executed here; actual adapters must reproduce every byte.
    fn scenario(
        c: &mut Client,
        out: &Path,
        routes: &mut Vec<serde_json::Value>,
        name: &str,
        actions: &[(&str, &str)],
    ) {
        fs::write(
            out.join(format!("{name}-before.cth")),
            c.checkpoint().unwrap(),
        )
        .unwrap();
        let mut inputs = Vec::new();
        for (kind, value) in actions {
            if *kind == "button" {
                let b = c
                    .buttons()
                    .into_iter()
                    .find(|b| b.label == *value)
                    .unwrap_or_else(|| panic!("missing {value}"));
                let i = Input::Click {
                    x: b.x + 12.,
                    y: b.y + 8.,
                };
                input(c, i.clone());
                inputs.push(i);
            } else {
                let i = Input::Key {
                    key: (*value).into(),
                };
                input(c, i.clone());
                inputs.push(i);
            }
        }
        fs::write(
            out.join(format!("{name}-after.cth")),
            c.checkpoint().unwrap(),
        )
        .unwrap();
        routes.push(serde_json::json!({"name":name,"inputs":inputs}));
    }
    // Keep starting state outside city: load restores only recorded context.
    assert!(!c.city_open);
    assert!(
        c.view().unwrap().city_production[0]
            .hurry
            .as_ref()
            .unwrap()
            .price
            > c.view().unwrap().observation.treasury
    );
    scenario(
        &mut c,
        &out,
        &mut routes,
        "poor",
        &[
            ("button", "Manage city"),
            ("button", "Buy production"),
            ("key", "Enter"),
            ("key", "Escape"),
        ],
    );
    // Poor Enter cancels; Escape leaves the city.
    assert!(!c.city_open);
    for _ in 0..150 {
        let v = c.view().unwrap();
        if let Ok(q) = &v.city_production[0].hurry {
            if q.price > 0 && q.price <= q.treasury {
                break;
            }
        }
        assert_eq!(
            v.observation.city_economy[0].buildings[building / 8] & (1 << (building % 8)),
            0,
            "buy before completion"
        );
        end_round(&mut c);
    }
    let q = c.view().unwrap().city_production[0].hurry.unwrap();
    assert!(
        q.price > 0 && q.price <= q.treasury,
        "normal play funds purchase"
    );
    scenario(
        &mut c,
        &out,
        &mut routes,
        "buycancel",
        &[
            ("button", "Manage city"),
            ("button", "Buy production"),
            ("key", "Escape"),
            ("key", "Escape"),
        ],
    );
    scenario(
        &mut c,
        &out,
        &mut routes,
        "buy",
        &[
            ("button", "Manage city"),
            ("button", "Buy production"),
            ("key", "Enter"),
        ],
    );
    assert_eq!(c.view().unwrap().city_production[0].stored, q.required);
    // Save/load of buy-after restores its active city context. Return to map
    // before advancing to completion, then isolate the sale route.
    key(&mut c, "Escape");
    end_round(&mut c);
    assert_ne!(
        c.view().unwrap().observation.city_economy[0].buildings[building / 8]
            & (1 << (building % 8)),
        0
    );
    button(&mut c, "Manage city");
    button(&mut c, "Improvements / sell");
    let at = c
        .current_choices()
        .iter()
        .position(|(id, _)| *id == i32::from(-target))
        .unwrap();
    key(&mut c, "Escape");
    key(&mut c, "Escape");
    // Cancel and acceptance share the same actual picker path.
    let mut sale = vec![("button", "Manage city"), ("button", "Improvements / sell")];
    for _ in 0..=at {
        sale.push(("key", "ArrowDown"));
    }
    sale.push(("key", "Enter"));
    sale.push(("key", "Escape"));
    sale.push(("key", "Escape"));
    scenario(&mut c, &out, &mut routes, "salecancel", &sale);
    let confirm = sale.len() - 2;
    sale[confirm] = ("key", "Enter");
    sale.pop();
    scenario(&mut c, &out, &mut routes, "sale", &sale);
    assert_eq!(
        c.view().unwrap().observation.city_economy[0].buildings[building / 8]
            & (1 << (building % 8)),
        0
    );
    key(&mut c, "Escape");
    choose(&mut c, 0);
    for _ in 0..20 {
        if c.view().unwrap().city_production[0].stored >= 3 {
            break;
        }
        end_round(&mut c);
    }
    let initial = c.view().unwrap().city_production[0].stored;
    assert!(initial >= 3);
    fn picker(c: &mut Client, item: i8) -> Vec<(&'static str, &'static str)> {
        button(c, "Choose production");
        let at = c
            .current_choices()
            .iter()
            .position(|(id, _)| *id == i32::from(item))
            .unwrap();
        key(c, "Escape");
        let mut a = vec![("button", "Choose production")];
        for _ in 0..=at {
            a.push(("key", "ArrowDown"));
        }
        a.push(("key", "Enter"));
        a
    }
    let mut switch = picker(&mut c, target);
    switch.insert(0, ("button", "Manage city"));
    let mut cancel = switch.clone();
    *cancel.last_mut().unwrap() = ("key", "Escape");
    cancel.push(("key", "Escape"));
    scenario(&mut c, &out, &mut routes, "switchcancel", &cancel);
    scenario(&mut c, &out, &mut routes, "switch", &switch);
    assert!(c.view().unwrap().city_production[0].stored < initial);
    let restore = picker(&mut c, 0);
    scenario(&mut c, &out, &mut routes, "restore", &restore);
    assert_eq!(c.view().unwrap().city_production[0].stored, initial);
    scenario(&mut c, &out, &mut routes, "close", &[("key", "Escape")]);
    fs::write(
        out.join("route.json"),
        serde_json::to_string_pretty(&routes).unwrap() + "\n",
    )
    .unwrap();
}

#[test]
fn city_panels_inspect_without_orders_and_retain_panel_through_navigation_and_reload() {
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    founding(&mut c);
    button(&mut c, "Choose production");
    let at = c
        .current_choices()
        .iter()
        .position(|(id, _)| {
            *id >= 0
                && c.loaded
                    .as_ref()
                    .unwrap()
                    .resources
                    .rules
                    .unit_role(*id as usize)
                    != Some(5)
        })
        .unwrap();
    for _ in 0..=at {
        key(&mut c, "ArrowDown");
    }
    key(&mut c, "Enter");
    for _ in 0..40 {
        if !c.view().unwrap().city_info[0].supported.is_empty() {
            break;
        }
        end_round(&mut c);
    }
    assert!(!c.view().unwrap().city_info[0].supported.is_empty());
    button(&mut c, "Manage city");
    let before = c.checkpoint().unwrap();
    for (label, panel) in [
        ("Info", CityPanel::Info),
        ("Support", CityPanel::Support),
        ("Improvements", CityPanel::Improvements),
        ("Happy", CityPanel::Happy),
    ] {
        button(&mut c, label);
        assert_eq!(c.city_panel, panel);
        assert_eq!(
            before,
            c.checkpoint().unwrap(),
            "panel inspection cannot issue an order"
        );
        if panel != CityPanel::Info {
            let tile = c.city_tile;
            key(&mut c, "ArrowRight");
            key(&mut c, " ");
            assert_eq!(
                c.city_tile, tile,
                "hidden tile keys have no action outside Info"
            );
            assert_eq!(before, c.checkpoint().unwrap());
        }
    }
    button(&mut c, "Choose production");
    let at = c
        .current_choices()
        .iter()
        .position(|(id, _)| *id == -4)
        .unwrap();
    for _ in 0..=at {
        key(&mut c, "ArrowDown");
    }
    key(&mut c, "Enter");
    let active = c.checkpoint().unwrap();
    assert!(c.view().unwrap().production_context_city.is_some());
    button(&mut c, "Support");
    button(&mut c, "Happy");
    assert_eq!(
        active,
        c.checkpoint().unwrap(),
        "tabs preserve the recorded city budget"
    );
    let economy = c.view().unwrap().observation.city_economy;
    key(&mut c, "PageDown");
    assert_eq!(c.city_panel, CityPanel::Happy);
    assert_eq!(c.view().unwrap().production_context_city, None);
    assert_eq!(
        economy,
        c.view().unwrap().observation.city_economy,
        "navigation closes interaction context, not economics"
    );
    let closed = c.checkpoint().unwrap();
    key(&mut c, "Ctrl+s");
    edit(&mut c, "panels");
    key(&mut c, "Enter");
    button(&mut c, "Info");
    key(&mut c, "Ctrl+l");
    edit(&mut c, "panels");
    key(&mut c, "Enter");
    assert!(c.city_open);
    assert_eq!(c.city_panel, CityPanel::Info);
    assert_eq!(
        closed,
        c.checkpoint().unwrap(),
        "load retains gameplay independently of presentation panel"
    );
}

#[test]
#[ignore = "install-backed recorded city panel input route"]
fn generated_city_panels_fixture() {
    let base =
        PathBuf::from(std::env::var("TOTENGINE_PANEL_INPUTS").expect("private v0.15 fixture root"));
    let out = PathBuf::from(std::env::var("TOTENGINE_PANELS_OUT").expect("private output"));
    fs::create_dir_all(&out).unwrap();
    let mut c = Client::new(std::env::var("CIVTOT_HOME").unwrap(), out.clone());
    let mut routes = Vec::new();
    fn load_city(c: &mut Client, source: &Path) {
        let name = format!("source{}", storage::nonce().unwrap());
        fs::copy(source, c.saves.join(format!("{name}.cth"))).unwrap();
        if c.loaded.is_none() {
            button(c, "Load engine checkpoint");
        } else {
            key(c, "Ctrl+l");
        }
        edit(c, &name);
        key(c, "Enter");
        if !c.city_open {
            button(c, "Manage city");
        }
        button(c, "Info");
    }
    fn mark(c: &Client, out: &Path, name: &str, end: &str) {
        fs::write(
            out.join(format!("{name}-{end}.cth")),
            c.checkpoint().unwrap(),
        )
        .unwrap();
    }
    fn step(c: &Client, inputs: &mut Vec<serde_json::Value>, input: Input) {
        inputs.push(serde_json::json!({"input":input,"panel":c.city_panel.label(),
            "page":c.city_panel_page.min(c.city_panel_pages()-1)+1,"pages":c.city_panel_pages(),
            "modal":if matches!(c.modal,Some(Modal::Sale{..})){Some("Confirm improvement sale")}else{None}}));
    }
    fn click(c: &mut Client, inputs: &mut Vec<serde_json::Value>, label: &str) {
        let b = c
            .buttons()
            .into_iter()
            .find(|b| b.label == label)
            .unwrap_or_else(|| panic!("missing {label}"));
        let i = Input::Click {
            x: b.x + 12.,
            y: b.y + 8.,
        };
        input(c, i.clone());
        step(c, inputs, i);
    }
    fn press(c: &mut Client, inputs: &mut Vec<serde_json::Value>, key: &str) {
        let i = Input::Key { key: key.into() };
        input(c, i.clone());
        step(c, inputs, i);
    }
    fn finish(
        c: &Client,
        out: &Path,
        routes: &mut Vec<serde_json::Value>,
        name: &str,
        inputs: Vec<serde_json::Value>,
    ) {
        mark(c, out, name, "after");
        routes.push(serde_json::json!({"name":name,"inputs":inputs,"open_city":true,"panel":c.city_panel.label(),"heading":c.frame().draws.iter().find_map(|d|match d {Draw::Text{x,y,text,..} if *x==18. && *y==14.=>Some(text),_=>None}).unwrap()}));
    }
    let mature = base.join("citizens/mature.cth");
    load_city(&mut c, &mature);
    mark(&c, &out, "overview", "before");
    let unchanged = c.checkpoint().unwrap();
    let mut actions = Vec::new();
    click(&mut c, &mut actions, "Support");
    if c.city_panel_pages() > 1 {
        click(&mut c, &mut actions, "Next list");
        click(&mut c, &mut actions, "Previous list");
    }
    click(&mut c, &mut actions, "Improvements");
    click(&mut c, &mut actions, "Happy");
    assert_eq!(unchanged, c.checkpoint().unwrap());
    finish(&c, &out, &mut routes, "overview", actions);

    load_city(&mut c, &mature);
    mark(&c, &out, "entertainer", "before");
    let tile = c.view().unwrap().city_citizens[0]
        .radius
        .iter()
        .take(20)
        .position(|t| t.worked)
        .unwrap();
    let mut actions = Vec::new();
    click(&mut c, &mut actions, "Happy");
    let (x, y) = super::city::spot(tile);
    let i = Input::Click { x, y };
    input(&mut c, i.clone());
    step(&c, &mut actions, i);
    assert_eq!(c.city_panel, CityPanel::Happy);
    assert_eq!(c.view().unwrap().city_citizens[0].specialists, vec![1]);
    let stages = c.view().unwrap().city_info[0].happiness;
    assert_ne!(
        stages[0], stages[1],
        "fixture must demonstrate a real luxury stage"
    );
    finish(&c, &out, &mut routes, "entertainer", actions);
    mark(&c, &out, "specialists", "before");
    let mut actions = Vec::new();
    click(&mut c, &mut actions, "Happy");
    for label in ["1: Entertainer", "1: Tax collector", "1: Scientist"] {
        click(&mut c, &mut actions, label);
    }
    finish(&c, &out, &mut routes, "specialists", actions);

    for (name, accept) in [("salecancel", false), ("sale", true)] {
        load_city(&mut c, &base.join("production/sale-before.cth"));
        mark(&c, &out, name, "before");
        let before = c.checkpoint().unwrap();
        let mut actions = Vec::new();
        click(&mut c, &mut actions, "Improvements");
        let b = c
            .buttons()
            .into_iter()
            .find(|b| matches!(b.action, Action::InspectSale(_, _)))
            .unwrap();
        let i = Input::Click {
            x: b.x + 3.,
            y: b.y + 3.,
        };
        input(&mut c, i.clone());
        step(&c, &mut actions, i);
        assert!(matches!(c.modal, Some(Modal::Sale { .. })));
        press(
            &mut c,
            &mut actions,
            if accept { "Enter" } else { "Escape" },
        );
        assert_eq!(c.city_panel, CityPanel::Improvements);
        if !accept {
            assert_eq!(before, c.checkpoint().unwrap());
        }
        finish(&c, &out, &mut routes, name, actions);
    }
    load_city(&mut c, &base.join("encounter/after-capture.cth"));
    assert!(c.view().unwrap().cities.len() >= 2);
    mark(&c, &out, "navigation", "before");
    let unchanged = c.checkpoint().unwrap();
    let original = c.city;
    let mut actions = Vec::new();
    click(&mut c, &mut actions, "Support");
    press(&mut c, &mut actions, "PageDown");
    assert_ne!(original, c.city);
    assert_eq!(c.city_panel, CityPanel::Support);
    press(&mut c, &mut actions, "PageUp");
    assert_eq!(original, c.city);
    assert_eq!(unchanged, c.checkpoint().unwrap());
    finish(&c, &out, &mut routes, "navigation", actions);

    for (name, navigate) in [("budgettabs", false), ("budgetnav", true)] {
        load_city(&mut c, &base.join("production/switch-after.cth"));
        assert!(c.view().unwrap().production_context_city.is_some());
        mark(&c, &out, name, "before");
        let before = c.checkpoint().unwrap();
        let economy = c.view().unwrap().observation.city_economy;
        let mut actions = Vec::new();
        for label in ["Support", "Improvements", "Happy"] {
            click(&mut c, &mut actions, label);
        }
        if navigate {
            press(&mut c, &mut actions, "PageDown");
            assert_eq!(c.view().unwrap().production_context_city, None);
        } else {
            assert_eq!(before, c.checkpoint().unwrap());
        }
        assert_eq!(economy, c.view().unwrap().observation.city_economy);
        finish(&c, &out, &mut routes, name, actions);
    }
    fs::write(
        out.join("route.json"),
        serde_json::to_string_pretty(&routes).unwrap() + "\n",
    )
    .unwrap();
}
