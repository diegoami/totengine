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

impl HumanSession {
    pub(super) fn install_saved_view(
        &mut self,
        projection: SavedMapKnowledge,
    ) -> Result<(), String> {
        let world = &self.game.world;
        if projection.civ != self.human
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
                moves_left: if saved.owner == self.human {
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
        self.known_tiles = projection
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
        self.known_cities = cities;
        self.saved_initial = Some(SavedInitialView { units });
        self.collect_event_texts();
        Ok(())
    }
}

#[cfg(test)]
mod tests;
