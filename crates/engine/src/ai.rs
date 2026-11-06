//! AI v0 (v0.6.0, #75 and #76): a deliberately simple, deterministic
//! player that makes an all-AI game run. It explores, founds cities, and
//! builds settlers and defenders. It is the engine's own design (decision
//! 0002), not a decode of the original's AI.
//!
//! - **Cities (build choice, #76):**
//!   - a city with no own defender on its tile builds the best defender
//!     it can (defence / cost, as the original picks a new city's first
//!     build);
//!   - otherwise Settlers once it has size 2 or more and the civ has fewer
//!     than 12 cities;
//!   - otherwise another defender.
//! - **Settlers:** found a city where no known city lies within distance
//!   2; otherwise move to the neighbouring land tile farthest from known
//!   cities.
//! - **Other units:** the first defender in a city stays as its garrison;
//!   the rest explore toward the neighbour that borders the most unseen
//!   tiles.
//!
//! Its randomness (tie-breaks) comes from its own generator, seeded from
//! the game seed and the civ, so it cannot change the world's dice.

use std::collections::HashSet;

mod strategy;

use tot_kit::Rules;

use crate::founding::ROLE_SETTLE;
use crate::movement::OCEAN_TERRAIN;
use crate::placement::diamond_distance;
use crate::player::{Observation, Order, Player};
use crate::rng::{GameRng, Lcg64};

const ROLE_DEFEND: u8 = 1;
const MAX_CITIES: usize = 12;
const NEIGHBOURS: [(i32, i32); 8] = [
    (1, -1),
    (2, 0),
    (1, 1),
    (0, 2),
    (-1, 1),
    (-2, 0),
    (-1, -1),
    (0, -2),
];

/// What AI v0 needs from the rules, read once.
#[derive(Debug, Clone)]
struct UnitInfo {
    role: u8,
    defense: i32,
    attack: i32,
    domain: u8,
    civ_mask: u8,
    cost: i32,
    /// Advance index required, `None` = none, `Some(None)` = never.
    prereq: Option<Option<usize>>,
}

pub struct AiV0 {
    rng: Lcg64,
    units: Vec<UnitInfo>,
    turn: i16,
    done: HashSet<usize>,
    production_done: bool,
}

impl AiV0 {
    pub fn new(rules: &Rules, seed: u64, civ: u8) -> Self {
        let tree = crate::research::TechTree::from_rules(rules);
        let units = (0..rules.unit_names().len())
            .map(|t| UnitInfo {
                role: rules.unit_role(t).unwrap_or(0),
                defense: rules.unit_defense(t).unwrap_or(0) as i32,
                attack: rules.unit_attack(t).unwrap_or(0) as i32,
                domain: rules.unit_domain(t).unwrap_or(0),
                civ_mask: rules.unit_civ_mask(t),
                cost: rules.unit_cost(t).unwrap_or(1).max(1) as i32,
                prereq: match rules.unit_prereq_raw(t) {
                    None | Some("nil") => None,
                    Some("no") => Some(None),
                    Some(a) => Some(tree.by_abbrev(a)),
                },
            })
            .collect();
        AiV0 {
            rng: Lcg64::new((seed as u32) ^ 0x5eed_0000 ^ (civ as u32).wrapping_mul(0x0101_0101)),
            units,
            turn: i16::MIN,
            done: HashSet::new(),
            production_done: false,
        }
    }

    fn buildable(&self, obs: &Observation, t: usize) -> bool {
        if self.units[t].civ_mask & (1 << obs.civ) == 0 {
            return false;
        }
        match self.units[t].prereq {
            None => true,
            Some(None) => false,
            Some(Some(k)) => obs.techs.get(k >> 3).is_some_and(|b| b >> (k & 7) & 1 != 0),
        }
    }

