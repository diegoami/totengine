//! City happiness stages from `civ2.bak 0x4c4925..0x4c4fa9`.
//! Luxury output, garrison and military unrest are separate inputs so
//! arithmetic can be checked independently against original x86 execution.

use crate::WorldState;
use tot_kit::Rules;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Happiness {
    pub happy: i32,
    pub unhappy: i32,
    pub angry: i32,
}

/// Inputs to the original arithmetic stages. Wonders have already had
/// ownership and obsolescence applied; `local` marks the containing city.
pub struct HappinessInput {
    pub index: i32,
    pub size: i32,
    pub government: u8,
    pub difficulty: u8,
    pub human: bool,
    pub cities: i32,
    pub content: i32,
    pub empire: i32,
    pub flags: u16,
    pub specialists: i32,
    pub luxuries: i32,
    pub garrison: i32,
    pub away: i32,
    pub home: i32,
    pub buildings: u64,
    pub techs: u128,
    pub own: u32,
    pub local: u32,
}

fn normalize(h: &mut Happiness, size: i32, specialists: i32) {
    h.happy = h.happy.clamp(0, size);
    while h.angry != 0 && h.angry > h.unhappy {
        h.angry -= 1;
        h.unhappy += 1;
    }
    h.unhappy = h.unhappy.clamp(0, size);
    while (size - specialists).clamp(0, 99) < h.happy + h.unhappy {
        if h.angry != 0 {
            h.angry -= 1;
        } else {
            h.happy = (h.happy - 1).clamp(0, size);
        }
        h.unhappy = (h.unhappy - 1).clamp(0, size);
    }
    while h.angry != 0 && size - specialists > h.happy + h.unhappy {
        h.angry -= 1;
        h.unhappy += 1;
    }
}

pub fn calculate_happiness(v: &HappinessInput) -> Happiness {
    let has = |i: usize| v.buildings & (1u64 << i) != 0u64;
    let knows = |i: usize| (v.techs & (1u128 << i) != 0u128) as i32;
    let holds = |i: usize| v.own & (1u32 << i) != 0u32;
    let here = |i: usize| v.local & (1u32 << i) != 0u32;
    let gov = v.government;
    let mut unhappy = if v.human {
        let factor = ((v.empire - 2 * v.difficulty as i32 + if v.flags & 4 != 0 { 2 } else { 0 })
            * ((gov as i32 >> 1) + 2)
            / 2)
        .max(1);
        let mut initial = v.content - v.difficulty as i32;
        if initial <= 2 && has(1) && v.away == 0 && v.home == 0 {
            initial = 2;
        }
        v.size + 1 - initial
            + if gov == 3 {
                0
            } else {
                (v.cities - factor + v.index % factor) / factor
            }
    } else {
        v.size + 4 - v.content
    };
    let mut h = Happiness {
        happy: 0,
        angry: (unhappy - v.size).max(0),
        unhappy: unhappy.min(v.size),
    };
    normalize(&mut h, v.size, v.specialists);
    h.happy = v.luxuries >> 1;
    normalize(&mut h, v.size, v.specialists);
    if has(14) {
        h.unhappy -= 3 + knows(24);
    }
    if knows(55) != 0 && (has(11) || holds(10)) {
        h.unhappy -= 3 - knows(15) + knows(82);
    }
    if has(4) {
        h.unhappy -= (knows(56) + knows(9)) * if holds(5) { 2 } else { 1 };
    }
    if (has(7) || has(1)) && gov == 6 {
        h.happy += 1;
    }
    normalize(&mut h, v.size, v.specialists);
    if gov == 4 {
        h.unhappy = 0;
        h.angry = 0;
    } else if gov < 5 {
        unhappy = (v.garrison * if gov == 3 { 2 } else { 1 }).min(if gov == 3 { 6 } else { 3 });
        h.unhappy -= unhappy.max(0).min(h.unhappy.max(0));
    } else {
        let penalty = (!(holds(21) || has(33))) as i32 + (gov == 6) as i32;
        h.unhappy += (v.away - (gov == 5 && v.away != 0) as i32) * penalty;
    }
    normalize(&mut h, v.size, v.specialists);
    if holds(1) {
        h.happy += 1 + 2 * here(1) as i32;
    }
    if holds(27) {
        h.happy += 1;
    }
    if here(13) {
        h.unhappy = 0;
    }
    if holds(15) {
        h.unhappy -= 2;
    }
    normalize(&mut h, v.size, v.specialists);
    h
}

