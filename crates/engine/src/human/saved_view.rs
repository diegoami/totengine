//! Stored initial human observation, followed by disclosed engine play.
use super::*;
use crate::saved_knowledge::SavedMapKnowledge;

pub(super) enum InitialKnowledge {
    Engine { resumed: bool },
    Saved(SavedMapKnowledge, ScenarioKnowledgeRecipe),
}

#[derive(Debug, Clone)]
pub(super) struct SavedInitialView {
    pub units: Vec<crate::player::UnitView>,
}

struct PreparedView {
    initial: SavedInitialView,
    tiles: Vec<Option<crate::player::TileView>>,
    cities: BTreeMap<usize, (u32, crate::player::CityView)>,
}

const DEFERRED: u16 = 0x4000;

struct SavedFocusCandidate {
    unit: Entity,
    distance: i32,
    deferred: bool,
}

impl HumanSession {
    /// Select a ready unit from a supplied map anchor in the checked
    /// surface context. Deferred units stay available for direct orders.
    pub fn saved_initial_focus(&self, anchor: (u16, u16)) -> Option<Entity> {
        self.saved_focus_candidates(anchor, true)?
            .into_iter()
            .filter(|candidate| !candidate.deferred)
            .min_by_key(|candidate| candidate.distance)
            .map(|candidate| candidate.unit)
    }

    fn saved_focus_candidates(
        &self,
        anchor: (u16, u16),
        allow_city_stack: bool,
    ) -> Option<Vec<SavedFocusCandidate>> {
        let saved = self.saved_move_state.as_ref()?;
        if !self.knowledge_recipe.checked_replay()
            || self.phase() != Phase::Human
            || self.decision.is_some()
            || (u32::from(anchor.0) + u32::from(anchor.1)) & 1 != 0
        {
            return None;
        }
        let world = &self.game.world;
        world.map.stored_index(anchor.0, anchor.1)?;
        let width = 2 * world.map.stored_w as i32;
        let flat = world.game_flags & 0x8000 != 0;
        let mut candidates = Vec::new();
        for (slot, unit) in world.units.iter().enumerate() {
            if unit.id == 0 || unit.owner != self.human || unit.moves_left == 0 {
                continue;
            }
            let raw = saved.units.get(slot)?;
            if raw.id() != unit.id
                || (raw.owner(), raw.x(), raw.y()) != (unit.owner, unit.x, unit.y)
                || raw.type_id() != unit.type_id
                || raw.map_id() != 0
                || ((raw.stack_prev(), raw.stack_next()) != (-1, -1)
                    && !(allow_city_stack && self.saved_city_mate(saved, slot).is_some()))
                || raw.orders() != 255
                || unit.orders != 255
                || unit.is_leader
                || self.game.rules.unit_domain(usize::from(unit.type_id)) != Some(0)
            {
                return None;
            }
            candidates.push(SavedFocusCandidate {
                unit: Entity { slot, id: unit.id },
                distance: crate::placement::diamond_distance(
                    width,
                    flat,
                    i32::from(anchor.0),
                    i32::from(anchor.1),
                    i32::from(unit.x),
                    i32::from(unit.y),
                ),
                deferred: raw.attr() & DEFERRED != 0,
            });
        }
        Some(candidates)
    }