    fn best_defender(&self, obs: &Observation) -> Option<usize> {
        let mut best: Option<(i32, usize)> = None;
        for (t, u) in self.units.iter().enumerate() {
            if u.role == ROLE_DEFEND && self.buildable(obs, t) {
                let score = u.defense * 100 / u.cost;
                if best.is_none_or(|(s, _)| score >= s) {
                    best = Some((score, t));
                }
            }
        }
        best.map(|(_, t)| t)
    }

    fn settler(&self, obs: &Observation) -> Option<usize> {
        (0..self.units.len()).find(|&t| self.units[t].role == ROLE_SETTLE && self.buildable(obs, t))
    }

    fn tile(&self, obs: &Observation, x: i32, y: i32) -> Option<crate::player::TileView> {
        if y < 0 || y >= obs.h as i32 {
            return None;
        }
        let x = x.rem_euclid(obs.w2.max(1) as i32);
        let i = y as usize * (obs.w2 as usize / 2) + x as usize / 2;
        obs.seen.get(i).copied().flatten()
    }

    fn land(&self, obs: &Observation, x: i32, y: i32) -> bool {
        self.tile(obs, x, y)
            .is_some_and(|t| t.terrain != OCEAN_TERRAIN)
    }

    fn near_city(&self, obs: &Observation, x: i32, y: i32) -> i32 {
        obs.cities
            .iter()
            .map(|c| diamond_distance(obs.w2 as i32, false, c.x as i32, c.y as i32, x, y))
            .min()
            .unwrap_or(99)
    }

    fn production(&mut self, obs: &Observation) -> Option<Order> {
        let own_cities = obs.cities.iter().filter(|c| c.owner == obs.civ).count();
        for c in obs.cities.iter().filter(|c| c.owner == obs.civ) {
            let defended = obs.units.iter().any(|u| {
                u.owner == obs.civ
                    && (u.x, u.y) == (c.x, c.y)
                    && self
                        .units
                        .get(u.type_id as usize)
                        .is_some_and(|i| i.role == ROLE_DEFEND)
            });
            let want = if !defended {
                self.best_defender(obs)
            } else if c.size >= 2 && own_cities < MAX_CITIES {
                self.settler(obs).or_else(|| self.best_defender(obs))
            } else {
                self.best_defender(obs)
            };
            if let Some(t) = want {
                if c.building != Some(t as i8) && !self.done.contains(&(usize::MAX - c.index)) {
                    self.done.insert(usize::MAX - c.index);
                    return Some(Order::SetProduction {
                        city: c.index,
                        item: t as i8,
                    });
                }
            }
        }
        None
    }

    fn unit_order(&mut self, obs: &Observation) -> Option<Order> {
        let mine: Vec<_> = obs
            .units
            .iter()
            .filter(|u| u.owner == obs.civ && u.moves_left > 0 && !self.done.contains(&u.index))
            .cloned()
            .collect();
        let garrisons: HashSet<(u16, u16)> = HashSet::new();
        let mut garrisons = garrisons;
        for u in mine {
            let role = self.units.get(u.type_id as usize).map_or(0, |i| i.role);
            let (x, y) = (u.x as i32, u.y as i32);
            if role == ROLE_SETTLE {
                if self.near_city(obs, x, y) > 2 && self.land(obs, x, y) {
                    self.done.insert(u.index);
                    return Some(Order::FoundCity { unit: u.index });
                }
                let mut best: Option<(i32, u32, (i32, i32))> = None;
                for (dx, dy) in NEIGHBOURS {
                    let (nx, ny) = (x + dx, y + dy);
                    if !self.land(obs, nx, ny) {
                        continue;
                    }
                    let key = (self.near_city(obs, nx, ny), self.rng.next_u32());
                    if best.is_none_or(|(d, r, _)| (key.0, key.1) > (d, r)) {
                        best = Some((key.0, key.1, (nx, ny)));
                    }
                }
                self.done.insert(u.index);
                if let Some((_, _, (nx, ny))) = best {
                    return Some(Order::Move {
                        unit: u.index,
                        to: (nx.rem_euclid(obs.w2.max(1) as i32) as u16, ny as u16),
                    });
                }
                continue;
            }
            let in_city = obs
                .cities
                .iter()
                .any(|c| c.owner == obs.civ && (c.x, c.y) == (u.x, u.y));
            if in_city && role == ROLE_DEFEND && garrisons.insert((u.x, u.y)) {
                self.done.insert(u.index);
                continue;
            }
            // explore: the neighbour bordering the most unseen tiles
            let mut best: Option<(usize, u32, (i32, i32))> = None;
            for (dx, dy) in NEIGHBOURS {
                let (nx, ny) = (x + dx, y + dy);
                if !self.land(obs, nx, ny) {
                    continue;
                }
                let unseen = NEIGHBOURS
                    .iter()
                    .filter(|(ex, ey)| self.tile(obs, nx + ex, ny + ey).is_none())
                    .count();
                let key = (unseen, self.rng.next_u32());
                if best.is_none_or(|(s, r, _)| key > (s, r)) {
                    best = Some((key.0, key.1, (nx, ny)));
                }
            }
            self.done.insert(u.index);
            if let Some((_, _, (nx, ny))) = best {
                return Some(Order::Move {
                    unit: u.index,
                    to: (nx.rem_euclid(obs.w2.max(1) as i32) as u16, ny as u16),
                });
            }
        }
        None
    }
}

