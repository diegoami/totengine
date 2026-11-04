//! Founding a city (v0.6.0, #75): what `create_city` (`civ2.bak`
//! `0x432810`) puts in a new city record, plus the engine's own rules where
//! the original is not decoded yet.
//!
//! **Decoded from `0x432810`** (record offsets = RAM city table offsets):
//! - x/y @0/@2, owner @10 and original owner @12 = the founder;
//! - size 1, flags `0x20000`; flag `0x80` when a neighbour is ocean
//!   (`0x4332e0`);
//! - city id = the next free id; boxes and worked tiles start at 0;
//!   improvements cleared;
//! - **the civ's first city gets the Palace** (`0x4300af(city, 1)`);
//! - the tile gets the city bit (byte +1 `0x02`);
//! - **build item** = among buildable units of Rules role 1 (defend), the
//!   best defence / cost; ties go to the later unit (`0x432d47`..
//!   `0x432e02`).
//!
//! **Engine rules (not decoded; flagged):**
//! - **Validity:** a Settlers-role unit (Rules role 5) of the civ, on land,
//!   with no city within distance 1.
//! - **Founding:** consumes the unit.
//! - **Name:** `"<plural> <n>"`. The original picks from per-tribe name
//!   lists (`0x4323af`).
//! - **First worked tile:** the radius tile with the most food, then
//!   shields, then trade. The original's worker assignment is not decoded.
//! - **Settlers cost one population when they complete** (classic rule;
//!   a size-1 city keeps the box until it grows). The original's
//!   population cost is not located yet.

use tot_kit::Rules;

use crate::movement::OCEAN_TERRAIN;
use crate::yields::{CENTRE, RADIUS_DX, RADIUS_DY};
use crate::{City, WorldState};

/// Rules role of settler-type units.
pub const ROLE_SETTLE: u8 = 5;
const ROLE_DEFEND: u8 = 1;
const PALACE: usize = 1;

