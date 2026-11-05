//! City production (T1-4, v0.4.0): shields fill the box and units or
//! improvements complete. Decoded from `civ2.bak` (`0x4c6210`
//! production step, `0x4c1bdf` shield rows); evidence §T1-4 production.
//!
//! Per city and turn:
//! - **Net shields** = [`WorldState::city_output`] shields − unit support.
//!   0 while the city is in disorder (flag `0x1`), never negative. Not
//!   added while building Capitalization (item −39).
//! - **Cost** = Rules cost × the civ's shield rows. Human civs use the
//!   @COSMIC shield rows (row 4). AI civs use `13 − difficulty`, +1 at
//!   difficulty 0, +1 below 3, scaled by the @COSMIC rows / 10 and rounded
//!   up to even only when the @COSMIC rows differ from 10.
//! - **Completion:** the box drops by the cost, then the unit is created
//!   in the city (homed there) or the improvement is set.
//!
//! - **Obsolete units** (#77, `0x4c63b0`..`0x4c6532`): when the civ knows
//!   the advance in the unit's "until" field, the build switches to the
//!   first unit whose prerequisite is that advance and whose role is the
//!   same, subject to the @UNITS_ADVANCED civilization mask.
//! - **Veterans** (#78, `0x4c6728`..`0x4c68aa`): the new unit is veteran
//!   (unit attr `0x2000`) when:
//!   - a land unit (or any unit while game option `0x10` is off) of role
//!     ≤ 5 comes from a city with Barracks (2), or its civ holds wonder 7;
//!   - a sea unit (option `0x10` on) comes from a city with a Port
//!     Facility (34), or its civ holds wonder 3;
//!   - an air unit (option `0x10` on) comes from a city with an Airport
//!     (32).
//!
//!   Type flag `0x1000` (`+5 & 0x10`, the one-shot flag) exempts units
//!   in every domain; 512 native eligibility cases verify this path.
//!
//! Not modelled yet:
//! - the Settlers-at-size-1 case at Chieftain (and the population cost:
//!   the engine's rule is in `founding.rs`);
//! - what an AI chooses next (the item stays).

use tot_kit::Rules;

use crate::{Unit, WorldState};

/// What a production step completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Built {
    /// A unit; its index in [`WorldState::units`].
    Unit(usize),
    /// An improvement id.
    Improvement(u8),
    /// A wonder slot (improvement ID minus 40).
    Wonder(u8),
}

const CAPITALIZATION: i8 = -39;
const FIRST_WONDER: usize = 40;

/// `0x4c1c2c..0x4c1dd3`: row arithmetic after unit-support evaluation.
/// The saved power ranking identifies the leader. Knowledge masks retain
/// the original's signed-byte extension, including its owner-7 behavior.
#[allow(clippy::too_many_arguments)] // Mirrors the independently executed arithmetic inputs.
pub fn production_rows(
    cosmic: i32,
    difficulty: u8,
    humans: u8,
    civ: u8,
    turn: i16,
    leader: u8,
    wonders: [i16; 2],
    masks: [u8; 3],
) -> i32 {
    if (humans as u32) & (1u32 << (civ & 31)) != 0 {
        return cosmic;
    }
    let d = difficulty as i32;
    let mut rows = 13 - d + (d < 3) as i32 + (d == 0) as i32;
    if turn >= 200 && d > 1 && (humans as u32) & (1u32 << (leader & 31)) != 0 {
        let [m73, m58, m76] = masks;
        let mut adjustment: i32 = 2;
        if wonders[0] != -1 && (m73 as i8 as i32) & !(humans as i32) != 0 {
            adjustment -= 1;
            if m73 & humans == 0 || m58 & humans == 0 {
                adjustment -= 1;
            }
        }
        if wonders[1] != -1 && (m76 as i8 as i32) & !(humans as i32) != 0 {
            adjustment -= 1;
            if m76 & humans == 0 {
                adjustment -= 1;
            }
        }
        rows -= adjustment.clamp(0, 2);
    }
    if cosmic != 10 {
        rows = cosmic * rows / 10;
        rows += rows & 1;
    }
    rows
}

impl WorldState {
    /// Shield-box rows of `civ` (`0x4c1bdf`).
    pub fn shield_rows(&self, rules: &Rules, civ: u8) -> i32 {
        let cosmic = rules.cosmic_row(4).unwrap_or(10) as i32;
        let mask = |t: usize| {
            (1..8).fold(0u8, |m, c| {
                m | (((self.techs[c][t / 8] >> (t % 8)) & 1) << c)
            })
        };
        production_rows(
            cosmic,
            self.difficulty,
            self.human_mask,
            civ,
            self.turn,
            self.power_order[7],
            [self.wonders[23], self.wonders[25]],
            [mask(73), mask(58), mask(76)],
        )
    }