impl Player for AiV0 {
    fn act(&mut self, obs: &Observation) -> Order {
        if obs.turn != self.turn {
            self.turn = obs.turn;
            self.done.clear();
            self.production_done = false;
        }
        if !self.production_done {
            if let Some(o) = self.production(obs) {
                return o;
            }
            self.production_done = true;
        }
        self.unit_order(obs).unwrap_or(Order::EndTurn)
    }
}

/// Economy-aware RG-8 player (#82). Policy is engine-designed: maintain a
/// garrison, constrain settler support, invest in growth/science/tax buildings,
/// and fight only favorable nearby visible targets. No hidden world inputs.
/// Research uses Rules AI values, with lowest-ID ties. Original-like AI is
/// the separate v0.10 target.
pub struct AiV1 {
    base: AiV0,
    strategic: bool,
    research_values: Vec<i32>,
    terrain_food: Vec<i32>,
    visited: HashSet<(usize, u16, u16)>,
    pending_unit: Option<usize>,
}

impl AiV1 {
    pub fn new(rules: &Rules, seed: u64, civ: u8) -> Self {
        Self {
            base: AiV0::new(rules, seed, civ),
            strategic: false,
            research_values: rules
                .tech_defs()
                .iter()
                .map(|t| t.ai_value.unwrap_or(0))
                .collect(),
            terrain_food: (0..33)
                .map(|t| rules.terrain_yield(t, 0).unwrap_or(0))
                .collect(),
            visited: HashSet::new(),
            pending_unit: None,
        }
    }

