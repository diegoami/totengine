//! Original road/rail connections (`0x471a13`, path search `0x48c9c5`).
//!
//! The search chooses a shortest land path before checking its roads.
//! A longer road detour does not qualify. Evidence: connections.json and
//! tools/connections_check.py execute the original x86 without call stubs.
//! Surface-map land unit type 2 is supported; scenarios replacing that
//! hard-coded pathfinding type with a sea/air type remain open.

use std::collections::VecDeque;

use crate::{placement::diamond_distance, WorldState};
use tot_kit::Rules;

const DX: [i32; 8] = [1, 2, 1, 0, -1, -2, -1, 0];
const DY: [i32; 8] = [-1, 0, 1, 2, 1, 0, -1, -2];

fn grid(x: i32, y: i32, tx: i32, ty: i32) -> Option<usize> {
    let dx = x - tx;
    let dy = y - ty;
    let a = (dx + dy) >> 1;
    let b = dx - a;
    if (-24..24).contains(&a) && (-24..24).contains(&b) {
        Some(((b + 24) * 48 + a + 24) as usize)
    } else {
        None
    }
}

impl WorldState {
    /// 0 = no connection, 1 = road, 2 = rail. Endpoints and an intervening
    /// own city need no road and do not downgrade the rail result.
    pub fn road_connection(&self, rules: &Rules, civ: u8, from: (u16, u16), to: (u16, u16)) -> i32 {
        if civ >= 8 || rules.unit_domain(2) != Some(0) {
            return 0;
        }
        let map = &self.map;
        let width = 2 * map.stored_w as i32;
        let flat = self.game_flags & 0x8000 != 0;
        let wrap = |x: i32| if flat { x } else { x.rem_euclid(width.max(1)) };
        let tile = |x: i32, y: i32| {
            if x < 0 || y < 0 {
                None
            } else {
                map.stored_index(x as u16, y as u16)
            }
        };
        let (mut x, mut y) = (from.0 as i32, from.1 as i32);
        let (tx, ty) = (to.0 as i32, to.1 as i32);
        let (Some(source), Some(target)) = (tile(x, y), tile(tx, ty)) else {
            return 0;
        };
        let mass = |i: usize| {
            if map.terrain[i] == 10 {
                -1
            } else {
                map.landmass[i] as i32
            }
        };
        let mut dx = (x - tx).abs();
        if !flat && dx > width / 2 {
            dx = width - dx;
        }
        if mass(source) != mass(target) || (dx + (y - ty).abs()) / 2 >= 23 {
            return 0;
        }
        // Rules loader 0x4141f4 copies terrain row field 15 to +22:
        // yes -> -2 (impassable), no -> -1. Unit advanced field 6,
        // bit 0x10 bypasses that check (0x57073a).
        let ignore = rules
            .section("UNITS_ADVANCED")
            .and_then(|s| s.rows.get(2))
            .and_then(|r| r.get(6))
            .and_then(|v| u32::from_str_radix(v.trim(), 2).ok())
            .is_some_and(|v| v & 16 != 0);
        let mut blocked = [false; 16];
        if !ignore {
            if let Some(terrain) = rules.section("TERRAIN") {
                for (i, row) in terrain.rows.iter().take(11).enumerate() {
                    blocked[i] = row.get(15).is_some_and(|s| s.eq_ignore_ascii_case("yes"));
                }
            }
        }
        let mut sx = if flat {
            x
        } else {
            tx + (x - tx + width / 2).rem_euclid(width) - width / 2
        };
        let mut costs = [0u8; 48 * 48];
        costs[grid(tx, ty, tx, ty).unwrap()] = 1;
        let mut queue = VecDeque::from([(tx, ty)]);
        let mut limit = 99;
        while let Some((px, py)) = queue.pop_front() {
            let cost = costs[grid(px, py, tx, ty).unwrap()];
            if cost >= 99 || cost > limit {
                continue;
            }
            if (wrap(px), py) == (x, y) {
                limit = cost;
                continue;
            }
            for d in 0..8 {
                let (nx, ny) = (px + DX[d], py + DY[d]);
                if (nx - tx).abs() + (ny - ty).abs() >= 48 {
                    continue;
                }
                let Some(i) = tile(wrap(nx), ny) else {
                    continue;
                };
                if blocked[map.terrain[i] as usize]
                    || (map.terrain[i] == 10
                        && (wrap(nx), ny) != (x, y)
                        && (wrap(nx), ny) != (tx, ty))
                {
                    continue;
                }
                let Some(g) = grid(nx, ny, tx, ty) else {
                    continue;
                };
                if costs[g] == 0 {
                    costs[g] = cost + 1;
                    queue.push_back((nx, ny));
                }
            }
        }
        let mut rail = true;
        for _ in 0..51 {
            let current = grid(sx, y, tx, ty).map(|g| costs[g]).unwrap_or(0);
            if current == 0 || current >= 99 {
                return 0;
            }
            let mut best = None;
            for d in 0..8 {
                let (nx, ny) = (sx + DX[d], y + DY[d]);
                // The original selector bounds the raw neighbour before
                // wrapping. This can reject a route across the map seam.
                if (x + DX[d] - tx).abs() + (ny - ty).abs() >= 48 {
                    continue;
                }
                let Some(i) = tile(wrap(nx), ny) else {
                    continue;
                };
                if blocked[map.terrain[i] as usize] {
                    continue;
                }
                let cost = grid(nx, ny, tx, ty).map(|g| costs[g]).unwrap_or(0);
                if cost == 0 || cost >= current {
                    continue;
                }
                let rank = (cost, diamond_distance(width, flat, wrap(nx), ny, tx, ty), d);
                if best.is_none_or(|(old, _, _)| rank < old) {
                    best = Some((rank, nx, ny));
                }
            }
            let Some((_, nx, ny)) = best else { return 0 };
            sx = nx;
            x = wrap(nx);
            y = ny;
            if (x, y) == (tx, ty) {
                return if rail { 2 } else { 1 };
            }
            let i = tile(x, y).unwrap();
            let flags = map.improvements[i];
            // Occupancy follows live objects: map unit-presence bits can
            // still describe the boot save after engine movement/deaths.
            let city_owner = self
                .cities
                .iter()
                .find(|c| c.city_id != 0 && c.map_id == 0 && c.x as i32 == x && c.y as i32 == y)
                .map(|c| (c.owner & 15) as u8);
            if city_owner == Some(civ) {
                return if rail { 2 } else { 1 };
            }
            let occupant = city_owner.or_else(|| {
                self.units
                    .iter()
                    .filter(|u| u.id != 0 && u.x as i32 == x && u.y as i32 == y)
                    .min_by_key(|u| u.stack_pos)
                    .map(|u| u.owner)
            });
            if let Some(owner) = occupant.filter(|&o| o < 8) {
                if (city_owner.is_some() || (owner > 0 && owner != civ))
                    && self.diplomacy[civ as usize][owner as usize] & 8 == 0
                {
                    return 0;
                }
            }
            if flags & 16 == 0 {
                return 0;
            }
            rail &= flags & 32 != 0;
        }
        0
    }

