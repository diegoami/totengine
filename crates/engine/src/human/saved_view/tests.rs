use super::*;
use crate::campaign::AiPolicy;
use crate::events::EventEngine;

const DEPS: &str = "totengine-scenario-dependencies 1\nrules synthetic 1 0000000000000001\n";

fn rules() -> Rules {
    tot_kit::parse_rules(&format!(
        "{}\n{}",
        include_str!("../../../tests/fixtures/campaign-rules.txt"),
        include_str!("../../../tests/fixtures/builtin-settings.txt")
    ))
}

fn snapshot() -> Vec<u8> {
    let original = crate::tests_support::synthetic_save_bytes();
    let mut bytes = original[..tot_kit::OFF_TRANSPORTERS + 172].to_vec();
    bytes[678] = 0;
    bytes[680] = 1;
    bytes[681] = 1;
    bytes[687] = 2;
    bytes[698..700].copy_from_slice(&4u16.to_le_bytes());
    for i in 0..28 {
        bytes[tot_kit::OFF_WONDERS + 2 * i..tot_kit::OFF_WONDERS + 2 * i + 2]
            .copy_from_slice(&(-1i16).to_le_bytes());
    }
    for civ in 0..8 {
        let at = tot_kit::OFF_CIV_PROPS + 3348 * civ;
        bytes[at + 29] = 1;
        bytes[at + 27] = 5;
        bytes[at + 28] = 5;
    }
    for value in [16i16, 12, 96, 1, 0, 0, 0, 0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend(vec![0x10; 96 * 7]);
    let mut tiles = [1, 0x30, 0, 1, 0, 0xf0].repeat(96);
    for (x, y) in [(6, 6), (8, 6), (14, 10), (12, 10), (10, 10)] {
        tiles[6 * (y * 8 + x / 2) + 4] = 2;
    }
    for (x, y, owner) in [(6, 6, 1), (8, 6, 2), (14, 10, 2), (12, 10, 2)] {
        let i = y * 8 + x / 2;
        tiles[6 * i + 1] |= 1;
        tiles[6 * i + 5] = owner << 4;
    }
    for (x, y, owner) in [(6, 6, 1), (8, 6, 2), (10, 10, 2)] {
        let i = y * 8 + x / 2;
        tiles[6 * i + 1] |= 2;
        tiles[6 * i + 5] = owner << 4;
    }
    bytes.extend(tiles);
    bytes.extend([0, 0]);
    bytes.extend(vec![0; 10240]);
    for (slot, (x, y, owner, visible)) in [
        (6u16, 6u16, 1u8, 0u8),
        (8, 6, 2, 0),
        (14, 10, 2, 2),
        (12, 10, 2, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let mut raw = [0u8; 40];
        raw[..2].copy_from_slice(&x.to_le_bytes());
        raw[2..4].copy_from_slice(&y.to_le_bytes());
        raw[8] = u8::from(slot != 0);
        raw[9] = owner;
        raw[10] = u8::from(slot == 0);
        raw[11] = visible;
        raw[17] = 255;
        raw[18] = 255;
        raw[26..28].copy_from_slice(&(-1i16).to_le_bytes());
        raw[28..30].copy_from_slice(&(-1i16).to_le_bytes());
        raw[30..34].copy_from_slice(&(slot as u32 + 1).to_le_bytes());
        bytes.extend(raw);
    }
    for slot in 0..8 {
        let mut raw = [0u8; 92];
        let (x, y, owner, size, name) = match slot {
            0 => (6u16, 6u16, 1u8, 2u8, "Own"),
            1 => (8, 6, 2, 9, "Known"),
            2 => (10, 10, 2, 7, "Hidden"),
            _ => (0, 0, 1, 1, "Spare"),
        };
        raw[..2].copy_from_slice(&x.to_le_bytes());
        raw[2..4].copy_from_slice(&y.to_le_bytes());
        raw[10] = owner;
        raw[11] = size;
        raw[12] = owner;
        if slot == 1 {
            raw[14] = 2;
            raw[16] = 3;
        }
        raw[34..34 + name.len()].copy_from_slice(name.as_bytes());
        raw[50..54].copy_from_slice(&(1u32 << 20).to_le_bytes());
        raw[82] = 4;
        if slot < 3 {
            raw[86..90].copy_from_slice(&(slot as u32 + 1).to_le_bytes());
        }
        bytes.extend(raw);
    }
    bytes
}

fn input<'a>(save: &'a [u8], events: Option<EventEngine>) -> ScenarioInput<'a> {
    ScenarioInput {
        path: "Original/SavedViewTest.sav",
        snapshot: save,
        rules: rules(),
        events,
    }
}
fn current(save: &[u8]) -> HumanSession {
    HumanSession::scenario_initialized_current(input(save, None), 1, AiPolicy::AiV1, 9, DEPS)
        .unwrap()
}
fn send(s: &mut HumanSession, command: Command) -> Result<Response, &'static str> {
    s.submit(Request {
        civ: 1,
        revision: s.revision,
        id: s.next_request_id().unwrap(),
        command,
    })
}
fn restore(save: &[u8], text: &str) -> Result<HumanSession, String> {
    HumanSession::restore_with(rules(), DEPS, text, Some((save, None)))
}