    fn production(&mut self, obs: &Observation) -> Option<Order> {
        let own: Vec<_> = obs.cities.iter().filter(|c| c.owner == obs.civ).collect();
        let settlers = obs
            .units
            .iter()
            .filter(|u| {
                u.owner == obs.civ
                    && self
                        .base
                        .units
                        .get(u.type_id as usize)
                        .is_some_and(|u| u.role == ROLE_SETTLE)
            })
            .count();
        let budget: i32 = obs
            .city_economy
            .iter()
            .map(|c| c.finance.taxes - c.upkeep)
            .sum();
        for c in &own {
            let key = usize::MAX - c.index;
            if self.base.done.contains(&key) {
                continue;
            }
            self.base.done.insert(key);
            let Some(e) = obs.city_economy.iter().find(|e| e.index == c.index) else {
                continue;
            };
            let defended = obs.units.iter().any(|u| {
                u.owner == obs.civ
                    && (u.x, u.y) == (c.x, c.y)
                    && self
                        .base
                        .units
                        .get(u.type_id as usize)
                        .is_some_and(|u| u.defense > 0 && u.role != ROLE_SETTLE)
            });
            let building = |id: u8| e.buildings[id as usize / 8] & (1 << (id % 8)) != 0;
            let affordable = |id: u8| {
                e.available_improvements.contains(&id)
                    && (budget >= obs.improvement_upkeep[id as usize]
                        || obs.treasury >= 20 + 10 * obs.improvement_upkeep[id as usize])
            };
            let mut want = None;
            if !defended {
                want = self.base.best_defender(obs).map(|t| t as i8);
            }
            if want.is_none() && e.unhappy > e.happy {
                want = [4, 14, 11]
                    .into_iter()
                    .find(|&id| affordable(id))
                    .map(|id| -(id as i8));
            }
            if want.is_none() && e.food_surplus > 0 {
                want = e
                    .growth_block
                    .filter(|&id| affordable(id))
                    .map(|id| -(id as i8));
            }
            if want.is_none() && self.strategic {
                let queued_settlers = own
                    .iter()
                    .filter(|v| {
                        v.index != c.index
                            && v.building.is_some_and(|id| {
                                id >= 0
                                    && self
                                        .base
                                        .units
                                        .get(id as usize)
                                        .is_some_and(|t| t.role == ROLE_SETTLE)
                            })
                    })
                    .count();
                if own.len() < 24
                    && c.size >= 2
                    && e.food_surplus >= 2
                    && settlers + queued_settlers < own.len() + 1
                    && e.net_shields > 0
                {
                    want = self.base.settler(obs).map(|id| id as i8);
                }
                let field = obs
                    .units
                    .iter()
                    .filter(|u| {
                        u.owner == obs.civ
                            && self
                                .base
                                .units
                                .get(u.type_id as usize)
                                .is_some_and(|t| t.role != ROLE_SETTLE && t.defense > 0)
                    })
                    .count();
                let queued_field = own
                    .iter()
                    .filter(|v| {
                        v.index != c.index
                            && v.building.is_some_and(|id| {
                                id >= 0
                                    && self
                                        .base
                                        .units
                                        .get(id as usize)
                                        .is_some_and(|t| t.role != ROLE_SETTLE && t.defense > 0)
                            })
                    })
                    .count();
                if want.is_none() && field + queued_field < own.len() * 4 + 2 {
                    want = self
                        .base
                        .units
                        .iter()
                        .enumerate()
                        .filter(|(id, t)| {
                            t.role <= 1
                                && t.domain == 0
                                && t.attack > 0
                                && self.base.buildable(obs, *id)
                        })
                        .max_by_key(|(id, t)| (t.attack * 100 / t.cost, *id))
                        .map(|(id, _)| id as i8);
                }
            }
            let queued = own
                .iter()
                .filter(|v| {
                    v.index != c.index
                        && v.building.is_some_and(|t| {
                            t >= 0
                                && self
                                    .base
                                    .units
                                    .get(t as usize)
                                    .is_some_and(|u| u.role == ROLE_SETTLE)
                        })
                })
                .count();
            if want.is_none()
                && own.len() < 24
                && c.size >= 3
                && e.food_surplus >= 2
                && settlers + queued < own.len() / 3 + 1
                && e.net_shields > 0
            {
                want = self.base.settler(obs).map(|t| t as i8);
            }
            // Continue an investment once started, after urgent safety/growth
            // checks. Available excludes already completed improvements.
            if want.is_none()
                && c.building
                    .is_some_and(|t| t < 0 && affordable((-i16::from(t)) as u8))
            {
                continue;
            }
            if want.is_none() {
                let priorities = [
                    (
                        3,
                        e.food_surplus > 0 && c.size < 7 && e.growth_block.is_none(),
                    ),
                    (4, c.size >= 3),
                    (6, e.finance.science >= 2),
                    (5, e.finance.taxes + e.finance.luxuries >= 2),
                    (7, c.size >= 5),
                    (12, building(6) && e.finance.science >= 4),
                    (10, building(5) && e.finance.taxes >= 4),
                    (15, e.net_shields >= 6),
                    (2, true),
                ];
                want = priorities
                    .into_iter()
                    .find(|&(id, useful)| useful && affordable(id))
                    .map(|(id, _)| -(id as i8));
            }
            if want.is_none() && self.strategic {
                let attacks = obs
                    .units
                    .iter()
                    .filter(|u| {
                        u.owner == obs.civ
                            && self.base.units.get(u.type_id as usize).is_some_and(|t| {
                                t.domain == 0 && t.role == 0 && t.attack > t.defense
                            })
                    })
                    .count();
                let queued = own
                    .iter()
                    .filter(|other| {
                        other.index != c.index
                            && other.building.is_some_and(|id| {
                                id >= 0
                                    && self
                                        .base
                                        .units
                                        .get(id as usize)
                                        .is_some_and(|t| t.role == 0 && t.attack > t.defense)
                            })
                    })
                    .count();
                if attacks + queued < own.len() / 2 + 1 {
                    want = self
                        .base
                        .units
                        .iter()
                        .enumerate()
                        .filter(|(id, t)| {
                            t.role <= 1
                                && t.domain == 0
                                && t.attack > t.defense
                                && self.base.buildable(obs, *id)
                        })
                        .max_by_key(|(id, t)| (t.attack * 100 / t.cost, *id))
                        .map(|(id, _)| id as i8);
                }
            }
            if want.is_none() {
                want = self.base.best_defender(obs).map(|t| t as i8);
            }
            if let Some(item) = want.filter(|&item| c.building != Some(item)) {
                return Some(Order::SetProduction {
                    city: c.index,
                    item,
                });
            }
        }
        None
    }

