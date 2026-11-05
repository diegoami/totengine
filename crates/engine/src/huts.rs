//! Goody-hut outcomes (#79, v0.6.0), decoded in part from `civ2.bak`
//! `0x54ab80` (`hut_outcome(unit)`; the dialog keys `SURPRISE*` are
//! pushed by its cases). All draws come from the CRT `rand()`, the
//! engine's [`crate::combat::CrtRand`].
//!
//! Mercenary selection, metals, tribe size/buildings and site-value gates
//! execute against original-x86 vectors in `tools/huts_check.py`.
//! Landmass-cache selection, nomad leader types, scroll redirects and
//! surface barbarian spawning are implemented; full entry parity and
//! cache refresh are still being checked (see the v0.8 readiness record).

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

fn selection(
    rng: &mut CrtRand,
    human: bool,
    cities: i16,
    landmass_cities: u8,
    flags: u8,
    near: i32,
) -> i32 {
    let rolled = (rng.rand() % 5) as i32;
    if human && !rng.rand().is_multiple_of(3) {
        return rolled;
    }
    if cities != 0 && landmass_cities == 0 && (flags & 127 == 0 || near > 12) {
        0
    } else {
        rolled
    }
}

fn nomad_redirect(cities: i16, unhomed: i32, known_28: bool) -> i32 {
    if known_28 {
        2
    } else if unhomed > (cities >> 3) as i32 {
        4
    } else {
        5
    }
}

struct BarbarianContext {
    near: i32,
    cities: i16,
    turn: i16,
    human: bool,
    difficulty: u8,
    level: u8,
    reference_rank: u8,
    rank: u8,
    landmass_cities: u8,
    own_near: i32,
}

