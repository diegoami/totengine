use super::*;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

// Reproducible fixtures explicitly select a seed; interactive clients are random.
fn seeded_client(home: String, saves: PathBuf) -> Client {
    let mut client = Client::new(home, saves);
    client.seed = "7".into();
    client
}
fn seeded_fallback(home: String, saves: PathBuf, pack: Option<PathBuf>) -> Client {
    let mut client = Client::with_fallback(home, saves, pack);
    client.seed = "7".into();
    client
}

#[test]
fn random_setup_and_quick_start_keep_explicit_games_reproducible() {
    let f = Fixture::new();
    let mut seeds = std::collections::BTreeSet::new();
    for _ in 0..16 {
        let c = Client::new(String::new(), f.0.join("unused"));
        assert!(c.seed.parse::<u32>().is_ok());
        let seed = c.seed.clone();
        c.frame();
        c.frame();
        assert_eq!(c.seed, seed, "refresh cannot reroll setup");
        seeds.insert(seed);
    }
    assert!(
        seeds.len() > 1,
        "interactive starts cannot use a fixed seed"
    );

    let mut quick = f.client();
    quick.seed = "not-a-number".into();
    button(&mut quick, "Quick Start");
    assert!(quick.loaded.is_some(), "{}", quick.status);
    assert!(quick.seed.parse::<u32>().is_ok());
    let mut explicit = f.client();
    explicit.seed = quick.seed.clone();
    button(&mut explicit, "Start game");
    assert_eq!(quick.checkpoint(), explicit.checkpoint());
    assert_eq!(
        quick.tech_name(0),
        "Advance 0",
        "owned rules keep their labels"
    );
    let saved = quick.checkpoint();
    let seed = quick.seed.clone();
    quick.action(Action::NewGame).unwrap();
    key(&mut quick, "Escape");
    assert_eq!(quick.checkpoint(), saved);
    assert_eq!(quick.seed, seed, "cancel keeps the current setup");
    quick.action(Action::NewGame).unwrap();
    key(&mut quick, "Enter");
    assert!(quick.loaded.is_none());
    assert!(quick.seed.parse::<u32>().is_ok());
}

#[test]
fn demo_research_names_preserve_rules_and_old_checkpoint_identity() {
    let root = tempfile::tempdir().unwrap();
    let pack = fallback_pack(root.path());
    let saves = root.path().join("saves");
    let mut c = seeded_fallback(String::new(), saves.clone(), Some(pack.clone()));
    button(&mut c, "Start open demo");
    let before = c.checkpoint().unwrap();
    let names = c.current_choices();
    assert_eq!(names.len(), 9, "native thinning drops 3, 6 and 9");
    assert_eq!(names[0], (0, "Agriculture".into()));
    assert!(names.iter().all(|(_, name)| !name.starts_with("Advance ")));
    assert_eq!(
        c.loaded.as_ref().unwrap().resources.rules.tech_defs()[0].name,
        "Advance 0"
    );
    c.frame();
    assert_eq!(c.checkpoint().unwrap(), before);
    storage::save(&saves, "existing-demo", &before).unwrap();
    let mut restored = Client::with_fallback(String::new(), saves, Some(pack));
    button(&mut restored, "Load engine checkpoint");
    edit(&mut restored, "existing-demo");
    key(&mut restored, "Enter");
    assert_eq!(restored.checkpoint().unwrap(), before);
    assert_eq!(restored.current_choices(), names);
    key(&mut restored, "ArrowDown");
    key(&mut restored, "Enter");
    assert!(restored
        .frame()
        .draws
        .iter()
        .any(|d| matches!(d, Draw::Text{text,..} if text == "Topic: Agriculture")));
}

fn fallback_pack(root: &std::path::Path) -> PathBuf {
    let pack = root.join("isotrident");
    fs::create_dir_all(&pack).unwrap();
    let image = image::RgbaImage::from_fn(64, 64, |x, y| {
        image::Rgba([x as u8, y as u8, 120, if x < 4 { 0 } else { 255 }])
    });
    image.save(pack.join("sheet.png")).unwrap();
    let mut sprites = serde_json::Map::new();
    for tag in [
        "desert",
        "plains",
        "grassland",
        "forest",
        "hills",
        "mountains",
        "tundra",
        "arctic",
        "swamp",
        "jungle",
    ] {
        sprites.insert(
            format!("t.l0.{tag}1"),
            serde_json::json!({"sheet":"sheet.png", "rect":[0,0,64,32]}),
        );
    }
    for tag in ["u", "r", "l", "d"] {
        sprites.insert(
            format!("t.l0.coast_cell_{tag}000"),
            serde_json::json!({"sheet":"sheet.png", "rect":[0,0,32,16]}),
        );
    }
    for tag in [
        "city.european_city_0",
        "city.european_city_2",
        "citizen.content_0",
        "specialist.entertainer_0",
        "specialist.taxman_0",
        "specialist.scientist_0",
        "u.settlers_Idle",
        "u.explorer_Idle",
        "u.phalanx_Idle",
    ] {
        sprites.insert(
            tag.into(),
            serde_json::json!({"sheet":"sheet.png", "rect":[0,0,30,30]}),
        );
    }
    fs::write(pack.join("atlas.json"), serde_json::to_vec(&serde_json::json!({"version":1,"upstream":"Freeciv 3.2.7","license":"GPL-2.0-or-later","source":"test fixture","sprites":sprites})).unwrap()).unwrap();
    pack
}

#[test]
fn stock_unit_aliases_render_the_intended_art_instead_of_neutral_markers() {
    let root = tempfile::tempdir().unwrap();
    let pack = fallback_pack(root.path());
    let path = pack.join("atlas.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for (i, tag) in ["elephants", "stealth_fighter", "stealth_bomber"]
        .iter()
        .enumerate()
    {
        // Distinct opaque swatches identify the actual semantic source selected.
        manifest["sprites"][format!("u.{tag}_Idle")] = serde_json::json!({
            "sheet":"sheet.png", "rect":[10 * (i + 1), 4, 1, 1]
        });
    }
    fs::write(path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let rules = root.path().join("Rules.txt");
    fs::write(
        &rules,
        "@UNITS\nElephant\nStlth Ftr.\nStlth Bmbr.\nUnmapped unit\n",
    )
    .unwrap();
    let resources = Resources::load_isotrident(&pack, Some(&rules)).unwrap();
    let atlas = Atlas::load(&resources).unwrap();
    let neutral = &atlas.sprites["unit-3"].rgba;
    for i in 0..3 {
        let sprite = &atlas.sprites[&format!("unit-{i}")];
        let center = ((32 * sprite.w + 32) * 4) as usize;
        assert_eq!(
            &sprite.rgba[center..center + 4],
            &[(10 * (i + 1)) as u8, 4, 120, 255]
        );
        assert_ne!(&sprite.rgba, neutral);
    }
}

#[test]
fn fallback_standalone_and_rules_only_reload_without_install() {
    let root = tempfile::tempdir().unwrap();
    let pack = fallback_pack(root.path());
    let mut c = seeded_fallback(String::new(), root.path().join("saves"), Some(pack.clone()));
    button(&mut c, "Start open demo");
    let loaded = c.loaded.as_ref().unwrap_or_else(|| panic!("{}", c.status));
    assert_eq!(loaded.resources.profile, "Open demo + Isotrident");
    assert_eq!(
        loaded.resources.rules.unit_names(),
        ["Pioneers", "Runners", "Guards"]
    );
    assert!(c
        .frame()
        .draws
        .iter()
        .any(|d| matches!(d, Draw::Text{text,..} if text.contains("Open demo / Isotrident"))));
    let checkpoint = loaded.session.checkpoint(&loaded.identity);
    let rules_path = root.path().join("rules.txt");
    fs::write(
        &rules_path,
        include_bytes!("../../data/open-demo-rules.txt"),
    )
    .unwrap();
    let r = Resources::load_isotrident(&pack, Some(&rules_path)).unwrap();
    assert!(
        HumanSession::restore(r.rules.clone(), &r.identity(), &checkpoint).is_err(),
        "different profile must not silently replace gameplay identity"
    );
    let r = Resources::load_isotrident(&pack, None).unwrap();
    assert_eq!(
        HumanSession::restore(r.rules.clone(), &r.identity(), &checkpoint)
            .unwrap()
            .checkpoint(&r.identity()),
        checkpoint
    );
    let c = seeded_fallback(
        rules_path.to_string_lossy().into(),
        root.path().join("saves2"),
        Some(pack.clone()),
    );
    assert!(c
        .load_resources()
        .unwrap()
        .0
        .profile
        .starts_with("ToT rules"));
    let c = seeded_fallback(
        root.path().join("typo").to_string_lossy().into(),
        root.path().join("saves3"),
        Some(pack),
    );
    assert!(
        c.load_resources().is_err(),
        "invalid explicit install must not silently start the demo"
    );
}

#[test]
fn fallback_rejects_invalid_manifest_and_tracks_art_changes() {
    let root = tempfile::tempdir().unwrap();
    let pack = fallback_pack(root.path());
    let first = Resources::load_isotrident(&pack, None).unwrap().identity();
    let path = pack.join("atlas.json");
    let original = fs::read(&path).unwrap();
    let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    manifest["sprites"]["u.settlers_Idle"]["rect"] = serde_json::json!([0, 0, 31, 30]);
    fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert_ne!(
        first,
        Resources::load_isotrident(&pack, None).unwrap().identity()
    );
    for rect in [
        serde_json::json!([u32::MAX, 0, 30, 30]),
        serde_json::json!([0, 0, 0, 30]),
        serde_json::json!([0, 0, 65, 30]),
    ] {
        manifest["sprites"]["u.settlers_Idle"]["rect"] = rect;
        fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(Resources::load_isotrident(&pack, None).is_err());
    }
    manifest = serde_json::from_slice(&original).unwrap();
    manifest["sprites"]["u.settlers_Idle"]["sheet"] = serde_json::json!("../sheet.png");
    fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(Resources::load_isotrident(&pack, None).is_err());
}

#[test]
fn installed_isotrident_pack_covers_shared_renderer() {
    let Some(pack) = std::env::var_os("TOTENGINE_ISOTRIDENT_TEST") else {
        return;
    };
    let pack = PathBuf::from(pack);
    let r = Resources::load_isotrident(&pack, None).unwrap();
    let a = Atlas::load(&r).unwrap();
    assert!(r.isotrident.as_ref().unwrap().len() > 300);
    assert_eq!(a.sprites["unit-0"].w, 64);
    for key in [
        "unit-0",
        "unit-1",
        "unit-2",
        "terrain-10-0",
        "city-0",
        "citizen-0",
    ] {
        assert!(a.sprites[key]
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[3] > 0));
        assert!(a.sprites[key]
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[3] == 0));
    }
    if let Some(home) = std::env::var_os("CIVTOT_HOME") {
        let root = PathBuf::from(home).canonicalize().unwrap();
        let path = crate::resources::client_rules_path(&root).unwrap().unwrap();
        let fallback = Resources::load_isotrident(&pack, Some(&path)).unwrap();
        assert_eq!(
            fallback.rules,
            Resources::load(&root, "Original").unwrap().rules
        );
        assert!(Atlas::load(&fallback)
            .unwrap()
            .sprites
            .contains_key("unit-79"));
    }
    let scratch = tempfile::tempdir().unwrap();
    let mut c = seeded_fallback(String::new(), scratch.path().join("saves"), Some(pack));
    button(&mut c, "Start open demo");
    research(&mut c);
    founding(&mut c);
    button(&mut c, "Choose production");
    for _ in 0..80 {
        if c.choice == Some(2) {
            break;
        }
        key(&mut c, "ArrowDown");
    }
    assert_eq!(c.choice, Some(2));
    key(&mut c, "Enter");
    for _ in 0..10 {
        research(&mut c);
        key(&mut c, "Ctrl+n");
        if matches!(c.modal, Some(Modal::Confirm { .. })) {
            key(&mut c, "Enter");
        }
        ready(&mut c);
    }
    research(&mut c);
    let actor = c
        .active
        .expect("normal production must produce a ready guard");
    let before = unit_at(&c, actor);
    key(&mut c, "v");
    let body = c
        .frame()
        .draws
        .iter()
        .find_map(|d| match d {
            Draw::UnitSprite {
                x,
                y,
                w,
                h,
                sprite,
                unit,
                ..
            } if *unit == actor.slot => {
                let s = &c.atlas().unwrap().sprites[sprite];
                let (px, py) = (20..55)
                    .find_map(|py| {
                        (20..44)
                            .find(|&px| s.rgba[(py * 64 + px) * 4 + 3] > 0)
                            .map(|px| (px, py))
                    })
                    .unwrap();
                Some((
                    *x + (px as f64 + 0.5) * *w / 64.,
                    *y + (py as f64 + 0.5) * *h / 64.,
                ))
            }
            _ => None,
        })
        .unwrap();
    click_point(&mut c, body);
    assert_eq!(c.active, Some(actor));
    assert!(c.moving);
    assert_eq!(unit_at(&c, actor).moves_left, before.moves_left);
    key(&mut c, "6");
    let after = unit_at(&c, actor);
    assert_ne!((after.x, after.y), (before.x, before.y));
    assert!(after.moves_left < before.moves_left);
}
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
        seeded_client(self.0.to_str().unwrap().into(), self.0.join("saves"))
    }
    pub(super) fn saves(&self) -> PathBuf {
        self.0.join("saves")
    }
    pub(super) fn rules_root(&self) -> PathBuf {
        self.0.clone()
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
/// Clicks the button labelled `label`; a map-screen command that lives in a
/// menu (layout B, #310) is reached the way a player does: click the menu
/// title, then the entry.
fn button(c: &mut Client, label: &str) {
    let buttons = c.buttons();
    let b = buttons
        .iter()
        .find(|b| b.label == label)
        .or_else(|| buttons.iter().find(|b| b.label.contains(label)))
        .filter(|b| !matches!(b.action, Action::Menu(_)));
    if let Some(b) = b {
        let (x, y) = (b.x + 3., b.y + 3.);
        input(c, Input::Click { x, y });
        return;
    }
    let (m, row) = (0..chrome::MENUS.len())
        .find_map(|m| {
            let items = c.menu_items(m);
            items
                .iter()
                .position(|i| i.label == label)
                .or_else(|| items.iter().position(|i| i.label.contains(label)))
                .map(|row| (m, row))
        })
        .unwrap_or_else(|| panic!("missing button {label}"));
    menu_entry(c, m, row);
}
/// Opens menu `m` by clicking its title and clicks entry `row`.
fn menu_entry(c: &mut Client, m: usize, row: usize) {
    let find = |c: &Client, a: &dyn Fn(&Action) -> bool| {
        c.buttons()
            .into_iter()
            .find(|b| a(&b.action))
            .map(|b| (b.x + 3., b.y + 3.))
    };
    let (x, y) = find(c, &|a| matches!(a, Action::Menu(n) if *n == m)).expect("menu title");
    input(c, Input::Click { x, y });
    assert_eq!(c.menu, Some(m), "menu {m} did not open: {}", c.status);
    let (x, y) = find(
        c,
        &|a| matches!(a, Action::MenuItem(n, r) if (*n, *r) == (m, row)),
    )
    .expect("menu entry");
    input(c, Input::Click { x, y });
    assert_eq!(c.menu, None);
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
    button(c, "Start game");
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
    button(&mut c, "Start game");
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
    let mut c = seeded_client(home, out.clone());
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
    let mut c = seeded_client(home, out.clone());
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
    let mut c = seeded_client(home, out.clone());
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

pub(super) fn end_round(c: &mut Client) {
    research(c);
    key(c, "Ctrl+n");
    if matches!(c.modal, Some(Modal::Confirm { .. })) {
        key(c, "Enter");
    }
    ready(c);
    research(c);
}
pub(super) fn build_pair(c: &mut Client) {
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
pub(super) fn land_step(c: &Client, e: Entity) -> (i32, i32) {
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
    let mut c = seeded_client(home, out.clone());
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
    let mut c = seeded_client(home, out.clone());
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
            c.buttons().iter().any(|b| b.label.contains("[Space]")),
            panel == CityPanel::Info,
            "only Info advertises the tile keyboard shortcut"
        );
        assert!(c
            .buttons()
            .iter()
            .any(|b| matches!(b.action, Action::Worker(_, _))));
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
    button(&mut c, "Support");
    let v = c.view().unwrap();
    let u = &v.city_info[0].supported[0];
    let name =
        c.loaded.as_ref().unwrap().resources.rules.unit_names()[u.type_id as usize].to_string();
    assert!(
        c.frame()
            .draws
            .iter()
            .any(|d| matches!(d,Draw::Text{text,..} if text==&format!("{name} #{}",u.slot))),
        "panel identity must use the same unit number as the map"
    );
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
    key(&mut c, "Ctrl+l");
    edit(&mut c, "panels");
    key(&mut c, "Enter");
    assert!(c.city_open);
    assert_eq!(c.city_panel, CityPanel::Happy);
    assert_eq!(c.view().unwrap().production_context_city, None);
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
    let mut c = seeded_client(std::env::var("CIVTOT_HOME").unwrap(), out.clone());
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
            "modal":if matches!(c.modal,Some(Modal::Sale{..})){Some("Confirm improvement sale")}else{None},
            "text":c.view().and_then(|v|c.city.and_then(|city|v.city_info.into_iter().find(|d|d.index==city.slot))).and_then(|d|match c.city_panel {
                CityPanel::Info=>Some(format!("Own units present: {}",d.present.len())),
                CityPanel::Support=>Some(format!("Supported units: {}",d.supported.len())),_=>None})}));
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
    // This generated fixture already has two own cities; the defeated size-one
    // foreign city was razed. Move the surviving home-city warrior into the
    // other own city through ordinary recorded keypad orders, with explicit
    // reselection after exhaustion. Do not inject a visiting-unit state.
    key(&mut c, "Escape");
    for direction in ["4", "7"] {
        if c.view()
            .unwrap()
            .observation
            .units
            .iter()
            .find(|u| u.index == 0)
            .unwrap()
            .moves_left
            == 0
        {
            end_round(&mut c);
        }
        for _ in 0..100 {
            if c.active.is_some_and(|u| u.slot == 0) {
                break;
            }
            key(&mut c, "w");
        }
        assert!(c.active.is_some_and(|u| u.slot == 0));
        if !c.moving {
            key(&mut c, "v");
        }
        key(&mut c, direction);
        assert!(
            c.modal.is_none(),
            "normal own-city route must not need a combat choice"
        );
    }
    button(&mut c, "Manage city");
    button(&mut c, "Info");
    assert!(c.view().unwrap().cities.len() >= 2);
    mark(&c, &out, "navigation", "before");
    let unchanged = c.checkpoint().unwrap();
    let original = c.city;
    let mut actions = Vec::new();
    click(&mut c, &mut actions, "Support");
    press(&mut c, &mut actions, "PageDown");
    assert_ne!(original, c.city);
    assert_eq!(c.city_panel, CityPanel::Support);
    let v = c.view().unwrap();
    let home = v
        .city_info
        .iter()
        .find(|d| d.index == original.unwrap().slot)
        .unwrap();
    assert!(
        home.supported
            .iter()
            .any(|u| !home.present.iter().any(|p| p.id == u.id)),
        "navigation fixture must include a supported-away unit"
    );
    let visited = v
        .city_info
        .iter()
        .find(|d| d.index == c.city.unwrap().slot)
        .unwrap();
    assert!(
        visited.present.iter().any(|u| u.shields.is_none()),
        "visited city must contain a unit supported elsewhere or without a home"
    );
    click(&mut c, &mut actions, "Info");
    click(&mut c, &mut actions, "Support");
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