    fn saved_city_mate(&self, saved: &tot_kit::SaveFile, slot: usize) -> Option<usize> {
        if !matches!(
            self.knowledge_recipe,
            ScenarioKnowledgeRecipe::SavedHumanProductionV1
                | ScenarioKnowledgeRecipe::SavedOpponentContactV1
                | ScenarioKnowledgeRecipe::SavedOpponentCityCombatV1
        ) {
            return None;
        }
        let unit = saved.units.get(slot)?;
        let (neighbor, head) = match (unit.stack_prev(), unit.stack_next()) {
            (-1, next) if next >= 0 => (next as usize, true),
            (prev, -1) if prev >= 0 => (prev as usize, false),
            _ => return None,
        };
        let world = &self.game.world;
        let live = world.units.get(slot)?;
        let other = saved.units.get(neighbor)?;
        let expected = if head {
            (slot as i16, -1)
        } else {
            (-1, slot as i16)
        };
        (unit.id() != 0
            && (live.id, live.owner, live.x, live.y) == (unit.id(), self.human, unit.x(), unit.y())
            && unit.owner() == self.human
            && other.id() != 0
            && (other.owner(), other.x(), other.y(), other.map_id())
                == (unit.owner(), unit.x(), unit.y(), 0)
            && (other.stack_prev(), other.stack_next()) == expected
            && world.units.get(neighbor).is_some_and(|u| {
                (u.id, u.owner, u.x, u.y, u.stack_pos)
                    == (
                        other.id(),
                        unit.owner(),
                        unit.x(),
                        unit.y(),
                        u16::from(head),
                    )
            })
            && world.units[slot].stack_pos == u16::from(!head)
            && world.cities.iter().any(|city| {
                city.city_id != 0
                    && city.map_id == 0
                    && (city.owner, city.x, city.y) == (u16::from(unit.owner()), unit.x(), unit.y())
            }))
        .then_some(neighbor)
    }

    fn saved_wait_candidates(&self, actor: Entity) -> Option<Vec<SavedFocusCandidate>> {
        let unit = self.game.world.units.get(actor.slot)?;
        if unit.id == 0 || unit.id != actor.id || unit.owner != self.human || unit.moves_left == 0 {
            return None;
        }
        let saved = self.saved_move_state.as_ref()?;
        let mate = self.saved_city_mate(saved, actor.slot);
        let candidates = self.saved_focus_candidates((unit.x, unit.y), mate.is_some())?;
        if mate.is_some_and(|slot| {
            !candidates
                .iter()
                .any(|candidate| candidate.unit.slot == slot && !candidate.deferred)
        }) {
            return None;
        }
        Some(candidates)
    }

    pub fn can_wait_in_saved_view(&self, actor: Entity) -> bool {
        self.saved_wait_candidates(actor).is_some()
    }

    /// Recover the checked focus after replaying the last selection-changing
    /// order. A Move with movement left keeps its active actor in the client.
    pub fn saved_order_focus(&self) -> Option<Entity> {
        let Some(Operation::Input(request)) = self.journal.last() else {
            return None;
        };
        let actor = match request.command {
            Command::Wait { unit } | Command::Move { unit, .. } | Command::Skip { unit } => unit,
            _ => return None,
        };
        let unit = self.game.world.units.get(actor.slot)?;
        if unit.id != actor.id
            || unit.owner != self.human
            || (!matches!(request.command, Command::Wait { .. }) && unit.moves_left != 0)
        {
            return None;
        }
        self.saved_initial_focus((unit.x, unit.y))
    }