#[test]
fn saved_initial_view_uses_stored_entity_gates_and_city_sizes() {
    let save = snapshot();
    let s = current(&save);
    let saved = tot_kit::parse_sav(&save).unwrap();
    assert_eq!(
        crate::scenario::saved_view_blocker(&saved, &rules(), None, 1),
        None
    );
    let view = s.view();
    assert_eq!(view.phase, Phase::Human);
    assert_eq!(view.revision, 0);
    assert_eq!(
        view.observation.seen.iter().filter(|t| t.is_some()).count(),
        5
    );
    assert_eq!(
        view.observation
            .units
            .iter()
            .map(|u| u.index)
            .collect::<Vec<_>>(),
        [0, 2]
    );
    assert_eq!(
        view.observation
            .cities
            .iter()
            .map(|c| (c.index, c.size))
            .collect::<Vec<_>>(),
        [(0, 2), (1, 3)]
    );
    assert_eq!(
        s.game.world.cities[1].size, 9,
        "remembered foreign size must not overwrite the real city"
    );
    assert_eq!(view.observation.units[1].moves_left, 0);
    assert!(!view.observation.seen[6 * 8 + 3].unwrap().rail);
    assert!(s.game.world.map.rail[6 * 8 + 3]);
    let mut expected = crate::boot(&saved, 9);
    expected.start_turn(&rules());
    for (unit, raw) in expected.units.iter_mut().zip(&saved.units) {
        if unit.id != 0 {
            unit.moves_left = unit.moves_left.saturating_sub(u32::from(raw.moves_spent()));
        }
    }
    assert_eq!(
        s.game.world.checksum(),
        expected.checksum(),
        "only declared movement normalization happens on load"
    );
    assert!(s
        .initializer()
        .unwrap()
        .descriptor()
        .starts_with("initializer 2 profile experimental-saved-view-v1 "));
}

#[test]
fn polling_non_gameplay_commands_and_restore_keep_the_saved_initial_epoch() {
    let save = snapshot();
    let mut s = current(&save);
    let before = s.checkpoint(DEPS);
    let view = s.view();
    for _ in 0..20 {
        assert_eq!(s.view(), view);
        assert_eq!(s.advance(256), 0);
    }
    assert_eq!(s.checkpoint(DEPS), before);
    let own = Entity {
        slot: 0,
        id: s.game.world.cities[0].city_id,
    };
    send(&mut s, Command::Goal { advance: None }).unwrap();
    send(&mut s, Command::CloseProduction { city: own }).unwrap();
    assert!(s.saved_initial.is_some());
    assert_eq!(
        s.view()
            .observation
            .seen
            .iter()
            .filter(|t| t.is_some())
            .count(),
        5
    );
    let text = s.checkpoint(DEPS);
    let back = restore(&save, &text).unwrap();
    assert_eq!(back.view(), s.view());
    assert_eq!(back.checkpoint(DEPS), text);
}

