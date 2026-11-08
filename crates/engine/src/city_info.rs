//! Immutable own-city information assembled from the existing rules kernels.
use crate::{happiness::Happiness, WorldState};
use tot_kit::Rules;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CityUnitInfo {
    pub slot: usize,
    pub id: u32,
    pub type_id: u8,
    pub position: (u16, u16),
    /// None for present units supported by another city (or without a home).
    pub shields: Option<i32>,
    pub food: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CityImprovementInfo {
    pub id: u8,
    pub upkeep: i32,
    pub wonder: bool,
    /// Wonders may remain in a city after their ongoing effect expires.
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CityInfo {
    pub index: usize,
    pub city_id: u32,
    pub supported: Vec<CityUnitInfo>,
    pub present: Vec<CityUnitInfo>,
    pub improvements: Vec<CityImprovementInfo>,
    /// Normalized base/luxury/building/military/wonder stages, not differences.
    pub happiness: [Happiness; 5],
}

impl WorldState {
    /// Only this civilization's live surface cities and units are exposed.
    /// Does not refresh totals, capture decision context or execute an order.
    pub fn city_info(&self, rules: &Rules, civ: u8, index: usize) -> Option<CityInfo> {
        let city = self.cities.get(index)?;
        if civ >= 8 || city.city_id == 0 || city.owner & 15 != u16::from(civ) || city.map_id != 0 {
            return None;
        }
        let government = self.governments[civ as usize];
        let free = std::array::from_fn(|k| rules.cosmic_row(13 + k).unwrap_or(0) as i32);
        let mut ordinal = 0;
        let mut supported = Vec::new();
        let mut present = Vec::new();
        for (slot, unit) in self.units.iter().enumerate() {
            if unit.id == 0 || unit.owner != civ {
                continue;
            }
            let homed = unit.home as usize == index;
            let role = rules.unit_role(unit.type_id as usize).unwrap_or(0);
            let shields = homed.then(|| {
                if role >= 6 {
                    return 0;
                }
                ordinal += 1;
                crate::support::unit_cost(
                    government,
                    city.size as u8 as i8 as i32,
                    free,
                    ordinal,
                    rules.unit_flags(unit.type_id as usize).unwrap_or(0) & 0x800 != 0,
                )
            });
            let info = CityUnitInfo {
                slot,
                id: unit.id,
                type_id: unit.type_id,
                position: (unit.x, unit.y),
                shields,
                food: homed.then(|| {
                    if role == 5 {
                        rules.cosmic_settler_food(government > 2) as i32
                    } else {
                        0
                    }
                }),
            };
            if homed {
                supported.push(info.clone());
            }
            if (unit.x, unit.y) == (city.x, city.y) {
                present.push(info);
            }
        }
        let mut improvements = (1..36)
            .filter(|&id| city.has_building(id))
            .map(|id| CityImprovementInfo {
                id: id as u8,
                upkeep: self.improvement_upkeep(rules, civ, id),
                wonder: false,
                active: true,
            })
            .collect::<Vec<_>>();
        for (wonder, &where_) in self.wonders.iter().enumerate() {
            if where_ >= 0 && where_ as usize == index {
                improvements.push(CityImprovementInfo {
                    id: 40 + wonder as u8,
                    upkeep: 0,
                    wonder: true,
                    active: self.active_wonder_city(rules, wonder) == Some(index),
                });
            }
        }
        Some(CityInfo {
            index,
            city_id: city.city_id,
            supported,
            present,
            improvements,
            happiness: self.city_happiness_stages_with_luxuries(
                rules,
                index,
                self.city_finance(rules, index).luxuries,
            ),
        })
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn city_info_composes_adjusted_upkeep_and_local_wonder_expiry() {
        let mut world = crate::boot(&crate::tests_support::synthetic_save(), 1);
        let rules = tot_kit::parse_rules("@CIVILIZE\nOne,1,0,nil,nil,0,0 ; One\n@ENDWONDER\nOne\n@IMPROVE\nNothing,0,0,nil\nPalace,10,0,nil\nBarracks,4,2,nil\nGranary,6,1,nil\nTemple,4,3,nil\n");
        world.wonders = [-1; 28];
        world.techs = [[0; 13]; 8];
        world.difficulty = 0;
        world.governments[1] = 4;
        world.cities[1].owner = 1;
        world.cities[1].city_id = 77;
        world.cities[1].map_id = 0;
        world.cities[1].buildings = [0; 5];
        world.cities[1].buildings[0] = (1 << 2) | (1 << 4);
        world.wonders[0] = 1;
        world.wonders[1] = 0; // A wonder elsewhere is not a local city row.
        for expired in [false, true] {
            if expired {
                world.techs[2][0] |= 1;
            }
            let before = world.checksum();
            let info = world.city_info(&rules, 1, 1).unwrap();
            assert_eq!(before, world.checksum());
            assert_eq!(
                info.improvements,
                vec![
                    super::CityImprovementInfo {
                        id: 2,
                        upkeep: 1,
                        wonder: false,
                        active: true
                    },
                    super::CityImprovementInfo {
                        id: 4,
                        upkeep: 0,
                        wonder: false,
                        active: true
                    },
                    super::CityImprovementInfo {
                        id: 40,
                        upkeep: 0,
                        wonder: true,
                        active: !expired
                    },
                ]
            );
        }
    }

    #[test]
    fn own_city_details_are_inert_and_support_lists_do_not_leak_foreign_units() {
        let mut world = crate::boot(&crate::tests_support::synthetic_save(), 1);
        let rules=tot_kit::parse_rules("@COSMIC\n3\n2\n2\n10\n10\n1\n2\n7\n14\n@UNITS\nArmy,nil,0,1.,0,1a,1d,1h,1f,1,0,0,nil,000000000000000\nSettler,nil,0,1.,0,0a,1d,1h,1f,1,0,5,nil,000000000000000\nTrader,nil,0,1.,0,0a,1d,1h,1f,1,0,6,nil,000000000000000\n@IMPROVE\nNothing,0,0,nil\nPalace,10,0,nil\nBarracks,4,1,nil\n");
        world.wonders = [-1; 28];
        world.cities[1].owner = 1;
        world.cities[1].city_id = 77;
        world.cities[1].map_id = 0;
        world.cities[1].size = 1;
        world.cities[1].buildings = [0; 5];
        world.cities[1].buildings[0] = (1 << 1) | (1 << 2);
        let city = world.cities[1].clone();
        world.governments[1] = 1;
        let mut unit = world.units[0].clone();
        unit.id = 1;
        unit.owner = 1;
        unit.home = 1;
        unit.type_id = 0;
        unit.x = city.x;
        unit.y = city.y;
        world.units = vec![unit.clone()];
        unit.id = 2;
        unit.type_id = 1;
        unit.x = city.x + 2;
        world.units.push(unit.clone());
        unit.id = 3;
        unit.type_id = 2;
        world.units.push(unit.clone());
        unit.id = 4;
        unit.home = 0xff;
        unit.x = city.x;
        world.units.push(unit.clone());
        unit.id = 5;
        unit.owner = 2;
        unit.home = 1;
        world.units.push(unit.clone());
        unit.id = 0;
        unit.owner = 1;
        world.units.push(unit);
        let before = world.checksum();
        let info = world.city_info(&rules, 1, 1).unwrap();
        assert_eq!(
            before,
            world.checksum(),
            "inspection must not refresh or capture state"
        );
        assert_eq!(info.city_id, 77);
        assert_eq!(
            info.supported.iter().map(|u| u.id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(
            info.present.iter().map(|u| u.id).collect::<Vec<_>>(),
            vec![1, 4]
        );
        assert_eq!(
            info.supported
                .iter()
                .map(|u| u.shields.unwrap())
                .collect::<Vec<_>>(),
            vec![0, 1, 0]
        );
        assert_eq!(
            info.supported
                .iter()
                .map(|u| u.shields.unwrap())
                .sum::<i32>(),
            world.unit_support(&rules, 1)
        );
        assert_eq!(
            info.supported.iter().map(|u| u.food.unwrap()).sum::<i32>(),
            1
        );
        assert_eq!(
            info.present[1].shields, None,
            "present does not imply supported here"
        );
        assert_eq!(info.happiness[4], world.city_happiness(&rules, 1));
        assert_eq!(
            info.improvements.iter().map(|b| b.id).collect::<Vec<_>>(),
            vec![1, 2]
        );
        for civ in [0, 2, 7, 8, 255] {
            assert!(world.city_info(&rules, civ, 1).is_none());
        }
        assert!(world.city_info(&rules, 1, usize::MAX).is_none());
        world.cities[1].map_id = 1;
        assert!(world.city_info(&rules, 1, 1).is_none());
        world.cities[1].map_id = 0;
        world.cities[1].city_id = 0;
        assert!(world.city_info(&rules, 1, 1).is_none());
    }
}
