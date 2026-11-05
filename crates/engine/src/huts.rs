//! Goody-hut outcomes (#79, v0.6.0), decoded in part from `civ2.bak`
//! `0x54ab80` (`hut_outcome(unit)`; the dialog keys `SURPRISE*` are
//! pushed by its cases). All draws come from the CRT `rand()`, the
//! engine's [`crate::combat::CrtRand`].
//!
//! Mercenary selection, metals, tribe size/buildings and site-value gates
//! execute against original-x86 vectors in `tools/huts_check.py`.
//! Remaining open: landmass-cache selection, nomad leader types, scroll
//! redirects and barbarian spawning (see the v0.8 readiness record).

use tot_kit::Rules;

use crate::combat::CrtRand;
use crate::founding::ROLE_SETTLE;
use crate::research::TechTree;
use crate::{Unit, WorldState};

/// Fixed mercenary candidates, based on global tech ownership rather than
/// the entering civ's buildable units (`0x54bae5..0x54bd12`).
fn mercenary(rng: &mut CrtRand, known: &[bool; 100], mode: u8, map: u8) -> usize {
    let fantasy = mode == 2;
    let mut a = 19;
    if !known[11] {
        a = if rng.rand().is_multiple_of(3) {
            if fantasy {
                4
            } else {
                16
            }
        } else if fantasy {
            21
        } else {
            15
        };
        if known[64] && !rng.rand().is_multiple_of(2) {
            a = 17;
        }
    }
    if known[55] {
        a = if fantasy { 16 } else { 18 };
    }
    if known[42] {
        a = 20;
    }
    if known[17] {
        a = if fantasy { 38 } else { 11 };
    }
    let mut b = 8;
    if !known[34] {
        b = if fantasy { 18 } else { 7 };
    }
    if !known[35] {
        b = if fantasy { 14 } else { 5 };
    }
    if !known[39] {
        b = if fantasy { 9 } else { 4 };
    }
    if fantasy {
        match map {
            1 => {
                a = 49;
                b = 3;
            }
            2 => {
                a = 17;
                b = 51;
            }
            _ => {}
        }
    }
    if rng.rand().is_multiple_of(2) {
        a
    } else {
        b
    }
}

fn metals(rng: &mut CrtRand, difficulty: u8, year: i16) -> i32 {
    let mut gold = 50;
    if rng.rand().is_multiple_of(3) {
        gold = if (rng.rand() % 10) as i32 - difficulty as i32 + 2 < 5 {
            25
        } else {
            100
        };
    }
    if year > 1000 {
        gold *= 2;
    }
    gold
}

/// The post-founding tribe additions (`0x54af81..0x54b03c`).
fn tribe(rng: &mut CrtRand, year: i16) -> (u8, [u8; 5]) {
    let mut buildings = [0; 5];
    let size = if year >= 1000 {
        let size = (rng.rand() % 4 + 1) as u8;
        buildings[0] |= 1 << 4;
        for (building, divisor) in [(5, 2), (3, 3), (6, 4)] {
            if rng.rand().is_multiple_of(divisor) {
                buildings[0] |= 1 << building;
            }
        }
        size
    } else {
        1
    };
    (size, buildings)
}

fn site_value(bits: u8, fertility: u8) -> u8 {
    if bits != 0 {
        bits
    } else if fertility != 0 {
        fertility.max(8)
    } else {
        0
    }
}

/// What entering a hut gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HutOutcome {
    Nothing,
    Gold(i32),
    Advance(usize),
    /// A unit (its index).
    Unit(usize),
    /// A city (its index).
    City(usize),
}

impl WorldState {
    fn nearest_city_distance(&self, x: u16, y: u16) -> i32 {
        let w2 = 2 * self.map.stored_w as i32;
        self.cities
            .iter()
            .filter(|c| c.city_id != 0 && c.map_id == 0)
            .map(|c| {
                crate::placement::diamond_distance(
                    w2,
                    self.game_flags & 0x8000 != 0,
                    c.x as i32,
                    c.y as i32,
                    x as i32,
                    y as i32,
                )
            })
            .min()
            .unwrap_or(99)
    }

    fn spawn_unit(&mut self, rules: &Rules, t: usize, civ: u8, x: u16, y: u16) -> usize {
        let id = self.units.iter().map(|u| u.id).max().unwrap_or(0) + 1;
        self.units.push(Unit {
            type_id: t as u8,
            owner: civ,
            x,
            y,
            is_leader: false,
            moves_left: 0,
            veteran: false,
            damage: 0,
            orders: 0xff,
            stack_pos: 0,
            home: 0xff,
            id,
        });
        let _ = rules;
        self.units.len() - 1
    }

