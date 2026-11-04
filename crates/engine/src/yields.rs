//! Tile yields (T1-4, v0.4.0): food, shields and trade of one city tile,
//! decoded from the original's tile-yield function (`civ2.bak` `0x4c22a7`,
//! `(city, radius index, kind)`); evidence §T1-4 tile yields.
//!
//! **Oracle check:** summed over each city's worked tiles, this reproduces
//! the game-written city records:
//! - the standard-game lab saves: 9/10 cities exact in food, shields and
//!   trade (the tenth differs by one trade);
//! - ROMFALL: food exact in 144/146.
//!
//! The record's shields and trade are stored *after* waste, corruption and
//! building multipliers. Those are not decoded yet, so trade in the record
//! is never above the tile sum, and shields go both ways.
//!
//! Not modelled: the network-game doubling branch (`0x607652`/`0x6031bc`,
//! off in single-player games), and AI side effects the function triggers
//! (`0x4c2943`, `0x4c2a7b`) that do not change the yield.

use tot_kit::Rules;

use crate::movement::OCEAN_TERRAIN;
use crate::{City, WorldState};

/// City radius, doubled-x diamond offsets; index 20 is the centre
/// (`0x5d1620` dx / `0x5d1650` dy).
pub const RADIUS_DX: [i32; 21] = [
    1, 2, 1, 0, -1, -2, -1, 0, 2, 2, -2, -2, 1, 3, 3, 1, -1, -3, -3, -1, 0,
];
pub const RADIUS_DY: [i32; 21] = [
    -1, 0, 1, 2, 1, 0, -1, -2, -2, 2, 2, -2, -3, -1, 1, 3, 3, 1, -1, -3, 0,
];
pub const CENTRE: usize = 20;

/// Yield kinds, as the original's `kind` argument.
pub const FOOD: usize = 0;
pub const SHIELDS: usize = 1;
pub const TRADE: usize = 2;

const GRASSLAND: u8 = 2;
// improvement / wonder / advance ids the yield code tests
const SUPERMARKET: usize = 0x18;
const SUPERHIGHWAYS: usize = 0x19;
const HARBOUR: usize = 0x1e;
const OFFSHORE_PLATFORM: usize = 0x1f;
const COLOSSUS: usize = 2;
const KING_RICHARDS: usize = 8;
const RAILROAD_TECH: usize = 0x43;

/// Special resource on a land tile (`0x5707a6`): 0 none, 1 or 2 = the
/// terrain's first or second special (@TERRAIN rows 11+t / 22+t).
/// Byte +0 bit `0x40` suppresses it; grassland has none (its shield is
/// [`grassland_shield`]).
pub fn special(seed: u16, terrain_byte: u8, x: i32, y: i32) -> usize {
    if terrain_byte & 0x40 != 0 || terrain_byte & 0x0F == GRASSLAND {
        return 0;
    }
    let a = (x + y) >> 1;
    let b = x - a;
    let v = seed as i32 + 11 * (a >> 2) + 13 * (b >> 2);
    if (a & 3) + 4 * (b & 3) != v & 15 {
        return 0;
    }
    let k = 1 << ((seed >> 4) & 3);
    if a & k == b & k {
        2
    } else {
        1
    }
}

/// Grassland yields its shield only here (`0x4095b0`).
pub fn grassland_shield(x: i32, y: i32) -> bool {
    let a = (x + y) >> 1;
    (-x - a) & 2 == 0
}

impl WorldState {
    fn civ_has_tech(&self, civ: u8, tech: usize) -> bool {
        civ > 0
            && self
                .techs
                .get(civ as usize)
                .and_then(|t| t.get(tech >> 3))
                .is_some_and(|b| b >> (tech & 7) & 1 != 0)
    }

    fn wonder_in(&self, wonder: usize, city_idx: usize) -> bool {
        self.wonders
            .get(wonder)
            .is_some_and(|&c| c == city_idx as i16)
    }