    fn unit_order(&mut self, obs: &Observation) -> Option<Order> {
        for u in obs
            .units
            .iter()
            .filter(|u| u.owner == obs.civ && u.moves_left > 0)
        {
            if self.base.done.contains(&u.index) {
                continue;
            }
            let Some(info) = self.base.units.get(u.type_id as usize) else {
                continue;
            };
            let in_city = obs
                .cities
                .iter()
                .any(|c| c.owner == obs.civ && (c.x, c.y) == (u.x, u.y));
            // Select the strongest present defender from the full observation,
            // including units that have already been visited by this player.
            let garrison = obs
                .units
                .iter()
                .filter(|v| v.owner == obs.civ && (v.x, v.y) == (u.x, u.y))
                .filter_map(|v| {
                    self.base
                        .units
                        .get(v.type_id as usize)
                        .filter(|t| t.defense > 0 && t.role != ROLE_SETTLE)
                        .map(|t| (t.defense, std::cmp::Reverse(v.index)))
                })
                .max()
                .map(|(_, index)| index.0);
            if in_city && garrison == Some(u.index) {
                self.base.done.insert(u.index);
                continue;
            }
            let (x, y) = (u.x as i32, u.y as i32);
            let food = self.base.tile(obs, x, y).map_or(0, |t| {
                self.terrain_food
                    .get(t.terrain as usize)
                    .copied()
                    .unwrap_or(0)
            });
            if info.role == ROLE_SETTLE && food >= 1 && self.base.near_city(obs, x, y) > 2 {
                self.base.done.insert(u.index);
                return Some(Order::FoundCity { unit: u.index });
            }
            self.visited.insert((u.index, u.x, u.y));
            let mut best = None;
            for (dx, dy) in NEIGHBOURS {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || nx >= obs.w2 as i32 {
                    continue;
                } // movement kernel has no seam crossing yet
                let Some(tile) = self.base.tile(obs, nx, ny) else {
                    continue;
                };
                if (info.domain == 0 && tile.terrain == OCEAN_TERRAIN)
                    || (info.domain == 2 && tile.terrain != OCEAN_TERRAIN)
                    || self.visited.contains(&(u.index, nx as u16, ny as u16))
                {
                    continue;
                }
                let enemy = obs
                    .units
                    .iter()
                    .filter(|v| v.owner != obs.civ && (v.x as i32, v.y as i32) == (nx, ny))
                    .filter_map(|v| self.base.units.get(v.type_id as usize))
                    .map(|v| v.defense)
                    .max();
                // Conservative visible-strength policy, not original combat AI.
                // Noncombatants never deliberately enter hostile units/cities.
                let foreign_city = obs
                    .cities
                    .iter()
                    .any(|c| c.owner != obs.civ && (c.x as i32, c.y as i32) == (nx, ny));
                if (enemy.is_some() || foreign_city)
                    && (info.attack <= 0
                        || info.role == ROLE_SETTLE
                        || enemy.is_some_and(|d| info.attack < 2 * d.max(1)))
                {
                    continue;
                }
                let danger = obs.units.iter().filter(|v| v.owner != obs.civ).any(|v| {
                    diamond_distance(obs.w2 as i32, false, v.x as i32, v.y as i32, nx, ny) <= 1
                });
                if info.role == ROLE_SETTLE && danger {
                    continue;
                }
                let unseen = NEIGHBOURS
                    .iter()
                    .filter(|&&(ex, ey)| self.base.tile(obs, nx + ex, ny + ey).is_none())
                    .count() as i32;
                let score = if info.role == ROLE_SETTLE {
                    let food = self
                        .terrain_food
                        .get(tile.terrain as usize)
                        .copied()
                        .unwrap_or(0);
                    self.base.near_city(obs, nx, ny).min(6) * 10 + food * 3 + unseen
                } else {
                    unseen * 4 + if enemy.is_some() { 20 } else { 0 } - i32::from(danger) * 2
                };
                let key = (score, self.base.rng.next_u32());
                if best.is_none_or(|(s, r, _, _)| key > (s, r)) {
                    best = Some((key.0, key.1, nx as u16, ny as u16));
                }
            }
            if let Some((_, _, nx, ny)) = best {
                self.pending_unit = Some(u.index);
                return Some(Order::Move {
                    unit: u.index,
                    to: (nx, ny),
                });
            }
            self.base.done.insert(u.index);
        }
        None
    }
}