/// Dispatch decision and early one-unit flag (`0x54b520..0x54b704`).
/// -1 is no reward; 0/1 redirect to tribe/mercenary; 3 spawns barbarians.
fn barbarian_gate(rng: &mut CrtRand, c: BarbarianContext) -> (i32, bool) {
    if c.near < 4 || (c.cities == 0 && c.turn < 50) {
        return (1, false);
    }
    if !c.human && c.reference_rank > c.rank && rng.rand() % 8 < c.difficulty as u32 {
        return (0, false);
    }
    if c.level < 3
        && (c.turn < 30 || (c.turn < 50 && rng.rand().is_multiple_of(2)))
        && c.landmass_cities != 0
        && c.own_near < 24
    {
        return (-1, false);
    }
    (
        3,
        c.turn < 50 || (c.turn < 75 && rng.rand().is_multiple_of(2)),
    )
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
    Barbarians(Vec<usize>),
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
            moves_left: rules.unit_move(t).unwrap_or(0) * 3,
            veteran: false,
            damage: 0,
            orders: 0xff,
            stack_pos: 0,
            home: 0xff,
            id,
        });
        self.units.len() - 1
    }

    /// Scrolls use the original tech-group gate (`0x4a2098`) in addition
    /// to prerequisites. Group-less advances and zero leader flags pass.
    fn hut_tech_group_allowed(&self, rules: &Rules, civ: u8, tech: usize) -> bool {
        self.tech_group_flag(rules, civ, tech) == 0
    }

    /// Barbarian ring (`0x54b704..0x54babd`). Directions begin at turn%8;
    /// few-city civs get a larger step. Impassable, water and occupied
    /// neighbours are skipped; early huts stop after the first spawn.
    fn spawn_hut_barbarians(
        &mut self,
        rules: &Rules,
        civ: u8,
        x: u16,
        y: u16,
        single: bool,
    ) -> Vec<usize> {
        let known: [bool; 100] =
            std::array::from_fn(|t| self.techs.iter().any(|b| b[t / 8] & (1 << (t % 8)) != 0));
        let tree = TechTree::from_rules(rules);
        let armor = rules
            .unit_prereq_raw(22)
            .and_then(|p| tree.by_abbrev(p))
            .is_some_and(|t| known.get(t).copied().unwrap_or(false));
        let fantasy = self.game_mode == 2;
        let (mut infantry, mut cavalry) = if fantasy { (14, 21) } else { (5, 15) };
        if known[51] {
            infantry = if fantasy { 18 } else { 7 };
            cavalry = 19;
        }
        if armor {
            infantry = if fantasy { 38 } else { 11 };
            cavalry = if fantasy { 20 } else { 21 };
        }
        if known[34] {
            infantry = 8;
            cavalry = if fantasy { 40 } else { 9 };
        }
        let mut spawned = Vec::new();
        let step = (4 - self.city_counts[civ as usize] as i32).clamp(1, 4) as usize;
        for n in (0..8).step_by(step) {
            let k = ((self.turn as i32 + n).rem_euclid(8)) as usize;
            let (dx, dy) = [
                (1, -1),
                (2, 0),
                (1, 1),
                (0, 2),
                (-1, 1),
                (-2, 0),
                (-1, -1),
                (0, -2),
            ][k];
            let w2 = 2 * self.map.stored_w as i32;
            let nx = if self.game_flags & 0x8000 == 0 {
                (x as i32 + dx).rem_euclid(w2.max(1))
            } else {
                x as i32 + dx
            };
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 {
                continue;
            }
            let (nx, ny) = (nx as u16, ny as u16);
            let Some(i) = self.map.stored_index(nx, ny) else {
                continue;
            };
            let terrain = self.map.terrain[i] as usize;
            let impassable = rules
                .section("TERRAIN")
                .and_then(|s| s.rows.get(terrain))
                .and_then(|r| r.get(15))
                .is_some_and(|v| v.eq_ignore_ascii_case("yes"));
            if impassable
                || terrain == crate::movement::OCEAN_TERRAIN as usize
                || self
                    .units
                    .iter()
                    .any(|v| v.id != 0 && v.x == nx && v.y == ny)
                || self
                    .cities
                    .iter()
                    .any(|c| c.city_id != 0 && c.map_id == 0 && c.x == nx && c.y == ny)
            {
                continue;
            }
            let t = if rules.terrain_move_cost(terrain).unwrap_or(1) > 2 {
                infantry
            } else {
                cavalry
            };
            if t >= rules.unit_names().len() {
                continue;
            }
            spawned.push(self.spawn_unit(rules, t, 0, nx, ny));
            if single {
                break;
            }
        }
        spawned
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
        let own_cities = self.city_counts[civ as usize];
        let landmass = self
            .map
            .stored_index(u.x, u.y)
            .map_or(0, |i| self.map.landmass[i]) as usize;
        let mut o = selection(
            &mut self.combat_rng,
            self.human_mask & (1 << civ) != 0,
            own_cities,
            self.ai_state.city_landmasses[civ as usize][landmass],
            self.ai_state.landmass_flags[civ as usize][landmass],
            near,
        );
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
                3 => {
                    let ranks = &self.ai_state.power_ranks;
                    let own_near = self
                        .cities
                        .iter()
                        .filter(|c| c.city_id != 0 && c.map_id == 0 && c.owner & 15 == civ as u16)
                        .map(|c| {
                            crate::placement::diamond_distance(
                                2 * self.map.stored_w as i32,
                                self.game_flags & 0x8000 != 0,
                                u.x as i32,
                                u.y as i32,
                                c.x as i32,
                                c.y as i32,
                            )
                        })
                        .min()
                        .unwrap_or(9999);
                    let (redirect, single) = barbarian_gate(
                        &mut self.combat_rng,
                        BarbarianContext {
                            near,
                            cities: own_cities,
                            turn: self.turn,
                            human: self.human_mask & (1 << civ) != 0,
                            difficulty: self.difficulty,
                            level: (self.seed_word & 255) as u8,
                            reference_rank: ranks
                                .get(self.ai_state.power_reference as usize)
                                .copied()
                                .unwrap_or(0),
                            rank: ranks[civ as usize],
                            landmass_cities: self.ai_state.city_landmasses[civ as usize][landmass],
                            own_near,
                        },
                    );
                    if redirect == -1 {
                        return HutOutcome::Nothing;
                    }
                    if redirect != 3 {
                        o = redirect;
                        continue;
                    }
                    let spawned = self.spawn_hut_barbarians(rules, civ, u.x, u.y, single);
                    return if spawned.is_empty() {
                        HutOutcome::Nothing
                    } else {
                        HutOutcome::Barbarians(spawned)
                    };
                }
                4 => {
                    if self.turn == 0 || self.techs[civ as usize][38 / 8] & (1 << (38 % 8)) != 0 {
                        o = 2;
                        continue;
                    }
                    let available = self.observe_research(rules, civ).available;
                    let mut t = (self.combat_rng.rand() % 100) as usize;
                    let mut found = None;
                    for _ in 0..999 {
                        if available.contains(&t) && self.hut_tech_group_allowed(rules, civ, t) {
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
                    let known_28 = self.techs.iter().any(|b| b[28 / 8] & (1 << (28 % 8)) != 0);
                    let unhomed = self
                        .units
                        .iter()
                        .filter(|v| {
                            v.id != 0
                                && v.owner == civ
                                && v.home == 0xff
                                && rules.unit_role(v.type_id as usize) == Some(ROLE_SETTLE)
                        })
                        .count() as i32;
                    let redirect = nomad_redirect(own_cities, unhomed, known_28);
                    if redirect != 5 {
                        o = redirect;
                        continue;
                    }
                    let leader = self.ai_state.leader_ids[civ as usize];
                    let t = usize::try_from(leader)
                        .ok()
                        .and_then(|i| {
                            rules
                                .section("INITIAL_SETTINGS")
                                .and_then(|s| s.rows.get(i))
                        })
                        .and_then(|r| r.get(1))
                        .and_then(|v| v.parse::<usize>().ok())
                        .unwrap_or(0);
                    return if t < rules.unit_names().len() {
                        HutOutcome::Unit(self.spawn_unit(rules, t, civ, u.x, u.y))
                    } else {
                        HutOutcome::Nothing
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
    fn original_dispatch_and_nomad_redirects() {
        for line in include_str!("../../../docs/evidence/hut-dispatch-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let v: Vec<i64> = line
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            let mut rng = CrtRand::new(v[0] as u32);
            let outcome = selection(
                &mut rng,
                v[1] != 0,
                v[2] as i16,
                v[3] as u8,
                v[4] as u8,
                v[5] as i32,
            );
            assert_eq!((outcome, rng.state()), (v[6] as i32, v[7] as u32), "{line}");
        }
        for line in include_str!("../../../docs/evidence/hut-nomad-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let v: Vec<i32> = line
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            assert_eq!(nomad_redirect(v[0] as i16, v[1], v[2] != 0), v[3], "{line}");
        }
    }

    #[test]
    fn original_barbarian_spawn_requests() {
        use std::fmt::Write;
        let mut text = String::from("@CIVILIZE\n");
        for t in 0..100 {
            writeln!(text, "Tech{t},5,1,nil,nil,0,0 ; T{t}").unwrap();
        }
        text.push_str("@UNITS\n");
        for t in 0..80 {
            let prereq = if t == 22 { "T50" } else { "nil" };
            writeln!(
                text,
                "Unit{t},nil,0,1.,0,1a,1d,1h,1f,1,0,1,{prereq},000000000000000"
            )
            .unwrap();
        }
        text.push_str("@TERRAIN\n");
        for t in 0..11 {
            writeln!(
                text,
                "Terrain{t},{},0,0,0,0,0,0,0,0,0,0,0,0,0,{}",
                if t == 2 { 3 } else { 1 },
                if t == 3 { "yes" } else { "no" }
            )
            .unwrap();
        }
        let rules = tot_kit::parse_rules(&text);
        let mut count = 0;
        for line in include_str!("../../../docs/evidence/hut-barbarian-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let v: Vec<i32> = line
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            let mut w = crate::boot(&synthetic_save(), 1);
            let mut blocker = w.units[0].clone();
            w.units.clear();
            w.cities.clear();
            w.map = crate::MapState {
                stored_w: 8,
                stored_h: 16,
                terrain: vec![1; 128],
                landmass: vec![1; 128],
                site_bits: vec![0; 128],
                fertility: vec![0; 128],
                special: vec![false; 128],
                river: vec![false; 128],
                road: vec![false; 128],
                rail: vec![false; 128],
                improvements: vec![0; 128],
                visible: vec![0; 128],
                owner: vec![15; 128],
                resource_seed: 0,
                huts_enabled: false,
            };
            w.turn = v[1] as i16;
            w.game_mode = v[3] as u8;
            w.game_flags = if v[9] != 0 { 0x8000 } else { 0 };
            w.city_counts[1] = v[0] as i16;
            w.techs = [[0; 13]; 8];
            for (i, t) in [51, 50, 34].into_iter().enumerate() {
                if v[4] & (1 << i) != 0 {
                    w.techs[1][t / 8] |= 1 << (t % 8);
                }
            }
            let (x, y) = (v[10], v[11]);
            for (k, (dx, dy)) in [
                (1, -1),
                (2, 0),
                (1, 1),
                (0, 2),
                (-1, 1),
                (-2, 0),
                (-1, -1),
                (0, -2),
            ]
            .into_iter()
            .enumerate()
            {
                let nx = if v[9] != 0 {
                    x + dx
                } else {
                    (x + dx).rem_euclid(16)
                };
                let ny = y + dy;
                if nx < 0 || ny < 0 {
                    continue;
                }
                let Some(i) = w.map.stored_index(nx as u16, ny as u16) else {
                    continue;
                };
                let bit = 1 << k;
                w.map.terrain[i] = if v[8] & bit != 0 {
                    3
                } else if v[7] & bit != 0 {
                    10
                } else if v[6] & bit != 0 {
                    2
                } else {
                    1
                };
                if v[5] & bit != 0 {
                    blocker.x = nx as u16;
                    blocker.y = ny as u16;
                    blocker.id = k as u16 + 1;
                    w.units.push(blocker.clone());
                }
            }
            let ids = w.spawn_hut_barbarians(&rules, 1, x as u16, y as u16, v[2] != 0);
            let got: Vec<(i32, i32, i32)> = ids
                .into_iter()
                .map(|i| {
                    let u = &w.units[i];
                    assert_eq!(u.owner, 0);
                    assert_eq!(u.moves_left, 3);
                    (u.type_id as i32, u.x as i32, u.y as i32)
                })
                .collect();
            let expected: Vec<_> = v[13..]
                .as_chunks::<3>()
                .0
                .iter()
                .map(|r| (r[0], r[1], r[2]))
                .collect();
            assert_eq!(got.len(), v[12] as usize, "{line}");
            assert_eq!(got, expected, "{line}");
            count += 1;
        }
        assert_eq!(count, 128);
    }

    #[test]
    fn original_barbarian_gates_and_rng_state() {
        let mut count = 0;
        for line in include_str!("../../../docs/evidence/hut-barbarian-gate-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let v: Vec<i64> = line
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            let mut rng = CrtRand::new(v[0] as u32);
            let (outcome, single) = barbarian_gate(
                &mut rng,
                BarbarianContext {
                    near: v[1] as i32,
                    cities: v[2] as i16,
                    turn: v[3] as i16,
                    human: v[4] != 0,
                    difficulty: v[5] as u8,
                    level: v[6] as u8,
                    reference_rank: v[7] as u8,
                    rank: v[8] as u8,
                    landmass_cities: v[9] as u8,
                    own_near: if v[10] != 0 { 32 } else { 8 },
                },
            );
            assert_eq!(
                (outcome, single, rng.state()),
                (v[11] as i32, v[12] != 0, v[13] as u32),
                "{line}"
            );
            count += 1;
        }
        assert_eq!(count, 256);
    }

    #[test]
    fn nomads_follow_leader_settings_and_scrolls_skip_disabled_advances() {
        let mut w = crate::boot(&synthetic_save(), 1);
        let mut explorer = w.units[0].clone();
        explorer.owner = 1;
        explorer.type_id = 1;
        explorer.id = 1;
        explorer.x = 0;
        explorer.y = 0;
        w.units = vec![explorer];
        w.cities.clear();
        w.city_counts = [0; 8];
        w.map.fertility.fill(0);
        w.map.site_bits.fill(0);
        w.map.terrain.fill(1);
        w.techs = [[0; 13]; 8];
        w.ai_state = tot_kit::SavedAiState::default();
        w.human_mask = 0;
        w.combat_rng = CrtRand::new(2);
        let rules = tot_kit::parse_rules(&format!("{RULES}\n@INITIAL_SETTINGS\n0,1,3\n"));
        assert_eq!(w.enter_hut(&rules, 0), HutOutcome::Unit(1));
        assert_eq!(
            (w.units[1].type_id, w.units[1].moves_left, w.units[1].home),
            (1, 3, 255)
        );
        let rules = tot_kit::parse_rules("@CIVILIZE\nDisabled,5,1,no,nil,0,0 ; Dis\nLegal,5,1,nil,nil,0,0 ; Leg\n@CIVILIZE2\n2\n-1\n@LEADERS2\n00100000\n");
        assert!(!w.hut_tech_group_allowed(&rules, 1, 0));
        assert!(w.hut_tech_group_allowed(&rules, 1, 1));
        w.turn = 10;
        let seed = (0..1000)
            .find(|&s| CrtRand::new(s).rand() % 5 == 4)
            .unwrap();
        w.combat_rng = CrtRand::new(seed);
        assert_eq!(w.enter_hut(&rules, 0), HutOutcome::Advance(1));
        assert_eq!(w.techs[1][0] & 3, 2);
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
                HutOutcome::City(_) | HutOutcome::Nothing | HutOutcome::Barbarians(_) => {
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