#[test]
fn declared_engine_continuation_and_partial_opponents_replay_exactly() {
    let save = snapshot();
    let mut s = current(&save);
    assert!(s.decision.is_none(), "fixture has a running project");
    let unit = Entity {
        slot: 0,
        id: s.game.world.units[0].id,
    };
    send(&mut s, Command::Skip { unit }).unwrap();
    assert!(s.saved_initial.is_none());
    assert!(
        s.view()
            .observation
            .seen
            .iter()
            .filter(|t| t.is_some())
            .count()
            > 5
    );
    assert_eq!(
        s.view()
            .observation
            .cities
            .iter()
            .find(|c| c.index == 1)
            .unwrap()
            .size,
        9
    );
    send(&mut s, Command::EndTurn).unwrap();
    assert_eq!(s.advance(1), 1);
    let text = s.checkpoint(DEPS);
    let back = restore(&save, &text).unwrap();
    assert_eq!(back.view(), s.view());
    assert_eq!(back.checkpoint(DEPS), text);
    let legacy =
        HumanSession::scenario_initialized(input(&save, None), 1, AiPolicy::AiV1, 9, DEPS).unwrap();
    assert!(legacy
        .initializer()
        .unwrap()
        .descriptor()
        .starts_with("initializer 1 profile experimental-scenario-v1 "));
    assert_eq!(
        restore(&save, &legacy.checkpoint(DEPS))
            .unwrap()
            .checkpoint(DEPS),
        legacy.checkpoint(DEPS)
    );
}

#[test]
fn schema_one_checkpoint_is_byte_identical_to_reviewed_main() {
    let save = snapshot();
    let expected = include_str!("../../../tests/fixtures/saved-view-legacy-v1.cth").replacen(
        "engine 0.21.0\n",
        &format!("engine {}\n", env!("CARGO_PKG_VERSION")),
        1,
    );
    let legacy =
        HumanSession::scenario_initialized(input(&save, None), 1, AiPolicy::AiV1, 9, DEPS).unwrap();
    assert_eq!(legacy.checkpoint(DEPS), expected);
    assert_eq!(
        restore(&save, &expected).unwrap().checkpoint(DEPS),
        expected
    );
}

#[test]
fn saved_view_assertions_reject_tampering_before_the_journal() {
    let save = snapshot();
    let s = current(&save);
    let text = s.checkpoint(DEPS);
    for (old, new) in [
        ("initializer 2 ", "initializer 99 "),
        (
            "profile experimental-saved-view-v1",
            "profile classic-surface-032-v1",
        ),
        (
            "knowledge saved-initial-then-engine-v1",
            "knowledge reveal-all",
        ),
        ("phase Human cursor 1", "phase Opponents cursor 1"),
    ] {
        let bad = text
            .replacen(old, new, 1)
            .replace("end\n", "invalid-operation\nend\n");
        assert_ne!(bad, text);
        assert!(restore(&save, &bad).err().unwrap().contains("initializer"));
    }
    let mut changed = save.clone();
    let parsed = tot_kit::parse_sav(&save).unwrap();
    changed[parsed.unit_region.0 + 2 * 40 + 11] = 0;
    assert!(restore(&changed, &text)
        .err()
        .unwrap()
        .contains("scenario file differs"));
    assert_eq!(s.checkpoint(DEPS), text);
}