    /// The decoded veteran rule (`0x4c6728`): see the module docs.
    fn made_veteran(&self, rules: &Rules, idx: usize, civ: u8, t: usize) -> bool {
        let Some(c) = self.cities.get(idx) else {
            return false;
        };
        if rules.unit_flags(t).unwrap_or(0) & crate::combat::FLAG_ONE_SHOT != 0 {
            return false;
        }
        let domain = rules.unit_domain(t).unwrap_or(0);
        let option = self.game_flags & 0x10 != 0;
        if domain == 0 || !option {
            (c.has_building(2) || self.has_active_wonder(rules, civ, 7))
                && rules.unit_role(t).unwrap_or(0) <= 5
        } else if domain == 2 {
            c.has_building(34) || self.has_active_wonder(rules, civ, 3)
        } else {
            c.has_building(32)
        }
    }

    /// The decoded obsolete-unit switch (`0x4c63b0`): see the module docs.
    pub fn upgraded_build(&self, rules: &Rules, civ: u8, t: usize) -> usize {
        let Some(until) = rules
            .unit_until_raw(t)
            .filter(|u| *u != "nil" && *u != "no")
        else {
            return t;
        };
        let tree = crate::research::TechTree::from_rules(rules);
        let Some(k) = tree.by_abbrev(until) else {
            return t;
        };
        let known = self
            .techs
            .get(civ as usize)
            .and_then(|b| b.get(k >> 3))
            .is_some_and(|b| b >> (k & 7) & 1 != 0);
        if !known {
            return t;
        }
        (0..rules.unit_names().len().min(80))
            .find(|&n| {
                rules.unit_civ_mask(n) & (1 << civ) != 0
                    && rules.unit_prereq_raw(n) == Some(until)
                    && rules.unit_role(n) == rules.unit_role(t)
            })
            .unwrap_or(t)
    }