#[test]
fn checkpoint_load_defaults_to_info_only_after_leaving_city_workspace() {
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    founding(&mut c);
    button(&mut c, "Manage city");
    button(&mut c, "Choose production");
    key(&mut c, "ArrowDown");
    key(&mut c, "Enter");
    assert!(c.view().unwrap().production_context_city.is_some());
    let saved = c.checkpoint().unwrap();
    key(&mut c, "Ctrl+s");
    edit(&mut c, "budget");
    key(&mut c, "Enter");
    button(&mut c, "Happy");
    key(&mut c, "Ctrl+l");
    edit(&mut c, "budget");
    key(&mut c, "Enter");
    assert!(c.city_open);
    assert_eq!(c.city_panel, CityPanel::Happy);
    assert_eq!(c.checkpoint().unwrap(), saved);

    key(&mut c, "Escape");
    assert!(!c.city_open);
    let before_failed_load = c.checkpoint().unwrap();
    key(&mut c, "Ctrl+l");
    edit(&mut c, "missing");
    key(&mut c, "Enter");
    assert!(matches!(c.modal, Some(Modal::Load)));
    assert_eq!(c.city_panel, CityPanel::Happy);
    assert_eq!(c.checkpoint().unwrap(), before_failed_load);
    edit(&mut c, "budget");
    key(&mut c, "Enter");
    assert!(c.city_open, "restored budget reopens its city");
    assert_eq!(c.city_panel, CityPanel::Info);
    assert_eq!(c.city_panel_page, 0);
    assert_eq!(c.checkpoint().unwrap(), saved);

    let mut fresh = f.client();
    button(&mut fresh, "Load engine checkpoint");
    edit(&mut fresh, "budget");
    key(&mut fresh, "Enter");
    assert!(fresh.city_open);
    assert_eq!(fresh.city_panel, CityPanel::Info);
    assert_eq!(fresh.city_panel_page, 0);
    assert_eq!(fresh.checkpoint().unwrap(), saved);
}

#[test]
fn city_double_click_is_bound_to_an_inert_own_city_click() {
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    founding(&mut c);
    let city = c.view().unwrap().observation.cities[0].clone();
    c.pan = (
        380. - f64::from(city.x) * 32.,
        390. - f64::from(city.y) * 16.,
    );
    let before = c.checkpoint().unwrap();
    let first = c.receipts.len() as u64 + 1;
    input(&mut c, Input::Click { x: 380., y: 370. });
    assert_eq!(
        c.frame().city_click,
        Some(first),
        "city roof and base share the rendered sprite hit area"
    );
    assert!(c.status.contains("double-click"));
    assert_eq!(
        before,
        c.checkpoint().unwrap(),
        "first city inspection is read-only"
    );
    input(
        &mut c,
        Input::DoubleClick {
            x: 380.,
            y: 370.,
            first_click: first + 999,
        },
    );
    assert!(!c.city_open);
    assert_eq!(c.frame().city_click, None);
    input(
        &mut c,
        Input::DoubleClick {
            x: 380.,
            y: 370.,
            first_click: first,
        },
    );
    assert!(
        !c.city_open,
        "invalid second click consumes the first gesture"
    );
    let first = c.receipts.len() as u64 + 1;
    input(&mut c, Input::Click { x: 380., y: 370. });
    input(
        &mut c,
        Input::DoubleClick {
            x: 480.,
            y: 370.,
            first_click: first,
        },
    );
    assert_eq!(c.frame().city_click, None);
    input(
        &mut c,
        Input::DoubleClick {
            x: 380.,
            y: 370.,
            first_click: first,
        },
    );
    assert!(
        !c.city_open,
        "an out-of-range second click cannot be repaired"
    );
    let first = c.receipts.len() as u64 + 1;
    input(&mut c, Input::Click { x: 380., y: 370. });
    key(&mut c, "v");
    input(
        &mut c,
        Input::DoubleClick {
            x: 380.,
            y: 370.,
            first_click: first,
        },
    );
    assert!(!c.city_open, "intervening input invalidates the gesture");
    let first = c.receipts.len() as u64 + 1;
    input(&mut c, Input::Click { x: 380., y: 370. });
    input(
        &mut c,
        Input::DoubleClick {
            x: 380.,
            y: 370.,
            first_click: first,
        },
    );
    assert!(c.city_open);
    assert_eq!(c.city.unwrap().slot, city.index);
    assert_eq!(before, c.checkpoint().unwrap(), "opening is read-only");
    input(
        &mut c,
        Input::DoubleClick {
            x: 500.,
            y: 300.,
            first_click: first,
        },
    );
    assert_eq!(
        before,
        c.checkpoint().unwrap(),
        "a repeated double click cannot turn into a worker edit"
    );
    key(&mut c, "Escape");
    let first = c.receipts.len() as u64 + 1;
    input(&mut c, Input::Click { x: 380., y: 370. });
    input(&mut c, Input::Focus { focused: false });
    input(&mut c, Input::Focus { focused: true });
    input(
        &mut c,
        Input::DoubleClick {
            x: 380.,
            y: 370.,
            first_click: first,
        },
    );
    assert!(!c.city_open);
    assert_eq!(before, c.checkpoint().unwrap());
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
        if c.view().unwrap().units.len() >= 2 {
            break;
        }
        end_round(&mut c);
    }
    assert!(c.view().unwrap().units.len() >= 2);
    c.pan = (
        380. - f64::from(city.x) * 32.,
        390. - f64::from(city.y) * 16.,
    );
    let before = c.checkpoint().unwrap();
    let first = c.receipts.len() as u64 + 1;
    input(&mut c, Input::Click { x: 380., y: 370. });
    assert!(
        matches!(c.modal, Some(Modal::Units { .. })),
        "first garrison click keeps explicit stack choice"
    );
    input(
        &mut c,
        Input::DoubleClick {
            x: 380.,
            y: 370.,
            first_click: first,
        },
    );
    assert!(c.city_open && c.modal.is_none());
    assert_eq!(before, c.checkpoint().unwrap());
}

#[test]
fn moving_unit_cannot_enter_an_empty_own_city_by_inspection_click() {
    let f = Fixture::new();
    let mut c = f.client();
    build_pair(&mut c);
    let city = c.view().unwrap().observation.cities[0].clone();
    let units = c.view().unwrap().units;
    for &u in &units {
        c.action(Action::Unit(u)).unwrap();
        let (dx, dy) = land_step(&c, u);
        c.step(dx, dy).unwrap();
    }
    end_round(&mut c);
    assert!(c.view().unwrap().city_info[0].present.is_empty());
    let actor = units[0];
    let p = point(&c, actor, -20.);
    click_point(&mut c, p);
    assert_eq!(c.active, Some(actor));
    assert!(c.moving && unit_at(&c, actor).moves_left > 0);
    let before = c.checkpoint().unwrap();
    let city_point = (
        c.pan.0 + f64::from(city.x) * 32. * c.zoom,
        c.pan.1 + f64::from(city.y) * 16. * c.zoom,
    );
    click_point(&mut c, city_point);
    assert!(c.frame().city_click.is_some());
    assert_eq!(c.active, Some(actor));
    assert_eq!(
        before,
        c.checkpoint().unwrap(),
        "city click cannot spend the ready external unit"
    );
    // Countercheck: this same unit can really enter that city through the
    // accepted keyboard path. Removing the own-city mouse guard fails above.
    let u = unit_at(&c, actor);
    let (dx, dy) = (
        i32::from(city.x) - i32::from(u.x),
        i32::from(city.y) - i32::from(u.y),
    );
    let digit = match (dx, dy) {
        (-1, -1) => "7",
        (0, -2) => "8",
        (1, -1) => "9",
        (-2, 0) => "4",
        (2, 0) => "6",
        (-1, 1) => "1",
        (0, 2) => "2",
        (1, 1) => "3",
        _ => panic!("external unit must be adjacent"),
    };
    key(&mut c, digit);
    assert_eq!(
        (unit_at(&c, actor).x, unit_at(&c, actor).y),
        (city.x, city.y)
    );
    assert_ne!(before, c.checkpoint().unwrap());
}