#[test]
fn unsupported_contexts_keep_the_legacy_recipe_and_failed_orders_keep_the_saved_view() {
    let save = snapshot();
    let event = EventEngine::new(
        tot_kit::parse_events("@IF\nSCENARIOLOADED\n@THEN\nTEXT\nLoaded.\nENDTEXT\n@ENDIF\n")
            .unwrap(),
    );
    let legacy = HumanSession::scenario_initialized_current(
        input(&save, Some(event)),
        1,
        AiPolicy::AiV1,
        9,
        DEPS,
    )
    .unwrap();
    assert_eq!(
        legacy.knowledge_recipe,
        ScenarioKnowledgeRecipe::EngineSightV1
    );
    let mut s = current(&save);
    let before = s.checkpoint(DEPS);
    let unit = Entity {
        slot: 0,
        id: s.game.world.units[0].id,
    };
    let city = Entity {
        slot: 2,
        id: s.game.world.cities[2].city_id,
    };
    assert_eq!(
        send(&mut s, Command::Occupy { unit, city }).err(),
        Some("city is not known")
    );
    assert!(s.saved_initial.is_some());
    assert_eq!(s.checkpoint(DEPS), before);
    assert!(send(&mut s, Command::Move { unit, to: (14, 10) }).is_err());
    assert!(s.saved_initial.is_some());
    assert_eq!(s.checkpoint(DEPS), before);
    let mut bad = save.clone();
    bad[678] = 1;
    assert_eq!(
        current(&bad).knowledge_recipe,
        ScenarioKnowledgeRecipe::EngineSightV1
    );
    let mut unexplored = save.clone();
    let tile_base = tot_kit::OFF_TRANSPORTERS + 172 + 16 + 7 * 96;
    unexplored[tile_base + 6 * (6 * 8 + 3) + 4] &= !2;
    assert_eq!(
        current(&unexplored).knowledge_recipe,
        ScenarioKnowledgeRecipe::EngineSightV1
    );
}

#[test]
#[ignore]
fn author_synthetic_legacy_checkpoint_input() {
    let out = std::path::PathBuf::from(
        std::env::var("TOTENGINE_SAVED_VIEW_BASELINE_OUT").expect("private output directory"),
    );
    std::fs::create_dir_all(&out).unwrap();
    let save = snapshot();
    std::fs::write(out.join("synthetic.sav"), &save).unwrap();
    let legacy =
        HumanSession::scenario_initialized(input(&save, None), 1, AiPolicy::AiV1, 9, DEPS).unwrap();
    std::fs::write(out.join("current-legacy.cth"), legacy.checkpoint(DEPS)).unwrap();
}

