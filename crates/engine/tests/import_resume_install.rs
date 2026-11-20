//! Resuming a plain single-player save (#197; evidence §Saved round
//! position) against the installed `Original/Ph_Auto2.SAV`. The original's
//! loader restarts the round at the player's civilization without its
//! turn-start step, so spent movement, the move order and saved knowledge
//! stand. Self-skips without `CIVTOT_HOME`.
use engine::campaign::AiPolicy;
use engine::human::{HumanSession, Phase};
use std::path::PathBuf;

fn inputs() -> Option<(Vec<u8>, tot_kit::Rules, Option<engine::events::EventEngine>)> {
    let home = PathBuf::from(std::env::var_os("CIVTOT_HOME")?);
    let save = std::fs::read(home.join("Original/Ph_Auto2.SAV")).ok()?;
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(home.join("Original/Rules.txt")).ok()?,
    ));
    let events = std::fs::read(home.join("Original/Events.txt"))
        .ok()
        .and_then(|b| tot_kit::parse_events(&tot_kit::latin1(&b)).ok())
        .map(engine::events::EventEngine::new);
    Some((save, rules, events))
}

#[test]
fn the_files_player_resumes_its_saved_turn() {
    let Some((mut save, rules, events)) = inputs() else {
        return;
    };
    let sav = tot_kit::parse_sav(&save).unwrap();
    // Single player; the player and the running step are civ 4 (mask 0x10).
    assert_eq!(
        (
            sav.multiplayer,
            sav.player_civ,
            sav.active_civ,
            sav.human_mask
        ),
        (0, 4, 4, 0x10)
    );
    // The autosave was written as civ 4's turn began, so nothing has moved.
    // Mark two of its units as moved, in memory only: one by a third of a
    // move, one far past its allowance (it must clamp at zero).
    let own: Vec<usize> = (0..sav.units.len())
        .filter(|&i| sav.units[i].owner() == 4 && sav.units[i].id() != 0)
        .collect();
    assert!(own.len() >= 2);
    for (&i, spent) in own.iter().zip([1u8, 250]) {
        let raw = sav.units[i].raw;
        let at = save.windows(40).position(|w| w == raw).unwrap();
        assert_eq!(save[at + 10], 0);
        save[at + 10] = spent;
    }
    let sav = tot_kit::parse_sav(&save).unwrap();
    assert_eq!(engine::scenario::resume_blocker(&sav, &rules, 4), None);
    assert!(engine::scenario::resume_blocker(&sav, &rules, 2).is_some());

    let s = HumanSession::scenario(
        "Original/Ph_Auto2.SAV",
        &save,
        rules.clone(),
        events,
        4,
        AiPolicy::BuiltIn1,
        7,
    )
    .unwrap();
    // The human moves first: no opponent work before the player's turn.
    let v = s.view();
    assert_eq!(v.phase, Phase::Human);
    assert_eq!(v.revision, 0);
    assert_eq!(v.observation.turn, sav.turn);

    // Saved spent movement stands (thirds, multiplier 3), clamped at zero.
    let world = engine::boot(&sav, 0);
    let mut moved = Vec::new();
    for u in v.observation.units.iter().filter(|u| u.owner == 4) {
        let r = &sav.units[u.index];
        let full = world.movement_allowance(&rules, u.type_id as usize, 4, r.damage());
        assert_eq!(
            u.moves_left,
            full.saturating_sub(u32::from(r.moves_spent())),
            "unit {}",
            u.index
        );
        if r.moves_spent() != 0 {
            moved.push((u.moves_left, full));
        }
    }
    assert_eq!(moved.len(), 2, "both marked units");
    assert!(moved[0].0 + 1 == moved[0].1 && moved[1].0 == 0, "{moved:?}");

    // Knowledge comes from the player's saved bits: every tile civ 4 has
    // seen is known. The one extra tile is the engine's own sight model
    // (decision 0003: a city sees its whole 21-tile radius); it lies in
    // Madrid's radius at stored (27, 12), and the save says civ 4 never saw
    // it. The original's reveal rules are not decoded.
    let extra: Vec<usize> = (0..world.map.visible.len())
        .filter(|&i| {
            let known = v.observation.seen[i].is_some();
            let saved = world.map.visible[i] & 0x10 != 0;
            assert!(known || !saved, "saved tile {i} lost");
            known && !saved
        })
        .collect();
    assert_eq!(extra, [27 + 12 * world.map.stored_w as usize]);

    // A checkpoint replays to the same view.
    let text = s.checkpoint("test-resources");
    let (_, rules2, events2) = inputs().unwrap();
    let back = HumanSession::restore_with(rules2, "test-resources", &text, Some((&save, events2)))
        .unwrap();
    assert_eq!(format!("{:?}", back.view()), format!("{:?}", s.view()));
}

#[test]
fn another_civilization_starts_a_fresh_engine_turn() {
    let Some((save, rules, events)) = inputs() else {
        return;
    };
    let s = HumanSession::scenario(
        "Original/Ph_Auto2.SAV",
        &save,
        rules.clone(),
        events,
        2,
        AiPolicy::BuiltIn1,
        7,
    )
    .unwrap();
    // Civilization 1 moves before the human, as in a fresh engine round.
    assert_eq!(s.view().phase, Phase::Opponents);
}
