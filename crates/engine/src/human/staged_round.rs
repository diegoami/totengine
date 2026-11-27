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
        // The witnessed per-civ turn start (import-turn-start-refresh.json):
        // civ 2's turn-50 pass begins with the native turn start clearing the
        // done flag of the pass's own live unit records. Scoped to the naval
        // allocation route's witnessed boundary.
        if self.knowledge_recipe == ScenarioKnowledgeRecipe::SavedNavalAllocationV1
            && civ == 2
            && self.game.world.turn == 50
        {
            if let Some(saved) = self.saved_move_state.as_mut() {
                for unit in &mut saved.units {
                    let id = u32::from_le_bytes(unit.raw[30..34].try_into().unwrap());
                    if 0 < id && id < 0x8000_0000 && unit.raw[9] == 2 {
                        unit.raw[6] &= !0x40;
                    }
                }
            }
        }
        let human = civ == self.human;
        let keep_view = self.saved_move_state.is_some()
            && (!human || self.knowledge_recipe.checked_replay())
            && matches!(
                self.knowledge_recipe,
                ScenarioKnowledgeRecipe::SavedCityViewStagedRoundV1
                    | ScenarioKnowledgeRecipe::SavedOpponentStackV1
                    | ScenarioKnowledgeRecipe::SavedOpponentCombatV1
                    | ScenarioKnowledgeRecipe::SavedHumanProductionV1
                    | ScenarioKnowledgeRecipe::SavedOpponentContactV1
                    | ScenarioKnowledgeRecipe::SavedOpponentCityCombatV1
                    | ScenarioKnowledgeRecipe::SavedOpponentHutV1
                    | ScenarioKnowledgeRecipe::SavedOpponentArmedStackV1
                    | ScenarioKnowledgeRecipe::SavedOpponentArmedStackV2
                    | ScenarioKnowledgeRecipe::SavedOpponentZoneV1
                    | ScenarioKnowledgeRecipe::SavedOpponentFoundingV1
                    | ScenarioKnowledgeRecipe::SavedOpponentWestwardV1
                    | ScenarioKnowledgeRecipe::SavedNavalArrivalV1
                    | ScenarioKnowledgeRecipe::SavedRemoteNavalStackV1
                    | ScenarioKnowledgeRecipe::SavedNavalAllocationV1
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
        let summary = if human {
            let mut saved_summary = None;
            self.record_turn_notices(false, |game| {
                let summary = pass(game);
                if keep_view {
                    saved_summary = Some(summary.clone());
                }
                summary
            });
            saved_summary
        } else {
            Some(pass(&mut self.game))
        };
        if let (Some(before), Some(summary)) = (before, summary) {
            if self.keep_city_pass_view(&before, civ, &summary).is_err() {
                self.finish_saved_movement();
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
    fn installed_turn_48_with_alternate_naval_slots_keeps_research_and_fog() {
        let Some((snapshot, rules, events)) = installed_original() else {
            return;
        };
        let deps = "totengine-scenario-dependencies 1\nalternate-turn-48-test\n";
        let make = |recipe| {
            HumanSession::scenario_initialized_recipe(
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
                recipe,
            )
            .unwrap()
        };
        let submit = |session: &mut HumanSession, command| {
            let revision = session.view().revision;
            session
                .submit(Request {
                    civ: 4,
                    revision,
                    id: revision + 1,
                    command,
                })
                .unwrap()
        };
        for recipe in [
            ScenarioKnowledgeRecipe::SavedRemoteNavalStackV1,
            ScenarioKnowledgeRecipe::SavedNavalAllocationV1,
        ] {
            let mut session = make(recipe);
            let start = session.view();
            for _ in 0..5 {
                submit(&mut session, Command::EndTurn);
                session.advance(1024);
            }
            let view = session.view();
            assert_eq!((view.observation.turn, view.phase), (48, Phase::Human));
            let Some(Decision::Research {
                id, ref available, ..
            }) = view.decision
            else {
                panic!("the alternate route should require a new research project");
            };
            if recipe == ScenarioKnowledgeRecipe::SavedRemoteNavalStackV1 {
                assert!(session.saved_move_state.is_none());
                assert_eq!(view.observation.seen.iter().flatten().count(), 60);
                continue;
            }
            assert_eq!(view.observation.seen, start.observation.seen);
            let saved = session.saved_move_state.as_ref().unwrap();
            assert_eq!(
                (saved.units[38].type_id(), saved.units[38].stack_next()),
                (32, 37)
            );
            assert_eq!(saved.units[37].stack_next(), 2);
            let pending = session.checkpoint(deps);
            let restore = |bytes: &str| {
                HumanSession::restore_with(
                    rules.clone(),
                    deps,
                    bytes,
                    Some((&snapshot, Some(events.clone()))),
                )
                .unwrap()
            };
            assert_eq!(restore(&pending).view(), view);
            let choice = available[0];
            let result = submit(
                &mut session,
                Command::Research {
                    decision: id,
                    advance: choice,
                },
            );
            assert_eq!(result.result, Some(OrderResult::ResearchSet));
            assert_eq!(session.view().observation.seen, start.observation.seen);
            assert!(session.view().decision.is_none());
            let chosen = session.checkpoint(deps);
            assert_eq!(restore(&chosen).checkpoint(deps), chosen);
            assert_eq!(restore(&chosen).view(), session.view());
        }
    }

    #[test]
    fn installed_wait_preserves_movement_and_replays_deferrals() {
        let Some((snapshot, rules, events)) = installed_original() else {
            return;
        };
        let deps = "totengine-scenario-dependencies 1\nwait-test\n";
        let create = |bytes: &[u8]| {
            HumanSession::scenario_initialized_saved_opponent_combat(
                ScenarioInput {
                    path: "Original/Ph_Auto2.SAV",
                    snapshot: bytes,
                    rules: rules.clone(),
                    events: Some(events.clone()),
                },
                4,
                AiPolicy::BuiltIn1,
                7,
                deps,
            )
            .unwrap()
        };
        let mut session = create(&snapshot);
        let initial = session.checkpoint(deps);
        let mut deferred_save = snapshot.clone();
        let units = tot_kit::parse_sav(&snapshot).unwrap().unit_region.0;
        deferred_save[units + 40 * 9 + 7] |= 0x40;
        assert_eq!(
            create(&deferred_save)
                .saved_initial_focus((61, 5))
                .unwrap()
                .slot,
            6
        );
        let before = session.view();
        let world = session.game.world.checksum();
        let log = session.game.log().to_owned();
        let restore = |text: &str| {
            HumanSession::restore_with(
                rules.clone(),
                deps,
                text,
                Some((&snapshot, Some(events.clone()))),
            )
            .unwrap()
        };
        let mut actor = session.saved_initial_focus((61, 5)).unwrap();
        assert_eq!(actor.slot, 9);
        for (next, held) in [
            (6, &[9][..]),
            (26, &[6, 9]),
            (18, &[6, 9, 26]),
            (26, &[18]),
            (9, &[18, 26]),
            (6, &[9, 18, 26]),
        ] {
            let revision = session.view().revision;
            let request = Request {
                civ: 4,
                revision,
                id: revision + 1,
                command: Command::Wait { unit: actor },
            };
            let response = session.submit(request).unwrap();
            assert_eq!(response.result, Some(OrderResult::Waited));
            assert_eq!(session.submit(request).unwrap(), response);
            let saved = session.saved_move_state.as_ref().unwrap();
            let flags: Vec<_> = [6, 9, 18, 26]
                .into_iter()
                .filter(|&slot| saved.units[slot].attr() & 0x4000 != 0)
                .collect();
            assert_eq!(flags, held);
            assert_eq!(session.game.world.checksum(), world);
            assert_eq!(session.game.log(), log);
            assert_eq!(session.view().observation.units, before.observation.units);
            assert_eq!(session.view().observation.seen, before.observation.seen);
            actor = session.saved_order_focus().unwrap();
            assert_eq!(actor.slot, next);
            let checkpoint = session.checkpoint(deps);
            let replay = restore(&checkpoint);
            assert_eq!(replay.saved_order_focus(), Some(actor));
            assert_eq!(replay.checkpoint(deps), checkpoint);
        }
        let mut moving = restore(&initial);
        let waiter = moving.saved_initial_focus((61, 5)).unwrap();
        moving
            .submit(Request {
                civ: 4,
                revision: 0,
                id: 1,
                command: Command::Wait { unit: waiter },
            })
            .unwrap();
        let mover = moving.saved_order_focus().unwrap();
        assert_eq!(mover.slot, 6);
        moving
            .submit(Request {
                civ: 4,
                revision: 1,
                id: 2,
                command: Command::Move {
                    unit: mover,
                    to: (62, 4),
                },
            })
            .unwrap();
        assert_eq!(
            moving.saved_move_state.as_ref().unwrap().units[9].attr() & 0x4000,
            0x4000
        );
        assert_eq!(moving.game.world.units[6].moves_left, 0);
        assert_eq!(moving.saved_order_focus().unwrap().slot, 26);
        let checkpoint = moving.checkpoint(deps);
        assert_eq!(
            restore(&checkpoint).saved_order_focus(),
            moving.saved_order_focus()
        );

        let mut solitary = restore(&initial);
        for slot in [6, 18, 26] {
            let view = solitary.view();
            let unit = *view.units.iter().find(|unit| unit.slot == slot).unwrap();
            solitary
                .submit(Request {
                    civ: 4,
                    revision: view.revision,
                    id: view.revision + 1,
                    command: Command::Skip { unit },
                })
                .unwrap();
        }
        let actor = solitary.saved_initial_focus((61, 5)).unwrap();
        assert_eq!(actor.slot, 9);
        let view = solitary.view();
        solitary
            .submit(Request {
                civ: 4,
                revision: view.revision,
                id: view.revision + 1,
                command: Command::Wait { unit: actor },
            })
            .unwrap();
        assert_eq!(solitary.saved_order_focus(), Some(actor));
        assert_eq!(
            solitary.saved_move_state.as_ref().unwrap().units[9].attr() & 0x4000,
            0
        );
        assert_eq!(solitary.game.world.units[9].moves_left, 3);
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
            visible.try_saved_opponent_move(
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
        let mut replay = restore(&checkpoint);
        let mut reached = false;
        for _ in 0..32 {
            assert_eq!(game.advance(1), 1);
            assert_eq!(replay.advance(1), 1);
            if game
                .game
                .log()
                .lines()
                .last()
                .unwrap()
                .contains(" C 5 move 10 41 37 = moved")
            {
                reached = true;
                break;
            }
        }
        assert!(reached, "did not reach the checked move to empty land");
        assert!(game.saved_move_state.is_some());
        assert_eq!(game.view().observation.seen, original_view.observation.seen);
        let unit = &game.game.world.units[10];
        assert_eq!((unit.x, unit.y, unit.stack_pos), (41, 37, 0));
        assert_eq!(replay.checkpoint(deps), game.checkpoint(deps));
        assert_eq!(game.advance(1), 1);
        assert_eq!(replay.advance(1), 1);
        assert!(game
            .game
            .log()
            .lines()
            .last()
            .unwrap()
            .contains(" C 5 move 19 42 36 = moved"));
        let observed = game.saved_move_state.as_ref().unwrap();
        assert_eq!(observed.units[34].stack_next(), 2);
        assert_eq!(observed.units[2].stack_prev(), 34);
        assert_eq!(
            (
                observed.units[19].stack_prev(),
                observed.units[19].stack_next()
            ),
            (-1, -1)
        );
        assert_eq!(game.view().observation.seen, original_view.observation.seen);
        assert_eq!(replay.checkpoint(deps), game.checkpoint(deps));
    }

    #[test]
    fn installed_paid_human_unit_keeps_the_saved_map() {
        let Some((snapshot, rules, events)) = installed_original() else {
            return;
        };
        let deps = "totengine-scenario-dependencies 1\nhuman-production-test\n";
        for (recipe, explored, shields) in [
            (ScenarioKnowledgeRecipe::SavedOpponentCombatV1, 60, 2),
            (ScenarioKnowledgeRecipe::SavedHumanProductionV1, 59, 0),
            (ScenarioKnowledgeRecipe::SavedOpponentContactV1, 59, 0),
            (ScenarioKnowledgeRecipe::SavedOpponentCityCombatV1, 59, 0),
            (ScenarioKnowledgeRecipe::SavedOpponentHutV1, 59, 0),
            (ScenarioKnowledgeRecipe::SavedOpponentArmedStackV1, 59, 0),
            (ScenarioKnowledgeRecipe::SavedOpponentArmedStackV2, 59, 0),
            (ScenarioKnowledgeRecipe::SavedOpponentZoneV1, 59, 0),
            (ScenarioKnowledgeRecipe::SavedOpponentFoundingV1, 59, 0),
            (ScenarioKnowledgeRecipe::SavedOpponentWestwardV1, 59, 0),
            (ScenarioKnowledgeRecipe::SavedNavalArrivalV1, 59, 0),
            (ScenarioKnowledgeRecipe::SavedRemoteNavalStackV1, 59, 0),
            (ScenarioKnowledgeRecipe::SavedNavalAllocationV1, 59, 0),
        ] {
            let mut session = HumanSession::scenario_initialized_recipe(
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
                recipe,
            )
            .unwrap();
            let initial = session.view();
            let city = *initial.cities.iter().find(|c| c.slot == 5).unwrap();
            let submit = |session: &mut HumanSession, command| {
                let revision = session.view().revision;
                session
                    .submit(Request {
                        civ: 4,
                        revision,
                        id: revision + 1,
                        command,
                    })
                    .unwrap()
            };
            submit(&mut session, Command::Worker { city, tile: 4 });
            let quote = session
                .view()
                .city_production
                .iter()
                .find(|p| p.index == city.slot)
                .unwrap()
                .hurry
                .unwrap();
            submit(&mut session, Command::Buy { city, quote });
            submit(&mut session, Command::EndTurn);
            session.advance(128);
            let after = session.view();
            assert_eq!(after.phase, Phase::Human);
            assert_eq!(after.observation.turn, 44);
            assert_eq!(after.observation.seen.iter().flatten().count(), explored);
            let new_unit = after
                .observation
                .units
                .iter()
                .find(|u| !initial.units.iter().any(|old| old.slot == u.index))
                .unwrap();
            assert_eq!(
                (new_unit.owner, new_unit.type_id, new_unit.x, new_unit.y),
                (4, 3, 53, 9)
            );
            let city_after = after
                .observation
                .cities
                .iter()
                .find(|c| c.index == city.slot)
                .unwrap();
            assert_eq!(city_after.shield_box, Some(shields));
            if recipe != ScenarioKnowledgeRecipe::SavedOpponentCombatV1 {
                assert_eq!(after.observation.seen, initial.observation.seen);
                assert!(session.saved_move_state.is_some());
                for (anchor, expected) in [
                    ((51, 9), 18),
                    ((53, 9), 26),
                    ((54, 8), 26),
                    ((61, 5), 9),
                    ((0, 0), 6),
                ] {
                    assert_eq!(
                        session.saved_initial_focus(anchor).map(|u| u.slot),
                        Some(expected)
                    );
                }
            }
            let checkpoint = session.checkpoint(deps);
            let replay = HumanSession::restore_with(
                rules.clone(),
                deps,
                &checkpoint,
                Some((&snapshot, Some(events.clone()))),
            )
            .unwrap();
            assert_eq!(replay.checkpoint(deps), checkpoint);
            assert_eq!(replay.view(), after);
            if recipe == ScenarioKnowledgeRecipe::SavedOpponentCombatV1 {
                continue;
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
            let unit = |slot| {
                *after
                    .units
                    .iter()
                    .find(|entity| entity.slot == slot)
                    .unwrap()
            };
            let city_stack = [unit(26), unit(new_unit.index)];
            let map = &session.saved_move_state.as_ref().unwrap().maps[0];
            let differences = |before: &[u8], after: &[u8]| {
                before
                    .iter()
                    .zip(after)
                    .enumerate()
                    .filter_map(|(i, (&a, &b))| (a != b).then_some((i, a, b)))
                    .collect::<Vec<_>>()
            };
            for [actor, mate] in [city_stack, [city_stack[1], city_stack[0]]] {
                let mut wait = restore(&checkpoint);
                assert!(wait.can_wait_in_saved_view(actor));
                assert_eq!(
                    submit(&mut wait, Command::Wait { unit: actor }).result,
                    Some(OrderResult::Waited)
                );
                assert_eq!(wait.saved_order_focus(), Some(mate));
                assert_eq!(wait.view().observation.seen, after.observation.seen);
                let saved = wait.saved_move_state.as_ref().unwrap();
                assert_eq!(&saved.maps[0], map);
                assert_eq!(saved.units[actor.slot].attr() & 0x4000, 0x4000);
                assert_eq!(saved.units[actor.slot].moves_spent(), 0);
                let deferred = wait.checkpoint(deps);
                let reloaded = restore(&deferred);
                assert_eq!(reloaded.saved_order_focus(), Some(mate));
                assert_eq!(reloaded.checkpoint(deps), deferred);
                assert_eq!(reloaded.view(), wait.view());

                let mut skip = restore(&checkpoint);
                assert_eq!(
                    submit(&mut skip, Command::Skip { unit: actor }).result,
                    Some(OrderResult::Skipped)
                );
                assert_eq!(skip.saved_order_focus(), Some(mate));
                assert_eq!(skip.view().observation.seen, after.observation.seen);
                let saved = skip.saved_move_state.as_ref().unwrap();
                let original = session.saved_move_state.as_ref().unwrap();
                assert_eq!(&saved.maps[0], map);
                assert_eq!(saved.units[mate.slot], original.units[mate.slot]);
                assert_eq!(
                    differences(
                        &original.units[actor.slot].raw,
                        &saved.units[actor.slot].raw
                    ),
                    [(10, 0, 3), (13, if actor.slot == 26 { 5 } else { 0 }, 255)]
                );
                assert_eq!(skip.game.world.units[actor.slot].moves_left, 0);
                let skipped = skip.checkpoint(deps);
                let reloaded = restore(&skipped);
                assert_eq!(reloaded.saved_order_focus(), Some(mate));
                assert_eq!(reloaded.checkpoint(deps), skipped);
                assert_eq!(reloaded.view(), skip.view());

                let mut moved = restore(&checkpoint);
                assert!(moved
                    .submit(Request {
                        civ: 4,
                        revision: after.revision,
                        id: after.revision + 1,
                        command: Command::Move {
                            unit: actor,
                            to: (54, 8)
                        },
                    })
                    .is_err());
                assert_eq!(moved.checkpoint(deps), checkpoint);
                let result = submit(
                    &mut moved,
                    Command::Move {
                        unit: actor,
                        to: (52, 8),
                    },
                );
                assert_eq!(result.result, Some(OrderResult::Moved));
                let saved = moved.saved_move_state.as_ref().unwrap();
                let record = &saved.units[actor.slot];
                assert_eq!((record.x(), record.y(), record.moves_spent()), (52, 8, 6));
                for entity in [actor, mate] {
                    let record = &saved.units[entity.slot];
                    assert_eq!((record.stack_prev(), record.stack_next()), (-1, -1));
                    assert_eq!(moved.game.world.units[entity.slot].stack_pos, 0);
                }
                assert_eq!(
                    (saved.units[mate.slot].x(), saved.units[mate.slot].y()),
                    (53, 9)
                );
                assert_eq!(
                    differences(&map.tile_fields, &saved.maps[0].tile_fields),
                    [(2077, 0, 1), (2081, 240, 64)]
                );
                let area = saved.map_header.surface();
                assert_eq!(
                    differences(&map.visibility, &saved.maps[0].visibility),
                    [(3 * area + 346, 0, 1)]
                );
                assert_eq!(moved.view().observation.seen.iter().flatten().count(), 59);
                assert_eq!(moved.game.world.combat_rng, session.game.world.combat_rng);
                let destination = moved.checkpoint(deps);
                let reloaded = restore(&destination);
                assert_eq!(reloaded.checkpoint(deps), destination);
                assert_eq!(reloaded.view(), moved.view());
            }
            // Reproduce the actual checked route so first-contact inputs
            // can be compared to the prepared native exchange independently.
            let mut contact = restore(&checkpoint);
            submit(&mut contact, Command::Wait { unit: unit(26) });
            submit(
                &mut contact,
                Command::Move {
                    unit: unit(new_unit.index),
                    to: (52, 8),
                },
            );
            submit(&mut contact, Command::EndTurn);
            assert_eq!(contact.advance(8), 8);
            let mut world = contact.game.world.clone();
            let saved = contact.saved_move_state.as_ref().unwrap();
            assert_eq!((world.turn, contact.cursor), (44, 5));
            assert_eq!((world.diplomacy[5][6], world.diplomacy[6][5]), (0, 0));
            assert_eq!(world.combat_rng.state(), 0xd766ce46);
            let rejected = crate::scenario::opponent_stack::stage(
                saved,
                &rules,
                &mut world,
                5,
                35,
                (46, 28),
                crate::scenario::opponent_stack::StackMovement::Extended,
            )
            .unwrap_err();
            assert_eq!(rejected, "saved opponent stack observation: native contact required: civ 5 with 6 at (48,28) on map 0");

            let mut world = contact.game.world.clone();
            // The native prefix sets these treaty words before valuation.
            world.diplomacy[5][6] = 0x4401;
            world.diplomacy[6][5] = 0x4401;
            let values5 = world.native_research_candidates(&rules, 5).unwrap();
            let values6 = world.native_research_candidates(&rules, 6).unwrap();
            for (civ, advance, native) in [
                (6, 1, 5),
                (6, 12, 4),
                (6, 36, 7),
                (5, 39, 6),
                (6, 46, 8),
                (6, 47, 4),
                (5, 56, 6),
                (5, 86, 5),
                (6, 88, 3),
            ] {
                let values = if civ == 5 { &values5 } else { &values6 };
                assert_eq!(values[advance].value, native);
            }
            if matches!(
                recipe,
                ScenarioKnowledgeRecipe::SavedOpponentContactV1
                    | ScenarioKnowledgeRecipe::SavedOpponentCityCombatV1
                    | ScenarioKnowledgeRecipe::SavedOpponentHutV1
                    | ScenarioKnowledgeRecipe::SavedOpponentArmedStackV1
                    | ScenarioKnowledgeRecipe::SavedOpponentArmedStackV2
                    | ScenarioKnowledgeRecipe::SavedOpponentZoneV1
                    | ScenarioKnowledgeRecipe::SavedOpponentFoundingV1
                    | ScenarioKnowledgeRecipe::SavedOpponentWestwardV1
                    | ScenarioKnowledgeRecipe::SavedNavalArrivalV1
                    | ScenarioKnowledgeRecipe::SavedRemoteNavalStackV1
                    | ScenarioKnowledgeRecipe::SavedNavalAllocationV1
            ) {
                let before = contact.view();
                let draws = contact.game.world.combat_rng.draws();
                assert_eq!(contact.advance(1), 1);
                assert_eq!(contact.game.world.diplomacy[5][6], 0x4405);
                assert_eq!(contact.game.world.diplomacy[6][5], 0x4405);
                assert_eq!(contact.game.world.combat_rng.draws(), draws + 9);
                assert_eq!(contact.game.world.research[5].acquired, 8);
                assert_eq!(contact.game.world.research[6].acquired, 7);
                assert_ne!(contact.game.world.techs[5][39 / 8] & (1 << (39 % 8)), 0);
                assert_ne!(contact.game.world.techs[6][46 / 8] & (1 << (46 % 8)), 0);
                assert_eq!(contact.game.world.combat_rng.state(), 0x87dd2a59);
                assert_eq!(contact.view().observation.seen, before.observation.seen);
                assert!(contact.saved_move_state.is_some());
                let reached = contact.checkpoint(deps);
                let replay = restore(&reached);
                assert_eq!(replay.checkpoint(deps), reached);
                assert_eq!(replay.view(), contact.view());
                if matches!(
                    recipe,
                    ScenarioKnowledgeRecipe::SavedOpponentCityCombatV1
                        | ScenarioKnowledgeRecipe::SavedOpponentHutV1
                        | ScenarioKnowledgeRecipe::SavedOpponentArmedStackV1
                        | ScenarioKnowledgeRecipe::SavedOpponentArmedStackV2
                        | ScenarioKnowledgeRecipe::SavedOpponentZoneV1
                        | ScenarioKnowledgeRecipe::SavedOpponentFoundingV1
                        | ScenarioKnowledgeRecipe::SavedOpponentWestwardV1
                        | ScenarioKnowledgeRecipe::SavedNavalArrivalV1
                        | ScenarioKnowledgeRecipe::SavedRemoteNavalStackV1
                        | ScenarioKnowledgeRecipe::SavedNavalAllocationV1
                ) {
                    let prior = contact.saved_move_state.as_ref().unwrap().clone();
                    assert_eq!(contact.advance(1), 1);
                    assert_eq!(contact.game.world.combat_rng.state(), 0x2413ad1c);
                    assert_eq!(contact.game.world.units[35].damage, 8);
                    assert_eq!(contact.game.world.units[36].id, 0);
                    assert!(contact.game.world.city_at(48, 28).is_none());
                    let after = contact.saved_move_state.as_ref().unwrap();
                    let offset = 3 * prior.map_header.surface();
                    let memory = offset..offset + prior.map_header.surface();
                    assert_eq!(
                        differences(
                            &prior.maps[0].visibility[memory.clone()],
                            &after.maps[0].visibility[memory]
                        ),
                        [(1144, 2, 0)]
                    );
                    assert_eq!(contact.view().observation.seen, before.observation.seen);
                    let fought = contact.checkpoint(deps);
                    assert_eq!(restore(&fought).checkpoint(deps), fought);
                    if matches!(
                        recipe,
                        ScenarioKnowledgeRecipe::SavedOpponentHutV1
                            | ScenarioKnowledgeRecipe::SavedOpponentArmedStackV1
                            | ScenarioKnowledgeRecipe::SavedOpponentArmedStackV2
                            | ScenarioKnowledgeRecipe::SavedOpponentZoneV1
                            | ScenarioKnowledgeRecipe::SavedOpponentFoundingV1
                            | ScenarioKnowledgeRecipe::SavedOpponentWestwardV1
                            | ScenarioKnowledgeRecipe::SavedNavalArrivalV1
                            | ScenarioKnowledgeRecipe::SavedRemoteNavalStackV1
                            | ScenarioKnowledgeRecipe::SavedNavalAllocationV1
                    ) {
                        assert_eq!(contact.advance(8), 8);
                        let prior = contact.view();
                        assert_eq!(contact.advance(1), 1);
                        assert_eq!(contact.game.world.combat_rng.state(), 0x44bc0476);
                        assert_ne!(contact.game.world.techs[6][0] & 2, 0);
                        let saved = contact.saved_move_state.as_ref().unwrap();
                        assert_eq!(saved.units[37].moves_spent(), 9);
                        assert_eq!(contact.view().observation.seen, prior.observation.seen);
                        let reward = contact.checkpoint(deps);
                        assert_eq!(restore(&reward).checkpoint(deps), reward);
                        if matches!(
                            recipe,
                            ScenarioKnowledgeRecipe::SavedOpponentArmedStackV1
                                | ScenarioKnowledgeRecipe::SavedOpponentArmedStackV2
                                | ScenarioKnowledgeRecipe::SavedOpponentZoneV1
                                | ScenarioKnowledgeRecipe::SavedOpponentFoundingV1
                                | ScenarioKnowledgeRecipe::SavedOpponentWestwardV1
                                | ScenarioKnowledgeRecipe::SavedNavalArrivalV1
                                | ScenarioKnowledgeRecipe::SavedRemoteNavalStackV1
                                | ScenarioKnowledgeRecipe::SavedNavalAllocationV1
                        ) {
                            assert_eq!(contact.advance(4), 4);
                            let prior = contact.view();
                            assert_eq!(contact.advance(1), 1);
                            let saved = contact.saved_move_state.as_ref().unwrap();
                            assert_eq!(
                                (saved.units[24].stack_prev(), saved.units[24].stack_next()),
                                (-1, 4)
                            );
                            assert_eq!(saved.units[4].stack_prev(), 24);
                            assert_eq!(saved.units[24].moves_spent(), 3);
                            assert_eq!(contact.view().observation.seen, prior.observation.seen);
                            let joined = contact.checkpoint(deps);
                            assert_eq!(restore(&joined).checkpoint(deps), joined);
                            contact.advance(128);
                            let fresh = contact.view();
                            assert_eq!((fresh.observation.turn, fresh.phase), (45, Phase::Human));
                            assert!(contact.saved_move_state.is_some());
                            assert_eq!(fresh.observation.seen.iter().flatten().count(), 59);

                            let actor = unit(new_unit.index);
                            let result = submit(
                                &mut contact,
                                Command::Move {
                                    unit: actor,
                                    to: (53, 9),
                                },
                            );
                            assert_eq!(result.result, Some(OrderResult::Moved));
                            let saved = contact.saved_move_state.as_ref().unwrap();
                            assert_eq!(
                                (saved.units[actor.slot].x(), saved.units[actor.slot].y()),
                                (53, 9)
                            );
                            assert_eq!(saved.units[actor.slot].stack_next(), 26);
                            assert_eq!(saved.units[26].stack_prev(), actor.slot as i16);
                            assert_eq!(saved.units[actor.slot].moves_spent(), 6);
                            assert_eq!(contact.view().observation.seen, fresh.observation.seen);
                            let returned = contact.checkpoint(deps);
                            assert_eq!(restore(&returned).checkpoint(deps), returned);
                            if matches!(
                                recipe,
                                ScenarioKnowledgeRecipe::SavedOpponentArmedStackV2
                                    | ScenarioKnowledgeRecipe::SavedOpponentZoneV1
                                    | ScenarioKnowledgeRecipe::SavedOpponentFoundingV1
                                    | ScenarioKnowledgeRecipe::SavedOpponentWestwardV1
                                    | ScenarioKnowledgeRecipe::SavedNavalArrivalV1
                                    | ScenarioKnowledgeRecipe::SavedRemoteNavalStackV1
                                    | ScenarioKnowledgeRecipe::SavedNavalAllocationV1
                            ) {
                                submit(&mut contact, Command::EndTurn);
                                assert_eq!(contact.advance(22), 22);
                                let prior = contact.view();
                                assert_eq!(contact.advance(1), 1);
                                let saved = contact.saved_move_state.as_ref().unwrap();
                                assert_eq!(saved.units[24].moves_spent(), 6);
                                assert_eq!(saved.units[24].stack_next(), 4);
                                assert_eq!(saved.units[4].stack_prev(), 24);
                                assert_eq!(contact.view().observation.seen, prior.observation.seen);
                                let second = contact.checkpoint(deps);
                                assert_eq!(restore(&second).checkpoint(deps), second);
                                contact.advance(128);
                                let fresh = contact.view();
                                assert_eq!(
                                    (fresh.observation.turn, fresh.phase),
                                    (46, Phase::Human)
                                );
                                assert!(contact.saved_move_state.is_some());
                                assert_eq!(fresh.observation.seen.iter().flatten().count(), 59);

                                let result = submit(
                                    &mut contact,
                                    Command::Move {
                                        unit: actor,
                                        to: (52, 8),
                                    },
                                );
                                assert_eq!(result.result, Some(OrderResult::Moved));
                                let saved = contact.saved_move_state.as_ref().unwrap();
                                assert_eq!(
                                    (saved.units[actor.slot].x(), saved.units[actor.slot].y()),
                                    (52, 8)
                                );
                                assert_eq!(saved.units[actor.slot].moves_spent(), 6);
                                assert_eq!(contact.view().observation.seen, fresh.observation.seen);
                                let departed = contact.checkpoint(deps);
                                let replay = restore(&departed);
                                assert_eq!(replay.checkpoint(deps), departed);
                                assert_eq!(replay.view(), contact.view());
                                if matches!(
                                    recipe,
                                    ScenarioKnowledgeRecipe::SavedOpponentZoneV1
                                        | ScenarioKnowledgeRecipe::SavedOpponentFoundingV1
                                        | ScenarioKnowledgeRecipe::SavedOpponentWestwardV1
                                        | ScenarioKnowledgeRecipe::SavedNavalArrivalV1
                                        | ScenarioKnowledgeRecipe::SavedRemoteNavalStackV1
                                        | ScenarioKnowledgeRecipe::SavedNavalAllocationV1
                                ) {
                                    submit(&mut contact, Command::EndTurn);
                                    assert_eq!(contact.advance(2), 2);
                                    let prior = contact.view();
                                    let crc = contact.game.world.checksum();
                                    assert_eq!(contact.advance(1), 1);
                                    assert_eq!(
                                        contact.last_ai,
                                        Some(OrderResult::Rejected("zone of control"))
                                    );
                                    assert_eq!(contact.game.world.checksum(), crc);
                                    assert_eq!(
                                        contact.saved_move_state.as_ref().unwrap().units[10].raw
                                            [16],
                                        1
                                    );

                                    assert!(contact.saved_move_state.is_some());
                                    assert_eq!(
                                        contact.view().observation.seen,
                                        prior.observation.seen
                                    );
                                    assert_eq!(contact.advance(30), 30);
                                    assert_eq!(contact.advance(1), 1);
                                    assert_eq!(
                                        contact.last_ai,
                                        Some(OrderResult::Rejected("zone of control"))
                                    );
                                    assert_eq!(
                                        contact.saved_move_state.as_ref().unwrap().units[3].raw[16],
                                        1
                                    );
                                    assert_eq!(contact.advance(4), 4);
                                    let crc = contact.game.world.checksum();
                                    assert_eq!(contact.advance(1), 1);
                                    assert_eq!(
                                        contact.last_ai,
                                        Some(OrderResult::Rejected("zone of control"))
                                    );
                                    assert_eq!(contact.game.world.checksum(), crc);
                                    assert_eq!(
                                        contact.saved_move_state.as_ref().unwrap().units[3].raw[16],
                                        2
                                    );

                                    for _ in 40..296 {
                                        if contact.advance(1) == 0 {
                                            break;
                                        }
                                    }
                                    let fresh = contact.view();
                                    assert_eq!(
                                        (fresh.observation.turn, fresh.phase),
                                        (47, Phase::Human)
                                    );
                                    assert!(contact.saved_move_state.is_some());
                                    assert_eq!(fresh.observation.seen.iter().flatten().count(), 59);

                                    let result = submit(
                                        &mut contact,
                                        Command::Move {
                                            unit: actor,
                                            to: (53, 9),
                                        },
                                    );
                                    assert_eq!(result.result, Some(OrderResult::Moved));
                                    let saved = contact.saved_move_state.as_ref().unwrap();
                                    assert_eq!(saved.units[actor.slot].stack_next(), 26);
                                    assert_eq!(saved.units[actor.slot].moves_spent(), 6);
                                    assert_eq!(
                                        contact.view().observation.seen,
                                        fresh.observation.seen
                                    );
                                    let returned = contact.checkpoint(deps);
                                    let replay = restore(&returned);
                                    assert_eq!(replay.checkpoint(deps), returned);
                                    assert_eq!(replay.view(), contact.view());
                                    submit(&mut contact, Command::EndTurn);
                                    assert_eq!(contact.advance(2), 2);
                                    assert_eq!(contact.last_ai, Some(OrderResult::Founded(9)));
                                    if matches!(
                                        recipe,
                                        ScenarioKnowledgeRecipe::SavedOpponentFoundingV1
                                            | ScenarioKnowledgeRecipe::SavedOpponentWestwardV1
                                            | ScenarioKnowledgeRecipe::SavedNavalArrivalV1
                                            | ScenarioKnowledgeRecipe::SavedRemoteNavalStackV1
                                            | ScenarioKnowledgeRecipe::SavedNavalAllocationV1
                                    ) {
                                        let saved = contact.saved_move_state.as_ref().unwrap();
                                        assert_eq!(saved.cities[9].city_id(), 11);
                                        assert_eq!(saved.cities[9].size(), 1);
                                        assert_eq!(saved.units[2].id(), 0);
                                        assert_eq!(saved.units[2].x() as i16, -600);
                                        assert_eq!(saved.maps[0].visibility[1702], 2);
                                        assert_eq!(
                                            contact.view().observation.seen,
                                            fresh.observation.seen
                                        );
                                        let founded = contact.checkpoint(deps);
                                        let replay = restore(&founded);
                                        assert_eq!(replay.checkpoint(deps), founded);
                                        assert_eq!(replay.view(), contact.view());
                                        assert_eq!(contact.advance(16), 16);
                                        assert_eq!(contact.last_ai, Some(OrderResult::Founded(10)));
                                        let saved = contact.saved_move_state.as_ref().unwrap();
                                        assert_eq!(saved.cities[10].city_id(), 12);
                                        assert_eq!(saved.cities[10].size(), 1);
                                        assert_eq!(saved.units[37].id(), 0);
                                        assert_eq!(saved.units[37].x() as i16, -700);
                                        assert_eq!(saved.maps[0].visibility[693], 2);
                                        assert_eq!(
                                            contact.view().observation.seen,
                                            fresh.observation.seen
                                        );
                                        let second = contact.checkpoint(deps);
                                        let replay = restore(&second);
                                        assert_eq!(replay.checkpoint(deps), second);
                                        assert_eq!(replay.view(), contact.view());
                                        if matches!(
                                            recipe,
                                            ScenarioKnowledgeRecipe::SavedOpponentWestwardV1
                                                | ScenarioKnowledgeRecipe::SavedNavalArrivalV1
                                                | ScenarioKnowledgeRecipe::SavedRemoteNavalStackV1
                                                | ScenarioKnowledgeRecipe::SavedNavalAllocationV1
                                        ) {
                                            assert_eq!(contact.advance(5), 5);
                                            let saved = contact.saved_move_state.as_ref().unwrap();
                                            assert_eq!(saved.units[28].moves_spent(), 0);
                                            assert_eq!(contact.advance(1), 1);
                                            assert_eq!(contact.last_ai, Some(OrderResult::Moved));
                                            let saved = contact.saved_move_state.as_ref().unwrap();
                                            assert_eq!(saved.units[28].stack_next(), 7);
                                            assert_eq!(saved.units[7].stack_prev(), 28);
                                            assert_eq!(saved.units[28].moves_spent(), 6);
                                            assert_eq!(saved.units[28].raw[13], 5);
                                            assert_eq!(
                                                contact.view().observation.seen,
                                                fresh.observation.seen
                                            );
                                            let stacked = contact.checkpoint(deps);
                                            let replay = restore(&stacked);
                                            assert_eq!(replay.checkpoint(deps), stacked);
                                            assert_eq!(replay.view(), contact.view());
                                            assert_eq!(contact.advance(3), 3);
                                            if matches!(recipe, ScenarioKnowledgeRecipe::SavedNavalArrivalV1 | ScenarioKnowledgeRecipe::SavedRemoteNavalStackV1 | ScenarioKnowledgeRecipe::SavedNavalAllocationV1) {
                                                let saved =
                                                    contact.saved_move_state.as_ref().unwrap();
                                                assert_eq!(saved.units[37].stack_next(), 36);
                                                assert_eq!(saved.units[36].stack_next(), 2);
                                                assert_eq!(saved.units[37].owner(), 0);
                                                assert_eq!(
                                                    (saved.units[37].x(), saved.units[37].y()),
                                                    (0, 40)
                                                );
                                                assert_eq!(contact.game.world.turn, 48);
                                                assert_eq!(
                                                    contact.view().observation.seen,
                                                    fresh.observation.seen
                                                );
                                                let arrived = contact.checkpoint(deps);
                                                let replay = restore(&arrived);
                                                assert_eq!(replay.checkpoint(deps), arrived);
                                                assert_eq!(replay.view(), contact.view());
                                                assert_eq!(contact.advance(4), 4);
                                                if matches!(recipe, ScenarioKnowledgeRecipe::SavedRemoteNavalStackV1 | ScenarioKnowledgeRecipe::SavedNavalAllocationV1) {
                                                    let saved = contact.saved_move_state.as_ref().unwrap();
                                                    assert_eq!((saved.units[0].x(), saved.units[0].y()), (64, 36));
                                                    assert_eq!(saved.units[0].moves_spent(), 6);
                                                    assert_eq!(saved.units[0].raw[13], 1);
                                                    assert_eq!(saved.units[37].stack_next(), 36);
                                                    assert_eq!(contact.view().observation.seen, fresh.observation.seen);
                                                    let moved = contact.checkpoint(deps);
                                                    let replay = restore(&moved);
                                                    assert_eq!(replay.checkpoint(deps), moved);
                                                    assert_eq!(replay.view(), contact.view());
                                                    contact.advance(128);
                                                    let poll = contact.view();
                                                    assert_eq!(poll.phase, Phase::Human);
                                                    assert_eq!(poll.observation.turn, 48);
                                                    assert_eq!(poll.observation.seen, fresh.observation.seen);
                                                    assert_eq!(submit(&mut contact, Command::Move {
                                                        unit: actor, to: (52, 8),
                                                    }).result, Some(OrderResult::Moved));
                                                    assert!(contact.saved_move_state.is_some());
                                                    assert_eq!(contact.view().observation.seen, poll.observation.seen);
                                                    let departed = contact.checkpoint(deps);
                                                    let replay = restore(&departed);
                                                    assert_eq!(replay.checkpoint(deps), departed);
                                                    assert_eq!(replay.view(), contact.view());
                                                } else {
                                                    assert!(contact.saved_move_state.is_none());
                                                }
                                            } else {
                                                assert!(contact.saved_move_state.is_none());
                                            }
                                        } else {
                                            assert_eq!(contact.advance(6), 6);
                                            assert!(contact.saved_move_state.is_none());
                                        }
                                    } else {
                                        assert!(contact.saved_move_state.is_none());
                                    }
                                } else {
                                    submit(&mut contact, Command::EndTurn);
                                    assert_eq!(contact.advance(3), 3);
                                    assert!(contact.saved_move_state.is_none());
                                    let old = contact.checkpoint(deps);
                                    assert_eq!(restore(&old).checkpoint(deps), old);
                                }
                            } else {
                                submit(&mut contact, Command::EndTurn);
                                assert_eq!(contact.advance(23), 23);
                                assert!(contact.saved_move_state.is_none());
                                let older = contact.checkpoint(deps);
                                assert_eq!(restore(&older).checkpoint(deps), older);
                            }
                        } else {
                            assert_eq!(contact.advance(5), 5);
                            assert!(contact.saved_move_state.is_none());
                            let old = contact.checkpoint(deps);
                            assert_eq!(restore(&old).checkpoint(deps), old);
                        }
                    } else {
                        assert_eq!(contact.advance(9), 9);
                        assert!(contact.saved_move_state.is_none());
                        let old = contact.checkpoint(deps);
                        assert_eq!(restore(&old).checkpoint(deps), old);
                    }
                } else {
                    assert_eq!(contact.advance(1), 1);
                    assert!(contact.saved_move_state.is_none());
                    let old = contact.checkpoint(deps);
                    assert_eq!(restore(&old).checkpoint(deps), old);
                }
            }
        }
    }

    /// The end-turn-only alternate route from the canonical entry replays
    /// civ 2's witnessed turn-49 exchange approach and its zone refusal with
    /// the human's saved view intact, then — after the witnessed movement
    /// refresh at the turn-49 -> 50 boundary — civ 2's whole turn-50 pass:
    /// the founding, the westward hop, and the southward refusal staged from
    /// the refreshed state with its owner-byte claim. Mirrors the recorded
    /// native witnesses: the eastward approach writes spent 3 and direction
    /// 1, the refusal changes only the refusal byte, and the refreshed
    /// turn-50 refusal writes the done flag and the counter while claiming
    /// the origin tile for civ 2.
    #[test]
    fn installed_turn49_exchange_approach_writes_the_witnessed_direction() {
        let Some((snapshot, rules, events)) = installed_original() else {
            return;
        };
        let deps = "totengine-scenario-dependencies 1\nturn49-approach-test\n";
        let mut session = HumanSession::scenario_initialized_recipe(
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
            ScenarioKnowledgeRecipe::SavedNavalAllocationV1,
        )
        .unwrap();
        let submit = |session: &mut HumanSession, command| {
            let view = session.view();
            session
                .submit(Request {
                    civ: view.observation.civ,
                    revision: view.revision,
                    id: view.revision + 1,
                    command,
                })
                .unwrap()
        };
        let tiles =
            |session: &HumanSession| session.view().observation.seen.iter().flatten().count();
        assert_eq!(tiles(&session), 59);
        // EndTurn-only to the pending turn-48 research choice.
        let mut choice = None;
        for _ in 0..8 {
            let view = session.view();
            if view.phase != Phase::Human || view.decision.is_some() {
                choice = view.decision;
                break;
            }
            let before = tiles(&session);
            submit(&mut session, Command::EndTurn);
            for _ in 0..16 {
                if session.view().phase == Phase::Human {
                    break;
                }
                session.advance(256);
            }
            let after = session.view();
            assert_eq!(after.phase, Phase::Human);
            assert_eq!(tiles(&session), before);
            choice = after.decision;
        }
        let Some(Decision::Research {
            id, ref available, ..
        }) = choice
        else {
            panic!("the end-turn-only route must reach the turn-48 research choice");
        };
        submit(
            &mut session,
            Command::Research {
                decision: id,
                advance: available[0],
            },
        );
        assert_eq!(tiles(&session), 59);
        assert!(session.saved_move_state.is_some());
        // The turn-48 close replays the opponents' turn-49 passes; civ 2's
        // witnessed exchange approach and its following zone refusal run
        // inside it, so the per-step watch rides that close.
        submit(&mut session, Command::EndTurn);
        let mut approach = false;
        let mut refusal = false;
        for _ in 0..2000 {
            if session.view().phase == Phase::Human {
                break;
            }
            if session.advance(1) == 0 {
                break;
            }
            let Some(saved) = session.saved_move_state.as_ref() else {
                break;
            };
            let mover = &saved.units[3];
            if !approach && (mover.x(), mover.y()) == (41, 35) && mover.moves_spent() == 3 {
                assert_eq!(mover.raw[13], 1);
                assert_eq!(mover.raw[16], 0);
                approach = true;
            }
            if approach && mover.raw[16] == 1 {
                assert_eq!((mover.x(), mover.y()), (41, 35));
                assert_eq!(mover.moves_spent(), 3);
                assert_eq!(mover.raw[13], 1);
                refusal = true;
            }
        }
        assert!(approach && refusal);
        let turn49 = session.view();
        assert_eq!(turn49.phase, Phase::Human);
        assert_eq!(turn49.observation.turn, 49);
        assert_eq!(tiles(&session), 59);
        assert!(session.saved_move_state.is_some());
        let refused = session.checkpoint(deps);
        let replay = HumanSession::restore_with(
            rules.clone(),
            deps,
            &refused,
            Some((&snapshot, Some(events.clone()))),
        )
        .unwrap();
        assert_eq!(replay.checkpoint(deps), refused);
        assert_eq!(replay.view(), session.view());
        // The human's turn-49 end replays civs 5-7, the annual (with the
        // witnessed movement refresh), and civ 2's whole turn-50 pass: the
        // witnessed turn-start flag clears, the staged founding (city 11,
        // knowledge retained), the recorded westward hop to (42, 36), and
        // the re-admitted zone case staging unit 3's southward refusal
        // with its owner-byte claim (import-turn-start-refresh.json).
        submit(&mut session, Command::EndTurn);
        let mut founded = false;
        let mut refused50 = false;
        for _ in 0..2000 {
            if session.view().phase == Phase::Human {
                break;
            }
            session.advance(1);
            if session.last_ai == Some(OrderResult::Founded(11)) {
                assert!(session.saved_move_state.is_some());
                assert_eq!(tiles(&session), 59);
                founded = true;
                let founded_cp = session.checkpoint(deps);
                let replay = HumanSession::restore_with(
                    rules.clone(),
                    deps,
                    &founded_cp,
                    Some((&snapshot, Some(events.clone()))),
                )
                .unwrap();
                assert_eq!(replay.checkpoint(deps), founded_cp);
                assert_eq!(replay.view(), session.view());
            }
            if founded && !refused50 {
                let Some(saved) = session.saved_move_state.as_ref() else {
                    continue;
                };
                let mover = &saved.units[3];
                if (mover.x(), mover.y()) == (42, 36)
                    && mover.raw[16] == 1
                    && mover.raw[6] & 0x40 != 0
                {
                    // The staged turn-50 refusal: the witnessed writes are the
                    // done flag and the counter bump from the refreshed state
                    // (spent stays 0); the origin tile owner claim changes no
                    // knowledge row, so the saved view keeps all 59 tiles.
                    assert_eq!(mover.moves_spent(), 0);
                    assert_eq!(tiles(&session), 59);
                    // The origin tile (42,36) is record 1461; its +5 byte is
                    // civ 2's claim (owner nibble 2, fertility 6) — a no-op
                    // in this route (the staged turn-49 hop already claimed
                    // it), pinned so it cannot silently move.
                    assert_eq!(session.game.world.map.owner[1461], 2);
                    assert_eq!(saved.maps[0].tile_fields[8771], 38);
                    refused50 = true;
                    let refused_cp = session.checkpoint(deps);
                    let replay = HumanSession::restore_with(
                        rules.clone(),
                        deps,
                        &refused_cp,
                        Some((&snapshot, Some(events.clone()))),
                    )
                    .unwrap();
                    assert_eq!(replay.checkpoint(deps), refused_cp);
                    assert_eq!(replay.view(), session.view());
                }
            }
        }
        assert!(founded && refused50);
        let boundary = session.view();
        assert_eq!(boundary.phase, Phase::Human);
        assert_eq!(boundary.observation.turn, 50);
        // The staged refusal keeps the saved view: all 59 tiles and the
        // saved records survive civ 2's whole turn-50 pass.
        assert_eq!(tiles(&session), 59);
        assert!(session.saved_move_state.is_some());

        // Turn-50 continuation: end the turn and step until the saved-view
        // projection falls back. The witnessed native order from the refreshed
        // state moves slot 19 from (44,40) to (44,42) and spends its 3 thirds
        // (import-turn50-civ5.json); pin the same engine outcome and the two
        // recorded orders here.
        session
            .submit(Request {
                civ: 4,
                revision: session.view().revision,
                id: 300,
                command: Command::EndTurn,
            })
            .unwrap();
        let mut new_log = String::new();
        let mut steps = 0;
        while session.saved_move_state.is_some() {
            let before = session.game.log().len();
            assert_eq!(session.advance(1), 1);
            new_log.push_str(&session.game.log()[before..]);
            steps += 1;
            assert!(steps < 500, "the fallback did not arrive");
        }
        assert_eq!(steps, 3);
        assert_eq!(
            new_log,
            "T 50 C 5 move 10 44 38 = moved\nT 50 C 5 move 19 44 42 = moved\n"
        );
        let unit = &session.game.world.units[19];
        assert_eq!(
            (unit.type_id, unit.owner, unit.x, unit.y, unit.moves_left),
            (2, 5, 44, 42, 0),
            "civ 5's slot 19 moves into its own city and spends its 3 thirds"
        );
    }

    #[test]
    fn installed_opponent_combat_replays_without_revealing_a_tile() {
        let Some((snapshot, rules, events)) = installed_original() else {
            return;
        };
        let deps = "totengine-scenario-dependencies 1\nsaved-opponent-combat-test\n";
        let mut game = HumanSession::scenario_initialized_saved_opponent_combat(
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
        assert_eq!(game.initializer().unwrap().schema(), 7);
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
        let initial = game.view();
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
                .contains(" C 5 move 20 22 46 = attack won")
            {
                reached = true;
                break;
            }
            prior = game.checkpoint(deps);
        }
        assert!(reached, "did not reach measured opponent battle");
        let after = game.view();
        let saved = game
            .saved_move_state
            .as_ref()
            .expect("battle keeps the saved projection");
        assert_eq!(initial.observation.seen, after.observation.seen);
        assert_eq!(initial.observation.units, after.observation.units);
        assert_eq!(after.observation.seen.iter().flatten().count(), 59);
        assert_eq!(saved.units[22].id(), 0);
        assert_eq!(
            (
                saved.units[20].x(),
                saved.units[20].y(),
                saved.units[20].damage()
            ),
            (23, 45, 0)
        );
        let index = game.game.world.map.stored_index(22, 46).unwrap();
        assert_eq!(saved.maps[0].tile_fields[6 * index + 1] & 1, 0);
        let restore = |text: &str| {
            HumanSession::restore_with(
                rules.clone(),
                deps,
                text,
                Some((&snapshot, Some(events.clone()))),
            )
            .unwrap()
        };
        // Vary only the combat CRT in a direct control. The observed
        // native seed-1 battle wounds the survivor without changing fog.
        let mut wounded = restore(&prior);
        wounded.game.world.combat_rng = crate::combat::CrtRand::new(1);
        assert_eq!(
            wounded.try_saved_opponent_move(
                5,
                Order::Move {
                    unit: 20,
                    to: (22, 46)
                }
            ),
            Some(OrderResult::Attacked { won: true })
        );
        assert_eq!(wounded.game.world.combat_rng.state(), 0x95475fb6);
        assert_eq!(wounded.game.world.units[20].damage, 5);
        assert_eq!(
            wounded.saved_move_state.as_ref().unwrap().units[20].damage(),
            5
        );
        assert_eq!(wounded.known_tiles, after.observation.seen);

        let mut eventful = restore(&prior);
        let unit = &eventful.game.world.units[22];
        let name = rules.unit_names()[usize::from(unit.type_id)];
        let script = format!("@IF\nUNITKILLED\nunit={name}\nattacker=ANYBODY\ndefender=ANYBODY\n@THEN\nTEXT\nObserved battle\nENDTEXT\n@ENDIF");
        eventful.game.events = Some(crate::events::EventEngine::new(
            tot_kit::parse_events(&script).unwrap(),
        ));
        let order = Order::Move {
            unit: 20,
            to: (22, 46),
        };
        let before = eventful.game.world.checksum();
        assert_eq!(eventful.try_saved_opponent_move(5, order), None);
        assert_eq!(eventful.game.world.checksum(), before);
        assert_eq!(eventful.game.world.combat_rng.state(), 7);
        assert!(eventful.game.events.as_ref().unwrap().log.is_empty());
        assert_eq!(
            eventful.game.submit(5, order),
            OrderResult::Attacked { won: true }
        );
        assert_eq!(eventful.game.events.as_ref().unwrap().log.len(), 1);

        let checkpoint = game.checkpoint(deps);
        let mut replay = restore(&prior);
        assert_eq!(replay.game.world.combat_rng.state(), 7);
        assert_eq!(replay.advance(1), 1);
        assert_eq!(game.game.world.combat_rng.state(), 0x0a14e6e2);
        assert_eq!(replay.checkpoint(deps), checkpoint);
        assert_eq!(restore(&checkpoint).view(), after);

        let mut reached_stack = false;
        for _ in 0..32 {
            assert_eq!(game.advance(1), 1);
            assert_eq!(replay.advance(1), 1);
            if game
                .game
                .log()
                .lines()
                .last()
                .unwrap()
                .contains(" C 7 move 4 9 45 = moved")
            {
                reached_stack = true;
                break;
            }
        }
        assert!(reached_stack, "did not reach measured field stack");
        let saved = game.saved_move_state.as_ref().unwrap();
        assert_eq!(game.game.world.units[4].moves_left, 0);
        assert_eq!(saved.units[4].stack_prev(), 7);
        assert_eq!(saved.units[7].stack_next(), 4);
        assert_eq!(game.view().observation.seen, initial.observation.seen);
        assert_eq!(game.checkpoint(deps), replay.checkpoint(deps));

        assert_eq!(game.advance(1), 1);
        assert_eq!(replay.advance(1), 1);
        assert!(game
            .game
            .log()
            .lines()
            .last()
            .unwrap()
            .contains(" C 7 move 7 8 46 = moved"));
        let saved = game
            .saved_move_state
            .as_ref()
            .expect("city arrival keeps the saved view");
        assert_eq!(saved.units[7].stack_next(), 24);
        assert_eq!(saved.units[24].stack_prev(), 7);
        assert_eq!(saved.units[4].stack_prev(), -1);
        assert_eq!(game.game.world.units[7].moves_left, 2);
        assert_eq!(game.view().observation.seen, initial.observation.seen);
        assert_eq!(game.checkpoint(deps), replay.checkpoint(deps));
        assert_eq!(game.advance(1), 1);
        assert_eq!(replay.advance(1), 1);
        assert!(game
            .game
            .log()
            .lines()
            .last()
            .unwrap()
            .contains(" C 7 move 24 9 45 = moved"));
        let saved = game
            .saved_move_state
            .as_ref()
            .expect("city departure keeps the saved view");
        assert_eq!(saved.units[24].stack_next(), 4);
        assert_eq!(saved.units[4].stack_prev(), 24);
        assert_eq!(saved.units[7].stack_next(), -1);
        assert_eq!(game.game.world.units[24].moves_left, 2);
        assert_eq!(game.view().observation.seen, initial.observation.seen);
        assert_eq!(game.checkpoint(deps), replay.checkpoint(deps));
        let before_partial_move = game.checkpoint(deps);
        let order = Order::Move {
            unit: 24,
            to: (10, 44),
        };
        for (seed, crt_after, outcome, position, spent) in [
            (0, 0x00269ec3, OrderResult::MovementExhausted, (9, 45), 3),
            (1, 0x0029e2c0, OrderResult::MovementExhausted, (9, 45), 3),
            (3, 0x00306aba, OrderResult::Moved, (10, 44), 7),
            (7, 0x003d7aae, OrderResult::Moved, (10, 44), 7),
        ] {
            let mut control = restore(&before_partial_move);
            control.game.world.combat_rng = crate::combat::CrtRand::new(seed);
            assert_eq!(control.try_saved_opponent_move(7, order), Some(outcome));
            assert_eq!(control.game.world.combat_rng.state(), crt_after);
            assert_eq!(control.game.world.combat_rng.draws(), 1);
            let unit = &control.game.world.units[24];
            assert_eq!((unit.x, unit.y), position);
            assert_eq!(unit.moves_left, 0);
            let saved = control.saved_move_state.as_ref().unwrap();
            assert_eq!(saved.units[24].raw[10], spent);
            assert_eq!(
                saved.units[24].stack_next(),
                if position == (9, 45) { 4 } else { -1 }
            );
            assert_eq!(control.view().observation.seen, initial.observation.seen);
        }

        let draws = game.game.world.combat_rng.draws();
        assert_eq!(game.advance(1), 1);
        assert!(game
            .game
            .log()
            .lines()
            .last()
            .unwrap()
            .contains(" C 7 move 24 10 44 = movement exhausted"));
        assert_eq!(game.game.world.combat_rng.state(), 0x23e1f21d);
        assert_eq!(game.game.world.combat_rng.draws(), draws + 1);
        assert_eq!(game.game.world.units[24].moves_left, 0);
        let saved = game.saved_move_state.as_ref().unwrap();
        assert_eq!((saved.units[24].x(), saved.units[24].y()), (9, 45));
        assert_eq!(saved.units[24].stack_next(), 4);
        assert_eq!(saved.units[24].raw[10], 3);
        assert_eq!(game.view().observation.seen, initial.observation.seen);
        let after_partial_move = game.checkpoint(deps);
        assert_eq!(replay.advance(1), 1);
        assert_eq!(replay.checkpoint(deps), after_partial_move);
        assert_eq!(restore(&after_partial_move).view(), game.view());
        assert_eq!(game.advance(1), 1);
        assert!(game
            .game
            .log()
            .lines()
            .last()
            .unwrap()
            .contains(" C 7 move 28 13 43 = moved"));
        let before_mountain = game.checkpoint(deps);
        let order = Order::Move {
            unit: 28,
            to: (15, 43),
        };
        for (seed, crt, outcome, position, spent) in [
            (0, 0x00269ec3, OrderResult::MovementExhausted, (13, 43), 3),
            (5, 0x36f2b4, OrderResult::Moved, (15, 43), 10),
        ] {
            let mut control = restore(&before_mountain);
            control.game.world.combat_rng = crate::combat::CrtRand::new(seed);
            assert_eq!(control.try_saved_opponent_move(7, order), Some(outcome));
            assert_eq!(control.game.world.combat_rng.state(), crt);
            assert_eq!(control.game.world.combat_rng.draws(), 1);
            let unit = &control.game.world.units[28];
            assert_eq!(
                (unit.x, unit.y, unit.moves_left),
                (position.0, position.1, 0)
            );
            assert_eq!(
                control.saved_move_state.as_ref().unwrap().units[28].raw[10],
                spent
            );
            assert_eq!(control.view().observation.seen, initial.observation.seen);
        }
        assert_eq!(replay.advance(1), 1);
        assert_eq!(replay.checkpoint(deps), before_mountain);
        assert_eq!(game.advance(1), 1);
        assert!(game
            .game
            .log()
            .lines()
            .last()
            .unwrap()
            .contains(" C 7 move 28 15 43 = movement exhausted"));
        assert_eq!(game.game.world.combat_rng.state(), 0x6f277c6c);
        let saved = game.saved_move_state.as_ref().unwrap();
        assert_eq!(
            (
                saved.units[28].x(),
                saved.units[28].y(),
                saved.units[28].raw[10]
            ),
            (13, 43, 3)
        );
        assert_eq!(game.view().observation.seen, initial.observation.seen);
        assert_eq!(replay.advance(1), 1);
        assert_eq!(replay.checkpoint(deps), game.checkpoint(deps));
        let mountain_crt = game.game.world.combat_rng.state();
        assert_eq!(game.advance(1), 1);
        assert!(game
            .game
            .log()
            .lines()
            .last()
            .unwrap()
            .contains(" C 7 move 29 9 43 = moved"));
        assert_eq!(game.game.world.combat_rng.state(), mountain_crt);
        let saved = game.saved_move_state.as_ref().unwrap();
        assert_eq!(
            (
                saved.units[29].x(),
                saved.units[29].y(),
                saved.units[29].raw[10]
            ),
            (9, 43, 3)
        );
        assert_eq!(game.game.world.units[29].moves_left, 0);
        assert_eq!(game.view().observation.seen, initial.observation.seen);
        assert_eq!(replay.advance(1), 1);
        assert_eq!(replay.checkpoint(deps), game.checkpoint(deps));
        for expected in [" C 7 move 33 10 42 = moved", " C 7 end = ended"] {
            assert_eq!(game.advance(1), 1);
            assert!(game.game.log().lines().last().unwrap().contains(expected));
            assert!(game.saved_move_state.is_some());
            assert_eq!(game.view().observation.seen, initial.observation.seen);
            assert_eq!(replay.advance(1), 1);
            assert_eq!(replay.checkpoint(deps), game.checkpoint(deps));
        }
        assert_eq!(game.cursor, 8);
        let before_round = game.checkpoint(deps);
        assert_eq!(game.game.world.turn, 43);
        let draws = game.game.world.combat_rng.draws();
        assert_eq!(game.advance(1), 1);
        assert_eq!(game.game.world.turn, 44);
        assert_eq!(game.game.world.combat_rng.state(), 0xd766ce46);
        assert_eq!(game.game.world.combat_rng.draws(), draws + 2);
        assert!(game.saved_move_state.is_some());
        assert_eq!(game.view().observation.seen, initial.observation.seen);
        assert_eq!(replay.advance(1), 1);
        let closed = game.checkpoint(deps);
        assert_eq!(replay.checkpoint(deps), closed);
        let mut restored = restore(&before_round);
        assert_eq!(restored.advance(1), 1);
        assert_eq!(restored.checkpoint(deps), closed);
        let mut deferred = restore(&before_round);
        deferred.saved_move_state.as_mut().unwrap().units[9].raw[7] |= 0x40;
        assert_eq!(deferred.advance(1), 1);
        assert_eq!(
            deferred.saved_move_state.as_ref().unwrap().units[9].attr() & 0x4000,
            0
        );
        assert_eq!(deferred.checkpoint(deps), closed);
        let mut eventful = restore(&before_round);
        eventful.game.events = Some(crate::events::EventEngine::new(
            tot_kit::parse_events(
                "@IF\nTURN\nturn=44\n@THEN\nTEXT\nRound starts.\nENDTEXT\n@ENDIF",
            )
            .unwrap(),
        ));
        assert_eq!(eventful.advance(1), 1);
        assert!(eventful.saved_move_state.is_none());
        assert_eq!(eventful.game.events.as_ref().unwrap().log.len(), 1);
        assert_eq!(eventful.game.world.turn, 44);

        for _ in 0..32 {
            if game.phase() == Phase::Human {
                break;
            }
            assert!(game.saved_move_state.is_some());
            assert_eq!(game.view().observation.seen, initial.observation.seen);
            assert_eq!(game.advance(1), 1);
            assert_eq!(replay.advance(1), 1);
            assert_eq!(replay.checkpoint(deps), game.checkpoint(deps));
        }
        assert_eq!(game.phase(), Phase::Human);
        assert_eq!(game.game.world.turn, 44);
        let saved = game.saved_move_state.as_ref().unwrap();
        assert_eq!(game.view().observation.seen, initial.observation.seen);
        assert!(game.decision.is_none());
        for slot in [6, 9, 18, 26] {
            assert_eq!(saved.units[slot].raw[10], 0);
            assert_eq!(game.game.world.units[slot].moves_left, 3);
        }
        let checkpoint = game.checkpoint(deps);
        assert_eq!(restore(&checkpoint).view(), game.view());
        let mut moving = restore(&checkpoint);
        let actor = *moving
            .view()
            .units
            .iter()
            .find(|unit| unit.slot == 18)
            .unwrap();
        let before = moving.saved_move_state.as_ref().unwrap().clone();
        let request = Request {
            civ: 4,
            id: 201,
            revision: moving.view().revision,
            command: Command::Move {
                unit: actor,
                to: (50, 8),
            },
        };
        assert_eq!(
            moving.submit(request).unwrap().result,
            Some(OrderResult::Moved)
        );
        let after = moving.saved_move_state.as_ref().unwrap();
        let raw = &after.units[18].raw;
        assert_eq!(
            (
                after.units[18].x(),
                after.units[18].y(),
                raw[6],
                raw[10],
                raw[13]
            ),
            (50, 8, 64, 6, 6)
        );
        assert_eq!(moving.game.world.units[18].moves_left, 0);
        let changes = |before: &[u8], after: &[u8]| -> Vec<(usize, u8, u8)> {
            before
                .iter()
                .zip(after)
                .enumerate()
                .filter_map(|(index, (&a, &b))| (a != b).then_some((index, a, b)))
                .collect()
        };
        assert_eq!(
            changes(&before.units[18].raw, &after.units[18].raw),
            [(0, 51, 50), (2, 9, 8), (6, 0, 64), (10, 0, 6), (13, 255, 6)]
        );
        assert_eq!(
            changes(&before.maps[0].tile_fields, &after.maps[0].tile_fields),
            [(2068, 0, 16), (2071, 0, 1), (2075, 240, 64), (2311, 1, 0)]
        );
        let area = before.map_header.surface();
        assert_eq!(
            changes(&before.maps[0].visibility, &after.maps[0].visibility),
            [(3 * area + 345, 0, 1), (3 * area + 385, 1, 0)]
        );
        assert_eq!(moving.view().observation.seen.iter().flatten().count(), 60);
        assert_eq!(restore(&moving.checkpoint(deps)).view(), moving.view());

        let unit = *game
            .view()
            .units
            .iter()
            .find(|unit| unit.slot == 18)
            .unwrap();
        assert_eq!(
            game.submit(Request {
                civ: 4,
                id: 200,
                revision: game.view().revision,
                command: Command::Skip { unit },
            })
            .unwrap()
            .result,
            Some(OrderResult::Skipped)
        );
        let stopped = game.saved_move_state.as_ref().unwrap();
        assert_eq!(
            (stopped.units[18].moves_spent(), stopped.units[18].raw[13]),
            (3, 255)
        );
        assert_eq!(game.view().observation.seen.iter().flatten().count(), 59);
        assert_eq!(restore(&game.checkpoint(deps)).view(), game.view());
    }
}
