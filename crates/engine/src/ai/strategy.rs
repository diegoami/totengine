//! Observation-only campaign movement. These are engine policies, not a port
//! of the original unit-AI scoring. Paths use known land and cannot cross seams.
use super::{BuiltInAi, NEIGHBOURS, ROLE_SETTLE};
use crate::placement::diamond_distance;
use crate::player::{Observation, Order, UnitView};
use std::collections::VecDeque;

impl BuiltInAi {
    pub(super) fn is_garrison(&self, obs: &Observation, unit: &UnitView) -> bool {
        if !obs
            .cities
            .iter()
            .any(|c| c.owner == obs.civ && (c.x, c.y) == (unit.x, unit.y))
        {
            return false;
        }
        obs.units
            .iter()
            .filter(|u| u.owner == obs.civ && (u.x, u.y) == (unit.x, unit.y))
            .filter_map(|u| {
                self.base
                    .base
                    .units
                    .get(u.type_id as usize)
                    .filter(|t| t.defense > 0 && t.role != ROLE_SETTLE)
                    .map(|t| (t.defense, std::cmp::Reverse(u.index)))
            })
            .max()
            .is_some_and(|(_, index)| index.0 == unit.index)
    }
    fn passable(&self, obs: &Observation, x: i32, y: i32) -> bool {
        x >= 0
            && x < i32::from(obs.w2)
            && y >= 0
            && y < i32::from(obs.h)
            && self
                .base
                .base
                .tile(obs, x, y)
                .is_some_and(|t| t.terrain != 10)
    }
    fn safe_attack(&self, obs: &Observation, unit: &UnitView, at: (u16, u16)) -> bool {
        let Some(att) = self.base.base.units.get(unit.type_id as usize) else {
            return false;
        };
        if att.attack <= 0 || att.role == ROLE_SETTLE {
            return false;
        }
        let Some(tile) = self.base.base.tile(obs, i32::from(at.0), i32::from(at.1)) else {
            return false;
        };
        let attack = att.attack * if unit.veteran { 12 } else { 8 };
        let terrain = self
            .terrain_defense
            .get(tile.terrain as usize)
            .copied()
            .unwrap_or(2)
            + i32::from(tile.river);
        let city = obs
            .cities
            .iter()
            .any(|c| c.owner != obs.civ && (c.x, c.y) == at);
        let defense = obs
            .units
            .iter()
            .filter(|u| u.owner != obs.civ && (u.x, u.y) == at)
            .filter_map(|u| {
                self.base
                    .base
                    .units
                    .get(u.type_id as usize)
                    .map(|t| t.defense * terrain * if u.veteran { 6 } else { 4 })
            })
            .max()
            .unwrap_or(0);
        // Unknown fortifications/health make this an estimate, with a margin.
        let defense = if city { defense * 3 / 2 } else { defense };
        attack * 4 >= defense.max(1) * 5
    }
    fn path(&self, obs: &Observation, unit: &UnitView, target: (u16, u16)) -> Option<(u16, u16)> {
        let width = usize::from(obs.w2) / 2;
        if width == 0 {
            return None;
        }
        let index = |x: u16, y: u16| usize::from(y) * width + usize::from(x) / 2;
        let start = index(unit.x, unit.y);
        let end = index(target.0, target.1);
        let mut parents = vec![usize::MAX; obs.seen.len()];
        if start >= parents.len() || end >= parents.len() || start == end {
            return None;
        }
        let mut queue = VecDeque::from([(unit.x, unit.y)]);
        parents[start] = start;
        while let Some((x, y)) = queue.pop_front() {
            for (dx, dy) in NEIGHBOURS {
                let (nx, ny) = (i32::from(x) + dx, i32::from(y) + dy);
                if !self.passable(obs, nx, ny) {
                    continue;
                }
                let next = (nx as u16, ny as u16);
                let ni = index(next.0, next.1);
                if parents[ni] != usize::MAX
                    || self.base.visited.contains(&(unit.index, next.0, next.1))
                {
                    continue;
                }
                let enemy = obs
                    .units
                    .iter()
                    .any(|u| u.owner != obs.civ && (u.x, u.y) == next)
                    || obs
                        .cities
                        .iter()
                        .any(|c| c.owner != obs.civ && (c.x, c.y) == next);
                if enemy && (next != target || !self.safe_attack(obs, unit, next)) {
                    continue;
                }
                if self.base.base.units[unit.type_id as usize].role == ROLE_SETTLE
                    && obs.units.iter().any(|u| {
                        u.owner != obs.civ
                            && diamond_distance(
                                i32::from(obs.w2),
                                false,
                                i32::from(u.x),
                                i32::from(u.y),
                                nx,
                                ny,
                            ) <= 1
                    })
                {
                    continue;
                }
                parents[ni] = index(x, y);
                if ni == end {
                    let mut step = ni;
                    while parents[step] != start {
                        step = parents[step];
                    }
                    let y = step / width;
                    let x = 2 * (step % width) + (y % 2);
                    return Some((x as u16, y as u16));
                }
                queue.push_back(next);
            }
        }
        None
    }
    pub(super) fn strategic_order(&self, obs: &Observation, order: Order) -> Order {
        let unit = match order {
            Order::Move { unit, .. } | Order::FoundCity { unit } => unit,
            _ => return order,
        };
        let Some(u) = obs.units.iter().find(|u| u.index == unit) else {
            return order;
        };
        let Some(info) = self.base.base.units.get(u.type_id as usize) else {
            return order;
        };
        if info.domain != 0 {
            return order;
        }
        let distance = |at: (u16, u16)| {
            diamond_distance(
                i32::from(obs.w2),
                false,
                i32::from(u.x),
                i32::from(u.y),
                i32::from(at.0),
                i32::from(at.1),
            )
        };
        let mut goals = Vec::new();
        if info.role == ROLE_SETTLE {
            for (i, tile) in obs.seen.iter().enumerate() {
                let Some(tile) = tile else {
                    continue;
                };
                if tile.terrain == 10 {
                    continue;
                }
                let y = i / (usize::from(obs.w2) / 2);
                let x = 2 * (i % (usize::from(obs.w2) / 2)) + y % 2;
                let at = (x as u16, y as u16);
                let d = distance(at);
                if d > 8 || self.base.base.near_city(obs, x as i32, y as i32) <= 3 {
                    continue;
                }
                let food = self
                    .base
                    .terrain_food
                    .get(tile.terrain as usize)
                    .copied()
                    .unwrap_or(0);
                if food < 2 {
                    continue;
                }
                let ring = NEIGHBOURS
                    .iter()
                    .filter_map(|(dx, dy)| self.base.base.tile(obs, x as i32 + dx, y as i32 + dy))
                    .map(|t| {
                        self.base
                            .terrain_food
                            .get(t.terrain as usize)
                            .copied()
                            .unwrap_or(0)
                    })
                    .sum::<i32>();
                goals.push((food * 10 + ring - d, at));
            }
            goals.sort_by_key(|&(score, at)| (std::cmp::Reverse(score), at));
            for (_, at) in goals {
                if at == (u.x, u.y) {
                    return Order::FoundCity { unit };
                }
                if let Some(to) = self.path(obs, u, at) {
                    return Order::Move { unit, to };
                }
            }
        } else {
            for city in &obs.cities {
                let at = (city.x, city.y);
                if city.owner == obs.civ {
                    let guarded = obs.units.iter().any(|u| {
                        u.owner == obs.civ
                            && (u.x, u.y) == at
                            && self
                                .base
                                .base
                                .units
                                .get(u.type_id as usize)
                                .is_some_and(|t| t.defense > 0 && t.role != ROLE_SETTLE)
                    });
                    if !guarded && info.defense > 0 {
                        goals.push((1000 - distance(at), at));
                    }
                } else if self.safe_attack(obs, u, at) {
                    goals.push((500 - distance(at), at));
                }
            }
            for other in obs.units.iter().filter(|other| other.owner != obs.civ) {
                let at = (other.x, other.y);
                if self.safe_attack(obs, u, at) {
                    goals.push((300 - distance(at), at));
                }
            }
            goals.sort_by_key(|&(score, at)| (std::cmp::Reverse(score), at));
            for (_, at) in goals {
                if let Some(to) = self.path(obs, u, at) {
                    if let Some(city) = obs
                        .cities
                        .iter()
                        .find(|c| c.owner != obs.civ && (c.x, c.y) == to)
                    {
                        if !obs
                            .units
                            .iter()
                            .any(|u| u.owner != obs.civ && (u.x, u.y) == to)
                        {
                            return Order::OccupyCity {
                                unit,
                                city: city.index,
                            };
                        }
                    }
                    return Order::Move { unit, to };
                }
            }
        }
        order
    }
}