impl WorldState {
    pub fn city_happiness(&self, rules: &Rules, idx: usize) -> Happiness {
        self.city_happiness_with_luxuries(rules, idx, self.city_finance(rules, idx).luxuries)
    }

    /// Evaluate happiness with supplied luxury arrows, independently of
    /// worked-tile and route calculations (stored-record parity boundary).
    pub fn city_happiness_with_luxuries(
        &self,
        rules: &Rules,
        idx: usize,
        luxuries: i32,
    ) -> Happiness {
        let Some(c) = self.cities.get(idx) else {
            return Happiness::default();
        };
        let civ = (c.owner & 15) as usize;
        let mut own = 0;
        let mut local = 0;
        for wonder in 0..28 {
            if let Some(city) = self.active_wonder_city(rules, wonder) {
                if self.cities[city].owner == c.owner {
                    own |= 1 << wonder;
                    if city == idx {
                        local |= 1 << wonder;
                    }
                }
            }
        }
        let mut techs = 0u128;
        for (i, &byte) in self.techs[civ].iter().enumerate() {
            techs |= (byte as u128) << (8 * i);
        }
        let mut buildings = 0u64;
        for (i, &byte) in c.buildings.iter().enumerate() {
            buildings |= (byte as u64) << (8 * i);
        }
        let armed =
            |u: &&crate::Unit| u.id != 0 && rules.unit_attack(u.type_id as usize).unwrap_or(0) != 0;
        let garrison = self
            .units
            .iter()
            .filter(armed)
            .filter(|u| (u.x, u.y) == (c.x, c.y))
            .count() as i32;
        let mut away = 0;
        let mut home = 0;
        for u in self
            .units
            .iter()
            .filter(armed)
            .filter(|u| u.owner as usize == civ && u.home as usize == idx)
        {
            let here = (u.x, u.y) == (c.x, c.y);
            if here {
                home += 1;
            }
            if rules.unit_domain(u.type_id as usize) == Some(1) {
                if rules.unit_role(u.type_id as usize) != Some(3) {
                    away += 1;
                }
            } else if !here
                && !self
                    .cities
                    .iter()
                    .any(|p| p.city_id != 0 && (p.x, p.y) == (u.x, u.y))
            {
                let fortress = self
                    .map
                    .stored_index(u.x, u.y)
                    .is_some_and(|i| self.map.improvements[i] & 0x42 == 0x40);
                let near = fortress
                    && self.cities.iter().any(|p| {
                        p.city_id != 0 && p.owner == c.owner && {
                            // 0x566c40: wrapped Manhattan / 2.
                            let width = 2 * self.map.stored_w as i32;
                            let mut dx = (u.x as i32 - p.x as i32).abs();
                            if self.game_flags & 0x8000 == 0 && dx > width / 2 {
                                dx = width - dx;
                            }
                            (dx + (u.y as i32 - p.y as i32).abs()) / 2 <= 3
                        }
                    });
                if !near {
                    away += 1;
                }
            }
        }
        calculate_happiness(&HappinessInput {
            index: idx as i32,
            size: c.size as i32,
            government: self.governments[civ],
            difficulty: self.difficulty,
            human: self.human_mask & (1 << civ) != 0,
            cities: self.city_counts[civ] as i32,
            content: rules.cosmic_row(7).unwrap_or(7) as i32,
            empire: rules.cosmic_row(8).unwrap_or(14) as i32,
            flags: self.research_settings.flags,
            specialists: (0..16)
                .filter(|k| c.specialists >> (2 * k) & 3 != 0)
                .count() as i32,
            luxuries,
            garrison,
            away,
            home,
            buildings,
            techs,
            own,
            local,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unit_unrest_home_cities_aircraft_and_fortresses() {
        // The expected input cases execute the full original 0x4c1bdf
        // routine in tools/happiness_check.py, including stack/city lookup.
        for (x, y, domain, role, fortress, expected) in [
            (8, 8, 0, 0, false, 3),
            (16, 8, 0, 0, false, 3),
            (28, 8, 0, 0, false, 3),
            (20, 8, 0, 0, false, 5),
            (14, 8, 0, 0, true, 3),
            (40, 8, 0, 0, true, 5),
            (26, 8, 0, 0, true, 5),
            (8, 8, 1, 0, false, 5),
            (8, 8, 1, 3, false, 3),
            (20, 8, 1, 3, false, 3),
        ] {
            let rules = tot_kit::parse_rules(&format!(
                "@UNITS\nArmy,nil,{domain},1.,0,1a,1d,1h,1f,1,0,{role},nil,000000000000000\n"
            ));
            let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
            w.wonders = [-1; 28];
            w.techs = [[0; 13]; 8];
            w.human_mask = 0;
            w.governments[1] = 6;
            w.science_rates[1] = 5;
            w.tax_rates[1] = 5;
            w.game_flags = 0;
            w.cities.truncate(3);
            for (i, &(cx, cy, owner)) in [(8, 8, 1), (16, 8, 1), (28, 8, 2)].iter().enumerate() {
                let c = &mut w.cities[i];
                c.x = cx;
                c.y = cy;
                c.owner = owner;
                c.city_id = i as u16 + 1;
                c.size = 6;
                c.flags = 0;
                c.buildings = [0; 5];
                c.specialists = 0;
                c.worked = 0;
            }
            w.map = crate::MapState {
                stored_w: 32,
                stored_h: 16,
                terrain: vec![1; 512],
                landmass: vec![1; 512],
                special: vec![false; 512],
                river: vec![false; 512],
                road: vec![true; 512],
                rail: vec![false; 512],
                improvements: vec![16; 512],
                owner: vec![15; 512],
                visible: vec![255; 512],
                resource_seed: 0,
                huts_enabled: false,
            };
            let tile = w.map.stored_index(x, y).unwrap();
            if fortress {
                w.map.improvements[tile] |= 64;
            }
            let mut unit = w.units[0].clone();
            unit.id = 1;
            unit.owner = 1;
            unit.home = 0;
            unit.x = x;
            unit.y = y;
            unit.type_id = 0;
            w.units = vec![unit];
            assert_eq!(
                w.city_happiness(&rules, 0).unhappy,
                expected,
                "{x},{y}, domain={domain} role={role}"
            );
        }
    }
    #[test]
    fn arithmetic_matches_original_x86() {
        let mut count = 0;
        for line in include_str!("../../../docs/evidence/happiness-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let v: Vec<u128> = line
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            let input = HappinessInput {
                index: v[0] as i32,
                size: v[1] as i32,
                government: v[2] as u8,
                difficulty: v[3] as u8,
                human: v[4] != 0,
                cities: v[5] as i32,
                content: v[6] as i32,
                empire: v[7] as i32,
                flags: v[8] as u16,
                specialists: v[9] as i32,
                luxuries: v[10] as i32,
                garrison: v[11] as i32,
                away: v[12] as i32,
                home: v[13] as i32,
                buildings: v[14] as u64,
                techs: v[15],
                own: v[16] as u32,
                local: v[17] as u32,
            };
            assert_eq!(
                calculate_happiness(&input),
                Happiness {
                    happy: v[18] as i32,
                    unhappy: v[19] as i32,
                    angry: v[20] as i32
                },
                "{line}"
            );
            count += 1;
        }
        assert_eq!(count, 128);
    }
}
