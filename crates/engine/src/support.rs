//! Original support charging and city disband choice (`0x4c18af`, `0x4c92a4`).
//! The selector accepts map-derived eligibility and distance separately.
#[derive(Debug, Clone)]
pub struct SupportedUnit {
    pub alive: bool,
    pub role: u8,
    pub owned: bool,
    pub homed: bool,
    pub same_map: bool,
    pub exempt: bool,
    /// Eligible for unrest disband: outside cities and protected fortresses.
    pub away_eligible: bool,
    pub distance: i32,
    /// Land unit's containing city, for the original shield refund.
    pub refund_city: Option<usize>,
    pub cost: i32,
}

pub struct SupportContext {
    pub size: i32,
    pub government: u8,
    pub human: bool,
    pub turn: i16,
    pub flags: u32,
    pub shields: i32,
    pub free: [i32; 3],
    pub rows: i32,
}

pub struct DisbandResult {
    pub flags: u32,
    pub removed: Vec<usize>,
    pub refunds: Vec<(usize, i32)>,
}

/// `ordinal` counts all role <6 units, including Fundamentalism-exempt types.
pub fn unit_cost(government: u8, size: i32, free: [i32; 3], ordinal: i32, exempt: bool) -> i32 {
    match government {
        0 | 1 => i32::from(ordinal > size),
        2 => i32::from(ordinal > free[0]),
        3 => i32::from(ordinal > free[1]),
        4 => i32::from(!exempt && ordinal > free[2]),
        _ => 1,
    }
}

pub fn select_disbands(c: &SupportContext, units: &[SupportedUnit]) -> DisbandResult {
    let mut alive: Vec<bool> = units.iter().map(|u| u.alive).collect();
    let mut result = DisbandResult {
        flags: c.flags,
        removed: Vec::new(),
        refunds: Vec::new(),
    };
    'restart: loop {
        let mut ordinal = 0;
        let mut support = 0;
        for (index, unit) in units.iter().enumerate() {
            if !alive[index] || !unit.owned || !unit.homed || unit.role >= 6 {
                continue;
            }
            ordinal += 1;
            support += unit_cost(c.government, c.size, c.free, ordinal, unit.exempt);
            let shortage = support > c.shields;
            if !shortage
                && !(result.flags & 1 != 0
                    && !c.human
                    && c.government >= 5
                    && (c.turn as i32 + index as i32) & 7 == 0)
            {
                continue;
            }
            let mut farthest = None;
            let mut distance = -1;
            for (k, candidate) in units.iter().enumerate() {
                if !alive[k] || !candidate.homed || candidate.role >= 6 || !candidate.same_map {
                    continue;
                }
                if !shortage && (candidate.role >= 5 || !candidate.away_eligible) {
                    continue;
                }
                if candidate.distance > distance {
                    distance = candidate.distance;
                    farthest = Some(k);
                }
            }
            let Some(k) = farthest else {
                continue;
            };
            if !shortage {
                if distance <= 0 {
                    continue;
                }
                if let Some(city) = units[k].refund_city {
                    result.refunds.push((city, units[k].cost * c.rows / 2));
                }
                result.flags &= !0x102001;
            }
            alive[k] = false;
            result.removed.push(k);
            continue 'restart;
        }
        return result;
    }
}

