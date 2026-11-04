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
//!   up to even.
//! - **Completion:** the box drops by the cost, then the unit is created
//!   in the city (homed there) or the improvement is set.
//!
//! Not modelled yet:
//! - the build switching to a newer unit once the civ knows the obsoleting
//!   advance;
//! - wonders (ids ≥ 40) and their box reset;
//! - veteran units from Barracks;
//! - the late-game AI rows adjustment (turn > 200);
//! - the Settlers-at-size-1 case at Chieftain;
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
}

const CAPITALIZATION: i8 = -39;
const FIRST_WONDER: usize = 40;

impl WorldState {
    /// Shield-box rows of `civ` (`0x4c1bdf`).
    pub fn shield_rows(&self, rules: &Rules, civ: u8) -> i32 {
        let cosmic = rules.cosmic_row(4).unwrap_or(10) as i32;
        if self.human_mask >> civ & 1 != 0 {
            return cosmic;
        }
        let d = self.difficulty as i32;
        let mut rows = 13 - d + (d < 3) as i32 + (d == 0) as i32;
        rows = cosmic * rows / 10;
        if rows & 1 != 0 {
            rows += 1;
        }
        rows
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
            c.shield_box = (box_now - cost) as u16;
            let (x, y) = (c.x, c.y);
            let id = self.units.iter().map(|u| u.id).max().unwrap_or(0) + 1;
            let stack_pos = self
                .units
                .iter()
                .filter(|u| (u.x, u.y) == (x, y) && u.id != 0)
                .map(|u| u.stack_pos + 1)
                .max()
                .unwrap_or(0);
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
                stack_pos,
                home: idx as u8,
                id,
            });
            Some(Built::Unit(self.units.len() - 1))
        } else {
            let id = -(item as i32) as usize;
            if id >= FIRST_WONDER || c.has_building(id) {
                return None;
            }
            let cost = rules.improvement_cost(id)? as i32 * rows;
            if box_now < cost {
                return None;
            }
            c.shield_box = (box_now - cost) as u16;
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
        assert_eq!(rows, vec![16, 14, 12, 10, 10, 8]);
    }
}