#[test]
fn installed_original_saved_view_matches_the_pinned_reader_projection() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        return;
    };
    let home = std::path::PathBuf::from(home);
    let save = std::fs::read(home.join("Original/Ph_Auto2.SAV")).expect("installed positive save");
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(home.join("Original/Rules.txt")).expect("Original Rules"),
    ));
    let event_file = tot_kit::parse_events(&tot_kit::latin1(
        &std::fs::read(home.join("Original/Events.txt")).expect("Original events"),
    ))
    .unwrap();
    let saved = tot_kit::parse_sav(&save).unwrap();
    assert_eq!(
        (saved.player_civ, saved.active_civ, saved.human_mask),
        (4, 4, 16)
    );
    let make = || {
        HumanSession::scenario_initialized_current(
            ScenarioInput {
                path: "Original/Ph_Auto2.SAV",
                snapshot: &save,
                rules: rules.clone(),
                events: Some(EventEngine::new(event_file.clone())),
            },
            4,
            AiPolicy::BuiltIn1,
            7,
            DEPS,
        )
        .unwrap()
    };
    let mut s = make();
    let initial = s.view();
    assert_eq!(
        s.knowledge_recipe,
        ScenarioKnowledgeRecipe::SavedInitialThenEngineV1
    );
    assert_eq!(initial.phase, Phase::Human);
    assert_eq!(initial.revision, 0);
    assert_eq!(saved.research[4].target, -1);
    assert!(
        matches!(&initial.decision, Some(Decision::Research { .. })),
        "the existing engine derives its required research choice"
    );
    let known = initial
        .observation
        .seen
        .iter()
        .filter(|t| t.is_some())
        .count();
    assert_eq!(known, 59);
    assert_eq!(
        initial
            .observation
            .units
            .iter()
            .map(|u| u.index)
            .collect::<Vec<_>>(),
        [6, 9, 18, 26]
    );
    assert_eq!(
        initial
            .observation
            .cities
            .iter()
            .map(|c| (c.index, c.size))
            .collect::<Vec<_>>(),
        [(5, 2)]
    );
    let extra = 27 + 12 * s.game.world.map.stored_w;
    assert!(initial.observation.seen[extra].is_none());
    let mut expected = crate::boot(&saved, 7);
    expected.start_turn(&rules);
    for (unit, raw) in expected.units.iter_mut().zip(&saved.units) {
        if unit.id != 0 {
            unit.moves_left = unit.moves_left.saturating_sub(u32::from(raw.moves_spent()));
        }
    }
    assert_eq!(expected.checksum(), s.game.world.checksum());
    for _ in 0..20 {
        assert_eq!(s.view(), initial);
        assert_eq!(s.advance(256), 0);
    }
    let before = s.checkpoint(DEPS);
    let back = HumanSession::restore_with(
        rules.clone(),
        DEPS,
        &before,
        Some((&save, Some(EventEngine::new(event_file.clone())))),
    )
    .unwrap();
    assert_eq!(back.view(), initial);
    assert_eq!(back.checkpoint(DEPS), before);
    let Some(Decision::Research { id, available, .. }) = &initial.decision else {
        unreachable!()
    };
    s.submit(Request {
        civ: 4,
        revision: s.revision,
        id: s.next_request_id().unwrap(),
        command: Command::Research {
            decision: *id,
            advance: available[0],
        },
    })
    .unwrap();
    let after_count = s
        .view()
        .observation
        .seen
        .iter()
        .filter(|t| t.is_some())
        .count();
    assert_eq!(after_count, 60);
    let after = s.checkpoint(DEPS);
    let back = HumanSession::restore_with(
        rules.clone(),
        DEPS,
        &after,
        Some((&save, Some(EventEngine::new(event_file.clone())))),
    )
    .unwrap();
    assert_eq!(back.view(), s.view());
    assert_eq!(back.checkpoint(DEPS), after);
    let legacy = HumanSession::scenario_initialized(
        ScenarioInput {
            path: "Original/Ph_Auto2.SAV",
            snapshot: &save,
            rules,
            events: Some(EventEngine::new(event_file)),
        },
        4,
        AiPolicy::BuiltIn1,
        7,
        DEPS,
    )
    .unwrap();
    assert_eq!(
        legacy
            .view()
            .observation
            .seen
            .iter()
            .filter(|t| t.is_some())
            .count(),
        60
    );
    if let Ok(path) = std::env::var("TOTENGINE_SAVED_VIEW_EVIDENCE") {
        let captured = s.initializer().unwrap();
        std::fs::write(path,format!("{{\n  \"schema\": 1,\n  \"checkpoint_format\": 3,\n  \"initializer_schema\": 2,\n  \"profile\": \"{}\",\n  \"source_fnv64\": \"{:016x}\",\n  \"source_length\": {},\n  \"human\": 4,\n  \"phase\": \"Human\",\n  \"saved_turn\": {},\n  \"initial_seen_tiles\": 59,\n  \"initial_unit_slots\": [6, 9, 18, 26],\n  \"initial_cities\": [[5, 2]],\n  \"legacy_initial_seen_tiles\": 60,\n  \"after_first_gameplay_order_seen_tiles\": 60,\n  \"initial_world\": \"{:016x}\",\n  \"initial_knowledge\": \"{:016x}\",\n  \"initial_checkpoint_fnv64\": \"{:016x}\",\n  \"after_checkpoint_fnv64\": \"{:016x}\",\n  \"roundtrip_initial_and_after_order\": true,\n  \"world_equals_declared_movement_only_normalization\": true\n}}\n",captured.profile(),crate::player::fnv64(&save),save.len(),saved.turn,captured.initial_world,captured.initial_knowledge,crate::player::fnv64(before.as_bytes()),crate::player::fnv64(after.as_bytes()))).unwrap();
    }
}
