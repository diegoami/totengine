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
fn only_accepted_peace_leaves_the_saved_view_and_replays_the_transition() {
    for attitude in [50, 51] {
        let mut save = snapshot();
        let name = tot_kit::OFF_CIV_NAMES + 242 + 26;
        save[name..name + 9].copy_from_slice(b"Neighbors");
        let human = tot_kit::OFF_CIV_PROPS + 3348;
        let other = human + 3348;
        let treaty = 0x2001u32.to_le_bytes();
        save[human + 48..human + 52].copy_from_slice(&treaty);
        save[other + 44..other + 48].copy_from_slice(&treaty);
        save[other + 73] = attitude;

        let mut session = current(&save);
        let initial = session.view().observation.seen;
        let reply = |session: &mut HumanSession, reply| {
            let Some(Decision::Talks { id, .. }) = session.view().decision else {
                panic!("expected a conversation");
            };
            send(
                session,
                Command::ReplyToTalks {
                    decision: id,
                    reply,
                },
            )
            .unwrap()
        };
        send(&mut session, Command::BeginTalks { other: 2 }).unwrap();
        for action in [
            TalksReply::PreviewPeace,
            TalksReply::Back,
            TalksReply::Leave,
        ] {
            reply(&mut session, action);
            assert!(session.saved_initial.is_some());
            assert_eq!(session.view().observation.seen, initial);
        }
        send(&mut session, Command::BeginTalks { other: 2 }).unwrap();
        reply(&mut session, TalksReply::PreviewPeace);
        let mut resumed = restore(&save, &session.checkpoint(DEPS)).unwrap();
        assert!(resumed.saved_initial.is_some());
        assert_eq!(
            reply(&mut resumed, TalksReply::OfferPeace),
            reply(&mut session, TalksReply::OfferPeace)
        );
        let accepted = attitude == 50;
        let expected_treaty = if accepted { 0x4005 } else { 0x2001 };
        assert_eq!(session.game.world.diplomacy[1][2], expected_treaty);
        assert_eq!(session.game.world.diplomacy[2][1], expected_treaty);
        assert_eq!(session.saved_initial.is_none(), accepted);
        assert_eq!(session.view().observation.seen != initial, accepted);
        let outcome = session.checkpoint(DEPS);
        assert_eq!(resumed.checkpoint(DEPS), outcome);
        let restored = restore(&save, &outcome).unwrap();
        assert_eq!(restored.saved_initial.is_none(), accepted);
        assert_eq!(restored.view(), session.view());
    }
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

/// Independent field comparisons, rather than merely re-running the current
/// constructor's normalization algorithm as an expected implementation.
#[test]
fn installed_snapshot_records_and_normalization_are_measured() {
    use crate::rng::GameRng;
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        return;
    };
    let home = std::path::PathBuf::from(home);
    let bytes = std::fs::read(home.join("Original/Ph_Auto2.SAV")).unwrap();
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(home.join("Original/Rules.txt")).unwrap(),
    ));
    let events = tot_kit::parse_events(&tot_kit::latin1(
        &std::fs::read(home.join("Original/Events.txt")).unwrap(),
    ))
    .unwrap();
    let checked = crate::scenario::snapshot::CheckedSnapshot::read(&bytes, &rules).unwrap();
    let saved = checked.save();
    let s = HumanSession::scenario_initialized_current(
        ScenarioInput {
            path: "Original/Ph_Auto2.SAV",
            snapshot: &bytes,
            rules: rules.clone(),
            events: Some(EventEngine::new(events)),
        },
        4,
        AiPolicy::BuiltIn1,
        7,
        DEPS,
    )
    .unwrap();
    assert_eq!(
        s.knowledge_recipe,
        ScenarioKnowledgeRecipe::SavedInitialThenEngineV1
    );
    assert_eq!(s.phase(), Phase::Human);
    let actual = &s.game.world;
    assert_eq!(actual.treasury, saved.treasury);
    assert_eq!(actual.science_rates, saved.science_rates);
    assert_eq!(actual.tax_rates, saved.tax_rates);
    assert_eq!(actual.governments, saved.governments);
    assert_eq!(actual.research, saved.research);
    assert_eq!(actual.techs, saved.techs);
    assert_eq!(actual.diplomacy, saved.diplomacy);
    assert_eq!(actual.wonders, saved.wonders);
    assert_eq!(actual.city_counts, saved.city_counts);
    let mut rows = Vec::new();
    for (slot, (u, raw)) in actual.units.iter().zip(&saved.units).enumerate() {
        assert_eq!(
            (u.id, u.owner, u.type_id, u.x, u.y),
            (raw.id(), raw.owner(), raw.type_id(), raw.x(), raw.y())
        );
        assert_eq!(
            (u.damage, u.orders, u.home, u.veteran),
            (raw.damage(), raw.orders(), raw.home(), raw.is_veteran())
        );
        if u.id != 0 {
            assert_eq!(Some(u.stack_pos), checked.stack_positions()[slot]);
            let allowance =
                actual.movement_allowance(&rules, usize::from(u.type_id), u.owner, u.damage);
            assert_eq!(
                u.moves_left,
                allowance.saturating_sub(u32::from(raw.moves_spent()))
            );
            rows.push(format!(
                "[{slot}, {}, {}, {}, {}, {}]",
                u.id,
                u.owner,
                raw.moves_spent(),
                allowance,
                u.moves_left
            ));
        }
    }
    for (c, raw) in actual.cities.iter().zip(&saved.cities) {
        assert_eq!(
            (c.city_id, c.x, c.y, c.owner, c.size),
            (raw.city_id(), raw.x(), raw.y(), raw.owner(), raw.size())
        );
        assert_eq!(
            (
                c.food_box,
                c.shield_box,
                c.building,
                c.specialists,
                c.worked
            ),
            (
                raw.food_box(),
                raw.shield_box(),
                raw.build_item(),
                raw.specialists(),
                raw.worked()
            )
        );
        assert_eq!(
            (
                c.gross_food,
                c.gross_shields,
                c.gross_trade,
                c.net_science,
                c.net_taxes
            ),
            (
                raw.gross_food(),
                raw.gross_shields(),
                raw.gross_trade(),
                raw.net_science(),
                raw.net_taxes()
            )
        );
        assert_eq!(
            (c.flags, c.buildings, c.known_to, c.known_sizes),
            (
                raw.flags(),
                raw.buildings(),
                raw.known_to(),
                raw.raw[15..23].try_into().unwrap()
            )
        );
    }
    for (tile, visible) in actual.map.visible.iter().enumerate() {
        assert_eq!(*visible, saved.maps[0].visible_to(tile));
        assert_eq!(
            actual.map.improvements[tile],
            saved.maps[0].improvements(tile)
        );
    }
    for civ in 0..7 {
        let n = saved.map_header.surface();
        assert_eq!(
            actual.map.known_improvements[civ],
            saved.maps[0].visibility[civ * n..(civ + 1) * n]
        );
    }
    // Compare the entire booted engine state after erasing only the permitted
    // movement difference. This catches income, caches, events, reveal or RNG
    // work outside the declared recipe, including fields not listed above.
    let boot = crate::boot(saved, 7);
    let mut no_movement = actual.clone();
    for unit in &mut no_movement.units {
        unit.moves_left = 0;
    }
    assert_eq!(no_movement.checksum(), boot.checksum());
    assert_eq!(format!("{no_movement:?}"), format!("{boot:?}"));
    assert_eq!((actual.rng.state(), actual.rng.draws()), (7, 0));
    assert_eq!(
        (actual.combat_rng.state(), actual.combat_rng.draws()),
        (7, 0)
    );
    let first = s.checkpoint(DEPS);
    assert!(matches!(s.decision, Some(Decision::Research { .. })));
    if let Ok(path) = std::env::var("TOTENGINE_SNAPSHOT_EVIDENCE") {
        std::fs::write(path, format!(
            "{{\n  \"schema\": 1,\n  \"record_check\": \"classic-snapshot-records-v1\",\n  \"runtime_admitted\": false,\n  \"source_length\": {},\n  \"source_fnv64\": \"{:016x}\",\n  \"trailing_bytes_unchecked\": {},\n  \"unit_slots\": {},\n  \"live_units\": {},\n  \"city_slots\": {},\n  \"live_cities\": {},\n  \"stack_positions\": {:?},\n  \"unit_rows_columns\": [\"slot\", \"identity\", \"owner\", \"spent\", \"allowance\", \"remaining\"],\n  \"unit_rows\": [{}],\n  \"initial_world\": \"{:016x}\",\n  \"boot_world\": \"{:016x}\",\n  \"initial_checkpoint_fnv64\": \"{:016x}\",\n  \"rng_states\": [7, 7],\n  \"rng_draws\": [0, 0],\n  \"only_world_difference_from_boot\": \"movement\",\n  \"required_choice\": \"engine-derived Research; native timing unverified\"\n}}\n",
            checked.source_length(),checked.source_fingerprint(),checked.trailing_bytes(),
            saved.units.len(),saved.units.iter().filter(|u|u.id()!=0).count(),saved.cities.len(),saved.cities.iter().filter(|c|c.city_id()!=0).count(),
            checked.stack_positions().iter().map(|p|p.map_or(-1,i32::from)).collect::<Vec<_>>(),rows.join(", "),
            actual.checksum(),boot.checksum(),crate::player::fnv64(first.as_bytes()),
        )).unwrap();
    }
}

