use super::*;

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