impl AiV1 {
    fn act_with_research(&mut self, obs: &Observation, research: bool) -> Order {
        if self.base.turn != obs.turn {
            self.base.turn = obs.turn;
            self.base.done.clear();
            self.base.production_done = false;
            self.visited.clear();
            self.pending_unit = None;
        }
        if let Some(unit) = self.pending_unit.take() {
            if matches!(
                obs.last_result,
                Some(crate::player::OrderResult::Rejected(_))
            ) {
                self.base.done.insert(unit);
            }
        }
        if research
            && obs
                .research
                .target
                .is_none_or(|t| !obs.research.available.contains(&t))
        {
            if let Some(&advance) = obs.research.available.iter().max_by_key(|&&t| {
                (
                    self.research_values.get(t).copied().unwrap_or(0),
                    std::cmp::Reverse(t),
                )
            }) {
                return Order::SetResearch { advance };
            }
        }
        if !self.base.production_done {
            if let Some(order) = self.production(obs) {
                return order;
            }
            self.base.production_done = true;
        }
        self.unit_order(obs).unwrap_or(Order::EndTurn)
    }
}

impl Player for AiV1 {
    fn act(&mut self, obs: &Observation) -> Order {
        self.act_with_research(obs, true)
    }
}

/// Original-derived research, with AI v1 economy/movement as the initial
/// foundation. Only own observations enter scoring. Foreign caches are unknown,
/// so this is not full native decision parity. Its policy ID is checkpointed.
pub struct BuiltInAi {
    base: AiV1,
    tables: crate::ai_research::ValueRules,
    leaders: Vec<i8>,
    domains: Vec<Option<u8>>,
    groups: Vec<i8>,
    leader_groups: Vec<[u8; 8]>,
    rng: crate::combat::CrtRand,
    terrain_defense: Vec<i32>,
}

impl BuiltInAi {
    pub const POLICY: &'static str = "builtin-1";

