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
