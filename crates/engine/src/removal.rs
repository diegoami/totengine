//! Surface city destruction (`0x43456d`), for the retained engine state.
//! Civilization elimination and sea-transport cargo removal are separate.
use crate::WorldState;
use tot_kit::Rules;

// Original radius table 21..44, used with the usual 21 city-radius entries.
const OUTER: [(i32, i32); 24] = [
    (4, 0),
    (-4, 0),
    (0, 4),
    (0, -4),
    (1, -5),
    (2, -4),
    (3, -3),
    (4, -2),
    (5, -1),
    (5, 1),
    (4, 2),
    (3, 3),
    (2, 4),
    (1, 5),
    (-1, 5),
    (-2, 4),
    (-3, 3),
    (-4, 2),
    (-5, 1),
    (-5, -1),
    (-4, -2),
    (-3, -3),
    (-2, -4),
    (-1, -5),
];

fn remove_route(city: &mut crate::City, slot: usize) {
    for k in slot..city.route_count as usize - 1 {
        city.route_partners[k] = city.route_partners[k + 1];
        city.route_commodities[k] = city.route_commodities[k + 1];
    }
    city.route_count -= 1;
    city.flags |= crate::founding::CITY_REFRESH;
}

impl WorldState {
    /// Remove one land unit, retaining its dead slot and clearing a now-empty
    /// surface unit marker. Stack order is represented by `stack_pos` here.
    pub(crate) fn remove_land_unit(&mut self, index: usize) {
        let Some(unit) = self.units.get(index).filter(|u| u.id != 0) else {
            return;
        };
        let (x, y, pos) = (unit.x, unit.y, unit.stack_pos);
        self.units[index].id = 0;
        for u in self
            .units
            .iter_mut()
            .filter(|u| u.id != 0 && (u.x, u.y) == (x, y) && u.stack_pos > pos)
        {
            u.stack_pos -= 1;
        }
        if !self.units.iter().any(|u| u.id != 0 && (u.x, u.y) == (x, y)) {
            if let Some(tile) = self.map.stored_index(x, y) {
                self.map.improvements[tile] &= !1;
            }
        }
    }

    /// Surface city record, homed units, incoming routes, wonders and nearby
    /// fertility/site invalidation. Interior dead slots remain available for
    /// reuse; deleting the last slot reduces the native extent by exactly one.
    pub fn destroy_surface_city(&mut self, rules: &Rules, index: usize) -> bool {
        let Some(city) = self
            .cities
            .get(index)
            .filter(|c| c.city_id != 0 && c.map_id == 0)
        else {
            return false;
        };
        let (x, y, owner) = (city.x as i32, city.y as i32, (city.owner & 15) as u8);
        self.city_counts[owner as usize] = self.city_counts[owner as usize].wrapping_sub(1);
        for k in (0..self.units.len()).rev() {
            let unit = &self.units[k];
            if unit.id == 0 || unit.home as usize != index {
                continue;
            }
            if unit.type_id == 9 {
                self.units[k].home = 255;
                continue;
            }
            if self.human_mask & (1 << unit.owner) == 0 {
                let other = self.cities.iter().position(|c| {
                    c.city_id != 0 && c.map_id == 0 && (c.x, c.y) == (unit.x, unit.y)
                });
                if let Some(other) = other.filter(|&c| c != index) {
                    let defenders = self
                        .units
                        .iter()
                        .filter(|u| {
                            u.id != 0
                                && (u.x, u.y) == (unit.x, unit.y)
                                && rules.unit_role(u.type_id as usize) == Some(1)
                        })
                        .count();
                    if rules.unit_role(unit.type_id as usize) == Some(1) && defenders == 1 {
                        self.units[k].home = other as u8;
                        continue;
                    }
                    self.cities[other].flags |= 0x20;
                }
            }
            self.remove_land_unit(k);
        }
        self.cities[index].city_id = 0;
        for city in self.cities.iter_mut().filter(|c| c.city_id != 0) {
            for slot in (0..city.route_count.clamp(0, 3) as usize).rev() {
                if city.route_partners[slot] == index as i16 {
                    remove_route(city, slot);
                }
            }
        }
        for wonder in self.wonders.iter_mut().filter(|w| **w == index as i16) {
            *wonder = -2;
        }
        if let Some(i) = self.map.stored_index(x as u16, y as u16) {
            self.map.improvements[i] &= !2;
        }
        let width = (2 * self.map.stored_w) as i32;
        for (k, (dx, dy)) in crate::yields::RADIUS_DX
            .into_iter()
            .zip(crate::yields::RADIUS_DY)
            .chain(OUTER)
            .enumerate()
        {
            let tx = if self.game_flags & 0x8000 != 0 {
                x + dx
            } else {
                (x + dx).rem_euclid(width.max(1))
            };
            let ty = y + dy;
            if tx < 0 || ty < 0 {
                continue;
            }
            let Some(i) = self.map.stored_index(tx as u16, ty as u16) else {
                continue;
            };
            if self.map.terrain[i] == crate::movement::OCEAN_TERRAIN {
                continue;
            }
            self.map.fertility[i] |= 8;
            if k < 21 && self.map.site_bits[i] == owner {
                self.map.site_bits[i] = 0;
            }
        }
        if index + 1 == self.cities.len() {
            self.cities.pop();
        }
        true
    }
}
