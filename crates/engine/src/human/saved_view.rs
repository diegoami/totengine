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

impl HumanSession {
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
                if unit.owner != civ
                    || !from_city
                    || self.known_tiles.get(at).is_none_or(Option::is_some)
                {
                    return Err("city pass created a unit outside an unseen processed city".into());
                }
            }
        }

        let mut observed = saved.clone();
        let mut updated_world = world.clone();
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
        }
        let projection = SavedMapKnowledge::read(&observed, self.human)?;
        let prepared = Self::prepare_saved_view(&updated_world, self.human, projection)?;
        self.game.world = updated_world;
        self.saved_move_state = Some(observed);
        self.set_saved_view(prepared);
        Ok(())
    }

    pub(super) fn keeps_city_view_for_opponent_order(&self, civ: u8, order: Order) -> bool {
        if self.knowledge_recipe != ScenarioKnowledgeRecipe::SavedCityViewStagedRoundV1
            || self.saved_move_state.is_none()
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
                );
                (step, OrderResult::Skipped)
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
        Ok(Some(result))
    }
}

#[cfg(test)]
mod tests;
