use super::*;
use crate::rng::GameRng;

fn rules() -> Rules {
    tot_kit::parse_rules(&format!(
        "{}\n{}",
        include_str!("../../tests/fixtures/campaign-rules.txt"),
        include_str!("../../tests/fixtures/builtin-settings.txt")
    ))
}
fn config() -> NewGameConfig {
    NewGameConfig {
        gameplay_seed: 7,
        world: crate::worldgen::WorldConfig {
            seed: 7,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn session(human: u8) -> HumanSession {
    HumanSession::generated(config(), rules(), human).unwrap()
}
fn send(s: &mut HumanSession, command: Command) -> Request {
    let r = Request {
        civ: s.human,
        revision: s.revision,
        id: s.receipts.len() as u64 + 1,
        command,
    };
    s.submit(r).unwrap();
    r
}
fn choose(s: &mut HumanSession) {
    while let Some(Decision::Research { id, available }) = s.decision.clone() {
        send(
            s,
            Command::Research {
                decision: id,
                advance: available[0],
            },
        );
    }
}
fn ready(s: &mut HumanSession, budget: usize) {
    for _ in 0..2000 {
        if s.phase() != Phase::Opponents {
            return;
        }
        assert!(s.advance(budget) > 0);
    }
    panic!("opponents never returned control");
}
fn round_trip(s: &HumanSession) -> HumanSession {
    let checkpoint = s.checkpoint("authored-test-resources-v1");
    let restored =
        HumanSession::restore(rules(), "authored-test-resources-v1", &checkpoint).unwrap();
    assert_eq!(restored.view(), s.view());
    assert_eq!(
        restored.checkpoint("authored-test-resources-v1"),
        checkpoint
    );
    assert_eq!(restored.game.log(), s.game.log());
    restored
}

#[test]
fn thinking_polling_rejections_and_retries_are_inert() {
    let mut s = session(1);
    let before = s.checkpoint("assets");
    let view = s.view();
    for _ in 0..300 {
        assert_eq!(s.view(), view);
        assert_eq!(s.advance(256), 0);
    }
    let base = Request {
        civ: 1,
        revision: 0,
        id: 1,
        command: Command::EndTurn,
    };
    for r in [
        base,
        Request { civ: 2, ..base },
        Request {
            revision: 99,
            ..base
        },
        Request {
            command: Command::Research {
                decision: 999,
                advance: 0,
            },
            ..base
        },
    ] {
        assert!(s.submit(r).is_err());
        assert_eq!(s.checkpoint("assets"), before);
    }
    let Some(Decision::Research { id, available }) = s.decision.clone() else {
        panic!()
    };
    let r = Request {
        command: Command::Research {
            decision: id,
            advance: available[0],
        },
        ..base
    };
    let response = s.submit(r).unwrap();
    let after = s.checkpoint("assets");
    assert_eq!(s.submit(r).unwrap(), response);
    assert!(s
        .submit(Request {
            command: Command::EndTurn,
            ..r
        })
        .is_err());
    assert_eq!(s.checkpoint("assets"), after);
    let restored = round_trip(&s);
    let mut restored = restored;
    assert_eq!(
        restored.submit(r).unwrap(),
        response,
        "receipt survives load"
    );
}

#[test]
fn named_founding_is_explicit_atomic_and_replayable() {
    let mut s = session(1);
    choose(&mut s);
    let unit = s.view().units[0];
    let before = s.checkpoint("assets");
    for bad in [
        Entity {
            id: unit.id + 100,
            ..unit
        },
        s.game
            .world
            .units
            .iter()
            .enumerate()
            .find(|(_, u)| u.owner == 2)
            .map(|(slot, u)| Entity { slot, id: u.id })
            .unwrap(),
    ] {
        assert!(s
            .submit(Request {
                civ: 1,
                revision: s.revision,
                id: 2,
                command: Command::BeginFound { unit: bad }
            })
            .is_err());
        assert_eq!(s.checkpoint("assets"), before);
    }
    send(&mut s, Command::BeginFound { unit });
    let Some(Decision::Founding { id, .. }) = s.decision else {
        panic!()
    };
    assert_eq!(
        s.game.world.cities.len(),
        0,
        "no city before name acceptance"
    );
    let mut restored = round_trip(&s);
    let name = CityName::new("New Áthens = Τ 1").unwrap();
    let r = send(&mut s, Command::NameCity { decision: id, name });
    assert_eq!(restored.submit(r).unwrap(), s.receipts[&r.id].1);
    assert_eq!(s.game.world.cities[0].name, name.as_str());
    assert_eq!(s.game.world.units[unit.slot].id, 0);
    assert_eq!(s.game.world.checksum(), restored.game.world.checksum());
    let replay = Game::replay_human(config(), rules(), 1, s.game.log()).unwrap();
    assert_eq!(replay.world.checksum(), s.game.world.checksum());
    assert!(Game::replay_generated(config(), rules(), s.game.log()).is_err());
    assert!(Game::replay_human(config(), rules(), 2, s.game.log()).is_err());
    for name in ["", " padded", "line\nbreak", "zero\0byte", &"a".repeat(64)] {
        assert!(CityName::new(name).is_err());
    }
    assert!(CityName::from_hex("aaα").is_none());
}

#[test]
fn cancelling_founding_and_invalid_moves_do_not_spend_units_or_rng() {
    let mut s = session(1);
    choose(&mut s);
    let unit = s.view().units[0];
    let world = s.game.world.checksum();
    send(&mut s, Command::BeginFound { unit });
    let Some(Decision::Founding { id, .. }) = s.decision else {
        panic!()
    };
    send(&mut s, Command::CancelFound { decision: id });
    assert_eq!(s.game.world.checksum(), world);
    let before = s.checkpoint("assets");
    let r = Request {
        civ: 1,
        revision: s.revision,
        id: 4,
        command: Command::Move {
            unit,
            to: (u16::MAX, u16::MAX),
        },
    };
    assert!(s.submit(r).is_err());
    assert_eq!(s.checkpoint("assets"), before);
}

#[test]
fn ten_rounds_match_across_budgets_and_partial_opponent_checkpoints() {
    for human in [1, 3, 5] {
        let mut small = session(human);
        let mut large = session(human);
        ready(&mut small, 1);
        ready(&mut large, 256);
        assert_eq!(small.view(), large.view());
        assert_eq!(small.game.world.human_mask, 1 << human);
        choose(&mut small);
        choose(&mut large);
        let unit = small.view().units[0];
        send(&mut small, Command::BeginFound { unit });
        send(&mut large, Command::BeginFound { unit });
        let Some(Decision::Founding { id, .. }) = small.decision else {
            panic!()
        };
        let command = Command::NameCity {
            decision: id,
            name: CityName::new("Human capital").unwrap(),
        };
        send(&mut small, command);
        send(&mut large, command);
        let city = small.view().cities[0];
        // More human actions than the AI guard permits never end a human turn.
        let item = small
            .view()
            .observation
            .cities
            .iter()
            .find(|c| c.index == city.slot)
            .unwrap()
            .building
            .unwrap();
        for _ in 0..ACTION_CAP + 1 {
            send(&mut small, Command::Production { city, item });
            send(&mut large, Command::Production { city, item });
        }
        assert_eq!(small.phase(), Phase::Human);
        for _ in 0..10 {
            choose(&mut small);
            choose(&mut large);
            let old_request = send(&mut small, Command::EndTurn);
            send(&mut large, Command::EndTurn);
            assert!(small.advance(1) <= 1);
            small = round_trip(&small);
            let after = small.game.world.checksum();
            small.submit(old_request).unwrap();
            assert_eq!(
                small.game.world.checksum(),
                after,
                "late retry cannot end another turn"
            );
            ready(&mut small, 1);
            ready(&mut large, 256);
            assert_eq!(small.view(), large.view());
            assert_eq!(small.game.log(), large.game.log());
            assert_eq!(small.game.world.checksum(), large.game.world.checksum());
        }
        let replay = Game::replay_human(config(), rules(), human, small.game.log()).unwrap();
        assert_eq!(replay.world.checksum(), small.game.world.checksum());
        round_trip(&small);
    }
}

#[test]
fn invalid_checkpoint_replacement_is_atomic_and_binds_resources_rules_and_version() {
    let s = session(1);
    let before = s.checkpoint("assets");
    let changed_rules =
        tot_kit::parse_rules(&(tot_kit::emit_rules(&rules()) + "\n@EXTRA\nchanged\n"));
    assert!(HumanSession::restore(changed_rules, "assets", &before).is_err());
    assert!(HumanSession::restore(rules(), "different assets", &before).is_err());
    for bad in [
        before.replace("totengine-human 2", "totengine-human 999"),
        before.replace(
            &format!("engine {}", env!("CARGO_PKG_VERSION")),
            "engine 0.0.0",
        ),
        before.replace("end\n", ""),
        before.replace("world ", "world 0"),
        before.clone() + "junk\n",
    ] {
        assert!(HumanSession::restore(rules(), "assets", &bad).is_err());
        assert_eq!(s.checkpoint("assets"), before);
    }
}

#[test]
fn discovery_in_first_processed_city_banks_later_city_science_until_human_choice() {
    let mut s = session(1);
    choose(&mut s);
    // Authored controlled city-phase fixture, not native round timing parity.
    s.game.world.map.terrain.fill(2);
    for (civ, x, y) in [(1, 10, 10), (1, 30, 10), (2, 50, 10)] {
        let k = s.game.world.create_city(&s.game.rules, civ, x, y).unwrap();
        let c = &mut s.game.world.cities[k];
        c.size = 3;
        c.specialists = 3; // one scientist
        c.worked = 1 << 20;
    }
    for civ in [1, 2] {
        s.game.world.research[civ].target = 0;
        s.game.world.research[civ].progress = (s
            .game
            .world
            .research_cost(&s.game.rules, civ as u8)
            .unwrap()
            - 1) as i16;
    }
    s.cursor = 8; // Exercise actual atomic run_turn city processing.
    assert_eq!(s.advance(1), 1);
    assert_eq!(s.game.world.research[1].target, -1);
    assert!(s.game.world.cities[0].net_science > 0);
    assert_eq!(
        s.game.world.research[1].progress,
        s.game.world.cities[0].net_science as i16
    );
    assert_eq!(
        s.notices
            .iter()
            .filter(|n| matches!(n, Notice::Discovery(_)))
            .count(),
        1,
        "foreign discovery is absent"
    );
    assert!(s.notices.contains(&Notice::Discovery(0)));
    assert!(matches!(s.decision, Some(Decision::Research { .. })));
    let banked = s.game.world.research[1].progress;
    assert_eq!(s.advance(100), 0);
    choose(&mut s);
    assert_eq!(s.game.world.research[1].progress, banked);
}

#[test]
fn accepted_movement_replays_and_hidden_changes_do_not_enter_human_view() {
    let mut s = session(1);
    choose(&mut s);
    let unit = s.view().units[0];
    let u = &s.game.world.units[unit.slot];
    let to = [
        (1, -1),
        (2, 0),
        (1, 1),
        (0, 2),
        (-1, 1),
        (-2, 0),
        (-1, -1),
        (0, -2),
    ]
    .into_iter()
    .filter_map(|(dx, dy)| {
        let x = i32::from(u.x) + dx;
        let y = i32::from(u.y) + dy;
        (x >= 0 && y >= 0).then_some((x as u16, y as u16))
    })
    .find(|&to| {
        let mut world = s.game.world.clone();
        world.apply_order(
            &s.game.rules,
            None,
            1,
            Order::Move {
                unit: unit.slot,
                to,
            },
        ) == OrderResult::Moved
    })
    .unwrap();
    send(&mut s, Command::Move { unit, to });
    assert_eq!(
        (
            s.game.world.units[unit.slot].x,
            s.game.world.units[unit.slot].y
        ),
        to
    );
    round_trip(&s);
    let view = s.view();
    s.game.world.treasury[2] += 1000;
    for u in s.game.world.units.iter_mut().filter(|u| u.owner == 2) {
        u.moves_left = 999;
    }
    let hidden = view
        .observation
        .seen
        .iter()
        .position(Option::is_none)
        .unwrap();
    s.game.world.map.terrain[hidden] = (s.game.world.map.terrain[hidden] + 1) % 11;
    assert_eq!(s.build_view(), view);
    // Simulate slot reuse while retaining the old client entity reference.
    s.game.world.units[unit.slot].id += 1;
    let before = s.checkpoint("assets");
    assert!(s
        .submit(Request {
            civ: 1,
            revision: s.revision,
            id: 99,
            command: Command::Move { unit, to }
        })
        .is_err());
    assert_eq!(s.checkpoint("assets"), before);
}

#[test]
fn unsupported_government_and_elimination_stop_with_a_readable_phase() {
    let mut s = session(1);
    choose(&mut s);
    s.game.world.governments[1] = 0;
    assert_eq!(s.build_view().phase, Phase::UnsupportedGovernment);
    let before = s.checkpoint("assets");
    assert_eq!(s.advance(256), 0);
    assert!(s
        .submit(Request {
            civ: 1,
            revision: s.revision,
            id: 99,
            command: Command::EndTurn
        })
        .is_err());
    assert_eq!(s.checkpoint("assets"), before);
    for u in s.game.world.units.iter_mut().filter(|u| u.owner == 1) {
        u.id = 0;
    }
    assert_eq!(s.phase(), Phase::Eliminated);
    assert_eq!(s.advance(1), 0);
}

#[test]
fn citizen_edits_are_immediate_atomic_and_replayable() {
    let mut s = session(1);
    choose(&mut s);
    let unit = s.view().units[0];
    send(&mut s, Command::BeginFound { unit });
    let Some(Decision::Founding { id, .. }) = s.decision else {
        panic!()
    };
    send(
        &mut s,
        Command::NameCity {
            decision: id,
            name: CityName::new("Citizens").unwrap(),
        },
    );
    let city = s.view().cities[0];
    // Human named founding allocates citizens without mutating on inspection.
    // An explicit centre command requests another automatic allocation.
    send(&mut s, Command::Worker { city, tile: 20 });
    let before = s.view();
    let tile = before.city_citizens[0]
        .radius
        .iter()
        .take(20)
        .position(|t| t.worked)
        .unwrap() as u8;
    let turn = before.observation.turn;
    let treasury = before.observation.treasury;
    let random = (
        s.game.world.rng.state(),
        s.game.world.combat_rng.state(),
        s.game.world.combat_rng.draws(),
    );
    let checkpoint = s.checkpoint("authored-test-resources-v1");
    let mut restored = round_trip(&s);
    let r = send(&mut s, Command::Worker { city, tile });
    restored.submit(r).unwrap();
    assert_eq!(restored.view(), s.view());
    let v = s.view();
    assert_eq!(v.city_citizens[0].specialists, vec![1]);
    assert!(!v.city_citizens[0].radius[tile as usize].worked);
    assert_eq!(v.observation.turn, turn);
    assert_eq!(v.observation.treasury, treasury);
    assert_eq!(
        random,
        (
            s.game.world.rng.state(),
            s.game.world.combat_rng.state(),
            s.game.world.combat_rng.draws()
        )
    );
    assert!(
        v.observation.city_economy[0].food_surplus
            < before.observation.city_economy[0].food_surplus
    );
    let after = s.checkpoint("authored-test-resources-v1");
    assert_eq!(s.submit(r).unwrap().revision, s.revision);
    assert_eq!(after, s.checkpoint("authored-test-resources-v1"));
    for command in [
        Command::Specialist { city, slot: 0 },
        Command::Worker { city, tile: 21 },
        Command::Worker {
            city: Entity {
                id: city.id + 1,
                ..city
            },
            tile,
        },
    ] {
        let bad = Request {
            civ: 1,
            revision: s.revision,
            id: 99,
            command,
        };
        assert!(s.submit(bad).is_err());
        assert_eq!(s.checkpoint("authored-test-resources-v1"), after);
    }
    send(&mut s, Command::Worker { city, tile });
    assert!(s.view().city_citizens[0].specialists.is_empty());
    assert!(s.view().city_citizens[0].radius[tile as usize].worked);
    round_trip(&s);
    let replay = Game::replay_human(config(), rules(), 1, s.game.log()).unwrap();
    assert_eq!(replay.world.checksum(), s.game.world.checksum());
    assert_ne!(checkpoint, after);
    send(&mut s, Command::EndTurn);
    assert_eq!(s.advance(1), 1);
    let mut partial = round_trip(&s);
    ready(&mut s, 1);
    ready(&mut partial, 256);
    assert_eq!(s.view(), partial.view());
    assert_eq!(s.game.world.checksum(), partial.game.world.checksum());
    let own = s.view().city_citizens;
    assert!(s
        .game
        .world
        .city_citizens(&s.game.rules, 2, city.slot)
        .is_none());
    s.game.world.treasury[2] += 999;
    assert_eq!(s.build_view().city_citizens, own);
}

#[test]
fn authored_city_roles_blocked_tiles_and_food_debt_refresh_without_a_round() {
    let mut s = session(1);
    choose(&mut s);
    let unit = s.view().units[0];
    send(&mut s, Command::BeginFound { unit });
    let Some(Decision::Founding { id, .. }) = s.decision else {
        panic!()
    };
    send(
        &mut s,
        Command::NameCity {
            decision: id,
            name: CityName::new("Role fixture").unwrap(),
        },
    );
    // Authored size-five fixture for the new command boundary. Native arithmetic
    // is independently covered by the retained worker/finance/mood vectors.
    let w = &mut s.game.world;
    w.cities[0].size = 5;
    w.cities[0].worked = 0;
    w.refresh_city_totals(&s.game.rules, 0, false).unwrap();
    let tile = w
        .city_citizens(&s.game.rules, 1, 0)
        .unwrap()
        .radius
        .iter()
        .take(20)
        .position(|t| t.worked)
        .unwrap() as u8;
    w.edit_citizen(&s.game.rules, 1, 0, tile, false).unwrap();
    let entertainer = w.city_finance(&s.game.rules, 0);
    w.edit_citizen(&s.game.rules, 1, 0, 0, true).unwrap();
    let tax = w.city_finance(&s.game.rules, 0);
    assert_eq!(tax.taxes, entertainer.taxes + 3);
    assert_eq!(tax.luxuries, entertainer.luxuries - 2);
    w.edit_citizen(&s.game.rules, 1, 0, 0, true).unwrap();
    let science = w.city_finance(&s.game.rules, 0);
    assert_eq!(science.science, tax.science + 3);
    assert_eq!(science.taxes, tax.taxes - 3);
    w.edit_citizen(&s.game.rules, 1, 0, 0, true).unwrap();
    assert_eq!(w.city_finance(&s.game.rules, 0), entertainer);
    let before = w.checksum();
    assert!(w.edit_citizen(&s.game.rules, 2, 0, tile, false).is_err());
    assert!(w.edit_citizen(&s.game.rules, 1, 0, 16, true).is_err());
    assert_eq!(w.checksum(), before);
    // Foreign overlap blocks assignment without exposing that city's worker mask.
    let mut neighbour = w.cities[0].clone();
    neighbour.city_id += 100;
    neighbour.owner = 2;
    neighbour.worked = 1 << tile;
    w.cities.push(neighbour);
    let before = w.checksum();
    assert!(!w.city_citizens(&s.game.rules, 1, 0).unwrap().radius[tile as usize].available);
    assert!(w.edit_citizen(&s.game.rules, 1, 0, tile, false).is_err());
    assert_eq!(w.checksum(), before);
    w.cities.pop();
    // Taking every worker off the land creates food debt; restoring workers
    // clears it. No turn, growth or treasury collection is performed here.
    loop {
        let detail = w.city_citizens(&s.game.rules, 1, 0).unwrap();
        let Some(k) = detail.radius.iter().take(20).position(|t| t.worked) else {
            break;
        };
        w.edit_citizen(&s.game.rules, 1, 0, k as u8, false).unwrap();
    }
    assert!((w.cities[0].food_box as i16) < 0);
    assert_eq!(w.cities[0].size, 5);
    w.edit_citizen(&s.game.rules, 1, 0, 20, false).unwrap();
    assert_eq!(w.cities[0].food_box as i16, 0);
}

#[test]
fn excess_entertainers_count_in_immediate_finance_and_mood() {
    let mut s = session(1);
    choose(&mut s);
    let unit = s.view().units[0];
    send(&mut s, Command::BeginFound { unit });
    let Some(Decision::Founding { id, .. }) = s.decision else {
        panic!()
    };
    send(
        &mut s,
        Command::NameCity {
            decision: id,
            name: CityName::new("Large city").unwrap(),
        },
    );
    // Authored large-city boundary, not a claimed naturally grown campaign.
    let w = &mut s.game.world;
    let rules = &s.game.rules;
    for count in [16, 17, 20, 30, 63] {
        w.cities[0].size = count;
        w.cities[0].worked = u32::from(count) << 26;
        w.cities[0].specialists = 0x55555555;
        let totals = w.refresh_city_totals(rules, 0, false).unwrap();
        assert_eq!(
            w.city_citizens(rules, 1, 0).unwrap().specialists,
            vec![1; count as usize]
        );
        assert_eq!(
            w.split_city_trade(rules, 0, 0, 0).luxuries,
            2 * i32::from(count)
        );
        assert_eq!(
            totals.mood,
            crate::happiness::Happiness::default(),
            "all citizens are specialists"
        );
        let at = w
            .city_citizens(rules, 1, 0)
            .unwrap()
            .radius
            .iter()
            .take(20)
            .position(|t| t.available)
            .unwrap() as u8;
        let turn = w.turn;
        let rng = (w.rng.state(), w.combat_rng.state());
        let treasury = w.treasury;
        let storage = (w.cities[0].shield_box, w.cities[0].building);
        // Cross the excess boundary in both directions through real edits.
        w.edit_citizen(rules, 1, 0, at, false).unwrap();
        assert_eq!(
            w.city_citizens(rules, 1, 0).unwrap().specialists.len(),
            count as usize - 1
        );
        assert_eq!(
            w.split_city_trade(rules, 0, 0, 0).luxuries,
            2 * (i32::from(count) - 1)
        );
        w.edit_citizen(rules, 1, 0, at, false).unwrap();
        assert_eq!(
            w.city_citizens(rules, 1, 0).unwrap().specialists.len(),
            count as usize
        );
        assert_eq!(w.city_finance(rules, 0).luxuries, totals.finance.luxuries);
        assert_eq!(
            w.city_happiness_with_luxuries(rules, 0, totals.finance.luxuries),
            crate::happiness::Happiness::default()
        );
        // Packed roles can change, excess citizens remain entertainers.
        w.edit_citizen(rules, 1, 0, 0, true).unwrap();
        assert_eq!(
            w.split_city_trade(rules, 0, 0, 0).luxuries,
            2 * (i32::from(count) - 1)
        );
        assert_eq!(w.split_city_trade(rules, 0, 0, 0).taxes, 3);
        assert_eq!(
            (
                w.turn,
                w.treasury,
                w.rng.state(),
                w.combat_rng.state(),
                w.cities[0].shield_box,
                w.cities[0].building
            ),
            (turn, treasury, rng.0, rng.1, storage.0, storage.1)
        );
    }
}

#[test]
fn quoted_production_budget_survives_reload_and_resets_on_close() {
    let mut custom = rules();
    let cosmic = custom.sections.get_mut("COSMIC").unwrap();
    cosmic.rows[18] = vec!["50".into()];
    cosmic.raw_lines[18] = "50 ; authored switching penalty".into();
    let mut cfg = config();
    cfg.difficulty = 2;
    let mut s = HumanSession::generated(cfg, custom, 1).unwrap();
    choose(&mut s);
    let unit = s.view().units[0];
    send(&mut s, Command::BeginFound { unit });
    let Some(Decision::Founding { id, .. }) = s.decision else {
        panic!()
    };
    send(
        &mut s,
        Command::NameCity {
            decision: id,
            name: CityName::new("Budget").unwrap(),
        },
    );
    let city = s.view().cities[0];
    for _ in 0..40 {
        if s.view().city_production[0].stored >= 10 {
            break;
        }
        send(&mut s, Command::EndTurn);
        ready(&mut s, 256);
        choose(&mut s);
    }
    let initial = s.view().city_production[0].stored;
    assert!(initial >= 10);
    let quote = s.preview_production(city, -4).unwrap();
    assert_eq!(quote.retained, initial - initial / 2);
    let before = s.checkpoint("budget-resources");
    assert!(s
        .submit(Request {
            civ: 1,
            revision: s.revision,
            id: 999,
            command: Command::QuotedProduction {
                city,
                quote: crate::city_production::ProductionChangeQuote {
                    initial_shields: quote.initial_shields + 1,
                    ..quote
                }
            }
        })
        .is_err());
    assert_eq!(before, s.checkpoint("budget-resources"));
    send(&mut s, Command::QuotedProduction { city, quote });
    assert_eq!(s.view().production_context_city, Some(city));
    let checkpoint = s.checkpoint("budget-resources");
    let mut kept =
        HumanSession::restore(s.game.rules.clone(), "budget-resources", &checkpoint).unwrap();
    assert_eq!(checkpoint, kept.checkpoint("budget-resources"));
    let restored = kept.preview_production(city, 0).unwrap();
    assert_eq!(restored.retained, initial);
    send(
        &mut kept,
        Command::QuotedProduction {
            city,
            quote: restored,
        },
    );
    assert_eq!(kept.view().city_production[0].stored, initial);
    let turn = s.view().observation.turn;
    let gold = s.view().observation.treasury;
    let stored = s.view().city_production[0].stored;
    send(&mut s, Command::CloseProduction { city });
    assert_eq!(s.view().production_context_city, None);
    assert_eq!(s.view().city_production[0].stored, stored);
    assert_eq!(
        (s.view().observation.turn, s.view().observation.treasury),
        (turn, gold)
    );
    let reopened = s.preview_production(city, 0).unwrap();
    assert_eq!(reopened.retained, stored - stored / 2);
    let checkpoint = s.checkpoint("budget-resources");
    let resumed =
        HumanSession::restore(s.game.rules.clone(), "budget-resources", &checkpoint).unwrap();
    assert_eq!(resumed.preview_production(city, 0).unwrap(), reopened);
}

#[test]
fn quoted_buy_and_sale_continue_through_recorded_play_and_checkpoints() {
    let mut s = session(1);
    let bogus = crate::city_production::HurryQuote {
        city_id: 1,
        item: 0,
        stored: 0,
        required: 20,
        price: 1,
        treasury: 0,
    };
    let before = s.checkpoint("authored-test-resources-v1");
    assert!(s
        .submit(Request {
            civ: 1,
            revision: s.revision,
            id: 999,
            command: Command::Buy {
                city: Entity { slot: 0, id: 1 },
                quote: bogus
            }
        })
        .is_err());
    assert_eq!(before, s.checkpoint("authored-test-resources-v1"));
    choose(&mut s);
    let unit = s.view().units[0];
    send(&mut s, Command::BeginFound { unit });
    let Some(Decision::Founding { id, .. }) = s.decision else {
        panic!()
    };
    send(
        &mut s,
        Command::NameCity {
            decision: id,
            name: CityName::new("Production").unwrap(),
        },
    );
    let city = s.view().cities[0];
    send(&mut s, Command::Production { city, item: -4 });
    let quote = s.view().city_production[0].hurry.unwrap();
    assert!(quote.price > quote.treasury);
    let before = s.checkpoint("authored-test-resources-v1");
    assert!(s
        .submit(Request {
            civ: 1,
            revision: s.revision,
            id: 1000,
            command: Command::Buy { city, quote }
        })
        .is_err());
    assert_eq!(before, s.checkpoint("authored-test-resources-v1"));
    let mut affordable = None;
    for _ in 0..80 {
        let view = s.view();
        assert!(
            view.observation
                .city_economy
                .iter()
                .all(|e| e.buildings[0] & (1 << 4) == 0),
            "must buy before ordinary completion"
        );
        if let Ok(q) = view.city_production[0].hurry {
            if q.price <= q.treasury {
                affordable = Some(q);
                break;
            }
        }
        send(&mut s, Command::EndTurn);
        ready(&mut s, 256);
        choose(&mut s);
    }
    let quote = affordable.expect("normal tax income funds a remaining production quote");
    let mut resumed = round_trip(&s);
    let before = s.checkpoint("authored-test-resources-v1");
    for command in [
        Command::Buy {
            city: Entity {
                id: city.id + 1,
                ..city
            },
            quote,
        },
        Command::Buy {
            city,
            quote: crate::city_production::HurryQuote {
                price: quote.price - 1,
                ..quote
            },
        },
    ] {
        assert!(s
            .submit(Request {
                civ: 1,
                revision: s.revision,
                id: 1001,
                command
            })
            .is_err());
        assert_eq!(before, s.checkpoint("authored-test-resources-v1"));
    }
    let turn = s.view().observation.turn;
    let random = (s.game.world.rng.state(), s.game.world.combat_rng.state());
    let request = send(&mut s, Command::Buy { city, quote });
    resumed.submit(request).unwrap();
    assert_eq!(s.view(), resumed.view());
    assert_eq!(s.view().observation.turn, turn);
    assert_eq!(
        random,
        (s.game.world.rng.state(), s.game.world.combat_rng.state())
    );
    assert!(!s.game.world.cities[city.slot].has_building(4));
    assert_eq!(s.game.world.cities[city.slot].shield_box, quote.required);
    assert_eq!(s.view().city_production[0].turns, Some(0));
    let after = s.checkpoint("authored-test-resources-v1");
    s.submit(request).unwrap();
    assert_eq!(after, s.checkpoint("authored-test-resources-v1"));
    round_trip(&s);
    send(&mut s, Command::EndTurn);
    ready(&mut s, 256);
    choose(&mut s);
    assert!(
        s.game.world.cities[city.slot].has_building(4),
        "purchase completes in the following city phase"
    );
    let sale = s.view().city_production[0]
        .improvements
        .iter()
        .find(|(id, _)| *id == 4)
        .unwrap()
        .1
        .unwrap();
    let mut resumed = round_trip(&s);
    let request = send(&mut s, Command::Sell { city, quote: sale });
    resumed.submit(request).unwrap();
    assert_eq!(s.view(), resumed.view());
    assert!(!s.game.world.cities[city.slot].has_building(4));
    assert_eq!(s.view().observation.treasury, sale.treasury + sale.income);
    let after = s.checkpoint("authored-test-resources-v1");
    s.submit(request).unwrap();
    assert_eq!(after, s.checkpoint("authored-test-resources-v1"));
    assert!(s
        .submit(Request {
            civ: 1,
            revision: s.revision,
            id: 1002,
            command: Command::Sell { city, quote: sale }
        })
        .is_err());
    assert_eq!(after, s.checkpoint("authored-test-resources-v1"));
    let mut resumed = round_trip(&s);
    let request = send(&mut s, Command::EndTurn);
    resumed.submit(request).unwrap();
    ready(&mut s, 256);
    ready(&mut resumed, 1);
    assert_eq!(s.view(), resumed.view());
}

#[test]
fn explored_tiles_and_foreign_cities_remember_last_sight_without_hidden_updates() {
    let mut s = session(1);
    choose(&mut s);
    let unit = s.view().units[0];
    send(&mut s, Command::BeginFound { unit });
    let Some(Decision::Founding { id, .. }) = s.decision else {
        panic!()
    };
    send(
        &mut s,
        Command::NameCity {
            decision: id,
            name: CityName::new("Seen city").unwrap(),
        },
    );
    // An authored visibility fixture: first observe a foreign city and its tile.
    let mut foreign = s.game.world.cities[0].clone();
    foreign.city_id += 100;
    foreign.owner = 2;
    foreign.known_to = 0;
    foreign.name = "Last seen".into();
    s.game.world.cities[0].city_id = 0;
    s.game.world.units[unit.slot].owner = 1;
    s.game.world.units[unit.slot].x = foreign.x;
    s.game.world.units[unit.slot].y = foreign.y;
    s.game.world.cities.push(foreign);
    s.game.world.units[unit.slot].id = unit.id;
    s.remember();
    let old = s.build_view();
    assert_eq!(old.observation.cities[0].name, "Last seen");
    let (x, y) = (s.game.world.cities[1].x, s.game.world.cities[1].y);
    let tile = s.game.world.map.stored_index(x, y).unwrap();
    // Remove own sight while preserving explored-map bits.
    for u in &mut s.game.world.units {
        if u.owner == 1 {
            u.id = 0;
        }
    }
    let hidden = &mut s.game.world.cities[1];
    hidden.size += 3;
    hidden.name = "Secret renamed city".into();
    hidden.owner = 3;
    s.game.world.map.road[tile] = !s.game.world.map.road[tile];
    s.remember();
    let now = s.build_view();
    assert_eq!(now.observation.cities, old.observation.cities);
    assert_eq!(now.observation.seen, old.observation.seen);
    assert_eq!(now.known_cities, old.known_cities);
    // A reused slot outside sight must not change the remembered identity.
    s.game.world.cities[1].city_id += 1;
    s.remember();
    assert_eq!(s.build_view().known_cities, old.known_cities);
    s.known_cities.clear();
    s.remember();
    assert!(
        s.build_view().observation.cities.is_empty(),
        "new city on an explored tile stays hidden"
    );
    // Re-establish actual sight and refresh both city and terrain memory.
    s.game.world.units[unit.slot].id = unit.id;
    s.remember();
    let seen = s.build_view();
    assert_eq!(seen.observation.cities[0].name, "Secret renamed city");
    assert_ne!(seen.observation.seen[tile], old.observation.seen[tile]);
    // Replacing a destroyed city at the same visible position removes its old slot.
    let mut replacement = s.game.world.cities[1].clone();
    replacement.city_id += 1;
    s.game.world.cities[1].city_id = 0;
    s.game.world.cities.push(replacement);
    s.remember();
    let replaced = s.build_view();
    assert_eq!(replaced.observation.cities.len(), 1);
    assert_eq!(replaced.observation.cities[0].index, 2);
}

#[test]
fn rate_commands_validate_commit_retry_and_survive_checkpoints() {
    use crate::rates::RatesView;
    let mut s = session(1);
    choose(&mut s);
    let start = s.view().rates.expect("human rates");
    assert_eq!(
        start,
        RatesView {
            government: 1,
            cap: 6,
            tax: 4,
            luxury: 0,
            science: 6
        }
    );
    let before = s.checkpoint("assets");
    // Illegal splits are rejected atomically: no revision, log or receipt change.
    for (science, tax) in [(7, 3), (6, 5), (1, 2), (0, 11)] {
        let r = Request {
            civ: 1,
            revision: s.revision,
            id: s.receipts.len() as u64 + 1,
            command: Command::Rates { science, tax },
        };
        assert!(s.submit(r).is_err(), "{science}/{tax}");
        assert_eq!(s.checkpoint("assets"), before);
        assert_eq!(s.view().rates, Some(start));
    }
    // A legal change commits, bumps the revision and shows the remainder as luxury.
    let legal = send(&mut s, Command::Rates { science: 3, tax: 4 });
    let view = s.view();
    assert_eq!(
        view.rates,
        Some(RatesView {
            tax: 4,
            luxury: 3,
            science: 3,
            ..start
        })
    );
    assert!(s.game.log().contains("rates 3 4 = rates-set"));
    // Retrying the same request returns the receipt; a changed payload is refused.
    let revision = s.revision;
    assert!(s.submit(legal).is_ok());
    assert_eq!(s.revision, revision);
    assert!(s
        .submit(Request {
            command: Command::Rates { science: 2, tax: 4 },
            ..legal
        })
        .is_err());
    // A no-op split is accepted without changing the shares.
    send(&mut s, Command::Rates { science: 3, tax: 4 });
    assert_eq!(s.view().rates.map(|r| r.luxury), Some(3));
    // Replaying the journal reproduces the same session.
    let restored = round_trip(&s);
    assert_eq!(restored.view().rates, s.view().rates);
}