    /// One turn of production in city `idx` (`0x4c6210`).
    pub fn production_step(&mut self, rules: &Rules, idx: usize) -> Option<Built> {
        let c = self.cities.get(idx)?;
        let civ = (c.owner & 0x0F) as u8;
        let item = c.building;
        let disorder = c.flags & 1 != 0;
        let net = if disorder {
            0
        } else {
            (self.city_output(rules, idx).shields - self.unit_support(rules, idx)).max(0)
        };
        let rows = self.shield_rows(rules, civ);
        let item = if item >= 0 {
            self.upgraded_build(rules, civ, item as usize) as i8
        } else {
            item
        };
        if let Some(c) = self.cities.get_mut(idx) {
            c.building = item;
        }
        let unit_slot = if item >= 0 {
            self.available_unit_slot(rules, civ, item as usize)
        } else {
            None
        };
        let c = &mut self.cities[idx];
        if item != CAPITALIZATION {
            c.shield_box = (c.shield_box as i32 + net).clamp(0, u16::MAX as i32) as u16;
        }
        let box_now = c.shield_box as i32;
        if item >= 0 {
            let t = item as usize;
            let cost = rules.unit_cost(t)? as i32 * rows;
            if box_now < cost {
                return None;
            }
            // A blocked allocation retains the completed box and population.
            unit_slot?;
            // engine rule (founding.rs): settlers cost one population; a
            // size-1 city keeps the box until it grows
            let settler = rules.unit_role(t) == Some(crate::founding::ROLE_SETTLE);
            if settler && c.size <= 1 {
                return None;
            }
            if settler {
                c.size -= 1;
            }
            c.flags &= !0x100;
            c.shield_box = (box_now - cost) as u16;
            let (x, y) = (c.x, c.y);
            let veteran = self.made_veteran(rules, idx, civ, t);
            let created = self.allocate_unit(
                rules,
                Unit {
                    type_id: t as u8,
                    owner: civ,
                    x,
                    y,
                    is_leader: false,
                    moves_left: self.movement_allowance(rules, t, civ, 0),
                    veteran,
                    damage: 0,
                    orders: 0xff,
                    stack_pos: 0,
                    home: idx as u8,
                    id: 0,
                },
            )?;
            Some(Built::Unit(created))
        } else {
            let id = -(item as i32) as usize;
            if id >= FIRST_WONDER {
                let wonder = id - FIRST_WONDER;
                if self.wonders.get(wonder) != Some(&-1) {
                    return None;
                }
                let cost = rules.improvement_cost(id)? as i32 * rows;
                if box_now < cost {
                    return None;
                }
                // 0x4c7916 and 0x4c7fa4: single-player registration and
                // the reset discard all accumulated shield overflow.
                self.wonders[wonder] = idx as i16;
                c.flags |= 0x100;
                c.shield_box = 0;
                self.wonder_completed(rules, civ, wonder);
                return Some(Built::Wonder(wonder as u8));
            }
            if c.has_building(id) {
                return None;
            }
            let cost = rules.improvement_cost(id)? as i32 * rows;
            if box_now < cost {
                return None;
            }
            // Human improvement completions pass through 0x4c7fa4;
            // ordinary AI improvements branch to 0x4c7ff5 and retain
            // their overflow. Wonders above always take the reset path.
            c.shield_box = if self.human_mask & (1 << civ) != 0 {
                0
            } else {
                (box_now - cost) as u16
            };
            c.flags &= !0x100;
            c.buildings[id >> 3] |= 1 << (id & 7);
            Some(Built::Improvement(id as u8))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_support::synthetic_save;

    const RULES: &str = "@COSMIC\n3 ; road\n2 ; trireme\n2 ; food\n10 ; food rows\n\
10 ; shield rows\n@UNITS\nSettlers, nil, 0, 1., 0, 0a, 1d, 1h, 1f, 3, 0, 5, nil, 000000000000000\n\
Warriors, nil, 0, 1., 0, 1a, 1d, 1h, 1f, 1, 0, 1, nil, 000000000000000\n\
@IMPROVE\nNothing, 0, 0, nil\nPalace, 10, 0, nil\nBarracks, 4, 1, nil\n";

    /// A human city building Warriors (cost 1 × 10 rows): a full box
    /// completes it, the unit is homed in the city, and the box keeps this
    /// turn's net shields; one short of the cost completes nothing.
    #[test]
    fn a_unit_completes_when_the_box_reaches_its_cost() {
        let rules = tot_kit::parse_rules(RULES);
        let mut w = crate::boot(&synthetic_save(), 1);
        w.human_mask = 0xFF;
        let net = (w.city_output(&rules, 1).shields - w.unit_support(&rules, 1)).max(0);
        let c = &mut w.cities[1];
        c.building = 1; // Warriors
        c.shield_box = 10;
        c.flags = 0;
        let before = w.units.len();
        assert_eq!(w.production_step(&rules, 1), Some(Built::Unit(before)));
        assert_eq!((w.units[before].home, w.units[before].owner), (1, 1));
        assert_eq!(w.cities[1].shield_box as i32, net);
        w.cities[1].shield_box = (9 - net).max(0) as u16;
        if net <= 9 {
            assert_eq!(w.production_step(&rules, 1), None);
        }
    }

    /// Barracks (cost 4 × 10) completes from a full box; Capitalization adds
    /// nothing; a city in disorder adds nothing.
    #[test]
    fn improvements_capitalization_and_disorder() {
        let rules = tot_kit::parse_rules(RULES);
        let mut w = crate::boot(&synthetic_save(), 1);
        w.human_mask = 0xFF;
        w.cities[2].building = -2;
        w.cities[2].shield_box = 40;
        assert_eq!(w.production_step(&rules, 2), Some(Built::Improvement(2)));
        assert!(w.cities[2].has_building(2));
        w.cities[3].building = -39;
        w.cities[3].shield_box = 5;
        w.production_step(&rules, 3);
        assert_eq!(w.cities[3].shield_box, 5);
        w.cities[4].building = -2;
        w.cities[4].flags = 1;
        w.cities[4].shield_box = 7;
        w.production_step(&rules, 4);
        assert_eq!(w.cities[4].shield_box, 7);
    }

    /// AI rows by difficulty: 13 − d (+1 below 3, +1 at 0), even.
    #[test]
    fn ai_shield_rows_follow_difficulty() {
        let rules = tot_kit::parse_rules(RULES);
        let mut w = crate::boot(&synthetic_save(), 1);
        w.human_mask = 0;
        let rows: Vec<i32> = (0..6)
            .map(|d| {
                w.difficulty = d;
                w.shield_rows(&rules, 1)
            })
            .collect();
        assert_eq!(rows, vec![15, 13, 12, 10, 9, 8]);
    }

    #[test]
    fn row_arithmetic_matches_original_x86() {
        let vectors = include_str!("../../../docs/evidence/production-row-vectors.txt");
        let mut count = 0;
        for line in vectors.lines().filter(|l| !l.starts_with('#')) {
            let v: Vec<i32> = line
                .split_whitespace()
                .map(|x| x.parse().unwrap())
                .collect();
            assert_eq!(
                production_rows(
                    v[0],
                    v[1] as u8,
                    v[2] as u8,
                    v[3] as u8,
                    v[4] as i16,
                    v[5] as u8,
                    [v[6] as i16, v[7] as i16],
                    [v[8] as u8, v[9] as u8, v[10] as u8]
                ),
                v[11],
                "{line}"
            );
            count += 1;
        }
        assert_eq!(count, 1108);
    }

    #[test]
    fn late_rows_read_leader_wonders_and_live_knowledge() {
        let rules = tot_kit::parse_rules(RULES);
        let mut w = crate::boot(&synthetic_save(), 1);
        w.difficulty = 4;
        w.human_mask = 2;
        w.power_order[7] = 1;
        w.wonders = [-1; 28];
        w.techs = [[0; 13]; 8];
        w.turn = 199;
        assert_eq!(w.shield_rows(&rules, 2), 9);
        w.turn = 200;
        assert_eq!(w.shield_rows(&rules, 2), 7);
        assert_eq!(w.shield_rows(&rules, 1), 10);
        w.wonders[23] = 0;
        w.techs[2][73 / 8] |= 1 << (73 % 8);
        assert_eq!(w.shield_rows(&rules, 2), 9);
        w.power_order[7] = 2;
        assert_eq!(w.shield_rows(&rules, 2), 9);
    }
}

#[cfg(test)]
mod upgrade_veteran_tests {
    use super::*;
    use crate::tests_support::synthetic_save;

    const RULES: &str = "@COSMIC\n3 ; road\n2 ; trireme\n2 ; food\n10 ; food rows\n\
10 ; shield rows\n@CIVILIZE\nAlphabet, 5, 1, nil, nil, 0, 3 ; Alp\n\
Feudalism, 5, 1, nil, nil, 0, 3 ; Feu\n\
@UNITS\nSettlers, nil, 0, 1., 0, 0a, 1d, 1h, 1f, 3, 0, 5, nil, 000000000000000\n\
Phalanx, Feu, 0, 1., 0, 1a, 2d, 1h, 1f, 2, 0, 1, nil, 000000000000000\n\
Pikemen, nil, 0, 1., 0, 1a, 2d, 1h, 1f, 2, 0, 1, Feu, 000000000000000\n\
Archers, nil, 0, 1., 0, 3a, 2d, 1h, 1f, 3, 0, 0, Feu, 000000000000000\n\
@IMPROVE\nNothing, 0, 0, nil\nPalace, 10, 0, nil\nBarracks, 4, 1, nil\n";

    /// #77: once the civ knows Feudalism, a Phalanx build becomes the
    /// first same-role unit that needs Feudalism (Pikemen, not Archers).
    #[test]
    fn obsolete_build_switches_to_the_same_role_successor() {
        let rules = tot_kit::parse_rules(RULES);
        let mut w = crate::boot(&synthetic_save(), 1);
        assert_eq!(w.upgraded_build(&rules, 1, 1), 1, "Feudalism unknown");
        w.techs[1][0] |= 1 << 1; // Feudalism = advance 1
        assert_eq!(w.upgraded_build(&rules, 1, 1), 2);
        assert_eq!(w.upgraded_build(&rules, 1, 3), 3, "Archers have no until");
    }

    /// #78: a unit from a city with Barracks is veteran, without it not.
    #[test]
    fn barracks_make_veterans() {
        let rules = tot_kit::parse_rules(RULES);
        let mut w = crate::boot(&synthetic_save(), 1);
        w.human_mask = 0xFF;
        w.wonders = [-1; 28]; // no wonder 7 either
        w.cities[1].building = 1;
        w.cities[1].flags = 0;
        w.cities[1].shield_box = 20;
        let Some(Built::Unit(u)) = w.production_step(&rules, 1) else {
            panic!("a Phalanx completes")
        };
        assert!(!w.units[u].veteran);
        w.cities[1].buildings[0] |= 1 << 2; // Barracks
        w.cities[1].shield_box = 20;
        let Some(Built::Unit(u)) = w.production_step(&rules, 1) else {
            panic!("a second Phalanx completes")
        };
        assert!(w.units[u].veteran);
    }
}

#[cfg(test)]
mod veteran_native_tests {
    #[test]
    fn original_veteran_eligibility() {
        for line in include_str!("../../../docs/evidence/production-veteran-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let v: Vec<u32> = line
                .split_whitespace()
                .map(|x| x.parse().unwrap())
                .collect();
            let rules = tot_kit::parse_rules(&format!(
                "@UNITS\nUnit,nil,{},1.,0,1a,1d,1h,1f,1,0,{},nil,{:015b}\n",
                v[0], v[1], v[2]
            ));
            let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
            w.cities[0].city_id = 1;
            w.cities[0].owner = 1;
            w.cities[0].buildings = [0; 5];
            w.wonders = [-1; 28];
            for (id, present) in [(2, v[4]), (34, v[5]), (32, v[6])] {
                if present != 0 {
                    w.cities[0].buildings[id / 8] |= 1 << (id % 8);
                }
            }
            if v[7] != 0 {
                w.wonders[7] = 0;
            }
            if v[8] != 0 {
                w.wonders[3] = 0;
            }
            w.game_flags = if v[3] != 0 { 16 } else { 0 };
            assert_eq!(w.made_veteran(&rules, 0, 1, 0), v[9] != 0, "{line}");
        }
    }
}