    pub fn new(rules: &Rules, seed: u64, civ: u8) -> Result<Self, String> {
        let tables = crate::ai_research::ValueRules::from_rules(rules)?;
        let leaders = rules
            .section("LEADERS")
            .ok_or("built-in AI requires LEADERS")?
            .rows
            .iter()
            .take(21)
            .map(|r| {
                r.get(9)
                    .and_then(|n| n.parse().ok())
                    .ok_or_else(|| "invalid built-in leader modifier".to_string())
            })
            .collect::<Result<Vec<i8>, _>>()?;
        if leaders.len() != 21 {
            return Err("built-in AI requires 21 leaders".into());
        }
        let mut groups = vec![-1; 100];
        if let Some(s) = rules.section("CIVILIZE2") {
            for (i, row) in s.rows.iter().take(100).enumerate() {
                groups[i] = row
                    .first()
                    .and_then(|n| n.parse::<i8>().ok())
                    .filter(|n| (-1..8).contains(n))
                    .ok_or("invalid built-in technology group")?;
            }
        }
        let mut leader_groups = vec![[0; 8]; 21];
        if let Some(s) = rules.section("LEADERS2") {
            for (i, row) in s.rows.iter().take(21).enumerate() {
                let raw = row.first().ok_or("missing built-in leader group flags")?;
                if raw.len() != 8 || !raw.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("invalid built-in leader group flags".into());
                }
                for (to, from) in leader_groups[i].iter_mut().zip(raw.bytes()) {
                    *to = from - b'0';
                }
            }
        }
        Ok(Self {
            base: {
                let mut base = AiV1::new(rules, seed, civ);
                base.strategic = true;
                base
            },
            tables,
            leaders,
            terrain_defense: (0..33)
                .map(|i| rules.terrain_defense(i).unwrap_or(2) as i32)
                .collect(),
            domains: (0..80).map(|i| rules.unit_domain(i)).collect(),
            groups,
            leader_groups,
            rng: crate::combat::CrtRand::new(
                (seed as u32) ^ ((seed >> 32) as u32) ^ u32::from(civ).wrapping_mul(0x9e37_79b9),
            ),
        })
    }

    fn research(&mut self, obs: &Observation) -> Option<Order> {
        use crate::ai_research::{choose, value, Candidate, Context, ValueCity, ValueContext};
        if obs
            .research
            .target
            .is_some_and(|id| obs.research.available.contains(&id))
        {
            return None;
        }
        let leader = usize::try_from(obs.leader_id).ok()?;
        let modifier = *self.leaders.get(leader)?;
        let scenario_flags = obs.scenario_flags?;
        let own: Vec<_> = obs.cities.iter().filter(|c| c.owner == obs.civ).collect();
        // Preserve stable host indices without copying any foreign city data.
        let extent = own.iter().map(|c| c.index + 1).max().unwrap_or(0);
        let mut cities = vec![
            ValueCity {
                owner: 0,
                size: 0,
                live: false
            };
            extent
        ];
        for c in own {
            cities[c.index] = ValueCity {
                owner: c.owner,
                size: c.size as u8 as i8,
                live: true,
            };
        }
        let mut state = ValueContext {
            civ: obs.civ,
            human_mask: 0,
            game_flags: obs.research_flags,
            scenario_flags,
            leader_id: obs.leader_id,
            leader_modifier: modifier,
            acquired: [0; 8],
            reputations: [0; 8],
            treaties: [0; 8],
            city_landmasses: [[0; 256]; 8],
            tech_owner_masks: std::array::from_fn(|i| {
                if obs.techs[i / 8] & (1 << (i % 8)) != 0 {
                    1 << obs.civ
                } else {
                    0
                }
            }),
            sea_units: obs
                .units
                .iter()
                .filter(|u| {
                    u.owner == obs.civ && self.domains.get(u.type_id as usize) == Some(&Some(2))
                })
                .count() as i16,
            cities,
            wonder_cities: obs
                .own_wonders
                .map(|host| host.and_then(|i| i16::try_from(i).ok()).unwrap_or(-1)),
        };
        state.acquired[obs.civ as usize] =
            obs.techs.iter().map(|b| b.count_ones()).sum::<u32>() as u8;
        let candidates = std::array::from_fn(|id| Candidate {
            eligible: obs.research.available.contains(&id)
                && (self.groups[id] < 0
                    || self.leader_groups[leader][self.groups[id] as usize] == 0),
            value: value(&self.tables, &state, id).unwrap_or(1),
        });
        choose(
            &candidates,
            Context {
                human: false,
                difficulty: 0,
                acquired: 0,
            },
            &mut self.rng,
        )
        .map(|advance| Order::SetResearch { advance })
    }
}