#[test]
fn snapshot_normalization_keeps_debt_cached_outputs_and_overdrawn_moves() {
    let mut bytes = snapshot();
    let parsed = tot_kit::parse_sav(&bytes).unwrap();
    bytes[parsed.unit_region.0 + 10] = 250;
    let city = parsed.city_offset;
    bytes[city + 28..city + 30].copy_from_slice(&(-19i16).to_le_bytes());
    bytes[city + 30..city + 32].copy_from_slice(&1200u16.to_le_bytes());
    bytes[city + 76..city + 78].copy_from_slice(&237u16.to_le_bytes());
    bytes[city + 78..city + 80].copy_from_slice(&431u16.to_le_bytes());
    let treasury = tot_kit::OFF_CIV_PROPS + 3348 + 10;
    bytes[treasury..treasury + 4].copy_from_slice(&(-123456i32).to_le_bytes());
    crate::scenario::snapshot::CheckedSnapshot::read(&bytes, &rules()).unwrap();
    let session = current(&bytes);
    let w = &session.game.world;
    assert_eq!(w.units[0].moves_left, 0);
    assert_eq!(w.cities[0].food_box as i16, -19);
    assert_eq!(
        (
            w.cities[0].shield_box,
            w.cities[0].net_science,
            w.cities[0].net_taxes
        ),
        (1200, 237, 431)
    );
    assert_eq!(w.treasury[1], -123456);
    let text = session.checkpoint(DEPS);
    assert_eq!(restore(&bytes, &text).unwrap().checkpoint(DEPS), text);
}