#[test]
fn city_list_previous_uses_the_visible_page_after_a_boundary_sale() {
    let f = Fixture::new();
    // Authored cheap, upkeep-free buildings make eleven real completions
    // practical. No world or view is patched to fabricate the sale boundary.
    let file = f.0.join("Rules.txt");
    let mut rules = tot_kit::parse_rules(&fs::read_to_string(&file).unwrap());
    let improvements = rules.sections.get_mut("IMPROVE").unwrap();
    for row in 2..36 {
        improvements.rows[row][1] = "1".into();
        improvements.rows[row][2] = "0".into();
        improvements.raw_lines[row] = improvements.rows[row].join(", ");
    }
    fs::write(file, tot_kit::emit_rules(&rules)).unwrap();
    let mut c = f.client();
    start(&mut c);
    founding(&mut c);
    while c.view().unwrap().city_info[0].improvements.len() < 11 {
        button(&mut c, "Choose production");
        let id = c
            .current_choices()
            .into_iter()
            .find(|(id, _)| (-35..=-2).contains(id))
            .unwrap()
            .0;
        c.action(Action::Choose(id)).unwrap();
        key(&mut c, "Enter");
        let count = c.view().unwrap().city_info[0].improvements.len();
        for _ in 0..30 {
            end_round(&mut c);
            if c.view().unwrap().city_info[0].improvements.len() > count {
                break;
            }
        }
        assert_eq!(c.view().unwrap().city_info[0].improvements.len(), count + 1);
    }
    button(&mut c, "Manage city");
    button(&mut c, "Improvements");
    button(&mut c, "Next list");
    button(&mut c, "Next list");
    assert_eq!(c.city_panel_page, 2);
    assert_eq!(c.city_panel_pages(), 3);
    let saved = c.checkpoint().unwrap();
    key(&mut c, "Ctrl+s");
    edit(&mut c, "paged");
    key(&mut c, "Enter");
    key(&mut c, "Ctrl+l");
    edit(&mut c, "paged");
    key(&mut c, "Enter");
    assert_eq!(c.city_panel, CityPanel::Improvements);
    assert_eq!(c.city_panel_page, 2, "open workspace retains list page");
    key(&mut c, "Escape");
    key(&mut c, "Ctrl+l");
    edit(&mut c, "paged");
    key(&mut c, "Enter");
    assert!(!c.city_open);
    button(&mut c, "Manage city");
    assert_eq!(c.city_panel, CityPanel::Info);
    assert_eq!(c.city_panel_page, 0, "closed workspace discards old page");
    assert_eq!(c.checkpoint().unwrap(), saved);
    button(&mut c, "Improvements");
    button(&mut c, "Next list");
    button(&mut c, "Next list");
    let sale = c
        .buttons()
        .into_iter()
        .find(|b| matches!(b.action, Action::InspectSale(_, _)))
        .unwrap();
    click_point(&mut c, (sale.x + 3., sale.y + 3.));
    assert!(matches!(c.modal, Some(Modal::Sale { .. })));
    key(&mut c, "Enter");
    assert_eq!(c.view().unwrap().city_info[0].improvements.len(), 10);
    assert_eq!(c.city_panel_pages(), 2);
    assert!(c
        .frame()
        .draws
        .iter()
        .any(|d| matches!(d, Draw::Text { text, .. } if text == "List 2 / 2")));
    let before = c.checkpoint().unwrap();
    button(&mut c, "Previous list");
    assert_eq!(
        c.city_panel_page, 0,
        "one Previous click reaches the preceding visible page"
    );
    assert_eq!(before, c.checkpoint().unwrap());
}

fn next_turn(c: &mut Client) {
    research(c);
    key(c, "Ctrl+n");
    if matches!(c.modal, Some(Modal::Confirm { .. })) {
        key(c, "Enter");
    }
    ready(c);
}

#[test]
fn rates_dialog_moves_locks_applies_and_discards_without_engine_changes() {
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    let before = c.checkpoint().unwrap();
    key(&mut c, "Shift+t");
    let Some(Modal::Rates { draft, cap, .. }) = c.modal.clone() else {
        panic!("{}", c.status)
    };
    assert_eq!((draft, cap), ([4, 0, 6], 6));
    // Escape discards the draft: no command, no revision, identical checkpoint.
    key(&mut c, "ArrowRight");
    assert!(matches!(c.modal, Some(Modal::Rates { draft, .. }) if draft == [5, 0, 5]));
    key(&mut c, "Escape");
    assert!(c.modal.is_none());
    assert_eq!(c.checkpoint().unwrap(), before);
    // A refused move (the other two shares locked) keeps the draft and says why.
    key(&mut c, "Shift+t");
    key(&mut c, "ArrowDown");
    key(&mut c, "l");
    key(&mut c, "ArrowDown");
    key(&mut c, "l");
    assert_eq!(c.rate_locks, [false, true, true]);
    key(&mut c, "ArrowUp");
    key(&mut c, "ArrowUp");
    key(&mut c, "ArrowRight");
    assert!(c.status.contains("locked"), "{}", c.status);
    assert!(matches!(c.modal, Some(Modal::Rates { draft, .. }) if draft == [4, 0, 6]));
    key(&mut c, "Escape");
    c.rate_locks = [false; 3];
    // Apply sends one validated command.
    key(&mut c, "Shift+t");
    key(&mut c, "ArrowRight");
    key(&mut c, "Enter");
    assert!(c.modal.is_none(), "{}", c.status);
    let r = c.view().unwrap().rates.unwrap();
    assert_eq!((r.tax, r.luxury, r.science), (5, 0, 5));
    // The draft shows an engine quote for the split it displays.
    key(&mut c, "Shift+t");
    key(&mut c, "ArrowDown");
    key(&mut c, "ArrowRight");
    let Some(Modal::Rates { draft, quote, .. }) = c.modal.clone() else {
        panic!()
    };
    let quote = quote.expect("quote follows the draft");
    assert_eq!(
        (quote.tax, quote.luxury, quote.science),
        (draft[0], draft[1], draft[2])
    );
    assert!(c
        .frame()
        .draws
        .iter()
        .any(|d| matches!(d, Draw::Text{text,..} if text.starts_with("Treasury "))));
}

#[test]
fn revolution_cancel_confirm_chooser_review_and_reload() {
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    let before = c.checkpoint().unwrap();
    key(&mut c, "Shift+r");
    assert!(matches!(c.modal, Some(Modal::Revolution)));
    key(&mut c, "Escape");
    assert!(c.modal.is_none());
    assert_eq!(c.checkpoint().unwrap(), before, "Cancel changes nothing");
    key(&mut c, "Shift+r");
    key(&mut c, "Enter");
    assert_eq!(c.view().unwrap().rates.unwrap().government, 0);
    key(&mut c, "Shift+r");
    assert!(c.status.contains("Already in Anarchy"), "{}", c.status);
    // The rate dialog stays available during Anarchy at its 60% cap.
    key(&mut c, "Shift+t");
    assert!(matches!(
        c.modal,
        Some(Modal::Rates {
            cap: 6,
            review: None,
            ..
        })
    ));
    key(&mut c, "Escape");
    for _ in 0..8 {
        if matches!(c.modal, Some(Modal::Government { .. })) {
            break;
        }
        next_turn(&mut c);
    }
    assert!(
        matches!(c.modal, Some(Modal::Government { .. })),
        "{}",
        c.status
    );
    // Required: Escape keeps it, a missing selection is refused, state is saved as pending.
    key(&mut c, "Escape");
    assert!(matches!(c.modal, Some(Modal::Government { .. })));
    key(&mut c, "Enter");
    assert!(c.status.contains("Select a government"), "{}", c.status);
    assert!(!c.buttons().iter().any(|b| b.label.contains("Cancel")));
    button(&mut c, "Save pending");
    edit(&mut c, "pending-government");
    key(&mut c, "Enter");
    button(&mut c, "Load checkpoint");
    edit(&mut c, "pending-government");
    key(&mut c, "Enter");
    assert!(
        matches!(c.modal, Some(Modal::Government { .. })),
        "reload reopens the chooser"
    );
    assert_eq!(
        c.current_choices(),
        vec![(1, "Despotism — each share up to 60%".to_string())]
    );
    key(&mut c, "ArrowDown");
    key(&mut c, "Enter");
    // Choosing opens the required review; Escape resets it and keeps it open.
    assert!(
        matches!(
            c.modal,
            Some(Modal::Rates {
                review: Some(_),
                ..
            })
        ),
        "{}",
        c.status
    );
    key(&mut c, "ArrowRight");
    key(&mut c, "Escape");
    assert!(
        matches!(c.modal, Some(Modal::Rates { review: Some(_), draft, opened, .. }) if draft == opened)
    );
    key(&mut c, "Enter");
    assert!(c.modal.is_none(), "{}", c.status);
    let r = c.view().unwrap().rates.unwrap();
    assert_eq!(r.government, 1);
    assert!(c.view().unwrap().decision.is_none());
}

#[test]
fn science_advisor_sets_and_clears_the_goal_and_marks_the_suggestion() {
    let f = Fixture::new();
    let mut c = f.client();
    button(&mut c, "Start game");
    let Some(Decision::Research { available, .. }) = c.view().unwrap().decision else {
        panic!()
    };
    key(&mut c, "Escape");
    assert!(c.modal.is_none());
    key(&mut c, "F6");
    assert!(matches!(c.modal, Some(Modal::Science)));
    button(&mut c, "Set goal");
    assert!(matches!(c.modal, Some(Modal::Goal)));
    let goal = available[1];
    c.action(Action::Choose(goal as i32)).unwrap();
    key(&mut c, "Enter");
    assert_eq!(c.view().unwrap().goal, Some(goal), "{}", c.status);
    // The pending research prompt returns with the goal's step marked and pre-selected.
    assert!(matches!(c.modal, Some(Modal::Research)));
    assert_eq!(c.choice, Some(goal as i32));
    assert!(c
        .current_choices()
        .iter()
        .any(|(id, label)| *id == goal as i32 && label.ends_with("(suggested)")));
    key(&mut c, "Escape");
    key(&mut c, "F6");
    button(&mut c, "Clear goal");
    assert_eq!(c.view().unwrap().goal, None);
    key(&mut c, "Escape");
    assert!(c.modal.is_none());
}

// Shared recorder for the real-input routes (`tools/check_*_route.{py,cjs}`).
pub(super) fn title(c: &Client) -> serde_json::Value {
    c.frame()
        .draws
        .iter()
        .find_map(|d| match d {
            Draw::Text {
                x, y, text, size, ..
            } if (*x, *y, *size) == (212., 125., style::TYPE_TITLE) => Some(text.clone().into()),
            _ => None,
        })
        .unwrap_or(serde_json::Value::Null)
}
pub(super) struct Rec(pub Vec<serde_json::Value>);
impl Rec {
    fn key(&mut self, c: &mut Client, k: &str) {
        key(c, k);
        self.0
            .push(serde_json::json!({"op":"key","key":k,"title":title(c)}));
    }
    fn button(&mut self, c: &mut Client, label: &str) {
        button(c, label);
        self.0
            .push(serde_json::json!({"op":"button","label":label,"title":title(c)}));
    }
    fn entry(&mut self, c: &mut Client, name: &str) {
        edit(c, name);
        key(c, "Enter");
        self.0
            .push(serde_json::json!({"op":"entry","value":name,"title":title(c)}));
    }
    fn save(&mut self, c: &mut Client, name: &str) {
        self.key(c, "Ctrl+s");
        self.entry(c, name);
        self.0.push(serde_json::json!({"op":"compare","name":name}));
    }
    fn research(&mut self, c: &mut Client) {
        research(c);
        self.0
            .push(serde_json::json!({"op":"research","title":title(c)}));
    }
    fn endturn(&mut self, c: &mut Client) {
        key(c, "Ctrl+n");
        if matches!(c.modal, Some(Modal::Confirm { .. })) {
            key(c, "Enter");
        }
        ready(c);
        self.0
            .push(serde_json::json!({"op":"endturn","title":title(c)}));
    }
}

