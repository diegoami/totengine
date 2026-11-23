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
            pass(&mut self.game);
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

    #[test]
    fn installed_original_replays_across_staged_city_and_order_boundaries() {
        let Some(home) = std::env::var_os("CIVTOT_HOME") else {
            return;
        };
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
    }
}