    pub(super) fn stage_saved_wait(&mut self, actor: Entity) -> Result<(), &'static str> {
        let candidates = self
            .saved_wait_candidates(actor)
            .ok_or("saved wait requires a ready own unit in the checked surface context")?;
        let saved = self
            .saved_move_state
            .as_ref()
            .ok_or("saved wait requires captured unit state")?;
        if let Some(mate) = self.saved_city_mate(saved, actor.slot) {
            let mut staged = saved.clone();
            staged.units[actor.slot].raw[7] |= 0x40;
            let observed = crate::saved_knowledge::unit_observation_after(
                &staged,
                &self.game.rules,
                mate,
                true,
            )
            .map_err(|_| "saved city-stack wait cannot observe the next unit")?;
            if observed != staged {
                return Err("saved city-stack wait changed captured observation");
            }
            self.saved_move_state = Some(staged);
            return Ok(());
        }
        let saved = self.saved_move_state.as_mut().unwrap();
        saved.units[actor.slot].raw[7] |= 0x40;
        if !candidates
            .iter()
            .any(|candidate| candidate.unit != actor && !candidate.deferred)
        {
            // The native selector clears deferred flags and searches again
            // when it runs out of candidates. Retain the previous unit's
            // flag on the first retry if another ready unit can take focus.
            let previous = (candidates.len() > 1).then_some(actor.slot);
            clear_saved_deferrals(saved, previous);
        }
        Ok(())
    }

    pub(super) fn reset_saved_deferrals(&mut self) {
        if let Some(saved) = &mut self.saved_move_state {
            clear_saved_deferrals(saved, None);
        }
    }

    fn reset_deferred_focus_after_exhaustion(&mut self, slot: usize) {
        let unit = &self.game.world.units[slot];
        if unit.moves_left != 0 {
            return;
        }
        if let Some(candidates) = self.saved_focus_candidates((unit.x, unit.y), false) {
            if candidates.iter().all(|unit| unit.deferred) {
                let previous = (!candidates.is_empty()).then_some(slot);
                if let Some(saved) = &mut self.saved_move_state {
                    clear_saved_deferrals(saved, previous);
                }
            }
        }
    }

    pub(super) fn install_saved_view(
        &mut self,
        projection: SavedMapKnowledge,
    ) -> Result<(), String> {
        let view = Self::prepare_saved_view(&self.game.world, self.human, projection)?;
        self.set_saved_view(view);
        Ok(())
    }

    fn prepare_saved_view(
        world: &crate::WorldState,
        human: u8,
        projection: SavedMapKnowledge,
    ) -> Result<PreparedView, String> {
        if projection.civ != human
            || projection.width != world.map.stored_w
            || projection.height != world.map.stored_h
            || projection.tiles.len() != world.map.terrain.len()
        {
            return Err("saved human projection and world context differ".into());
        }
        let mut units = Vec::new();
        for saved in projection.units {
            let unit = world
                .units
                .get(saved.slot)
                .filter(|u| u.id != 0)
                .ok_or("saved human unit slot differs")?;
            if (unit.owner, unit.type_id, unit.x, unit.y)
                != (saved.owner, saved.type_id, saved.x, saved.y)
            {
                return Err("saved human unit context differs".into());
            }
            units.push(crate::player::UnitView {
                index: saved.slot,
                type_id: saved.type_id,
                owner: saved.owner,
                x: saved.x,
                y: saved.y,
                moves_left: if saved.owner == human {
                    unit.moves_left
                } else {
                    0
                },
                veteran: unit.veteran,
            });
        }
        let mut cities = BTreeMap::new();
        for saved in projection.cities {
            let city = world
                .cities
                .get(saved.slot)
                .filter(|c| c.city_id != 0)
                .ok_or("saved human city slot differs")?;
            if (city.owner, city.x, city.y) != (u16::from(saved.owner), saved.x, saved.y) {
                return Err("saved human city context differs".into());
            }
            cities.insert(
                saved.slot,
                (
                    city.city_id,
                    crate::player::CityView {
                        index: saved.slot,
                        name: saved.name,
                        owner: saved.owner,
                        x: saved.x,
                        y: saved.y,
                        size: u16::from(saved.size),
                        building: None,
                        shield_box: None,
                        output: None,
                    },
                ),
            );
        }
        let tiles = projection
            .tiles
            .iter()
            .enumerate()
            .map(|(i, t)| {
                t.map(|t| crate::player::TileView {
                    terrain: world.map.terrain[i],
                    river: world.map.river[i],
                    special: world.map.special[i],
                    road: t.improvements & 0x10 != 0,
                    rail: t.improvements & 0x20 != 0,
                })
            })
            .collect();
        Ok(PreparedView {
            initial: SavedInitialView { units },
            tiles,
            cities,
        })
    }

    fn set_saved_view(&mut self, view: PreparedView) {
        self.known_tiles = view.tiles;
        self.known_cities = view.cities;
        self.saved_initial = Some(view.initial);
        self.collect_event_texts();
    }

    pub(super) fn keep_city_pass_view(
        &mut self,
        before: &crate::WorldState,
        civ: u8,
        summary: &crate::turn::TurnSummary,
    ) -> Result<(), String> {
        let world = &self.game.world;
        let saved = self
            .saved_move_state
            .as_ref()
            .ok_or("saved city view missing")?;
        let human_pass = civ == self.human;
        if human_pass
            && (!summary.discoveries.is_empty()
                || !summary.events.is_empty()
                || world.free_research[usize::from(civ)] != 0)
        {
            return Err("saved city view requires a human pass without a pending choice".into());
        }
        if world.map != before.map
            || world.diplomacy != before.diplomacy
            || world.wonders[25] != before.wonders[25]
            || world.cities.len() != before.cities.len()
            || world.units.len() < before.units.len()
        {
            return Err("city pass changed saved observation inputs".into());
        }
        let same_city = |a: &crate::City, b: &crate::City| {
            a.name == b.name
                && (a.city_id, a.map_id, a.owner, a.x, a.y, a.known_to)
                    == (b.city_id, b.map_id, b.owner, b.x, b.y, b.known_to)
        };
        if !before
            .cities
            .iter()
            .zip(&world.cities)
            .all(|(a, b)| same_city(a, b))
        {
            return Err("city pass changed a city identity or discovery mask".into());
        }
        let checked_human_birth = human_pass
            && matches!(
                self.knowledge_recipe,
                ScenarioKnowledgeRecipe::SavedHumanProductionV1
                    | ScenarioKnowledgeRecipe::SavedOpponentContactV1
                    | ScenarioKnowledgeRecipe::SavedOpponentCityCombatV1
            );
        let mut births = Vec::new();
        for (slot, unit) in world.units.iter().enumerate() {
            if let Some(old) = before.units.get(slot).filter(|unit| unit.id != 0) {
                if (
                    old.id,
                    old.owner,
                    old.type_id,
                    old.x,
                    old.y,
                    old.damage,
                    old.veteran,
                ) != (
                    unit.id,
                    unit.owner,
                    unit.type_id,
                    unit.x,
                    unit.y,
                    unit.damage,
                    unit.veteran,
                ) {
                    return Err("city pass changed an existing unit".into());
                }
            } else if unit.id != 0 {
                let at = world
                    .map
                    .stored_index(unit.x, unit.y)
                    .ok_or("city birth has invalid coordinates")?;
                let from_city = summary.city_accounts.iter().any(|(slot, _)| {
                    let city = &world.cities[*slot];
                    city.owner == u16::from(civ) && (city.x, city.y) == (unit.x, unit.y)
                });
                let known = self
                    .known_tiles
                    .get(at)
                    .ok_or("city birth is off the saved map")?;
                let covered_birth = if checked_human_birth {
                    known.is_some()
                        && summary.built.as_slice()
                            == [(usize::from(unit.home), crate::production::Built::Unit(slot))]
                        && self.game.rules.unit_domain(usize::from(unit.type_id)) == Some(0)
                        && unit.damage == 0
                        && !unit.veteran
                } else {
                    known.is_none()
                };
                if unit.owner != civ || !from_city || !covered_birth {
                    return Err("city pass created a unit outside the checked birth context".into());
                }
                births.push((slot, at));
            }
        }

        let mut observed = saved.clone();
        if human_pass {
            for (slot, unit) in world.units.iter().enumerate() {
                if unit.id == 0
                    || unit.owner != civ
                    || (checked_human_birth && births.iter().any(|&(born, _)| born == slot))
                {
                    continue;
                }
                let record = observed
                    .units
                    .get_mut(slot)
                    .ok_or("saved human unit missing")?;
                if record.id() != unit.id || record.orders() != unit.orders {
                    return Err("saved human unit context changed at turn start".into());
                }
                record.raw[10] = 0;
            }
        }
        let mut updated_world = world.clone();
        if matches!(
            self.knowledge_recipe,
            ScenarioKnowledgeRecipe::SavedOpponentStackV1
                | ScenarioKnowledgeRecipe::SavedOpponentCombatV1
                | ScenarioKnowledgeRecipe::SavedHumanProductionV1
                | ScenarioKnowledgeRecipe::SavedOpponentContactV1
                | ScenarioKnowledgeRecipe::SavedOpponentCityCombatV1
        ) {
            crate::scenario::opponent_stack::record_city_births(
                &mut observed,
                &mut updated_world,
                &births,
            )?;
        }
        let human_production_city =
            if let (true, &[(slot, _)]) = (checked_human_birth, births.as_slice()) {
                let after = crate::saved_knowledge::unit_observation_after(
                    &observed,
                    &self.game.rules,
                    slot,
                    true,
                )
                .map_err(|error| error.to_string())?;
                if after != observed {
                    return Err("saved human city birth changed captured observation".into());
                }
                Some(usize::from(updated_world.units[slot].home))
            } else {
                None
            };
        for (slot, _) in &summary.city_accounts {
            updated_world.refresh_city_phase_memory(*slot)?;
            let city = &updated_world.cities[*slot];
            let record = observed
                .cities
                .get_mut(*slot)
                .ok_or("city memory slot missing")?;
            if record.city_id() != city.city_id {
                return Err("city memory reference differs from current city".into());
            }
            record.raw[11] = city.size as u8;
            record.raw[15..23].copy_from_slice(&city.known_sizes);
            if Some(*slot) == human_production_city {
                // The native human production tail at 0x4c748e clears overflow.
                updated_world.cities[*slot].shield_box = 0;
                record.raw[30..32].fill(0);
            }
        }
        let projection = SavedMapKnowledge::read(&observed, self.human)?;
        let prepared = Self::prepare_saved_view(&updated_world, self.human, projection)?;
        self.game.world = updated_world;
        self.saved_move_state = Some(observed);
        self.set_saved_view(prepared);
        Ok(())
    }

    pub(super) fn keeps_city_view_for_opponent_order(&self, civ: u8, order: Order) -> bool {
        if !matches!(
            self.knowledge_recipe,
            ScenarioKnowledgeRecipe::SavedCityViewStagedRoundV1
                | ScenarioKnowledgeRecipe::SavedOpponentStackV1
                | ScenarioKnowledgeRecipe::SavedOpponentCombatV1
                | ScenarioKnowledgeRecipe::SavedHumanProductionV1
                | ScenarioKnowledgeRecipe::SavedOpponentContactV1
                | ScenarioKnowledgeRecipe::SavedOpponentCityCombatV1
        ) || self.saved_move_state.is_none()
        {
            return false;
        }
        match order {
            Order::SetProduction { .. } | Order::EndTurn | Order::SkipUnit { .. } => true,
            // Choosing a free advance can grant it immediately, affecting
            // units, wonders and the saved context.
            Order::SetResearch { .. } => self.game.world.free_research[usize::from(civ)] == 0,
            _ => false,
        }
    }

    pub(super) fn try_saved_opponent_move(&mut self, civ: u8, order: Order) -> Option<OrderResult> {
        if !matches!(
            self.knowledge_recipe,
            ScenarioKnowledgeRecipe::SavedOpponentStackV1
                | ScenarioKnowledgeRecipe::SavedOpponentCombatV1
                | ScenarioKnowledgeRecipe::SavedHumanProductionV1
                | ScenarioKnowledgeRecipe::SavedOpponentContactV1
                | ScenarioKnowledgeRecipe::SavedOpponentCityCombatV1
        ) {
            return None;
        }
        let (Some(saved), Order::Move { unit, to }) = (&self.saved_move_state, order) else {
            return None;
        };
        let before = SavedMapKnowledge::read(saved, self.human).ok()?;
        let moving = self.game.world.units.get(unit)?;
        for (x, y) in [(moving.x, moving.y), to] {
            let tile = self.game.world.map.stored_index(x, y)?;
            if before.tiles.get(tile)?.is_some() {
                return None;
            }
        }
        let mut world = self.game.world.clone();
        let is_combat = self.knowledge_recipe.checked_replay()
            && world
                .units
                .iter()
                .any(|u| u.id != 0 && u.owner != civ && (u.x, u.y) == to);
        let mut events = None;
        let city_recipe =
            self.knowledge_recipe == ScenarioKnowledgeRecipe::SavedOpponentCityCombatV1;
        let city_before = world.city_at(to.0, to.1);
        let (observed, result) = if is_combat {
            events = self.game.events.clone();
            let observed = crate::scenario::opponent_combat::stage(
                saved,
                &self.game.rules,
                &mut world,
                events.as_mut(),
                (civ, unit),
                to,
                city_recipe,
            )
            .ok()?;
            (observed, OrderResult::Attacked { won: true })
        } else {
            crate::scenario::opponent_stack::stage(
                saved,
                &self.game.rules,
                &mut world,
                civ,
                unit,
                to,
                if city_recipe
                    || self.knowledge_recipe == ScenarioKnowledgeRecipe::SavedOpponentContactV1
                {
                    crate::scenario::opponent_stack::StackMovement::StockContact
                } else if self.knowledge_recipe.checked_replay() {
                    crate::scenario::opponent_stack::StackMovement::Extended
                } else {
                    crate::scenario::opponent_stack::StackMovement::City
                },
            )
            .ok()?
        };
        let after = SavedMapKnowledge::read(&observed, self.human).ok()?;
        let area = saved.map_header.surface();
        let offset = (usize::from(self.human) - 1) * area;
        let memory = offset..offset + area;
        let prior_memory = &saved.maps[0].visibility[memory.clone()];
        let new_memory = &observed.maps[0].visibility[memory];
        let memory_matches = if city_recipe
            && is_combat
            && city_before.is_some()
            && world.city_at(to.0, to.1).is_none()
        {
            let tile = world.map.stored_index(to.0, to.1)?;
            prior_memory
                .iter()
                .zip(new_memory)
                .enumerate()
                .all(|(i, (&prior, &new))| new == if i == tile { prior & !3 } else { prior })
        } else {
            prior_memory == new_memory
        };
        if before != after || !memory_matches {
            return None;
        }
        let view = Self::prepare_saved_view(&world, self.human, after).ok()?;
        if is_combat {
            self.game.events = events;
        }
        let result = self.game.commit_saved_view_order(world, civ, order, result);
        self.set_saved_view(view);
        self.saved_move_state = Some(observed);
        Some(result)
    }

    pub(super) fn finish_saved_movement(&mut self) {
        if self.saved_move_state.take().is_some() {
            self.saved_initial = None;
        }
    }

    pub(super) fn try_saved_view_order(
        &mut self,
        order: Order,
    ) -> Result<Option<OrderResult>, &'static str> {
        let Some(saved) = &self.saved_move_state else {
            return Ok(None);
        };
        let refused = "the saved-view movement phase cannot resolve that order";
        let checked_replay = self.knowledge_recipe.checked_replay();
        let mut world;
        let (observed, result) = match order {
            Order::Move { unit, to } => {
                let index = self
                    .game
                    .world
                    .map
                    .stored_index(to.0, to.1)
                    .ok_or(refused)?;
                if self.known_tiles.get(index).is_none_or(Option::is_none) {
                    return Err(refused);
                }
                world = self.game.world.clone();
                let step = crate::scenario::snapshot::stage_saved_land_move(
                    saved,
                    &self.game.rules,
                    &mut world,
                    unit,
                    to,
                    checked_replay,
                    self.saved_city_mate(saved, unit),
                );
                (step, OrderResult::Moved)
            }
            Order::SkipUnit { unit } => {
                world = self.game.world.clone();
                let step = crate::scenario::snapshot::stage_saved_unit_skip(
                    saved,
                    &self.game.rules,
                    &mut world,
                    unit,
                    checked_replay,
                    self.saved_city_mate(saved, unit),
                );
                (step, OrderResult::Skipped)
            }
            Order::SetRates { .. }
            | Order::QuotedProduction { .. }
            | Order::ToggleWorker { .. }
            | Order::BuyProduction { .. }
                if checked_replay && self.decision.is_none() =>
            {
                let before = &self.game.world;
                world = before.clone();
                let result = world.apply_order(&self.game.rules, None, self.human, order);
                if !matches!(
                    (order, &result),
                    (Order::SetRates { .. }, OrderResult::RatesSet)
                        | (Order::QuotedProduction { .. }, OrderResult::ProductionSet)
                        | (Order::ToggleWorker { .. }, OrderResult::CitizensSet)
                        | (Order::BuyProduction { .. }, OrderResult::ProductionBought)
                ) || world.map != before.map
                    || world.units != before.units
                    || world.cities.len() != before.cities.len()
                    || world.cities.iter().zip(&before.cities).any(|(city, old)| {
                        (city.size, city.known_to, city.known_sizes)
                            != (old.size, old.known_to, old.known_sizes)
                    })
                {
                    return Ok(None);
                }
                let mut updated = saved.clone();
                match order {
                    Order::SetRates { science, tax } => {
                        updated.science_rates[usize::from(self.human)] = science;
                        updated.tax_rates[usize::from(self.human)] = tax;
                    }
                    Order::QuotedProduction { city, .. } | Order::BuyProduction { city, .. } => {
                        let world_city = &world.cities[city];
                        let record = updated.cities.get_mut(city).ok_or(refused)?;
                        if record.city_id() != world_city.city_id {
                            return Err(refused);
                        }
                        record.raw[30..32].copy_from_slice(&world_city.shield_box.to_le_bytes());
                        record.raw[59] = world_city.building as u8;
                        if matches!(order, Order::BuyProduction { .. }) {
                            updated.treasury[usize::from(self.human)] =
                                world.treasury[usize::from(self.human)];
                        }
                    }
                    Order::ToggleWorker { city, .. } => {
                        let world_city = &world.cities[city];
                        let record = updated.cities.get_mut(city).ok_or(refused)?;
                        if record.city_id() != world_city.city_id {
                            return Err(refused);
                        }
                        record.raw[24..28].copy_from_slice(&world_city.specialists.to_le_bytes());
                        record.raw[50..54].copy_from_slice(&world_city.worked.to_le_bytes());
                    }
                    _ => unreachable!(),
                }
                (Ok(updated), result)
            }
            _ => return Ok(None),
        };
        let observed = observed.map_err(|_| refused)?;
        let projection = SavedMapKnowledge::read(&observed, self.human).map_err(|_| refused)?;
        let view = Self::prepare_saved_view(&world, self.human, projection).map_err(|_| refused)?;
        let result = self
            .game
            .commit_saved_view_order(world, self.human, order, result);
        self.set_saved_view(view);
        self.saved_move_state = Some(observed);
        if let Order::SkipUnit { unit } | Order::Move { unit, .. } = order {
            self.reset_deferred_focus_after_exhaustion(unit);
        }
        Ok(Some(result))
    }
}

fn clear_saved_deferrals(saved: &mut tot_kit::SaveFile, except: Option<usize>) {
    for (slot, unit) in saved.units.iter_mut().enumerate() {
        if unit.id() != 0 && Some(slot) != except {
            unit.raw[7] &= !0x40;
        }
    }
}

#[cfg(test)]
mod tests;