/// Records the science/rates/government route for the real-input runners
/// (`tools/check_government_route.{py,cjs}`): each step carries the dialog title
/// that must be on screen afterwards, and every save is compared byte for byte.
#[test]
#[ignore = "install-backed recorded science/rates/government input route"]
fn generated_government_route() {
    let out = PathBuf::from(std::env::var("TOTENGINE_GOVERNMENT_OUT").expect("private output"));
    fs::create_dir_all(&out).unwrap();
    let home = std::env::var("CIVTOT_HOME").unwrap();
    let mut c = seeded_client(home.clone(), out.clone());
    start(&mut c);
    key(&mut c, "Ctrl+s");
    edit(&mut c, "govstart");
    key(&mut c, "Enter");
    // A fresh client loads the start state, exactly as each real client will.
    let mut c = seeded_client(home, out.clone());
    button(&mut c, "Load engine checkpoint");
    edit(&mut c, "govstart");
    key(&mut c, "Enter");
    assert!(c.loaded.is_some(), "{}", c.status);
    let mut r = Rec(Vec::new());
    let read = |name: &str| fs::read(out.join(format!("{name}.cth"))).unwrap();
    // Rates: Escape discards, locked shares refuse a move, Enter applies.
    r.key(&mut c, "Shift+t");
    assert_eq!(title(&c), "Tax, luxury and science rates");
    r.key(&mut c, "ArrowRight");
    r.key(&mut c, "Escape");
    r.save(&mut c, "govdiscard");
    assert_eq!(read("govdiscard"), read("govstart"));
    r.key(&mut c, "Shift+t");
    for k in [
        "ArrowDown",
        "l",
        "ArrowDown",
        "l",
        "ArrowUp",
        "ArrowUp",
        "ArrowRight",
    ] {
        r.key(&mut c, k);
    }
    assert!(c.status.contains("locked"), "{}", c.status);
    // Locks outlive the dialog (session only): unlock both, then apply a move.
    for k in [
        "ArrowDown",
        "l",
        "ArrowDown",
        "l",
        "ArrowUp",
        "ArrowUp",
        "ArrowRight",
    ] {
        r.key(&mut c, k);
    }
    assert_eq!(c.rate_locks, [false; 3]);
    r.key(&mut c, "Enter");
    r.save(&mut c, "govrates");
    assert_ne!(read("govrates"), read("govstart"));
    // The advisor sets a goal that the checkpoint keeps.
    r.key(&mut c, "F6");
    r.button(&mut c, "Set goal…");
    r.key(&mut c, "ArrowDown");
    r.key(&mut c, "Enter");
    assert!(c.view().unwrap().goal.is_some(), "{}", c.status);
    r.key(&mut c, "Escape");
    r.save(&mut c, "govgoal");
    // Revolution: Cancel changes nothing, Yes starts Anarchy.
    r.key(&mut c, "Shift+r");
    assert_eq!(title(&c), "Revolution");
    r.key(&mut c, "Escape");
    r.save(&mut c, "govrevoltcancel");
    assert_eq!(read("govrevoltcancel"), read("govgoal"));
    r.key(&mut c, "Shift+r");
    r.key(&mut c, "Enter");
    r.save(&mut c, "govanarchy");
    for _ in 0..8 {
        if matches!(c.modal, Some(Modal::Government { .. })) {
            break;
        }
        r.research(&mut c);
        r.endturn(&mut c);
    }
    assert_eq!(title(&c), "Choose a government");
    // The chooser is required: Escape and an empty Enter keep it; reload restores it.
    r.key(&mut c, "Escape");
    r.key(&mut c, "Enter");
    r.button(&mut c, "Save pending choice");
    r.entry(&mut c, "govchooser");
    r.0.push(serde_json::json!({"op":"compare","name":"govchooser"}));
    r.button(&mut c, "Load checkpoint");
    r.entry(&mut c, "govchooser");
    assert_eq!(title(&c), "Choose a government");
    r.key(&mut c, "ArrowDown");
    r.key(&mut c, "Enter");
    assert_eq!(title(&c), "Review your rates");
    r.key(&mut c, "ArrowRight");
    r.key(&mut c, "Escape");
    r.button(&mut c, "Save pending choice");
    r.entry(&mut c, "govreview");
    r.0.push(serde_json::json!({"op":"compare","name":"govreview"}));
    r.button(&mut c, "Load checkpoint");
    r.entry(&mut c, "govreview");
    assert_eq!(title(&c), "Review your rates");
    r.key(&mut c, "Enter");
    assert_eq!(title(&c), serde_json::Value::Null);
    r.save(&mut c, "govfinal");
    assert_eq!(c.view().unwrap().rates.unwrap().government, 1);
    fs::write(
        out.join("route.json"),
        serde_json::to_vec_pretty(&r.0).unwrap(),
    )
    .unwrap();
}

#[test]
fn report_sorting_is_stable_numeric_and_text_aware() {
    use super::reports::{sort_table, Table, TableRow};
    let row = |name: &str, n: i64| TableRow {
        cells: vec![name.into()],
        keys: vec![(0, name.to_lowercase()), (n, String::new())],
        city: None,
        warn: false,
    };
    let mut t = Table {
        headers: vec!["City", "Size"],
        columns: vec![(0., 100.), (100., 40.)],
        rows: vec![
            row("beta", 9),
            row("Alpha", 10),
            row("gamma", 10),
            row("delta", 2),
        ],
        footer: vec![],
    };
    let names = |t: &Table| {
        t.rows
            .iter()
            .map(|r| r.cells[0].clone())
            .collect::<Vec<_>>()
    };
    sort_table(&mut t, 0, false);
    assert_eq!(
        names(&t),
        ["Alpha", "beta", "delta", "gamma"],
        "case-insensitive text"
    );
    sort_table(&mut t, 1, true);
    // 10 sorts above 9 (numeric, not text); equal sizes keep their previous order.
    assert_eq!(names(&t), ["Alpha", "gamma", "beta", "delta"]);
    sort_table(&mut t, 9, false);
    assert_eq!(names(&t).len(), 4, "an out-of-range column clamps");
}

#[test]
fn public_reports_open_from_keys_and_list_the_whole_world() {
    let f = Fixture::new();
    let mut c = f.client();
    button(&mut c, "Start game");
    c.required();
    key(&mut c, "ArrowDown");
    key(&mut c, "Enter");
    founding(&mut c);
    let before = c.checkpoint().unwrap();
    let revision = c.view().unwrap().revision;
    let has = |c: &Client, s: &str| {
        c.frame()
            .draws
            .iter()
            .any(|d| matches!(d, Draw::Text{text,..} if text.contains(s)))
    };
    // F8: the own city is ranked first among the world's cities.
    key(&mut c, "F8");
    assert!(matches!(
        c.modal,
        Some(Modal::Report {
            family: ReportFamily::RankedCities,
            ..
        })
    ));
    assert!(has(&c, "Athena"), "ranked rows list the own city");
    assert!(has(&c, "(you)"), "the own civilization is marked");
    // Switching tabs inside the report reaches Score and Demographics.
    key(&mut c, "F9");
    assert!(matches!(
        c.modal,
        Some(Modal::Report {
            family: ReportFamily::Score,
            ..
        })
    ));
    assert!(has(&c, "Standard score = 3"), "native score formula footer");
    key(&mut c, "F11");
    assert!(matches!(
        c.modal,
        Some(Modal::Report {
            family: ReportFamily::Demographics,
            ..
        })
    ));
    assert!(has(&c, "Population"));
    assert!(has(&c, "Treasury"));
    // Opening the public families mutates nothing.
    assert_eq!(c.checkpoint().unwrap(), before);
    assert_eq!(c.view().unwrap().revision, revision);
    key(&mut c, "Escape");
    assert!(c.modal.is_none());
}

#[test]
fn blocked_routes_give_a_visible_reason_during_required_choices() {
    let f = Fixture::new();
    let mut c = f.client();
    button(&mut c, "Start game");
    // The opening research choice is pending; the report and route keys are
    // refused, and the refusal must be VISIBLE (#283/#284: keys looked
    // "not connected" live because the status never changed).
    for k in ["F1", "F8", "F9", "F11", "F6"] {
        key(&mut c, k);
        assert!(
            c.status.contains("required choice"),
            "{k} refusal must be visible, status was: {}",
            c.status
        );
    }
    // Answer the choice; now the same keys work.
    c.required();
    key(&mut c, "ArrowDown");
    key(&mut c, "Enter");
    key(&mut c, "F8");
    assert!(matches!(
        c.modal,
        Some(Modal::Report {
            family: ReportFamily::RankedCities,
            ..
        })
    ));
    // With no choice pending, the toolbar Research button is not a dead
    // button: it shows the Science advisor (#292, live owner report).
    key(&mut c, "Escape");
    assert!(c.modal.is_none());
    button(&mut c, "Research");
    assert!(matches!(c.modal, Some(Modal::Science)), "{}", c.status);
}

#[test]
fn turn_feedback_banner_and_title_round() {
    let f = Fixture::new();
    let mut c = f.client();
    button(&mut c, "Start game");
    // The title, at the right end of the menu bar (layout B, #310), carries
    // the round once the game runs.
    let title = |c: &Client| {
        c.frame().draws.iter().find_map(|d| match d {
            Draw::Text { y, text, .. }
                if *y < chrome::MENU_H && text.starts_with("Original rules") =>
            {
                Some(text.clone())
            }
            _ => None,
        })
    };
    assert!(title(&c).unwrap().contains("· Turn 1"), "{:?}", title(&c));
    // While the opponents run, a framed strip explains the wait (#283).
    c.advancing = true;
    let has = |c: &Client, s: &str| {
        c.frame()
            .draws
            .iter()
            .any(|d| matches!(d, Draw::Text{text,..} if text.starts_with(s)))
    };
    assert!(has(&c, "Opponents are moving"));
    c.advancing = false;
    assert!(!has(&c, "Opponents are moving"));
}

#[test]
fn setup_memory_round_trips_and_survives_garbage() {
    let f = Fixture::new();
    let saves = f.saves();
    let home = f.rules_root();
    // First client: change the choices, start a game, memory is written.
    let mut c = Client::new(home.to_string_lossy().into_owned(), saves.clone());
    c.civs = 6;
    c.difficulty = 4;
    c.size = 1;
    button(&mut c, "Start game");
    assert!(c.loaded.is_some(), "{}", c.status);
    let written = super::storage::load_setup_memory(&saves).unwrap();
    assert_eq!(written.format, "totengine-setup 1");
    assert_eq!(written.home, home.to_string_lossy());
    assert_eq!((written.civs, written.difficulty, written.size), (6, 4, 1));
    // Second client with no explicit home: setup is prefilled from memory.
    let c2 = Client::new(String::new(), saves.clone());
    assert_eq!(c2.home, home.to_string_lossy(), "ToT dir remembered");
    assert_eq!((c2.civs, c2.difficulty, c2.size), (6, 4, 1));
    // An explicit home still wins over memory (CIVTOT_HOME semantics).
    let c3 = Client::new("/elsewhere".into(), saves.clone());
    assert_eq!(c3.home, "/elsewhere");
    // Garbage or out-of-range memory falls back to the defaults, quietly.
    let path = super::storage::setup_memory_path(&saves).unwrap();
    std::fs::write(&path, b"{\"format\":\"nope\"}").unwrap();
    let c4 = Client::new(String::new(), saves.clone());
    assert_eq!(
        (c4.home.as_str(), c4.civs, c4.difficulty, c4.size),
        ("", 5, 2, 0)
    );
    std::fs::write(&path, b"not json at all").unwrap();
    let c5 = Client::new(String::new(), saves.clone());
    assert_eq!(c5.civs, 5);
}

#[test]
fn reports_open_switch_sort_drill_down_and_return_without_mutation() {
    let f = Fixture::new();
    let mut c = f.client();
    button(&mut c, "Start game");
    // A required choice blocks the reports; nothing opens.
    key(&mut c, "Escape");
    key(&mut c, "F1");
    assert!(c.modal.is_none());
    assert!(c.status.contains("required choice"), "{}", c.status);
    c.required();
    key(&mut c, "ArrowDown");
    key(&mut c, "Enter");
    // No city yet: an empty report says so and has nothing to open.
    key(&mut c, "F1");
    assert!(matches!(c.modal, Some(Modal::Report { .. })));
    assert!(c
        .frame()
        .draws
        .iter()
        .any(|d| matches!(d, Draw::Text{text,..} if text == "Nothing to report yet.")));
    key(&mut c, "Enter");
    assert!(c.status.contains("Select a row"), "{}", c.status);
    key(&mut c, "Escape");
    founding(&mut c);
    let before = c.checkpoint().unwrap();
    let revision = c.view().unwrap().revision;
    key(&mut c, "F1");
    let title = |c: &Client| {
        c.frame().draws.iter().find_map(|d| match d {
            Draw::Text {
                x, y, text, size, ..
            } if (*x, *y, *size) == (212., 125., style::TYPE_TITLE) => Some(text.clone()),
            _ => None,
        })
    };
    assert_eq!(title(&c).as_deref(), Some("Reports"));
    let has = |c: &Client, s: &str| {
        c.frame()
            .draws
            .iter()
            .any(|d| matches!(d, Draw::Text{text,..} if text.starts_with(s)))
    };
    assert!(has(&c, "Athena"), "own city row");
    key(&mut c, "F2");
    assert!(matches!(
        c.modal,
        Some(Modal::Report {
            family: ReportFamily::Defense,
            ..
        })
    ));
    key(&mut c, "F4");
    assert!(matches!(
        c.modal,
        Some(Modal::Report {
            family: ReportFamily::Attitude,
            ..
        })
    ));
    assert!(has(&c, "Athena"), "attitude rows are own cities");
    key(&mut c, "F7");
    assert!(matches!(
        c.modal,
        Some(Modal::Report {
            family: ReportFamily::Wonders,
            ..
        })
    ));
    assert!(has(&c, "Nothing to report yet."), "no wonders at the start");
    key(&mut c, "F5");
    assert!(matches!(
        c.modal,
        Some(Modal::Report {
            family: ReportFamily::Trade,
            ..
        })
    ));
    assert!(has(&c, "Total: taxes"), "trade totals");
    // `S` and a header click sort; the same header flips direction.
    key(&mut c, "s");
    assert!(matches!(
        c.modal,
        Some(Modal::Report {
            sort: 1,
            descending: false,
            ..
        })
    ));
    button(&mut c, "Tax");
    assert!(matches!(
        c.modal,
        Some(Modal::Report {
            sort: 1,
            descending: true,
            ..
        })
    ));
    // A header can be focused and activated with the keyboard.
    let headers: Vec<_> = c.buttons().iter().map(|b| b.label.clone()).collect();
    assert!(
        headers.iter().any(|l| l.starts_with("Tax v")),
        "{headers:?}"
    );
    // Drill into the city and come back to the same report and sort.
    key(&mut c, "Enter");
    assert!(c.city_open && c.modal.is_none(), "{}", c.status);
    key(&mut c, "Escape");
    assert!(!c.city_open);
    assert!(matches!(
        c.modal,
        Some(Modal::Report {
            family: ReportFamily::Trade,
            sort: 1,
            descending: true,
            ..
        })
    ));
    key(&mut c, "Escape");
    assert!(c.modal.is_none());
    // Defense has no city to open.
    key(&mut c, "F2");
    key(&mut c, "Enter");
    assert!(c.status.contains("no city"), "{}", c.status);
    key(&mut c, "Escape");
    // The toolbar button opens the same report; none of this changed the game.
    button(&mut c, "City status");
    assert!(matches!(
        c.modal,
        Some(Modal::Report {
            family: ReportFamily::CityStatus,
            ..
        })
    ));
    key(&mut c, "Escape");
    assert_eq!(c.checkpoint().unwrap(), before);
    assert_eq!(c.view().unwrap().revision, revision);
}