    /// The outcome of `unit` entering a hut on its tile (see module docs).
    pub fn enter_hut(&mut self, rules: &Rules, unit: usize) -> HutOutcome {
        let Some(u) = self.units.get(unit).cloned() else {
            return HutOutcome::Nothing;
        };
        let civ = u.owner;
        if civ == 0 || rules.unit_domain(u.type_id as usize).unwrap_or(0) != 0 {
            return HutOutcome::Nothing;
        }
        let near = self.nearest_city_distance(u.x, u.y);
        let mut o = (self.combat_rng.rand() % 5) as i32;
        // the human civ's rand() % 3 is drawn (it gates the landmass rule,
        // which is not modelled: module docs)
        if self.human_mask >> civ & 1 != 0 {
            let _ = self.combat_rng.rand();
        }
        let own_cities = self.city_counts[civ as usize];
        let mut first_scroll_miss = true;
        for _ in 0..16 {
            match o {
                0 => {
                    let unhomed_settlers = self
                        .units
                        .iter()
                        .filter(|v| {
                            v.id != 0
                                && v.owner == civ
                                && v.home == 0xff
                                && rules.unit_role(v.type_id as usize) == Some(ROLE_SETTLE)
                        })
                        .count();
                    if near < 4 || (own_cities == 0 && self.turn < 50 && unhomed_settlers > 1) {
                        o = 1;
                        continue;
                    }
                    if self.map.stored_index(u.x, u.y).is_none_or(|i| {
                        site_value(self.map.site_bits[i], self.map.fertility[i]) < 12
                    }) {
                        o = 5;
                        continue;
                    }
                    let c = self.create_city(rules, civ, u.x, u.y);
                    let (size, buildings) = tribe(&mut self.combat_rng, self.year);
                    self.cities[c].size = size as u16;
                    for (a, b) in self.cities[c].buildings.iter_mut().zip(buildings) {
                        *a |= b;
                    }
                    return HutOutcome::City(c);
                }
                1 => {
                    let known = std::array::from_fn(|t| {
                        self.techs.iter().any(|b| b[t / 8] & (1 << (t % 8)) != 0)
                    });
                    let t = mercenary(&mut self.combat_rng, &known, self.game_mode, 0);
                    return if t < rules.unit_names().len() {
                        HutOutcome::Unit(self.spawn_unit(rules, t, civ, u.x, u.y))
                    } else {
                        HutOutcome::Nothing
                    };
                }
                2 => {
                    let g = metals(&mut self.combat_rng, self.difficulty, self.year);
                    let t = &mut self.treasury[civ as usize];
                    *t = t.wrapping_add(g);
                    return HutOutcome::Gold(g);
                }
                3 => return HutOutcome::Nothing,
                4 => {
                    let tree = TechTree::from_rules(rules);
                    let known: Vec<bool> = (0..tree.techs.len())
                        .map(|k| {
                            self.techs
                                .get(civ as usize)
                                .and_then(|b| b.get(k >> 3))
                                .is_some_and(|b| b >> (k & 7) & 1 != 0)
                        })
                        .collect();
                    if self.turn == 0 || known.get(0x26).copied().unwrap_or(false) {
                        o = 2;
                        continue;
                    }
                    let mut t = (self.combat_rng.rand() % 100) as usize;
                    let mut found = None;
                    for _ in 0..999 {
                        if tree.can_research(t, &known) {
                            found = Some(t);
                            break;
                        }
                        t = (t + 1) % 100;
                    }
                    match found {
                        Some(t) => {
                            self.grant_advance(civ, t);
                            return HutOutcome::Advance(t);
                        }
                        None => {
                            o = if first_scroll_miss { 0 } else { 2 };
                            first_scroll_miss = false;
                        }
                    }
                }
                _ => {
                    let t = (0..rules.unit_names().len())
                        .find(|&t| rules.unit_role(t) == Some(ROLE_SETTLE));
                    return match t {
                        Some(t) => HutOutcome::Unit(self.spawn_unit(rules, t, civ, u.x, u.y)),
                        None => HutOutcome::Nothing,
                    };
                }
            }
        }
        HutOutcome::Nothing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::CrtRand;
    use crate::tests_support::synthetic_save;

    const RULES: &str = "@COSMIC\n3 ; road\n2 ; trireme\n2 ; food\n10 ; food rows\n\
10 ; shield rows\n@CIVILIZE\nAlphabet, 5, 1, nil, nil, 0, 3 ; Alp\n\
Writing, 5, 1, Alp, nil, 0, 3 ; Wri\n\
@UNITS\nSettlers, nil, 0, 1., 0, 0a, 1d, 1h, 1f, 3, 0, 5, nil, 000000000000000\n\
Warriors, nil, 0, 1., 0, 1a, 1d, 1h, 1f, 1, 0, 1, nil, 000000000000000\n\
@TERRAIN\nDesert, 1, 2, 0, 1, 0\nPlains, 1, 2, 1, 1, 0\nGrassland, 1, 2, 2, 0, 0\n";

    fn outcome(seed: u32) -> (HutOutcome, i32, [u8; 13]) {
        let rules = tot_kit::parse_rules(RULES);
        let mut w = crate::boot(&synthetic_save(), 1);
        w.turn = 10;
        w.human_mask = 0;
        w.combat_rng = CrtRand::new(seed);
        let u = w.units.iter().position(|u| u.owner != 0).unwrap();
        let civ = w.units[u].owner as usize;
        let before = w.treasury[civ];
        let o = w.enter_hut(&rules, u);
        (o, w.treasury[civ] - before, w.techs[civ])
    }

    #[test]
    fn original_outcome_vectors_and_rng_state() {
        let mut count = 0;
        for line in include_str!("../../../docs/evidence/hut-outcome-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let v: Vec<i64> = line
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            let mut rng = CrtRand::new(v[1] as u32);
            let (a, b) = match v[0] {
                0 => {
                    let mut known = [false; 100];
                    for (i, t) in [11, 64, 55, 42, 17, 34, 35, 39].into_iter().enumerate() {
                        known[t] = v[4] & (1 << i) != 0;
                    }
                    (
                        mercenary(&mut rng, &known, v[2] as u8, v[3] as u8) as i64,
                        0,
                    )
                }
                1 => (metals(&mut rng, v[6] as u8, v[5] as i16) as i64, 0),
                2 => {
                    let (size, buildings) = tribe(&mut rng, v[5] as i16);
                    (size as i64, buildings[0] as i64)
                }
                3 => (site_value(v[3] as u8, v[4] as u8) as i64, 0),
                _ => panic!("unexpected case"),
            };
            assert_eq!((a, b, rng.state() as i64), (v[7], v[8], v[9]), "{line}");
            count += 1;
        }
        assert_eq!(count, 896);
    }

    #[test]
    fn tribe_creates_city_without_creating_or_consuming_explorer() {
        let rules = tot_kit::parse_rules(RULES);
        let mut w = crate::boot(&synthetic_save(), 1);
        w.cities.clear();
        w.city_counts = [0; 8];
        w.map.fertility.fill(15);
        w.map.site_bits.fill(0);
        w.map.terrain.fill(1);
        w.techs = [[0; 13]; 8];
        w.human_mask = 0;
        w.combat_rng = CrtRand::new(2); // first draw selects tribe
        let u = w.units.iter().position(|u| u.owner != 0).unwrap();
        w.units[u].x = 0;
        w.units[u].y = 0;
        let explorer = w.units[u].clone();
        let units = w.units.len();
        assert_eq!(w.enter_hut(&rules, u), HutOutcome::City(0));
        assert_eq!(w.units.len(), units);
        assert_eq!(w.units[u], explorer);
        assert_eq!(w.city_counts[explorer.owner as usize], 1);
    }

    /// The same generator state gives the same outcome; metals pay 25, 50
    /// or 100 and land in the treasury; scrolls grant a researchable
    /// advance the civ lacked.
    #[test]
    fn outcomes_follow_the_decoded_rules() {
        let mut kinds = std::collections::HashSet::new();
        for seed in 0..200u32 {
            let (o, gold, techs) = outcome(seed);
            assert_eq!(outcome(seed).0, o, "deterministic");
            match o {
                HutOutcome::Gold(g) => {
                    assert!([25, 50, 100].contains(&g));
                    assert_eq!(gold, g);
                    kinds.insert("gold");
                }
                HutOutcome::Advance(t) => {
                    assert!(t <= 1 && techs[0] >> t & 1 == 1);
                    kinds.insert("advance");
                }
                HutOutcome::Unit(_) => {
                    kinds.insert("unit");
                }
                HutOutcome::City(_) | HutOutcome::Nothing => {
                    kinds.insert("other");
                }
            }
        }
        assert!(
            kinds.contains("gold") && kinds.contains("advance"),
            "{kinds:?}"
        );
    }
}
