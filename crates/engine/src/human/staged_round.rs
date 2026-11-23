//! Replayable per-civilization city passes for the staged saved-round recipe.
use super::*;

#[derive(Debug, Default)]
pub(super) struct StagedRound {
    pub(super) pending_city_pass: bool,
    pub(super) food_routes: crate::city_phase::FoodRouteState,
    pub(super) research_due: bool,
}

impl HumanSession {
    pub(super) fn run_staged_city_pass(&mut self, civ: u8) {
        let keep_view = civ != self.human
            && self.saved_move_state.is_some()
            && matches!(
                self.knowledge_recipe,
                ScenarioKnowledgeRecipe::SavedCityViewStagedRoundV1
                    | ScenarioKnowledgeRecipe::SavedOpponentStackV1
            );
        let before = keep_view.then(|| self.game.world.clone());
        if !keep_view {
            self.finish_saved_movement();
        }
        let mut food_routes = self.staged_round.as_ref().unwrap().food_routes;
        let mut discovery = false;
        let mut pass = |game: &mut Game| {
            let mut summary = crate::turn::TurnSummary::default();
            game.world
                .run_civ_city_phases(&game.rules, civ, &mut food_routes, &mut summary);
            let world = &mut game.world;
            for slot in 0..world.units.len() {
                let unit = &world.units[slot];
                if unit.id != 0 && unit.owner == civ {
                    world.units[slot].moves_left = world.movement_allowance(
                        &game.rules,
                        usize::from(unit.type_id),
                        civ,
                        unit.damage,
                    );
                }
            }
            discovery = !summary.discoveries.is_empty();
            summary
        };
        if civ == self.human {
            self.record_turn_notices(false, pass);
        } else {
            let summary = pass(&mut self.game);
            if let Some(before) = before {
                if self.keep_city_pass_view(&before, civ, &summary).is_err() {
                    self.finish_saved_movement();
                }
            }
        }
        let staged = self.staged_round.as_mut().unwrap();
        staged.food_routes = food_routes;
        staged.research_due |= civ == self.human && discovery;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::AiPolicy;

    fn installed_original() -> Option<(Vec<u8>, Rules, crate::events::EventEngine)> {
        let home = std::env::var_os("CIVTOT_HOME")?;
        let folder = std::path::Path::new(&home).join("Original");
        let snapshot = std::fs::read(folder.join("Ph_Auto2.SAV")).unwrap();
        let rules = tot_kit::parse_rules(&tot_kit::latin1(
            &std::fs::read(folder.join("Rules.txt")).unwrap(),
        ));
        let events = crate::events::EventEngine::new(
            tot_kit::parse_events(&tot_kit::latin1(
                &std::fs::read(folder.join("Events.txt")).unwrap(),
            ))
            .unwrap(),
        );
        Some((snapshot, rules, events))
    }

    #[test]
    fn installed_original_replays_across_staged_city_and_order_boundaries() {
        let Some((snapshot, rules, events)) = installed_original() else {
            return;
        };
        let input = || ScenarioInput {
            path: "Original/Ph_Auto2.SAV",
            snapshot: &snapshot,
            rules: rules.clone(),
            events: Some(events.clone()),
        };
        let dependencies = "totengine-scenario-dependencies 1
staged-round-test
";
        let policy = AiPolicy::BuiltIn1;
        let mut game =
            HumanSession::scenario_initialized_staged_round(input(), 4, policy, 7, dependencies)
                .unwrap();
        let legacy =
            HumanSession::scenario_initialized_current(input(), 4, policy, 7, dependencies)
                .unwrap();
        assert_eq!(legacy.initializer().unwrap().schema(), 2);
        assert!(matches!(legacy.decision, Some(Decision::Research { .. })));
        assert_eq!(game.initializer().unwrap().schema(), 3);
        assert_eq!(game.phase(), Phase::Human);
        assert!(game.decision.is_none());
        assert_eq!(game.view().observation.seen.iter().flatten().count(), 59);
        let initial_turn = game.game.world.turn;
        let old_human_city = game.game.world.cities[5].clone();
        let old_opponent_city = game.game.world.cities[6].clone();
        let restore = |checkpoint: &str| {
            HumanSession::restore_with(
                rules.clone(),
                dependencies,
                checkpoint,
                Some((&snapshot, Some(events.clone()))),
            )
            .unwrap()
        };

        game.submit(Request {
            civ: 4,
            revision: 0,
            id: 1,
            command: Command::EndTurn,
        })
        .unwrap();
        let before_pass = game.checkpoint(dependencies);
        let mut resumed = restore(&before_pass);
        assert_eq!(resumed.cursor, 5);
        assert!(resumed.staged_round.as_ref().unwrap().pending_city_pass);
        let log_before_pass = game.game.log().to_string();
        assert_eq!(game.advance(1), 1);
        assert_eq!(resumed.advance(1), 1);
        assert_eq!(game.game.log(), log_before_pass);
        assert_eq!(game.cursor, 5);
        assert_eq!(game.actions, 0);
        assert_ne!(game.game.world.cities[6], old_opponent_city);
        assert_eq!(game.game.world.cities[5], old_human_city);
        let after_pass = game.checkpoint(dependencies);
        assert_eq!(resumed.checkpoint(dependencies), after_pass);
        let wrong_schema = after_pass.replacen("initializer 3 ", "initializer 2 ", 1);
        assert!(HumanSession::restore_with(
            rules.clone(),
            dependencies,
            &wrong_schema,
            Some((&snapshot, Some(events.clone())))
        )
        .is_err());
        let mut stepwise = restore(&after_pass);

        let before_order = game.game.log().len();
        let mut resumed = restore(&after_pass);
        assert!(!resumed.staged_round.as_ref().unwrap().pending_city_pass);
        assert_eq!(game.advance(1), 1);
        assert_eq!(resumed.advance(1), 1);
        assert!(game.game.log()[before_order..].starts_with(&format!("T {initial_turn} C 5 ")));
        assert_eq!(
            resumed.checkpoint(dependencies),
            game.checkpoint(dependencies)
        );

        for _ in 0..32 {
            if game.phase() != Phase::Opponents {
                break;
            }
            assert_eq!(resumed.advance(64), game.advance(64));
        }
        assert_eq!(game.phase(), Phase::Human);
        assert_eq!(game.game.world.turn, initial_turn + 1);
        assert_eq!(game.cursor, 4);
        assert_eq!(game.game.world.research[4].target, -1);
        assert_eq!(game.game.world.research[4].progress, 0);
        assert!(game.decision.is_none());
        assert_ne!(game.game.world.cities[5], old_human_city);
        let resumed_checkpoint = resumed.checkpoint(dependencies);
        assert_eq!(game.checkpoint(dependencies), resumed_checkpoint);
        assert_eq!(restore(&resumed_checkpoint).view(), game.view());
        for _ in 0..2048 {
            if stepwise.advance(1) == 0 {
                break;
            }
        }
        assert_eq!(stepwise.view(), game.view());
        assert_eq!(stepwise.game.world.checksum(), game.game.world.checksum());
        assert_eq!(stepwise.game.log(), game.game.log());

        // The measured positive-science control reaches a choice. Check the
        // engine's opportunity separately from the replayed zero-science turn.
        stepwise.game.world.research[4].progress = 1;
        stepwise.require_research();
        assert!(matches!(stepwise.decision, Some(Decision::Research { .. })));
    }

    #[test]
    fn installed_saved_view_moves_replay_without_revealing_the_legacy_tile() {
        let Some((snapshot, rules, events)) = installed_original() else {
            return;
        };
        let deps = "totengine-scenario-dependencies 1\nsaved-movement-test\n";
        let mut game = HumanSession::scenario_initialized_saved_movement(
            ScenarioInput {
                path: "Original/Ph_Auto2.SAV",
                snapshot: &snapshot,
                rules: rules.clone(),
                events: Some(events.clone()),
            },
            4,
            AiPolicy::BuiltIn1,
            7,
            deps,
        )
        .unwrap();
        assert_eq!(game.initializer().unwrap().schema(), 4);
        let restore = |checkpoint: &str| {
            HumanSession::restore_with(
                rules.clone(),
                deps,
                checkpoint,
                Some((&snapshot, Some(events.clone()))),
            )
            .unwrap()
        };
        let order = |game: &HumanSession, slot: usize, to| {
            let view = game.view();
            Request {
                civ: 4,
                revision: view.revision,
                id: view.revision + 1,
                command: Command::Move {
                    unit: *view.units.iter().find(|unit| unit.slot == slot).unwrap(),
                    to,
                },
            }
        };
        let before = game.view();
        let origin = game.checkpoint(deps);
        let blocked = order(&game, 6, (61, 5));
        assert!(game.submit(blocked).is_err());
        assert_eq!(game.checkpoint(deps), origin);
        let movement = order(&game, 6, (62, 4));
        let response = game.submit(movement).unwrap();
        assert_eq!(response.result, Some(OrderResult::Moved));
        let after = game.view();
        let map = &game.game.world.map;
        let revealed: Vec<_> = before
            .observation
            .seen
            .iter()
            .zip(&after.observation.seen)
            .enumerate()
            .filter_map(|(i, (old, new))| (old.is_none() && new.is_some()).then_some(i))
            .collect();
        assert_eq!(revealed, [map.stored_index(62, 2).unwrap()]);
        assert!(after.observation.seen[12 * map.stored_w + 27].is_none());
        let unit = after
            .observation
            .units
            .iter()
            .find(|unit| unit.index == 6)
            .unwrap();
        assert_eq!((unit.x, unit.y, unit.moves_left), (62, 4, 0));
        let checkpoint = game.checkpoint(deps);
        assert_eq!(game.submit(movement).unwrap(), response);
        assert_eq!(game.checkpoint(deps), checkpoint);
        let mut replay = restore(&checkpoint);
        assert_eq!(replay.view(), after);
        assert_eq!(replay.game.world.map, game.game.world.map);
        let occupied = order(&game, 9, (62, 4));
        assert!(game.submit(occupied).is_err());
        assert_eq!(game.checkpoint(deps), checkpoint);
        let follow_up = order(&game, 9, (62, 6));
        assert_eq!(
            game.submit(follow_up).unwrap().result,
            Some(OrderResult::Moved)
        );
        assert_eq!(
            replay.submit(follow_up).unwrap().result,
            Some(OrderResult::Moved)
        );
        assert_eq!(replay.checkpoint(deps), game.checkpoint(deps));
        let mut skips = restore(&origin);
        let initial_view = skips.view();
        let initial_map = skips.game.world.map.clone();
        for slot in [6, 9, 26, 18] {
            let view = skips.view();
            let unit = *view.units.iter().find(|u| u.slot == slot).unwrap();
            let request = Request {
                civ: 4,
                revision: view.revision,
                id: view.revision + 1,
                command: Command::Skip { unit },
            };
            assert_eq!(
                skips.submit(request).unwrap().result,
                Some(OrderResult::Skipped)
            );
        }
        let skipped = skips.view();
        assert_eq!(skipped.observation.seen, initial_view.observation.seen);
        assert_eq!(skips.game.world.map, initial_map);
        assert!(skipped
            .observation
            .units
            .iter()
            .filter(|u| u.owner == 4)
            .all(|u| u.moves_left == 0));
        assert_eq!(restore(&skips.checkpoint(deps)).view(), skipped);
        assert!(skips.saved_move_state.is_some());

        let end_turn = Request {
            civ: 4,
            revision: 2,
            id: 3,
            command: Command::EndTurn,
        };
        let before_handoff = game.view();
        game.submit(end_turn).unwrap();
        assert_eq!(game.phase(), Phase::Opponents);
        assert_eq!(
            game.view().observation.seen,
            before_handoff.observation.seen
        );
        assert!(game.saved_move_state.is_some());
        let mut resumed = restore(&game.checkpoint(deps));
        assert_eq!(resumed.view(), game.view());
        assert_eq!(resumed.advance(1), 1);
        assert_eq!(game.advance(1), 1);
        assert!(game.saved_move_state.is_none());
        assert!(game.saved_initial.is_none());
        assert_eq!(game.checkpoint(deps), resumed.checkpoint(deps));
    }

    #[test]
    fn installed_city_pass_and_production_order_preserve_the_saved_view() {
        let Some((snapshot, rules, events)) = installed_original() else {
            return;
        };
        let deps = "totengine-scenario-dependencies 1\nsaved-city-view-test\n";
        let mut game = HumanSession::scenario_initialized_saved_city_view(
            ScenarioInput {
                path: "Original/Ph_Auto2.SAV",
                snapshot: &snapshot,
                rules: rules.clone(),
                events: Some(events.clone()),
            },
            4,
            AiPolicy::BuiltIn1,
            7,
            deps,
        )
        .unwrap();
        assert_eq!(game.initializer().unwrap().schema(), 5);
        for slot in [6, 9, 26, 18] {
            let view = game.view();
            let unit = *view.units.iter().find(|unit| unit.slot == slot).unwrap();
            assert_eq!(
                game.submit(Request {
                    civ: 4,
                    revision: view.revision,
                    id: view.revision + 1,
                    command: Command::Skip { unit },
                })
                .unwrap()
                .result,
                Some(OrderResult::Skipped)
            );
        }
        let view = game.view();
        game.submit(Request {
            civ: 4,
            revision: view.revision,
            id: view.revision + 1,
            command: Command::EndTurn,
        })
        .unwrap();
        let before = game.view();
        let before_player_cache = game.game.world.map.known_improvements[3].clone();
        assert_eq!(before.observation.seen.iter().flatten().count(), 59);
        assert_eq!(game.game.world.units[34].id, 0);
        let restore = |text: &str| {
            HumanSession::restore_with(
                rules.clone(),
                deps,
                text,
                Some((&snapshot, Some(events.clone()))),
            )
            .unwrap()
        };
        let handoff = game.checkpoint(deps);
        let mut visible_birth = restore(&handoff);
        let tile = visible_birth.game.world.map.stored_index(43, 37).unwrap();
        // Make this destination known in a control. Keeping the old unit
        // projection here would hide a birth at a known tile.
        visible_birth.known_tiles[tile] = before.observation.seen.iter().copied().flatten().next();
        assert_eq!(visible_birth.advance(1), 1);
        assert!(visible_birth.saved_move_state.is_none());
        let mut replay = restore(&handoff);
        assert_eq!(game.advance(1), 1);
        assert_eq!(replay.advance(1), 1);
        assert!(game.saved_move_state.is_some());
        let created = &game.game.world.units[34];
        assert_ne!(created.id, 0);
        assert_eq!((created.owner, created.x, created.y), (5, 43, 37));
        let after = game.view();
        assert_eq!(before.observation.seen, after.observation.seen);
        assert_eq!(before.observation.units, after.observation.units);
        assert_eq!(
            game.game.world.map.known_improvements[3],
            before_player_cache
        );
        let checkpoint = game.checkpoint(deps);
        assert_eq!(replay.checkpoint(deps), checkpoint);
        let mut replay = restore(&checkpoint);
        assert_eq!(replay.view(), after);
        assert_eq!(game.advance(1), 1);
        assert_eq!(replay.advance(1), 1);
        assert!(game
            .game
            .log()
            .lines()
            .last()
            .unwrap()
            .contains(" C 5 prod "));
        assert!(game.saved_move_state.is_some());
        assert_eq!(game.view().observation.seen, after.observation.seen);
        assert_eq!(game.checkpoint(deps), replay.checkpoint(deps));
        for _ in 0..64 {
            if game.saved_move_state.is_none() {
                break;
            }
            assert_eq!(game.advance(1), 1);
            assert_eq!(replay.advance(1), 1);
        }
        assert!(game.saved_move_state.is_none());
        assert_eq!(game.checkpoint(deps), replay.checkpoint(deps));
    }

    #[test]
    fn installed_opponent_stack_order_replays_with_the_saved_view() {
        let Some((snapshot, rules, events)) = installed_original() else {
            return;
        };
        let deps = "totengine-scenario-dependencies 1\nsaved-opponent-stack-test\n";
        let mut game = HumanSession::scenario_initialized_saved_opponent_stack(
            ScenarioInput {
                path: "Original/Ph_Auto2.SAV",
                snapshot: &snapshot,
                rules: rules.clone(),
                events: Some(events.clone()),
            },
            4,
            AiPolicy::BuiltIn1,
            7,
            deps,
        )
        .unwrap();
        assert_eq!(game.initializer().unwrap().schema(), 6);
        for slot in [6, 9, 26, 18] {
            let view = game.view();
            let unit = *view.units.iter().find(|unit| unit.slot == slot).unwrap();
            game.submit(Request {
                civ: 4,
                revision: view.revision,
                id: view.revision + 1,
                command: Command::Skip { unit },
            })
            .unwrap();
        }
        let revision = game.view().revision;
        game.submit(Request {
            civ: 4,
            revision,
            id: revision + 1,
            command: Command::EndTurn,
        })
        .unwrap();
        let original_view = game.view();
        let mut prior = game.checkpoint(deps);
        let mut reached = false;
        for _ in 0..64 {
            assert_eq!(game.advance(1), 1);
            if game
                .game
                .log()
                .lines()
                .last()
                .unwrap()
                .contains(" C 5 move 2 43 37 = moved")
            {
                reached = true;
                break;
            }
            prior = game.checkpoint(deps);
        }
        assert!(reached, "did not reach checked stack move");
        let after = game.view();
        assert!(game.saved_move_state.is_some());
        assert_eq!(original_view.observation.seen, after.observation.seen);
        assert_eq!(original_view.observation.units, after.observation.units);
        assert_eq!(after.observation.seen.iter().flatten().count(), 59);
        let units = &game.game.world.units;
        for (position, slot) in [34, 19, 2].into_iter().enumerate() {
            assert_eq!(
                (units[slot].x, units[slot].y, units[slot].stack_pos),
                (43, 37, position as u16)
            );
        }
        let restore = |text: &str| {
            HumanSession::restore_with(
                rules.clone(),
                deps,
                text,
                Some((&snapshot, Some(events.clone()))),
            )
            .unwrap()
        };
        let saved = game.saved_move_state.as_ref().unwrap();
        assert_eq!(saved.units[34].stack_next(), 19);
        assert_eq!(
            (saved.units[19].stack_prev(), saved.units[19].stack_next()),
            (34, 2)
        );
        assert_eq!(saved.units[2].stack_prev(), 19);
        let mut visible = restore(&prior);
        let dest = visible.game.world.map.stored_index(43, 37).unwrap();
        visible.saved_move_state.as_mut().unwrap().maps[0].tile_fields[6 * dest + 4] |= 1 << 4;
        let world_identity = visible.game.world.checksum();
        let log = visible.game.log().to_owned();
        assert_eq!(
            visible.try_saved_opponent_stack(
                5,
                Order::Move {
                    unit: 2,
                    to: (43, 37)
                }
            ),
            None
        );
        assert_eq!(visible.game.world.checksum(), world_identity);
        assert_eq!(visible.game.log(), log);
        let checkpoint = game.checkpoint(deps);
        let mut replay = restore(&prior);
        assert_eq!(replay.advance(1), 1);
        assert_eq!(replay.checkpoint(deps), checkpoint);
        assert_eq!(restore(&checkpoint).view(), after);
    }
}