impl WorldState {
    /// Why `unit` cannot found a city here, if it cannot.
    pub fn found_city_problem(&self, rules: &Rules, unit: usize) -> Option<&'static str> {
        let u = self.units.get(unit)?;
        if u.id == 0 {
            return Some("no such unit");
        }
        if rules.unit_role(u.type_id as usize) != Some(ROLE_SETTLE) {
            return Some("not a settler");
        }
        if self
            .map
            .terrain_at(u.x, u.y)
            .is_none_or(|t| t == OCEAN_TERRAIN)
        {
            return Some("not on land");
        }
        let w2 = 2 * self.map.stored_w as i32;
        let too_close = self.cities.iter().any(|c| {
            crate::placement::diamond_distance(
                w2, false, c.x as i32, c.y as i32, u.x as i32, u.y as i32,
            ) <= 1
        });
        if too_close {
            return Some("too close to a city");
        }
        None
    }

    /// Whether `civ` may build unit type `t` now (its Rules prerequisite is
    /// known, or none).
    pub fn can_build_unit(&self, rules: &Rules, civ: u8, t: usize) -> bool {
        match rules.unit_prereq_raw(t) {
            None | Some("nil") => true,
            Some("no") => false,
            Some(abbrev) => crate::research::TechTree::from_rules(rules)
                .by_abbrev(abbrev)
                .is_some_and(|k| {
                    self.techs
                        .get(civ as usize)
                        .and_then(|b| b.get(k >> 3))
                        .is_some_and(|b| b >> (k & 7) & 1 != 0)
                }),
        }
    }

    /// The decoded initial build item: the buildable defender (role 1) with
    /// the best defence per cost, ties to the later unit; the first
    /// buildable unit if there is no defender.
    pub fn initial_build(&self, rules: &Rules, civ: u8) -> i8 {
        let n = rules.unit_names().len();
        let mut item = (0..n)
            .find(|&t| self.can_build_unit(rules, civ, t))
            .unwrap_or(0);
        let mut best = 0;
        for t in 2..n.min(80) {
            if rules.unit_role(t) != Some(ROLE_DEFEND) || !self.can_build_unit(rules, civ, t) {
                continue;
            }
            let cost = rules.unit_cost(t).unwrap_or(1).max(1) as i32;
            let score = rules.unit_defense(t).unwrap_or(0) as i32 / cost;
            if score >= best {
                best = score;
                item = t;
            }
        }
        item as i8
    }

    /// Found a city with `unit` (checked by [`WorldState::found_city_problem`]);
    /// returns the new city's index.
    pub fn found_city(&mut self, rules: &Rules, unit: usize) -> Result<usize, &'static str> {
        if let Some(why) = self.found_city_problem(rules, unit) {
            return Err(why);
        }
        let (x, y, civ) = {
            let u = &self.units[unit];
            (u.x, u.y, u.owner)
        };
        let first = !self.cities.iter().any(|c| (c.owner & 0x0F) as u8 == civ);
        let n = self
            .cities
            .iter()
            .filter(|c| (c.owner & 0x0F) as u8 == civ)
            .count()
            + 1;
        let plural = self
            .civs
            .iter()
            .find(|c| c.id == civ)
            .map_or_else(|| format!("Civ{civ}"), |c| c.plural.clone());
        let w2 = 2 * self.map.stored_w as i32;
        let coastal = (0..8).any(|k| {
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
            let nx = (x as i32 + dx).rem_euclid(w2.max(1)) as u16;
            let ny = y as i32 + dy;
            ny >= 0 && self.map.terrain_at(nx, ny as u16) == Some(OCEAN_TERRAIN)
        });
        let mut buildings = [0u8; 5];
        if first {
            buildings[PALACE >> 3] |= 1 << (PALACE & 7);
        }
        let city_id = self.cities.iter().map(|c| c.city_id).max().unwrap_or(0) + 1;
        let building = self.initial_build(rules, civ);
        self.cities.push(City {
            name: format!("{plural} {n}"),
            x,
            y,
            size: 1,
            owner: civ as u16,
            city_id,
            gross_food: 0,
            gross_shields: 0,
            gross_trade: 0,
            net_science: 0,
            net_taxes: 0,
            food_box: 0,
            shield_box: 0,
            worked: 1 << CENTRE,
            buildings,
            flags: 0x20000 | if coastal { 0x80 } else { 0 },
            building,
        });
        let idx = self.cities.len() - 1;
        // first worked tile (engine rule): most food, then shields, trade
        let best = (0..CENTRE)
            .filter(|&k| {
                let ty = y as i32 + RADIUS_DY[k];
                ty >= 0
                    && (ty as usize) < self.map.stored_h
                    && self.cities.iter().all(|c| {
                        !(c.x as i32 == (x as i32 + RADIUS_DX[k]).rem_euclid(w2.max(1))
                            && c.y as i32 == ty)
                    })
            })
            .max_by_key(|&k| {
                let v = self.tile_yield(rules, idx, k);
                (v[0], v[1], v[2], std::cmp::Reverse(k))
            });
        if let Some(k) = best {
            self.cities[idx].worked |= 1 << k;
        }
        if let Some(i) = self.map.stored_index(x, y) {
            self.map.improvements[i] |= 0x02;
            self.map.owner[i] = civ & 0x0F;
        }
        // the settler is consumed
        self.units[unit].id = 0;
        self.units[unit].x = 0xFFFF;
        self.units[unit].y = 0xFFFF;
        // yields so the turn loop does not skip the young city
        let out = self.city_output(rules, idx);
        let c = &mut self.cities[idx];
        c.gross_food = out.food.clamp(0, 255) as u8;
        c.gross_shields = out.shields.clamp(0, 255) as u8;
        c.gross_trade = out.trade.clamp(0, u16::MAX as i32) as u16;
        Ok(idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_support::synthetic_save;

    const RULES: &str = "@COSMIC\n3 ; road\n2 ; trireme\n2 ; food\n10 ; food rows\n\
10 ; shield rows\n@UNITS\nSettlers, nil, 0, 1., 0, 0a, 1d, 1h, 1f, 3, 0, 5, nil, 000000000000000\n\
Engineers, nil, 0, 2., 0, 0a, 2d, 2h, 1f, 4, 0, 5, Exp, 000000000000000\n\
Warriors, nil, 0, 1., 0, 1a, 1d, 1h, 1f, 1, 0, 1, nil, 000000000000000\n\
Phalanx, nil, 0, 1., 0, 1a, 2d, 1h, 1f, 2, 0, 1, nil, 000000000000000\n\
Pikemen, nil, 0, 1., 0, 1a, 2d, 1h, 1f, 2, 0, 1, Feu, 000000000000000\n\
@IMPROVE\nNothing, 0, 0, nil\nPalace, 10, 0, nil\n\
@TERRAIN\nDesert, 1, 2, 0, 1, 0\nPlains, 1, 2, 1, 1, 0\nGrassland, 1, 2, 2, 0, 0\n";

    /// The decoded first build: best defence/cost among buildable role-1
    /// units, ties to the later one (Warriors 1/1 = Phalanx 2/2 → Phalanx;
    /// Pikemen needs Feudalism).
    #[test]
    fn initial_build_is_the_best_affordable_defender() {
        let rules = tot_kit::parse_rules(RULES);
        let w = crate::boot(&synthetic_save(), 1);
        assert_eq!(w.initial_build(&rules, 1), 3);
    }

    /// Founding: only settlers, only on land away from cities; the settler
    /// is consumed, the city starts at size 1 with the tile marked, and a
    /// civ's first city gets the Palace.
    #[test]
    fn founding_rules() {
        let rules = tot_kit::parse_rules(RULES);
        let mut w = crate::boot(&synthetic_save(), 1);
        let mut u = w.units[0].clone();
        u.type_id = 2; // Warriors
        u.owner = 3;
        u.x = 7;
        u.y = 3;
        u.id = 99;
        w.units.push(u.clone());
        let warrior = w.units.len() - 1;
        assert_eq!(w.found_city(&rules, warrior), Err("not a settler"));
        let near = w.cities[0].clone();
        u.type_id = 0;
        u.x = near.x + 1;
        u.y = near.y + 1;
        u.id = 100;
        w.units.push(u.clone());
        assert_eq!(
            w.found_city(&rules, w.units.len() - 1),
            Err("too close to a city")
        );
        // a civ with no city founds its first one far from the others
        w.cities.retain(|c| c.owner != 3);
        u.x = 6;
        u.y = 0;
        u.id = 101;
        let far_ok = w.cities.iter().all(|c| {
            crate::placement::diamond_distance(8, false, c.x as i32, c.y as i32, 6, 0) > 1
        });
        assert!(far_ok, "the fixture tile (6,0) is clear of cities");
        w.units.push(u);
        let s = w.units.len() - 1;
        {
            let c = w.found_city(&rules, s).unwrap();
            assert_eq!(w.units[s].id, 0, "settler consumed");
            let city = &w.cities[c];
            assert_eq!((city.size, city.owner, city.building), (1, 3, 3));
            assert!(city.has_building(PALACE), "first city gets the Palace");
            let i = w.map.stored_index(6, 0).unwrap();
            assert!(w.map.improvements[i] & 0x02 != 0);
        }
    }
}