impl Player for BuiltInAi {
    fn act(&mut self, obs: &Observation) -> Order {
        if let Some(order) = self.research(obs) {
            return order;
        }
        let order = self.base.act_with_research(obs, false);
        let mut chosen = self.strategic_order(obs, order);
        if chosen == Order::EndTurn {
            // The baseline may have refused every neighbouring enemy because
            // of its stricter strength threshold. Reconsider free field units
            // against the campaign policy's visible-strength estimate.
            for unit in obs
                .units
                .iter()
                .filter(|u| u.owner == obs.civ && u.moves_left > 0)
            {
                if self.is_garrison(obs, unit) {
                    continue;
                }
                let candidate = self.strategic_order(
                    obs,
                    Order::Move {
                        unit: unit.index,
                        to: (unit.x, unit.y),
                    },
                );
                if matches!(candidate, Order::Move {to,..} if to != (unit.x, unit.y))
                    || matches!(
                        candidate,
                        Order::FoundCity { .. } | Order::OccupyCity { .. }
                    )
                {
                    chosen = candidate;
                    break;
                }
            }
        }
        if let Order::Move { unit, .. } | Order::OccupyCity { unit, .. } = chosen {
            self.base.base.done.remove(&unit);
            self.base.pending_unit = Some(unit);
            if let Some(u) = obs.units.iter().find(|u| u.index == unit) {
                self.base.visited.insert((unit, u.x, u.y));
            }
        }
        chosen
    }
}

#[cfg(test)]
mod tests {
    use crate::player::{Game, Player};
    use crate::tests_support::synthetic_save_bytes;

    const RULES: &str = "@COSMIC\n3 ; road\n2 ; trireme\n2 ; food\n10 ; food rows\n\
10 ; shield rows\n@UNITS\nSettlers, nil, 0, 1., 0, 0a, 1d, 1h, 1f, 3, 0, 5, nil, 000000000000000\n\
Warriors, nil, 0, 1., 0, 1a, 1d, 1h, 1f, 1, 0, 1, nil, 000000000000000\n\
@IMPROVE\nNothing, 0, 0, nil\nPalace, 10, 0, nil\n\
@TERRAIN\nDesert, 1, 2, 0, 1, 0\nPlains, 1, 2, 1, 1, 0\nGrassland, 1, 2, 2, 0, 0\n";

    fn run(seed: u64) -> String {
        let rules = tot_kit::parse_rules(RULES);
        let mut g = Game::new("s.sav", &synthetic_save_bytes(), seed, rules, None).unwrap();
        for civ in 1..8u8 {
            let p: Box<dyn Player> = Box::new(super::AiV0::new(&g.rules, seed, civ));
            g.players[civ as usize] = Some(p);
        }
        for _ in 0..15 {
            g.play_turn();
        }
        g.log().to_string()
    }

    /// AI v0 is deterministic and every logged game replays.
    #[test]
    fn ai_games_are_deterministic_and_replay() {
        let a = run(4);
        assert_eq!(a, run(4));
        let rules = tot_kit::parse_rules(RULES);
        Game::replay("s.sav", &synthetic_save_bytes(), rules, None, &a).unwrap();
        assert!(
            a.contains("= prod-set") || a.contains("= moved"),
            "the AI acts"
        );
    }
}