    /// Food, shields and trade of radius tile `k` of city `city_idx`.
    pub fn tile_yield(&self, rules: &Rules, city_idx: usize, k: usize) -> [i32; 3] {
        let Some(city) = self.cities.get(city_idx) else {
            return [0; 3];
        };
        let w2 = 2 * self.map.stored_w as i32;
        let x = (city.x as i32 + RADIUS_DX[k]).rem_euclid(w2.max(1));
        let y = city.y as i32 + RADIUS_DY[k];
        if y < 0 || y >= self.map.stored_h as i32 {
            return [0; 3];
        }
        let Some(i) = self.map.stored_index(x as u16, y as u16) else {
            return [0; 3];
        };
        let terrain = self.map.terrain[i];
        let byte0 = terrain
            | if self.map.special[i] { 0x40 } else { 0 }
            | if self.map.river[i] { 0x80 } else { 0 };
        let row = special(self.map.resource_seed, byte0, x, y) * 11 + terrain as usize;
        let t = terrain as usize;
        let f = self.map.improvements[i];
        let centre = f & 0xC2 == 2;
        let civ = (city.owner & 0x0F) as u8;
        let gov = self.governments.get(civ as usize).copied().unwrap_or(0);
        let celebrating = city.flags & 2 != 0;
        let irr = rules.irrigation_bonus(t).unwrap_or(0);
        let mine = rules.mine_bonus(t).unwrap_or(0);
        let mut out = [0; 3];
        for (kind, o) in out.iter_mut().enumerate() {
            let mut v = rules.terrain_yield(row, kind).unwrap_or(0);
            if terrain == OCEAN_TERRAIN {
                v += match kind {
                    FOOD => city.has_building(HARBOUR) as i32,
                    SHIELDS => city.has_building(OFFSHORE_PLATFORM) as i32,
                    _ => self.wonder_in(COLOSSUS, city_idx) as i32,
                };
            } else if kind == FOOD {
                if f & 0x04 != 0 || centre {
                    v += irr;
                    if city.has_building(SUPERMARKET) && (f & 0x08 != 0 || centre) {
                        v += v / 2;
                    }
                }
            } else if kind == SHIELDS {
                if f & 0x0C == 0x08 || (centre && irr == 0) {
                    v += mine;
                }
                if terrain == GRASSLAND && !grassland_shield(x, y) {
                    v = 0;
                }
                if k == CENTRE && v == 0 {
                    v = 1;
                }
            } else {
                if byte0 & 0x80 != 0 {
                    v += 1;
                }
                if (f & 0x10 != 0 || centre) && (terrain <= GRASSLAND || v != 0) {
                    v += 1;
                }
                if v != 0 && self.wonder_in(COLOSSUS, city_idx) {
                    v += 1;
                }
            }
            if kind == SHIELDS && self.wonder_in(KING_RICHARDS, city_idx) {
                v += 1;
            }
            if kind == SHIELDS
                && (f & 0x20 != 0 || (centre && self.civ_has_tech(civ, RAILROAD_TECH)))
            {
                v += v / 2;
            }
            if v > 2 && !celebrating && gov <= 1 {
                v -= 1;
            }
            if v != 0 && kind == TRADE && gov >= if celebrating { 2 } else { 5 } {
                v += 1;
            }
            if kind == TRADE && (f & 0x10 != 0 || centre) && city.has_building(SUPERHIGHWAYS) {
                v += v / 2;
            }
            if f & 0x82 == 0x80 {
                v = (v + 1) >> 1;
            }
            *o = v.max(0);
        }
        out
    }

    /// Sum of [`WorldState::tile_yield`] over the city's worked tiles.
    pub fn worked_yield(&self, rules: &Rules, city_idx: usize) -> [i32; 3] {
        let worked = self.cities.get(city_idx).map_or(0, |c: &City| c.worked);
        (0..21)
            .filter(|&k| worked >> k & 1 != 0)
            .map(|k| self.tile_yield(rules, city_idx, k))
            .fold([0; 3], |a, y| [a[0] + y[0], a[1] + y[1], a[2] + y[2]])
    }
}
