//! Goody-hut outcomes (#79, v0.6.0), decoded in part from `civ2.bak`
//! `0x54ab80` (`hut_outcome(unit)`; the dialog keys `SURPRISE*` are
//! pushed by its cases). All draws come from the CRT `rand()`, the
//! engine's [`crate::combat::CrtRand`].
//!
//! **Decoded:**
//! - **No outcome** for barbarian units, or for non-land units except a
//!   scenario-specific air case (not modelled).
//! - **Selection:** `o = rand() % 5`. For a human civ, `rand() % 3`; a
//!   nonzero roll keeps `o`. Otherwise, and always for an AI civ, `o`
//!   becomes 0 when the civ has cities but no claim on this landmass. That
//!   test reads per-landmass civ bytes (civ +0x5D0/+0x8D0) that the engine
//!   does not keep: **the engine draws the `% 3` but never forces 0.**
//! - **Dispatch** (`0x54bdbf`), with redirects:
//!
//!   | o | Outcome |
//!   |---|---|
//!   | 0 | tribe → 1 if a city is within distance 4, or if the civ has no city before turn 50 and over one unhomed settler; → 5 if the site value < 12 (not modelled: no tile fertility in the engine map) |
//!   | 1 | mercenaries |
//!   | 2 | metals |
//!   | 3 | barbarians or nothing |
//!   | 4 | scrolls → 2 on turn 0 or once the civ knows advance 0x26 |
//!   | 5 | nomads |
//!
//! - **Metals:** 50 gold. With `rand() % 3 == 0`, then
//!   `r = rand() % 10`: 25 gold if `r − difficulty + 2 < 5`, else 100.
//!   The amount doubles while the global `0x60764a` exceeds 1000 (not
//!   modelled).
//! - **Scrolls:**
//!   - `t = rand() % 100`;
//!   - scan up to 999 steps for the first advance the civ can research
//!     (prerequisites known) and lacks;
//!   - grant it, else re-dispatch (first to 0, then to 2).
//!
//! **Simplified, flagged** (the cases are long and only partly read):
//! - tribe founds a size-1 city on the tile; the original also rolls
//!   size and buildings when `0x60764a` ≥ 1000;
//! - mercenaries give the civ's best defender;
//! - nomads give the first settler-role unit;
//! - case 3 gives nothing. Barbarian spawning goes with RG-9 (#83).
//!
//! Oracle check: one hut entry, metals 100 gold (evidence §RG-2 goody
//! huts). A same-seed replay of several outcomes is still open.

use tot_kit::Rules;

use crate::founding::ROLE_SETTLE;
use crate::research::TechTree;
use crate::{Unit, WorldState};

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
            .map(|c| {
                crate::placement::diamond_distance(
                    w2, false, c.x as i32, c.y as i32, x as i32, y as i32,
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
        let own_cities = self
            .cities
            .iter()
            .filter(|c| (c.owner & 0x0F) as u8 == civ)
            .count();
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
                    // (the site-value < 12 → nomads redirect needs tile
                    // fertility, not kept in MapState: not modelled)
                    // the tribe city reuses founding with a temporary settler
                    let settler = (0..rules.unit_names().len())
                        .find(|&t| rules.unit_role(t) == Some(ROLE_SETTLE));
                    if let Some(t) = settler {
                        let s = self.spawn_unit(rules, t, civ, u.x, u.y);
                        match self.found_city(rules, s) {
                            Ok(c) => return HutOutcome::City(c),
                            Err(_) => {
                                self.units[s].id = 0;
                                o = 5;
                                continue;
                            }
                        }
                    }
                    return HutOutcome::Nothing;
                }
                1 => {
                    let t = self.initial_build(rules, civ) as usize;
                    return HutOutcome::Unit(self.spawn_unit(rules, t, civ, u.x, u.y));
                }
                2 => {
                    let mut g = 50;
                    if self.combat_rng.rand().is_multiple_of(3) {
                        let r = (self.combat_rng.rand() % 10) as i32;
                        if r - self.difficulty as i32 + 2 < 5 {
                            g >>= 1;
                        } else {
                            g <<= 1;
                        }
                    }
                    let t = &mut self.treasury[civ as usize];
                    *t = t.saturating_add(g);
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