/// Records the reports route (F1/F2/F5, sorting, city drill-down and return) for
/// the real-input runners; every inspection must leave the checkpoint unchanged.
#[test]
#[ignore = "install-backed recorded reports input route"]
fn generated_reports_route() {
    let out = PathBuf::from(std::env::var("TOTENGINE_REPORTS_OUT").expect("private output"));
    fs::create_dir_all(&out).unwrap();
    let home = std::env::var("CIVTOT_HOME").unwrap();
    let mut c = seeded_client(home.clone(), out.clone());
    start(&mut c);
    founding(&mut c);
    key(&mut c, "Ctrl+s");
    edit(&mut c, "repstart");
    key(&mut c, "Enter");
    let mut c = seeded_client(home, out.clone());
    button(&mut c, "Load engine checkpoint");
    edit(&mut c, "repstart");
    key(&mut c, "Enter");
    assert!(c.loaded.is_some(), "{}", c.status);
    let mut r = Rec(Vec::new());
    r.key(&mut c, "F1");
    assert_eq!(title(&c), "Reports");
    r.key(&mut c, "F2");
    r.key(&mut c, "F4");
    r.key(&mut c, "F7");
    r.key(&mut c, "F5");
    r.key(&mut c, "s");
    r.button(&mut c, "Tax ^");
    assert!(matches!(
        c.modal,
        Some(Modal::Report {
            family: ReportFamily::Trade,
            sort: 1,
            descending: true,
            ..
        })
    ));
    // Enter opens the selected city; Escape returns to the same report.
    r.key(&mut c, "Enter");
    assert_eq!(title(&c), serde_json::Value::Null);
    assert!(c.city_open);
    r.key(&mut c, "Escape");
    assert_eq!(title(&c), "Reports");
    assert!(matches!(
        c.modal,
        Some(Modal::Report {
            family: ReportFamily::Trade,
            sort: 1,
            descending: true,
            ..
        })
    ));
    r.key(&mut c, "Escape");
    r.button(&mut c, "City status");
    assert_eq!(title(&c), "Reports");
    r.key(&mut c, "Escape");
    r.save(&mut c, "repdone");
    let read = |n: &str| fs::read(out.join(format!("{n}.cth"))).unwrap();
    assert_eq!(read("repdone"), read("repstart"), "reports never mutate");
    fs::write(
        out.join("route.json"),
        serde_json::to_vec_pretty(&r.0).unwrap(),
    )
    .unwrap();
}

fn synthetic_pedia() -> String {
    // Authored for the tests; the real Describe.txt text is never committed.
    let mut text = String::from("; synthetic\r\n@ADVANCE_DESCRIPTIONS\r\n@@ADVANCE_INDEX\r\n");
    for id in 0..100 {
        text += &if id < 5 {
            format!("{id},\t; a{id}\r\n")
        } else {
            "-1,\r\n".into()
        };
    }
    text += "-2,\r\n";
    for n in 0..5 {
        text += &format!(
            "\r\n@@First topic {n}\r\n{}\r\n\r\nSecond paragraph {n}.\r\n",
            "word ".repeat(400)
        );
    }
    text +=
        "\r\n@CONCEPT_DESCRIPTIONS\r\n@@Idea one\r\nShort.\r\n\r\n@@Idea two\r\nAlso short.\r\n";
    text += "@This must be here to terminate search!!!\r\n";
    text
}

#[test]
fn civilopedia_wraps_text_and_is_unavailable_without_a_describe_file() {
    use super::pedia::wrap;
    assert_eq!(wrap("aa bb cc dd", 5), ["aa bb", "cc dd"]);
    assert_eq!(
        wrap("averyveryverylongword x", 5),
        ["averyveryverylongword", "x"]
    );
    assert!(wrap("   ", 10).is_empty());
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    button(&mut c, "Civilopedia");
    assert!(c.modal.is_none());
    assert!(c.status.contains("not available"), "{}", c.status);
}

#[test]
fn civilopedia_browses_reads_and_returns_without_touching_the_game() {
    let f = Fixture::new();
    fs::write(f.0.join("Describe.txt"), synthetic_pedia()).unwrap();
    let mut c = f.client();
    start(&mut c);
    founding(&mut c);
    let before = c.checkpoint().unwrap();
    button(&mut c, "Civilopedia");
    assert!(matches!(
        c.modal,
        Some(Modal::Pedia {
            section: 0,
            reading: false,
            ..
        })
    ));
    let shown = |c: &Client, s: &str| {
        c.frame()
            .draws
            .iter()
            .any(|d| matches!(d, Draw::Text{text,..} if text.starts_with(s)))
    };
    assert!(shown(&c, "First topic 0"));
    key(&mut c, "ArrowRight");
    assert!(
        matches!(c.modal, Some(Modal::Pedia { section: 6, .. })),
        "only categories with text appear"
    );
    key(&mut c, "ArrowDown");
    key(&mut c, "Enter");
    assert!(matches!(
        c.modal,
        Some(Modal::Pedia {
            reading: true,
            selected: 1,
            ..
        })
    ));
    assert!(shown(&c, "Idea two") && shown(&c, "Also short."));
    key(&mut c, "Escape");
    assert!(matches!(c.modal, Some(Modal::Pedia { reading: false, .. })));
    // A tab click switches category; long entries scroll.
    button(&mut c, "Advances");
    key(&mut c, "Enter");
    key(&mut c, "PageDown");
    assert!(matches!(c.modal, Some(Modal::Pedia { scroll, .. }) if scroll > 0));
    key(&mut c, "Escape");
    key(&mut c, "Escape");
    assert!(c.modal.is_none());
    assert_eq!(c.checkpoint().unwrap(), before);
}

#[test]
fn research_help_describes_the_highlighted_advance_and_returns_to_the_choice() {
    let f = Fixture::new();
    fs::write(f.0.join("Describe.txt"), synthetic_pedia()).unwrap();
    let mut c = f.client();
    button(&mut c, "Start game");
    assert!(matches!(c.modal, Some(Modal::Research)));
    let before = c.checkpoint().unwrap();
    // Nothing is highlighted yet.
    button(&mut c, "Civilopedia");
    assert!(matches!(c.modal, Some(Modal::Research)));
    assert!(c.status.contains("Select a research topic"), "{}", c.status);
    key(&mut c, "ArrowDown");
    let chosen = c.choice;
    button(&mut c, "Civilopedia");
    assert!(
        matches!(
            c.modal,
            Some(Modal::Pedia {
                reading: true,
                section: 0,
                ..
            })
        ),
        "{}",
        c.status
    );
    key(&mut c, "Escape");
    assert!(matches!(c.modal, Some(Modal::Research)));
    assert_eq!(
        c.choice, chosen,
        "the highlighted topic survives the lookup"
    );
    assert_eq!(c.checkpoint().unwrap(), before);
    key(&mut c, "Enter");
    assert!(c.modal.is_none(), "{}", c.status);
    assert_eq!(c.status, "research-set");
}

/// Records the Civilopedia route (research help, category browsing, reading and
/// scrolling, Esc paths) for the real-input runners against the installed text.
#[test]
#[ignore = "install-backed recorded Civilopedia input route"]
fn generated_pedia_route() {
    let out = PathBuf::from(std::env::var("TOTENGINE_PEDIA_OUT").expect("private output"));
    fs::create_dir_all(&out).unwrap();
    let home = std::env::var("CIVTOT_HOME").unwrap();
    let mut c = seeded_client(home.clone(), out.clone());
    button(&mut c, "Start game");
    assert!(matches!(c.modal, Some(Modal::Research)));
    button(&mut c, "Save pending");
    edit(&mut c, "pedstart");
    key(&mut c, "Enter");
    let mut c = seeded_client(home, out.clone());
    button(&mut c, "Load engine checkpoint");
    edit(&mut c, "pedstart");
    key(&mut c, "Enter");
    assert!(matches!(c.modal, Some(Modal::Research)), "{}", c.status);
    assert!(
        c.loaded.as_ref().unwrap().resources.pedia.is_some(),
        "the install's Describe.txt must load"
    );
    let mut r = Rec(Vec::new());
    // Research help describes the highlighted advance and returns to the choice.
    r.key(&mut c, "ArrowDown");
    r.button(&mut c, "Civilopedia");
    assert_eq!(title(&c), "Civilopedia");
    assert!(matches!(c.modal, Some(Modal::Pedia { reading: true, .. })));
    r.key(&mut c, "Escape");
    assert_eq!(title(&c), "Choose research");
    r.key(&mut c, "Enter");
    assert_eq!(title(&c), serde_json::Value::Null);
    // The main lookup: categories, reading, scrolling, back.
    r.button(&mut c, "Civilopedia");
    assert_eq!(title(&c), "Civilopedia");
    r.key(&mut c, "ArrowRight");
    r.key(&mut c, "ArrowRight");
    r.key(&mut c, "ArrowDown");
    r.key(&mut c, "Enter");
    assert!(matches!(c.modal, Some(Modal::Pedia { reading: true, .. })));
    r.key(&mut c, "PageDown");
    r.key(&mut c, "Escape");
    r.key(&mut c, "Escape");
    assert_eq!(title(&c), serde_json::Value::Null);
    r.save(&mut c, "pedend");
    fs::write(
        out.join("route.json"),
        serde_json::to_vec_pretty(&r.0).unwrap(),
    )
    .unwrap();
}

#[test]
fn toolbar_cities_opens_a_city_window_or_explains_why_not() {
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    // No cities yet: a visible notice, not a silent status line (#292).
    let before = c.checkpoint().unwrap();
    button(&mut c, "Cities");
    assert!(
        matches!(
            &c.modal,
            Some(Modal::Notice {
                title: "Cities",
                ..
            })
        ),
        "{}",
        c.status
    );
    assert!(c
        .frame()
        .draws
        .iter()
        .any(|d| matches!(d, Draw::Text{text,..} if text == "You have no cities yet.")));
    key(&mut c, "Escape");
    assert!(c.modal.is_none());
    assert_eq!(c.checkpoint().unwrap(), before, "the notice is inert");
    // With a city, Cities opens that city's window directly.
    founding(&mut c);
    assert!(!c.city_open);
    let snapshot = c.checkpoint().unwrap();
    button(&mut c, "Cities");
    assert!(c.city_open, "{}", c.status);
    assert_eq!(c.city, Some(c.view().unwrap().cities[0]));
    assert_eq!(c.checkpoint().unwrap(), snapshot, "opening is inert");
}

#[test]
fn keyboard_help_opens_from_map_button_and_city_and_is_inert() {
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    let before = c.checkpoint().unwrap();
    let shows = |c: &Client, s: &str| {
        c.frame()
            .draws
            .iter()
            .any(|d| matches!(d, Draw::Text{text,..} if text == s))
    };
    key(&mut c, "?");
    assert!(matches!(c.modal, Some(Modal::Help)), "{}", c.status);
    assert!(shows(&c, "Keys"));
    assert!(shows(&c, "City status"));
    assert!(shows(&c, "This help (our addition)"));
    key(&mut c, "Escape");
    assert!(c.modal.is_none());
    button(&mut c, "Keys");
    assert!(matches!(c.modal, Some(Modal::Help)));
    key(&mut c, "Enter");
    assert!(c.modal.is_none());
    assert_eq!(c.checkpoint().unwrap(), before, "help is inert");
    founding(&mut c);
    button(&mut c, "Manage city");
    assert!(c.city_open);
    key(&mut c, "?");
    assert!(matches!(c.modal, Some(Modal::Help)));
    key(&mut c, "Escape");
    assert!(c.modal.is_none() && c.city_open);
}

#[test]
fn notice_log_keeps_each_turn_once_with_an_inert_overlay_and_paged_dialog() {
    let f = Fixture::new();
    let mut c = f.client();
    // The overlay is an opt-in enhancement (#284), off by default.
    assert!(!c.enhancements.notice_overlay);
    c.enhancements.notice_overlay = true;
    start(&mut c);
    founding(&mut c);
    let shows = |c: &Client, s: &str| {
        c.frame()
            .draws
            .iter()
            .any(|d| matches!(d, Draw::Text{text,..} if text.starts_with(s)))
    };
    assert!(c.notice_log.is_empty());
    assert!(!shows(&c, "Turn 1 — new messages"));
    // Play until the engine reports notices (growth, production, discovery).
    for _ in 0..30 {
        research(&mut c);
        if c.modal.is_some() {
            key(&mut c, "Escape");
        }
        key(&mut c, "Ctrl+n");
        if matches!(c.modal, Some(Modal::Confirm { .. })) {
            key(&mut c, "Enter");
        }
        ready(&mut c);
        if !c.notice_log.is_empty() {
            break;
        }
    }
    assert!(!c.notice_log.is_empty(), "no notices in 30 turns");
    let turn = c.notice_turn.unwrap();
    let logged = c.notice_log.len();
    // Visible as an overlay on the map, and logged once however often the
    // client handles input during the same turn.
    research(&mut c);
    assert!(
        c.modal.is_some() || shows(&c, &format!("Turn {turn} — new messages")),
        "overlay missing"
    );
    if c.modal.is_some() {
        key(&mut c, "Escape");
    }
    key(&mut c, "c");
    key(&mut c, "c");
    assert_eq!(c.notice_log.len(), logged, "a turn is logged once");
    let before = c.checkpoint().unwrap();
    // A click on the overlay body dismisses it and is never a map order.
    if c.notice_overlay() {
        let (_, (x, y, _, _)) = c.notice_overlay_rect();
        input(
            &mut c,
            Input::Click {
                x: x + 30.,
                y: y + 20.,
            },
        );
        assert!(!c.notice_overlay());
    }
    // The log dialog lists the entries newest first.
    button(&mut c, &format!("Messages ({logged})"));
    assert!(matches!(c.modal, Some(Modal::Messages)));
    assert!(shows(&c, &format!("Turn {turn}")));
    assert!(shows(&c, &c.notice_log.last().unwrap().1.clone()));
    key(&mut c, "Escape");
    assert!(c.modal.is_none());
    assert_eq!(
        c.checkpoint().unwrap(),
        before,
        "messages are presentation-only"
    );
    // A new game starts an empty log.
    c.reset_notices();
    assert!(c.notice_log.is_empty() && !c.notice_overlay());
}