impl crate::WorldState {
    /// Original city support/unrest selection over the retained surface state.
    /// Land-unit removal and refunds; sea cargo and the game-options AI-stat
    /// branch remain outside this boundary.
    pub fn disband_city_units(
        &mut self,
        rules: &tot_kit::Rules,
        city: usize,
        shields: i32,
        rows: i32,
    ) -> Vec<usize> {
        let Some(c) = self
            .cities
            .get(city)
            .filter(|c| c.city_id != 0 && c.map_id == 0)
        else {
            return Vec::new();
        };
        let owner = (c.owner & 15) as u8;
        // Native selection can only mutate retained state on a shortage,
        // or in the AI Republic/Democracy unrest branch. Avoid building map
        // context for every world unit when neither trigger is possible.
        if self.unit_support(rules, city) <= shields
            && (c.flags & 1 == 0
                || self.human_mask & (1 << owner) != 0
                || self.governments[owner as usize] < 5)
        {
            return Vec::new();
        }

        let width = (2 * self.map.stored_w) as i32;
        let flat = self.game_flags & 0x8000 != 0;
        let units: Vec<_> = self
            .units
            .iter()
            .map(|u| {
                let tile = self.map.stored_index(u.x, u.y);
                let improvements = tile.map_or(0, |t| self.map.improvements[t]);
                let in_city = improvements & 0xc2 == 2;
                let protected = improvements & 0x42 == 0x40
                    && self.cities.iter().any(|c| {
                        if c.city_id == 0 || c.map_id != 0 || c.owner & 15 != owner as u16 {
                            return false;
                        }
                        let mut dx = (u.x as i32 - c.x as i32).abs();
                        if !flat && width / 2 < dx {
                            dx = width - dx;
                        }
                        (dx + (u.y as i32 - c.y as i32).abs()) / 2 <= 3
                    });
                SupportedUnit {
                    alive: u.id != 0,
                    role: rules.unit_role(u.type_id as usize).unwrap_or(0),
                    owned: u.owner == owner,
                    homed: u.home as usize == city,
                    same_map: true,
                    exempt: rules.unit_flags(u.type_id as usize).unwrap_or(0) & 0x800 != 0,
                    away_eligible: !in_city && !protected,
                    distance: crate::placement::diamond_distance(
                        width, flat, u.x as i32, u.y as i32, c.x as i32, c.y as i32,
                    ),
                    refund_city: (in_city && rules.unit_domain(u.type_id as usize) == Some(0))
                        .then(|| {
                            self.cities.iter().position(|c| {
                                c.city_id != 0 && c.map_id == 0 && (c.x, c.y) == (u.x, u.y)
                            })
                        })
                        .flatten(),
                    cost: rules.unit_cost(u.type_id as usize).unwrap_or(0) as u8 as i8 as i32,
                }
            })
            .collect();
        let result = select_disbands(
            &SupportContext {
                size: c.size as u8 as i8 as i32,
                government: self.governments[owner as usize],
                human: self.human_mask & (1 << owner) != 0,
                turn: self.turn,
                flags: c.flags,
                shields,
                free: std::array::from_fn(|k| rules.cosmic_row(13 + k).unwrap_or(0) as i32),
                rows,
            },
            &units,
        );
        self.cities[city].flags = result.flags;
        for (index, value) in result.refunds {
            self.cities[index].shield_box =
                (self.cities[index].shield_box as i16 as i32 + value) as u16;
        }
        for &k in &result.removed {
            self.remove_land_unit(k);
        }
        result.removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn world_disband_context_matches_original() {
        for line in include_str!("../../../docs/evidence/support-disband-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let (input, expected) = line.split_once('|').unwrap();
            let v: Vec<i32> = input
                .split_whitespace()
                .map(|x| x.parse().unwrap())
                .collect();
            let e: Vec<i32> = expected
                .split_whitespace()
                .map(|x| x.parse().unwrap())
                .collect();
            let mut text = String::from("@COSMIC\n");
            for k in 0..16 {
                text.push_str(&format!(
                    "{}\n",
                    if (13..16).contains(&k) {
                        v[6 + k - 13]
                    } else {
                        0
                    }
                ));
            }
            text.push_str("@UNITS\n");
            for k in 0..8 {
                let a = &v[10 + 7 * k..];
                text.push_str(&format!(
                    "U{k},nil,0,1.,0,1a,1d,1h,1f,{},0,{},nil,{:015b}\n",
                    k + 1,
                    a[1],
                    if a[6] != 0 { 0x800 } else { 0 }
                ));
            }
            let rules = tot_kit::parse_rules(&text);
            let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
            w.map.stored_w = 32;
            w.map.stored_h = 16;
            w.map.improvements = vec![0; 512];
            w.map.owner = vec![15; 512];
            w.map.terrain = vec![2; 512];
            w.cities = w.cities[..3].to_vec();
            for (k, (x, y)) in [(8, 8), (16, 8), (40, 8)].into_iter().enumerate() {
                let c = &mut w.cities[k];
                c.owner = 1;
                c.map_id = 0;
                c.city_id = k as u32 + 1;
                c.size = v[0] as u16;
                c.x = x;
                c.y = y;
                c.shield_box = 0;
                c.flags = if k == 0 { v[4] as u32 } else { 0 };
                let t = w.map.stored_index(x, y).unwrap();
                w.map.improvements[t] = 2;
                if k == 1 && v[3] & 1 != 0 {
                    w.map.improvements[t] |= 64;
                }
                w.map.owner[t] = 1;
            }
            for (x, y) in [(10, 8), (30, 8), (54, 8)] {
                let t = w.map.stored_index(x, y).unwrap();
                w.map.improvements[t] |= 64;
            }
            w.units = vec![w.units[0].clone(); 8];
            for k in 0..8 {
                let a = &v[10 + 7 * k..];
                let u = &mut w.units[k];
                u.id = a[0] as u32;
                u.type_id = k as u8;
                u.owner = a[2] as u8;
                u.home = a[3] as u8;
                u.x = a[4] as u16;
                u.y = a[5] as u16;
                u.stack_pos = 0;
            }
            w.human_mask = if v[2] != 0 { 2 } else { 0 };
            w.governments[1] = v[1] as u8;
            w.game_flags = 0;
            w.turn = v[3] as i16;
            let removed = w.disband_city_units(&rules, 0, v[5], v[9]);
            let mut actual = vec![w.cities[0].flags as i32];
            actual.extend(w.cities.iter().map(|c| c.shield_box as i16 as i32));
            actual.push(removed.len() as i32);
            actual.extend(removed.iter().map(|&k| k as i32));
            assert_eq!(actual, e, "{line}");
        }
    }

    #[test]
    fn original_disband_choices_and_refunds() {
        let mut cases = 0;
        for line in include_str!("../../../docs/evidence/support-disband-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let (input, expected) = line.split_once('|').unwrap();
            let v: Vec<i32> = input
                .split_whitespace()
                .map(|x| x.parse().unwrap())
                .collect();
            let e: Vec<i32> = expected
                .split_whitespace()
                .map(|x| x.parse().unwrap())
                .collect();
            let cities = [(8, 8), (16, 8), (40, 8)];
            let units: Vec<_> = (0..8)
                .map(|k| {
                    let a = &v[10 + 7 * k..];
                    let (x, y) = (a[4], a[5]);
                    let city = cities.iter().position(|&p| p == (x, y));
                    let near = cities.iter().any(|&(cx, cy)| {
                        let dx = (x - cx).abs();
                        let dx = dx.min(64 - dx);
                        (dx + (y - cy).abs()) / 2 <= 3
                    });
                    SupportedUnit {
                        alive: a[0] != 0,
                        role: a[1] as u8,
                        owned: a[2] == 1,
                        homed: a[3] == 0,
                        same_map: true,
                        exempt: a[6] != 0,
                        away_eligible: (city.is_none() || (city == Some(1) && v[3] & 1 != 0))
                            && !([(10, 8), (30, 8), (54, 8)].contains(&(x, y)) && near),
                        distance: crate::placement::diamond_distance(64, false, x, y, 8, 8),
                        refund_city: city.filter(|&i| i != 1 || v[3] & 1 == 0),
                        cost: k as i32 + 1,
                    }
                })
                .collect();
            let r = select_disbands(
                &SupportContext {
                    size: v[0],
                    government: v[1] as u8,
                    human: v[2] != 0,
                    turn: v[3] as i16,
                    flags: v[4] as u32,
                    shields: v[5],
                    free: [v[6], v[7], v[8]],
                    rows: v[9],
                },
                &units,
            );
            let mut boxes = [0i32; 3];
            for (city, value) in r.refunds {
                boxes[city] += value;
            }
            let mut actual = vec![
                r.flags as i32,
                boxes[0],
                boxes[1],
                boxes[2],
                r.removed.len() as i32,
            ];
            actual.extend(r.removed.iter().map(|&k| k as i32));
            assert_eq!(actual, e, "{line}");
            cases += 1;
        }
        assert_eq!(cases, 1026);
    }
}