    /// Capital connection used to scale waste (`0x4c1736..0x4c18aa`).
    pub fn capital_connection(&self, rules: &Rules, idx: usize) -> i32 {
        let Some(city) = self.cities.get(idx) else {
            return 0;
        };
        let civ = (city.owner & 15) as u8;
        if city.map_id != 0 || civ >= 8 {
            return 0;
        }
        if city.has_building(1) {
            return 1 + ((self.techs[civ as usize][67 / 8] >> (67 % 8)) & 1) as i32;
        }
        let Some(capital) = self
            .cities
            .iter()
            .filter(|c| c.owner == city.owner && c.city_id != 0 && c.has_building(1))
            .min_by_key(|c| {
                diamond_distance(
                    2 * self.map.stored_w as i32,
                    self.game_flags & 0x8000 != 0,
                    city.x as i32,
                    city.y as i32,
                    c.x as i32,
                    c.y as i32,
                )
            })
        else {
            return 0;
        };
        if capital.map_id != 0 {
            return 0;
        }
        let from = (city.x, city.y);
        let to = (capital.x, capital.y);
        let (Some(a), Some(b)) = (
            self.map.stored_index(from.0, from.1),
            self.map.stored_index(to.0, to.1),
        ) else {
            return 0;
        };
        if self.map.landmass[a] == self.map.landmass[b] {
            self.road_connection(rules, civ, from, to)
        } else {
            (city.has_building(32) && capital.has_building(32)) as i32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (WorldState, Rules) {
        let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
        let n = 32 * 16;
        w.map = crate::MapState {
            stored_w: 32,
            stored_h: 16,
            terrain: vec![1; n],
            landmass: vec![1; n],
            site_bits: vec![0; n],
            fertility: vec![0; n],
            special: vec![false; n],
            river: vec![false; n],
            road: vec![true; n],
            rail: vec![false; n],
            improvements: vec![16; n],
            owner: vec![15; n],
            visible: vec![255; n],
            resource_seed: 0,
            huts_enabled: false,
        };
        w.game_flags = 0;
        w.diplomacy = [[0; 8]; 8];
        let rules = tot_kit::parse_rules("@UNITS\na,no,0\nb,no,0\nc,no,0\n");
        (w, rules)
    }

    fn place_city(w: &mut WorldState, x: u16, y: u16, owner: u16) {
        let mut c = w.cities[0].clone();
        c.x = x;
        c.y = y;
        c.map_id = 0;
        c.city_id = 1;
        c.owner = owner;
        w.cities.push(c);
    }

    #[test]
    fn shortest_path_roads_rails_and_endpoints() {
        let (mut w, rules) = fixture();
        assert_eq!(w.road_connection(&rules, 1, (8, 8), (16, 8)), 1);
        w.map.improvements.fill(48);
        assert_eq!(w.road_connection(&rules, 1, (8, 8), (16, 8)), 2);
        let middle = w.map.stored_index(12, 8).unwrap();
        w.map.improvements[middle] = 0;
        // Roads around the missing tile provide a longer detour. The
        // original still selects the shorter path and rejects it.
        assert_eq!(w.road_connection(&rules, 1, (8, 8), (16, 8)), 0);
        let target = w.map.stored_index(10, 8).unwrap();
        w.map.improvements[target] = 0;
        assert_eq!(w.road_connection(&rules, 1, (8, 8), (10, 8)), 2);
        w.map.improvements[target] = 2;
        w.map.owner[target] = 1;
        place_city(&mut w, 10, 8, 1);
        assert_eq!(w.road_connection(&rules, 1, (8, 8), (16, 8)), 2);
    }

    #[test]
    fn occupied_tiles_diplomacy_and_geography() {
        let (mut w, rules) = fixture();
        let i = w.map.stored_index(12, 8).unwrap();
        w.map.improvements[i] = 17;
        w.map.owner[i] = 2;
        w.units[0].x = 12;
        w.units[0].y = 8;
        w.units[0].owner = 2;
        w.units[0].id = 1;
        assert_eq!(w.road_connection(&rules, 1, (8, 8), (16, 8)), 0);
        w.diplomacy[1][2] = 8;
        assert_eq!(w.road_connection(&rules, 1, (8, 8), (16, 8)), 1);
        w.map.improvements[i] = 18;
        w.diplomacy[1][2] = 0;
        place_city(&mut w, 12, 8, 2);
        assert_eq!(w.road_connection(&rules, 1, (8, 8), (16, 8)), 0);
        w.map.owner[i] = 15;
        let t = w.map.stored_index(16, 8).unwrap();
        w.map.landmass[t] = 2;
        assert_eq!(w.road_connection(&rules, 1, (8, 8), (16, 8)), 0);
        assert_eq!(w.road_connection(&rules, 1, (2, 8), (62, 8)), 0);
        assert_eq!(w.road_connection(&rules, 1, (0, 0), (40, 14)), 0);
    }

    #[test]
    fn live_unit_movement_and_death_override_boot_occupancy() {
        let (mut w, rules) = fixture();
        w.units[0].owner = 2;
        w.units[0].id = 1;
        w.units[0].x = 12;
        w.units[0].y = 8;
        assert_eq!(w.road_connection(&rules, 1, (8, 8), (16, 8)), 0);
        w.units[0].x = 12;
        w.units[0].y = 6;
        assert_eq!(w.road_connection(&rules, 1, (8, 8), (16, 8)), 1);
        w.units[0].y = 8;
        w.units[0].id = 0;
        assert_eq!(w.road_connection(&rules, 1, (8, 8), (16, 8)), 1);
    }

    #[test]
    fn terrain_passability_and_unit_override() {
        let (w, _) = fixture();
        let text = "@UNITS\na,no,0\nb,no,0\nc,no,0\n@TERRAIN\na,1,2,0,1,0,yes,1,1,1,yes,1,1,1,a,no\nb,1,2,0,1,0,yes,1,1,1,yes,1,1,1,b,yes\n";
        let rules = tot_kit::parse_rules(text);
        assert_eq!(w.road_connection(&rules, 1, (8, 8), (16, 8)), 0);
        let rules = tot_kit::parse_rules(&format!(
            "{text}@UNITS_ADVANCED\n0,0,0,0,0,0,0\n0,0,0,0,0,0,0\n0,0,0,0,0,0,00010000\n"
        ));
        assert_eq!(w.road_connection(&rules, 1, (8, 8), (16, 8)), 1);
    }

    #[test]
    fn capital_railroad_airports_and_map_scope() {
        let (mut w, rules) = fixture();
        let mut capital = w.cities[0].clone();
        capital.x = 16;
        capital.y = 8;
        capital.owner = 1;
        capital.city_id = 1;
        capital.map_id = 0;
        capital.buildings = [2, 0, 0, 0, 0];
        let mut city = capital.clone();
        city.x = 8;
        city.buildings[0] = 0;
        w.cities = vec![capital, city];
        assert_eq!(w.capital_connection(&rules, 0), 1);
        w.techs[1][67 / 8] |= 1 << (67 % 8);
        assert_eq!(w.capital_connection(&rules, 0), 2);
        assert_eq!(w.capital_connection(&rules, 1), 1);
        let t = w.map.stored_index(16, 8).unwrap();
        w.map.landmass[t] = 2;
        assert_eq!(w.capital_connection(&rules, 1), 0);
        w.cities[0].buildings[4] |= 1;
        w.cities[1].buildings[4] |= 1;
        assert_eq!(w.capital_connection(&rules, 1), 1);
        w.cities[0].map_id = 1;
        assert_eq!(w.capital_connection(&rules, 1), 0);
    }
}