#[test]
fn enhancements_default_off_toggle_persist_and_never_touch_the_game() {
    let f = Fixture::new();
    let mut c = f.client();
    assert!(!c.enhancements.notice_overlay);
    assert!(!c.enhancements.confirm_moves);
    let shows = |c: &Client, s: &str| {
        c.frame()
            .draws
            .iter()
            .any(|d| matches!(d, Draw::Text{text,..} if text.starts_with(s)))
    };
    // Reachable from the setup screen, before any game exists.
    button(&mut c, "Enhancements…");
    assert!(matches!(c.modal, Some(Modal::Settings)));
    assert!(shows(&c, "Lists a turn's new notices"));
    button(&mut c, "[ ] Confirm every move");
    assert!(c.enhancements.confirm_moves, "{}", c.status);
    assert!(shows(&c, "[x] Confirm every move"));
    key(&mut c, "Escape");
    assert!(c.modal.is_none());
    // Persisted beside the saves: a second client sees the same choice.
    let c2 = f.client();
    assert!(c2.enhancements.confirm_moves);
    assert!(!c2.enhancements.notice_overlay);
    // In game: Settings replaces the old bottom-bar toggle and is inert.
    start(&mut c);
    let before = c.checkpoint().unwrap();
    button(&mut c, "Settings");
    assert!(matches!(c.modal, Some(Modal::Settings)));
    button(&mut c, "[ ] New-messages overlay on the map");
    button(&mut c, "[x] Confirm every move");
    assert!(c.enhancements.notice_overlay && !c.enhancements.confirm_moves);
    key(&mut c, "Escape");
    assert_eq!(
        c.checkpoint().unwrap(),
        before,
        "enhancements never reach the game"
    );
    // Garbage on disk means "all off", never a failed start.
    let path = super::storage::enhancements_path(&c.saves).unwrap();
    fs::write(&path, b"{not json").unwrap();
    let c3 = f.client();
    assert!(!c3.enhancements.notice_overlay && !c3.enhancements.confirm_moves);
}

#[test]
fn mouse_wheel_scrolls_lists_only_and_never_touches_the_game() {
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    founding(&mut c);
    let wheel = |c: &mut Client, x: f64, delta: i32| input(c, Input::Wheel { x, y: 300., delta });
    // Over the map the wheel does nothing: no pan, no selection, no order.
    let before = c.checkpoint().unwrap();
    let (pan, frame) = (c.pan, c.frame().draws.len());
    wheel(&mut c, 300., 1);
    wheel(&mut c, 300., -1);
    assert_eq!((c.pan, c.frame().draws.len()), (pan, frame));
    assert_eq!(c.checkpoint().unwrap(), before);
    // More than one page of checkpoints now pages, by buttons and wheel.
    for n in 0..=LOAD_ROWS {
        key(&mut c, "Ctrl+s");
        edit(&mut c, &format!("wheel{n:02}"));
        key(&mut c, "Enter");
        assert!(c.modal.is_none(), "{}", c.status);
    }
    key(&mut c, "Ctrl+l");
    let slots = |c: &Client| {
        c.buttons()
            .into_iter()
            .filter(|b| matches!(b.action, Action::Slot(_)))
            .count()
    };
    assert_eq!(slots(&c), LOAD_ROWS);
    assert!(c.buttons().iter().any(|b| b.label == "Next page"));
    wheel(&mut c, 400., 1);
    assert_eq!(c.page, 1);
    assert!(slots(&c) >= 1);
    wheel(&mut c, 400., 5);
    assert_eq!(c.page, 1, "the wheel stops at the last page");
    wheel(&mut c, 400., -1);
    assert_eq!(c.page, 0);
    key(&mut c, "Escape");
    // The Civilopedia list moves one page per notch (it needs the
    // install's Describe.txt, which the asset-free fixture lacks).
    button(&mut c, "Civilopedia");
    if c.modal.is_some() {
        wheel(&mut c, 400., 1);
        assert!(matches!(c.modal, Some(Modal::Pedia { selected, .. }) if selected > 0));
        wheel(&mut c, 400., -1);
        assert!(matches!(c.modal, Some(Modal::Pedia { selected: 0, .. })));
        key(&mut c, "Escape");
    }
    // In the city window only the panel scrolls; one page stays put.
    button(&mut c, "Manage city");
    wheel(&mut c, 300., 1);
    wheel(&mut c, 900., 1);
    assert_eq!(c.city_panel_page, 0);
    assert!(c.city_open);
    assert_eq!(
        c.checkpoint().unwrap(),
        before,
        "the wheel is presentation-only"
    );
}

#[test]
fn city_panel_lists_sort_by_presentation_order_only() {
    use super::city_panels::CitySort;
    use engine::city_info::CityUnitInfo;
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    founding(&mut c);
    button(&mut c, "Manage city");
    let before = c.checkpoint().unwrap();
    let sort_label = |c: &Client| {
        c.buttons()
            .into_iter()
            .find(|b| matches!(b.action, Action::CityPanelSort))
            .map(|b| b.label)
    };
    button(&mut c, "Support");
    assert_eq!(sort_label(&c).as_deref(), Some("Sort: listed"));
    for want in ["Sort: name", "Sort: shields", "Sort: food", "Sort: listed"] {
        c.action(Action::CityPanelSort).unwrap();
        assert_eq!(sort_label(&c).as_deref(), Some(want));
    }
    button(&mut c, "Improvements");
    assert_eq!(sort_label(&c).as_deref(), Some("Sort: listed"));
    c.action(Action::CityPanelSort).unwrap();
    c.action(Action::CityPanelSort).unwrap();
    assert_eq!(sort_label(&c).as_deref(), Some("Sort: upkeep"));
    button(&mut c, "Happy");
    assert_eq!(sort_label(&c), None, "Happy has no list to sort");
    assert_eq!(c.checkpoint().unwrap(), before, "sorting issues no order");
    // Numeric orders put the largest first; ties keep the listed order.
    let unit = |slot, shields, food| CityUnitInfo {
        slot,
        id: slot as u32,
        type_id: 0,
        position: (0, 0),
        shields: Some(shields),
        food: Some(food),
    };
    let units = [unit(1, 0, 1), unit(2, 1, 0), unit(3, 1, 1)];
    let order =
        |c: &Client| -> Vec<usize> { c.sorted_units(&units).iter().map(|u| u.slot).collect() };
    button(&mut c, "Support");
    assert_eq!(order(&c), [1, 2, 3]);
    c.city_panel_sort = CitySort::Shields;
    assert_eq!(order(&c), [2, 3, 1]);
    c.city_panel_sort = CitySort::Food;
    assert_eq!(order(&c), [1, 3, 2]);
    c.city_panel_sort = CitySort::Name;
    assert_eq!(order(&c), [1, 2, 3], "same names keep the listed order");
}

#[test]
fn map_tooltips_are_opt_in_describe_the_filtered_view_and_never_touch_the_game() {
    use super::tips::moves_text;
    assert_eq!(
        [moves_text(3), moves_text(4), moves_text(2), moves_text(0)],
        ["1", "1 1/3", "2/3", "0"]
    );
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    // Off by default: the frame carries no tips and serializes none.
    assert!(!c.enhancements.map_tooltips);
    assert!(c.frame().tips.is_empty());
    assert!(!serde_json::to_string(&c.frame())
        .unwrap()
        .contains("\"tips\""));
    let before = c.checkpoint().unwrap();
    button(&mut c, "Settings");
    button(&mut c, "[ ] Map tooltips");
    assert!(c.enhancements.map_tooltips);
    key(&mut c, "Escape");
    assert!(c.modal.is_none());
    let v = c.view().unwrap();
    let own = v
        .observation
        .units
        .iter()
        .find(|u| u.owner == v.observation.civ)
        .unwrap()
        .clone();
    let frame = c.frame();
    assert!(!frame.tips.is_empty());
    let (px, py) = (
        c.pan.0 + f64::from(own.x) * 32. * c.zoom,
        c.pan.1 + f64::from(own.y) * 16. * c.zoom,
    );
    let lines = |t: &Tip| -> Vec<String> {
        t.draws
            .iter()
            .filter_map(|d| match d {
                Draw::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    };
    // Both the unit sprite (above the tile) and the tile diamond describe it.
    for (x, y) in [(px, py - 30.), (px + 20., py)] {
        let tip = tip_at(&frame.tips, x, y).expect("tip under the unit");
        let text = lines(tip);
        assert!(
            text[0].ends_with(&format!("({}, {})", own.x, own.y)),
            "{text:?}"
        );
        assert!(
            text.iter()
                .any(|l| l.contains(&format!("#{} · moves", own.index))),
            "{text:?}"
        );
    }
    // Every tip is a known tile and its box stays on the map.
    for t in &frame.tips {
        for d in &t.draws {
            if let Draw::Rect { x, y, w, h, .. } = d {
                assert!(*x >= 12. && x + w <= 752. && *y >= 100. && y + h <= 680.);
            }
        }
    }
    let o = &v.observation;
    let unknown = o.seen.iter().position(Option::is_none).unwrap();
    let uy = (unknown / (usize::from(o.w2) / 2)) as u16;
    let ux = ((unknown % (usize::from(o.w2) / 2)) * 2) as u16 + (uy & 1);
    assert!(
        c.tip_lines(&v, ux, uy).is_none(),
        "unknown tiles have no tip"
    );
    // Dialogs and the city window hide the map tips.
    button(&mut c, "Settings");
    assert!(c.frame().tips.is_empty());
    key(&mut c, "Escape");
    assert_eq!(
        c.checkpoint().unwrap(),
        before,
        "tooltips are presentation-only"
    );
    founding(&mut c);
    button(&mut c, "Manage city");
    assert!(c.frame().tips.is_empty());
    key(&mut c, "Escape");
    let city = c.frame();
    let tip = tip_at(&city.tips, px, py - 30.).expect("city tip");
    assert!(lines(tip)
        .iter()
        .any(|l| l.starts_with("Athena · size 1 · yours")));
    // A file from before the option existed still loads, with it off.
    let saved = storage::load_enhancements(&f.saves());
    assert!(saved.map_tooltips);
    let path = storage::enhancements_path(&f.saves()).unwrap();
    std::fs::write(
        &path,
        br#"{"format":"totengine-enhancements 1","notice_overlay":false,"confirm_moves":false}"#,
    )
    .unwrap();
    assert!(!storage::load_enhancements(&f.saves()).map_tooltips);
}

#[test]
fn path_preview_is_opt_in_knowledge_bounded_and_never_touches_the_game() {
    use super::paths::{logical, plan, stored};
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    assert!(!c.enhancements.path_preview);
    assert!(c.frame().route.is_empty());
    let before = c.checkpoint().unwrap();
    let revision = c.revision;
    button(&mut c, "Settings");
    button(&mut c, "[ ] Path preview with turn counts");
    assert!(c.enhancements.path_preview && !c.enhancements.map_tooltips);
    key(&mut c, "Escape");
    let v = c.view().unwrap();
    let o = v.observation.clone();
    let active = c.active.expect("a unit is selected to move");
    let unit = o
        .units
        .iter()
        .find(|u| u.index == active.slot)
        .unwrap()
        .clone();
    let rules = c.loaded.as_ref().unwrap().resources.rules.clone();

    // Search rules on a controlled copy of the view: every tile known, flat
    // land of move cost 1, the unit with one full move (three thirds).
    let flat = (0..rules.base_terrain_names().len())
        .find(|&t| t != 10 && rules.terrain_move_cost(t) == Some(1))
        .unwrap() as u8;
    let mut w = o.clone();
    for t in w.seen.iter_mut() {
        *t = Some(engine::player::TileView {
            terrain: flat,
            river: false,
            special: false,
            road: false,
            rail: false,
        });
    }
    w.units.retain(|u| u.owner == w.civ);
    w.cities.clear();
    let mut u = unit.clone();
    (u.x, u.y, u.moves_left) = (20, 20, 3);
    assert_eq!(rules.unit_move(u.type_id as usize), Some(1), "Settlers");
    let half = usize::from(w.w2) / 2;
    let at = |p: &super::paths::Plan, x: u16, y: u16| {
        p.reach[usize::from(y) * half + usize::from(x) / 2]
    };
    let p = plan(&rules, &w, &u);
    assert_eq!(at(&p, 22, 20).unwrap().turns, 0, "one step this turn");
    assert_eq!(at(&p, 24, 20).unwrap().turns, 1);
    assert_eq!(
        at(&p, 30, 20).unwrap().turns,
        4,
        "five steps, four turn ends"
    );
    // The route walks back to the unit along parents.
    let mut steps = 0;
    let mut i = stored(&w, 30, 20);
    while let Some(parent) = p.reach[i].unwrap().parent {
        i = parent;
        steps += 1;
    }
    assert_eq!((i, steps), (p.origin, 5));
    // Roads: three tiles per move; railroad: free.
    for t in w.seen.iter_mut().flatten() {
        t.road = true;
    }
    let p = plan(&rules, &w, &u);
    assert_eq!(at(&p, 26, 20).unwrap().turns, 0);
    assert_eq!(at(&p, 28, 20).unwrap().turns, 1);
    for t in w.seen.iter_mut().flatten() {
        t.rail = true;
    }
    assert_eq!(at(&plan(&rules, &w, &u), 60, 20).unwrap().turns, 0);
    for t in w.seen.iter_mut().flatten() {
        (t.road, t.rail) = (false, false);
    }
    // Unknown tiles and ocean wall the land unit in; a gap lets it through.
    for i in 0..w.seen.len() {
        let (x, _) = logical(&w, i);
        if x == 24 || x == 25 {
            w.seen[i] = None;
        }
    }
    let p = plan(&rules, &w, &u);
    assert!(at(&p, 30, 20).is_none(), "unknown column blocks");
    for i in 0..w.seen.len() {
        let (x, y) = logical(&w, i);
        if x == 24 || x == 25 {
            w.seen[i] = Some(engine::player::TileView {
                terrain: if y == 22 { flat } else { 10 },
                river: false,
                special: false,
                road: false,
                rail: false,
            });
        }
    }
    let gap = at(&plan(&rules, &w, &u), 30, 20).expect("around through the gap");
    assert!(gap.turns > 4);
    // A visible foreign unit blocks its tile and its zone of control ends
    // the turn when moving between tiles next to it.
    let mut z = w.clone();
    for t in z.seen.iter_mut() {
        if t.is_none() || t.unwrap().terrain == 10 {
            *t = w.seen[stored(&w, 20, 20)];
        }
    }
    u.moves_left = 9;
    let free = at(&plan(&rules, &z, &u), 24, 20).unwrap();
    assert_eq!((free.turns, free.left), (0, 3));
    let mut enemy = u.clone();
    (enemy.owner, enemy.x, enemy.y) = (z.civ % 7 + 1, 23, 19);
    z.units.push(enemy);
    let p = plan(&rules, &z, &u);
    assert!(at(&p, 23, 19).is_none(), "foreign unit tile is not a route");
    let zoc = at(&p, 24, 20).unwrap();
    assert_eq!(
        (zoc.turns, zoc.left),
        (0, 0),
        "zone of control ends the move"
    );
    let next = at(&p, 26, 20).unwrap();
    assert_eq!(next.turns, 1, "the next step waits a turn");

    // On the client: tips carry the path line and route steps, tooltips off.
    let frame = c.frame();
    assert!(!frame.tips.is_empty());
    assert!(!frame.route.is_empty());
    let texts = |draws: &[Draw]| -> Vec<String> {
        draws
            .iter()
            .filter_map(|d| match d {
                Draw::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    };
    let real = plan(&rules, &o, &unit);
    let mut checked = 0;
    for t in &frame.tips {
        let lines = texts(&t.draws);
        assert_eq!(lines.len(), 1, "path line only: {lines:?}");
        assert!(lines[0].starts_with("Path: "), "{lines:?}");
        if t.route.is_some() {
            let all = tip_draws(&frame.route, t);
            assert!(all.len() > t.draws.len() || lines[0].contains("this turn"));
            checked += 1;
        }
    }
    assert!(checked > 0);
    let reachable = real.reach.iter().filter(|r| r.is_some()).count();
    assert!(reachable > 1, "the start area is reachable");
    // The unit's own tile has no tip; an unknown tile has none either.
    let (px, py) = (
        c.pan.0 + f64::from(unit.x) * 32. * c.zoom,
        c.pan.1 + f64::from(unit.y) * 16. * c.zoom,
    );
    assert!(tip_at(&frame.tips, px + 20., py).is_none());
    // Viewing mode and dialogs show no route.
    button(&mut c, "Settings");
    assert!(c.frame().route.is_empty());
    key(&mut c, "Escape");
    assert_eq!(c.checkpoint().unwrap(), before, "presentation only");
    assert!(
        c.revision > revision,
        "only the settings toggles changed it"
    );
    assert!(storage::load_enhancements(&f.saves()).path_preview);
}

#[test]
fn layout_b_menus_and_column_reach_every_command_and_never_touch_the_game() {
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    founding(&mut c);
    assert!(c.modal.is_none(), "{}", c.status);
    let before = c.checkpoint().unwrap();
    // The two button bars are gone: map-screen buttons are menu titles or
    // live in the right column (#310/#311).
    for b in c.buttons() {
        assert!(
            matches!(b.action, Action::Menu(_)) || b.x >= chrome::COLUMN.0,
            "stray map-screen button {} at ({}, {})",
            b.label,
            b.x,
            b.y
        );
    }
    let labels: Vec<String> = (0..chrome::MENUS.len())
        .flat_map(|m| c.menu_items(m).into_iter().map(|i| i.label))
        .collect();
    for want in [
        "Save game…",
        "Load game…",
        "New game…",
        "Settings…",
        "Keys…",
        "Close client",
        "Tax rates…",
        "Revolution…",
        "Cities",
        "Center view",
        "Zoom in",
        "Zoom out",
        "Standard zoom",
        "Found city",
        "Wait",
        "Skip turn",
        "Activate unit",
        "End player turn",
        "City status",
        "Defense",
        "Attitude",
        "Trade",
        "Science",
        "Research…",
        "Wonders of the World",
        "Top cities",
        "Civilization score",
        "Demographics",
        "Civilopedia…",
    ] {
        assert!(labels.iter().any(|l| l == want), "no menu entry {want}");
    }
    assert!(labels.iter().any(|l| l.starts_with("Messages")));
    assert!(labels.iter().any(|l| l.starts_with("Switch to")));
    // Every menu opens and closes by mouse; a click elsewhere only closes it
    // and never reaches the map underneath.
    let title = |c: &Client, m: usize| {
        c.buttons()
            .into_iter()
            .find(|b| matches!(b.action, Action::Menu(n) if n == m))
            .map(|b| (b.x + 4., b.y + 4.))
            .unwrap()
    };
    let inspect = c.inspect;
    for m in 0..chrome::MENUS.len() {
        let (x, y) = title(&c, m);
        input(&mut c, Input::Click { x, y });
        assert_eq!(c.menu, Some(m));
        assert!(c
            .frame()
            .draws
            .iter()
            .any(|d| matches!(d, Draw::Rect { color, .. } if color == style::MENU_DROP)));
        assert!(c.frame().tips.is_empty(), "no map tips under an open menu");
        input(&mut c, Input::Click { x: 380., y: 390. });
        assert_eq!(c.menu, None);
        assert!(c.modal.is_none());
    }
    assert_eq!(
        c.inspect, inspect,
        "the closing click did not reach the map"
    );
    // Keyboard: F10 opens Game, arrows move, Escape closes; other keys close
    // the menu and keep their meaning.
    key(&mut c, "F10");
    assert_eq!((c.menu, c.menu_row), (Some(0), Some(0)));
    key(&mut c, "ArrowRight");
    key(&mut c, "ArrowDown");
    assert_eq!((c.menu, c.menu_row), (Some(1), Some(1)));
    key(&mut c, "ArrowLeft");
    key(&mut c, "ArrowLeft");
    assert_eq!(c.menu, Some(chrome::MENUS.len() - 1));
    key(&mut c, "Escape");
    assert_eq!(c.menu, None);
    key(&mut c, "F10");
    key(&mut c, "?");
    assert_eq!(c.menu, None);
    assert!(matches!(c.modal, Some(Modal::Help)));
    key(&mut c, "Escape");
    // Enter runs the highlighted entry: Game > Save game… opens the dialog.
    key(&mut c, "F10");
    key(&mut c, "Enter");
    assert!(matches!(c.modal, Some(Modal::Save)), "{}", c.status);
    key(&mut c, "Escape");
    // Menus only open over the map, never over a dialog.
    key(&mut c, "?");
    key(&mut c, "F10");
    assert_eq!(c.menu, None);
    key(&mut c, "Escape");
    // An unavailable entry explains itself and does nothing.
    c.active = None;
    button(&mut c, "Found city");
    assert_eq!(c.status, "Select an own unit first.");
    assert!(c.modal.is_none());
    // Menu entries run the same actions the keys do.
    button(&mut c, "City status");
    assert!(
        matches!(c.modal, Some(Modal::Report { .. })),
        "{}",
        c.status
    );
    key(&mut c, "Escape");
    button(&mut c, "Zoom in");
    assert!(c.zoom > 1.);
    button(&mut c, "Standard zoom");
    assert_eq!(c.zoom, 1.);
    // The right column: minimap, status and the end-of-turn slot.
    let shows = |c: &Client, s: &str| {
        c.frame()
            .draws
            .iter()
            .any(|d| matches!(d, Draw::Text { text, x, .. } if *x >= chrome::COLUMN.0 && text.starts_with(s)))
    };
    assert!(shows(&c, "Turn "));
    assert!(shows(&c, "Treasury: "));
    assert!(shows(&c, "Research: "));
    assert!(shows(&c, "Athena · size 1"));
    assert!(c
        .buttons()
        .iter()
        .any(|b| matches!(b.action, Action::End) && b.x >= chrome::COLUMN.0));
    // A minimap click only scrolls.
    let pan = c.pan;
    input(&mut c, Input::Click { x: 1000., y: 160. });
    assert_ne!(c.pan, pan);
    assert!(c.modal.is_none() && c.menu.is_none());
    assert_eq!(
        c.checkpoint().unwrap(),
        before,
        "menus, the column and the minimap never touch the game"
    );
}

/// No two buttons of the current screen overlap (layout B windows).
fn assert_no_overlaps(c: &Client, screen: &str) {
    let b = c.buttons();
    for (i, p) in b.iter().enumerate() {
        for q in &b[i + 1..] {
            let apart =
                p.x + p.w <= q.x || q.x + q.w <= p.x || p.y + p.h <= q.y || q.y + q.h <= p.y;
            assert!(apart, "{screen}: {} overlaps {}", p.label, q.label);
        }
    }
}

#[test]
fn layout_b_city_and_science_windows_rearrange_without_touching_the_game() {
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    founding(&mut c);
    let before = c.checkpoint().unwrap();
    let texts = |c: &Client| -> Vec<(f64, f64, String, String)> {
        c.frame()
            .draws
            .into_iter()
            .filter_map(|d| match d {
                Draw::Text {
                    x, y, text, color, ..
                } => Some((x, y, text, color)),
                _ => None,
            })
            .collect()
    };
    // #314: the city window is one framed window with a title bar, the
    // citizens strip, three columns and a footer.
    button(&mut c, "Manage city");
    assert!(c.city_open);
    assert!(matches!(c.buttons()[0].action, Action::CloseCity));
    assert!(texts(&c)
        .iter()
        .any(|(_, y, t, _)| (30.0..66.0).contains(y) && t.contains(" — population ")));
    // The menu bar stays visible but inert over the city window.
    assert!(texts(&c).iter().any(|(_, y, t, color)| *y < chrome::MENU_H
        && t == "Orders"
        && color == style::MENU_DISABLED));
    key(&mut c, "F10");
    assert_eq!(c.menu, None);
    for panel in ["Info", "Support", "Improvements", "Happy"] {
        button(&mut c, panel);
        assert_no_overlaps(&c, panel);
        for b in c.buttons() {
            assert!(
                b.x >= 4. && b.x + b.w <= 1020. && b.y >= 30. && b.y + b.h <= 742.,
                "{} leaves the city window",
                b.label
            );
        }
    }
    button(&mut c, "Info");
    assert!(texts(&c).iter().any(|(_, _, t, _)| t == "City resources"));
    assert!(texts(&c).iter().any(|(_, _, t, _)| t == "Food storage"));
    assert!(texts(&c).iter().any(|(_, _, t, _)| t.starts_with("Tile ")));
    key(&mut c, "PageDown");
    assert!(c.city_open);
    key(&mut c, "Escape");
    assert!(!c.city_open);
    // #315: the science advisor window.
    key(&mut c, "F6");
    assert!(matches!(c.modal, Some(Modal::Science)), "{}", c.status);
    assert_no_overlaps(&c, "science");
    let shows = |c: &Client, s: &str| texts(c).iter().any(|(_, _, t, _)| t.starts_with(s));
    assert!(shows(&c, "Researching: "));
    assert!(shows(&c, "Goal: none"));
    assert!(shows(&c, "Available next ("));
    assert!(shows(&c, "Known advances ("));
    assert!(!shows(&c, "Path: "), "estimates are opt-in");
    assert!(!c
        .buttons()
        .iter()
        .any(|b| matches!(b.action, Action::ScienceSort)));
    key(&mut c, "Escape");
    assert_eq!(
        c.checkpoint().unwrap(),
        before,
        "the rearranged windows are inert"
    );
    // The opt-in estimates: the fifth Settings row.
    button(&mut c, "Settings");
    assert_no_overlaps(&c, "settings");
    button(&mut c, "[ ] Goal estimates in the science advisor");
    assert!(c.enhancements.goal_estimates);
    assert!(storage::load_enhancements(&f.saves()).goal_estimates);
    key(&mut c, "Escape");
    key(&mut c, "F6");
    button(&mut c, "Sort: listed");
    assert!(c.science_by_name);
    assert_no_overlaps(&c, "science with estimates");
    assert_eq!(c.checkpoint().unwrap(), before, "sorting is presentation");
    // A goal shows its path and a labelled estimate.
    button(&mut c, "Set goal…");
    assert!(matches!(c.modal, Some(Modal::Goal)));
    key(&mut c, "ArrowDown");
    key(&mut c, "Enter");
    assert!(matches!(c.modal, Some(Modal::Science)), "{}", c.status);
    assert!(shows(&c, "Path: "));
    assert!(texts(&c)
        .iter()
        .any(|(_, _, t, _)| t.contains("(estimate)") || t.contains("no estimate")));
}

#[test]
fn layout_b_order_strip_and_message_dock_are_opt_in_and_inert() {
    let f = Fixture::new();
    let mut c = f.client();
    start(&mut c);
    founding(&mut c);
    // Off by default: the map keeps its full height, no strip, no dock.
    assert_eq!(c.map_rect(), chrome::MAP);
    let has_button = |c: &Client, s: &str| c.buttons().iter().any(|b| b.label.starts_with(s));
    assert!(!has_button(&c, "Wait [W]"));
    assert!(!has_button(&c, "All messages"));
    // Play two turns so the log has messages.
    for _ in 0..2 {
        key(&mut c, "Ctrl+n");
        if matches!(c.modal, Some(Modal::Confirm { .. })) {
            key(&mut c, "Enter");
        }
        while c.advancing {
            input(&mut c, Input::Tick);
        }
        research(&mut c);
        if c.modal.is_some() {
            key(&mut c, "Escape");
        }
    }
    let before = c.checkpoint().unwrap();
    button(&mut c, "Settings");
    assert_no_overlaps(&c, "settings with seven rows");
    button(&mut c, "[ ] Order buttons under the map");
    button(&mut c, "[ ] Message log in the right column");
    key(&mut c, "Escape");
    let saved = storage::load_enhancements(&f.saves());
    assert!(saved.order_strip && saved.message_dock);
    // #312: the map is shorter by the strip; the strip's buttons sit under it.
    let (mx, my, mw, mh) = c.map_rect();
    assert_eq!(mh, chrome::MAP.3 - chrome::STRIP_H);
    for label in [
        "Move [V]",
        "Wait [W]",
        "Skip [Space]",
        "Found city [B]",
        "Activate [A]",
    ] {
        let b = c
            .buttons()
            .into_iter()
            .find(|b| b.label == label || (label == "Move [V]" && b.label == "View [V]"))
            .unwrap_or_else(|| panic!("no strip button {label}"));
        assert!(
            b.y >= my + mh && b.x >= mx && b.x + b.w <= mx + mw,
            "{label}"
        );
    }
    assert!(!c.on_map(400., my + mh + 10.), "the strip is not map");
    // A strip button sends what its key sends: Wait cycles the active unit.
    let active = c.active;
    button(&mut c, "Wait [W]");
    assert!(c.active.is_some() || active.is_none());
    // #313: the dock lists the latest messages, newest first.
    let texts: Vec<String> = c
        .frame()
        .draws
        .into_iter()
        .filter_map(|d| match d {
            Draw::Text { x, text, .. } if x >= chrome::COLUMN.0 => Some(text),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|t| t == "Messages"));
    let newest = c.notice_log.last().map(|(t, _)| format!("T{t} "));
    if let Some(prefix) = newest {
        assert!(texts.iter().any(|t| t.starts_with(&prefix)), "{texts:?}");
    }
    assert_no_overlaps(&c, "map with strip and dock");
    button(&mut c, "All messages");
    assert!(matches!(c.modal, Some(Modal::Messages)));
    key(&mut c, "Escape");
    // Still no overlaps with a stack inspected and the dock squeezed.
    let city = c.view().unwrap().observation.cities[0].clone();
    c.inspect = Some((city.x, city.y));
    assert_no_overlaps(&c, "map with a stack, strip and dock");
    // A minimap click still only scrolls in the dock layout.
    let pan = c.pan;
    input(&mut c, Input::Click { x: 1000., y: 100. });
    assert_ne!(c.pan, pan);
    assert_eq!(
        c.checkpoint().unwrap(),
        before,
        "the strip's Wait, the dock and the settings never touch the game"
    );
}

#[test]
fn wide_stage_widens_the_map_centres_windows_and_toggles_full_screen() {
    use super::{stage_width, MAX_WIDTH};
    assert_eq!(stage_width(1024, 768), WIDTH);
    assert_eq!(stage_width(1920, 1080), 1365);
    assert_eq!(stage_width(2560, 1080), MAX_WIDTH);
    assert_eq!(stage_width(800, 600), WIDTH);
    assert_eq!(stage_width(1000, 0), WIDTH);
    let f = Fixture::new();
    let mut c = f.client();
    // Setup is centred too.
    let display = |c: &mut Client, width: u32, fullscreen: bool| {
        let id = c.receipts.len() as u64 + 1;
        // Accepted at any revision: a window can resize at any time.
        c.input(
            c.revision.saturating_sub(3),
            id,
            Input::Display { width, fullscreen },
        )
        .unwrap();
    };
    display(&mut c, 1365, false);
    let quick = c
        .buttons()
        .into_iter()
        .find(|b| matches!(b.action, Action::QuickStart))
        .unwrap();
    assert_eq!(quick.x, 100. + 170.);
    start(&mut c);
    founding(&mut c);
    let before = c.checkpoint().unwrap();
    assert_eq!(c.frame().width, 1365.);
    display(&mut c, 4000, false);
    assert_eq!(c.frame().width, f64::from(MAX_WIDTH));
    display(&mut c, 500, false);
    assert_eq!(c.frame().width, f64::from(WIDTH));
    display(&mut c, 1365, false);
    // The map takes the extra width less the side column (#337); the column
    // stays at the right edge.
    let (mx, _, mw, _) = c.map_rect();
    assert_eq!((mx, mw), (8., 768. + 341. - 240.));
    assert!(c.on_map(860., 300.));
    assert!(!c.on_map(900., 300.), "the side column is not map");
    assert_eq!(c.column_x(), 1365. - 240.);
    assert_eq!(c.side_x(), Some(1365. - 480.));
    for b in c.buttons() {
        if !matches!(b.action, Action::Menu(_)) {
            assert!(
                b.x >= c.side_x().unwrap(),
                "{} left of the columns",
                b.label
            );
        }
    }
    assert!(c.frame().draws.iter().any(
        |d| matches!(d, Draw::Rect { w, color, .. } if *w == 1365. && color == style::MENU_BAR)
    ));
    // Centring keeps a unit in the middle of the wider map.
    c.action(Action::Center).unwrap();
    if c.active.is_none() {
        let city = c.view().unwrap().observation.cities[0].clone();
        let px = c.pan.0 + f64::from(city.x) * 32. * c.zoom;
        assert_eq!(px, 380. + 50.5);
    }
    // Dialogs keep their layout, centred: Help's Close sits 170 px right.
    key(&mut c, "?");
    let close = c
        .buttons()
        .into_iter()
        .find(|b| matches!(b.action, Action::Accept))
        .unwrap();
    assert_eq!(close.x, 212. + 170.);
    input(
        &mut c,
        Input::Click {
            x: close.x + 3.,
            y: close.y + 3.,
        },
    );
    assert!(c.modal.is_none());
    key(&mut c, "Ctrl+s");
    assert_eq!(c.frame().entry.unwrap().x, 212. + 170.);
    key(&mut c, "Escape");
    // The city window too.
    button(&mut c, "Manage city");
    for b in c.buttons() {
        assert!(
            b.x >= 170. && b.x + b.w <= 170. + 1024.,
            "{} outside the centred city window",
            b.label
        );
    }
    key(&mut c, "Escape");
    // Full screen: Alt+Enter or View > Full screen asks; the adapter reports.
    key(&mut c, "Alt+Enter");
    assert!(c.frame().fullscreen);
    assert!(c.menu_items(2).iter().any(|i| i.label == "Window"));
    button(&mut c, "Window");
    assert!(!c.frame().fullscreen);
    button(&mut c, "Full screen");
    assert!(c.frame().fullscreen);
    display(&mut c, 1365, false);
    assert!(!c.frame().fullscreen, "the adapter's report wins");
    assert_eq!(
        c.checkpoint().unwrap(),
        before,
        "display changes are presentation"
    );
}

/// Experimental scenario entry (decision 0008, 2026-11-18): the setup screen
/// lists the installation's scenarios, shows the preflight with its gaps,
/// starts only on Accept, and engine checkpoints of the scenario reload from
/// a fresh client. Self-skips without CIVTOT_HOME or FallRome.
#[test]
fn experimental_scenario_entry_lists_preflights_starts_and_reloads() {
    let Ok(home) = std::env::var("CIVTOT_HOME") else {
        eprintln!("skipping: CIVTOT_HOME unset");
        return;
    };
    if !Path::new(&home).join("FallRome/ROMFALL.scn").is_file() {
        eprintln!("skipping: no FallRome scenario");
        return;
    }
    let scratch = tempfile::tempdir().unwrap();
    let saves = scratch.path().join("saves");
    let mut c = seeded_client(home.clone(), saves.clone());
    button(&mut c, "Open scenario (experimental)");
    assert!(matches!(c.modal, Some(Modal::Scenarios)), "{}", c.status);
    assert!(c.scenarios.iter().any(|p| p == "FallRome/ROMFALL.scn"));
    // Multi-map files are listed but rejected with a reason, never coerced.
    if c.scenarios.iter().any(|p| p == "Midgard/Midgard.SCN") {
        c.pick_scenario("Midgard/Midgard.SCN").unwrap_err();
    }
    while !c
        .buttons()
        .iter()
        .any(|b| b.label == "FallRome/ROMFALL.scn")
    {
        key(&mut c, "PageDown");
    }
    button(&mut c, "FallRome/ROMFALL.scn");
    let Some(Modal::ScenarioStart(choice)) = &c.modal else {
        panic!("no preflight: {}", c.status);
    };
    assert!(c.loaded.is_none(), "the preflight starts nothing");
    assert!(choice
        .preflight
        .gaps
        .iter()
        .any(|g| g.contains("fresh engine turn")));
    let civs = choice.preflight.civs.len();
    assert!(civs >= 2);
    // Esc goes back to the list; picking again shows the same preflight.
    key(&mut c, "Escape");
    assert!(matches!(c.modal, Some(Modal::Scenarios)));
    c.pick_scenario("FallRome/ROMFALL.scn").unwrap();
    button(&mut c, "Start experimental game");
    assert!(
        c.modal
            .as_ref()
            .is_none_or(|m| !matches!(m, Modal::ScenarioStart(_))),
        "{}",
        c.status
    );
    let frame = c.frame();
    assert!(
        frame.draws.iter().any(
            |d| matches!(d, Draw::Text { text, .. } if text.contains("ROMFALL · experimental"))
        ),
        "title names the experimental scenario"
    );
    let saved = c.checkpoint().unwrap();
    assert!(saved.contains("\nsetup scenario "));
    key(&mut c, "Ctrl+s");
    edit(&mut c, "rome");
    key(&mut c, "Enter");
    // A fresh client (setup screen, nothing loaded) reloads it.
    let mut d = seeded_client(home, saves);
    key(&mut d, "Ctrl+l");
    edit(&mut d, "rome");
    key(&mut d, "Enter");
    assert_eq!(d.checkpoint().unwrap(), saved, "{}", d.status);
    assert!(
        d.status.contains("Experimental scenario checkpoint"),
        "{}",
        d.status
    );
}

/// #337 (owner 2026-11-19): from 16:9 a side column left of the right column
/// holds the messages, replacing the dock and the new-messages overlay; 4:3
/// and 16:10 keep layout B unchanged. Presentation only.
#[test]
fn wide_stage_moves_messages_into_a_side_column() {
    let f = Fixture::new();
    let mut c = f.client();
    let display = |c: &mut Client, width: u32| {
        let id = c.receipts.len() as u64 + 1;
        c.input(
            c.revision.saturating_sub(3),
            id,
            Input::Display {
                width,
                fullscreen: true,
            },
        )
        .unwrap();
    };
    start(&mut c);
    founding(&mut c);
    // Dock and overlay on, so the wide layout visibly takes them over.
    c.enhancements.message_dock = true;
    c.enhancements.notice_overlay = true;
    let before = c.checkpoint().unwrap();
    c.notice_log = vec![
        (1, "Rome grew".into()),
        (
            2,
            "A long message that does not fit on one side-column line".into(),
        ),
        (2, "Veii built Warriors".into()),
    ];
    let now = c.view().unwrap().observation.turn;
    c.notice_log = c
        .notice_log
        .iter()
        .map(|(t, l)| (now - 2 + t, l.clone()))
        .collect();
    c.notice_turn = Some(now);
    c.notice_unread = true;
    let texts = |c: &mut Client| -> Vec<(f64, String)> {
        c.frame()
            .draws
            .iter()
            .filter_map(|d| match d {
                Draw::Text { x, text, .. } => Some((*x, text.clone())),
                _ => None,
            })
            .collect()
    };
    let all = |c: &mut Client| {
        c.buttons()
            .into_iter()
            .filter(|b| matches!(b.action, Action::Messages) && b.label.starts_with("All messages"))
            .map(|b| b.x)
            .collect::<Vec<_>>()
    };
    // 4:3 and 16:10: no side column; the dock and overlay as before.
    for width in [1024, 1228] {
        display(&mut c, width);
        assert_eq!(c.side_x(), None, "{width}");
        assert_eq!(c.map_rect().2, 768. + f64::from(width) - 1024.);
        assert!(c.notice_overlay(), "{width}");
        assert_eq!(
            all(&mut c),
            vec![c.column_x() + 6.],
            "{width}: the dock's button"
        );
    }
    // 16:9 full screen: the side column.
    display(&mut c, 1365);
    let sx = c.side_x().unwrap();
    assert!(!c.notice_overlay(), "the side column replaces the overlay");
    assert_eq!(
        all(&mut c),
        vec![sx + 6.],
        "one All messages, in the side column"
    );
    let side: Vec<String> = texts(&mut c)
        .into_iter()
        .filter(|(x, _)| *x == sx + 6.)
        .map(|(_, t)| t)
        .collect();
    assert_eq!(side[0], format!("Turn {now} — new messages"));
    // This turn's lines wrap in full, in order; earlier ones follow.
    let at = |s: &str| side.iter().position(|t| t == s).unwrap_or(usize::MAX);
    assert!(
        at("A long message that does not") < at("Veii built Warriors"),
        "{side:?}"
    );
    assert!(
        side.iter().any(|t| t == "fit on one side-column line"),
        "{side:?}"
    );
    assert!(
        at("Earlier") < at(&format!("T{} Rome grew", now - 1)),
        "{side:?}"
    );
    // The All messages button opens the log.
    button(&mut c, "All messages (3)");
    assert!(matches!(c.modal, Some(Modal::Messages)));
    key(&mut c, "Escape");
    assert_eq!(
        c.checkpoint().unwrap(),
        before,
        "the side column is presentation"
    );
}
