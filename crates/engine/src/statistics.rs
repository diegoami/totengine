//! City/population caches and hut-facing landmass flags from original statistics.
//! Native admitted-city accumulation: `0x507794..0x507908`; enclosing filters
//! reject deleted cities and invalid coordinates (`0x507491..0x50754c`).
use crate::WorldState;
use tot_kit::Rules;

impl WorldState {
    /// Refresh the hut-facing flags from current surface military and city
    /// caches. The original's separate landmass build-strategy byte is not
    /// needed by hut selection and is not changed here.
    pub fn refresh_landmass_flags(&mut self, rules: &Rules) {
        let mut military = [[0u16; 256]; 8];
        let mut strength = [[0u16; 256]; 8];
        for unit in &self.units {
            if unit.id == 0 || unit.owner >= 8 {
                continue;
            }
            let Some(tile) = self.map.stored_index(unit.x, unit.y) else {
                continue;
            };
            if self.map.terrain[tile] == 10 {
                continue;
            }
            let attack = rules.unit_attack(unit.type_id as usize).unwrap_or(0) as u8 as i8;
            if attack >= 99 {
                continue;
            }
            let civ = unit.owner as usize;
            let land = self.map.landmass[tile] as usize;
            if rules.unit_role(unit.type_id as usize).unwrap_or(0) <= 4 {
                military[civ][land] = military[civ][land].wrapping_add(1);
            }
            strength[civ][land] = strength[civ][land].wrapping_add(attack as u16);
        }
        self.ai_state.landmass_flags = [[0; 256]; 8];
        for civ in 1..8 {
            for land in 1..255 {
                let own = strength[civ][land] as u32;
                for other in 1..8 {
                    let count = military[other][land] as u32;
                    let power = strength[other][land] as u32;
                    let cities = self.ai_state.city_landmasses[other][land];
                    if other == civ || count == 0 || (own / 4 >= power && cities == 0) {
                        continue;
                    }
                    let treaty = self.diplomacy[civ][other];
                    let hostile = treaty & 8192 != 0
                        || (treaty & 8 == 0
                            && treaty & 5 == 1
                            && self.ai_state.attitudes[civ][other] >= 50);
                    let flags = &mut self.ai_state.landmass_flags[civ][land];
                    if (hostile || treaty & 32 != 0)
                        && count >= own
                        && military[civ][land] as u32 <= power
                        && self.ai_state.city_landmasses[civ][land] != 0
                    {
                        *flags |= 16;
                    }
                    if treaty & 8 != 0 {
                        continue;
                    }
                    if cities != 0 {
                        *flags |= if treaty & 4 != 0 { 2 } else { 1 };
                    }
                    *flags |= if treaty & 4 != 0 { 8 } else { 4 };
                }
            }
        }
    }

    /// Refresh the own-city cache fields the arrival, hut and power routines
    /// read. Landmass military flags refresh separately after this cache.
    /// Non-surface cities contribute population/counts; their
    /// landmass cannot be resolved by the engine's surface-only map.
    pub fn refresh_city_statistics(&mut self) {
        self.city_counts = [0; 8];
        self.ai_state.population = [0; 8];
        self.ai_state.city_landmasses = [[0; 256]; 8];
        for city in &self.cities {
            if city.city_id == 0 {
                continue;
            }
            let Some(tile) = self.map.stored_index(city.x, city.y) else {
                continue;
            };
            let civ = (city.owner & 15) as usize;
            if civ >= 8 {
                continue;
            }
            self.city_counts[civ] = self.city_counts[civ].wrapping_add(1);
            self.ai_state.population[civ] =
                self.ai_state.population[civ].wrapping_add(city.size as u8 as i8 as i16);
            if city.map_id == 0 && self.map.terrain[tile] != 10 {
                let land = self.map.landmass[tile] as usize;
                self.ai_state.city_landmasses[civ][land] =
                    self.ai_state.city_landmasses[civ][land].wrapping_add(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn original_hut_facing_landmass_flags_and_world_adapter() {
        let mut cases = 0;
        for line in include_str!("../../../docs/evidence/landmass-flags-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let parts: Vec<_> = line.split('|').collect();
            let header: Vec<usize> = parts[0]
                .split_whitespace()
                .map(|v| v.parse().unwrap())
                .collect();
            let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
            let unit = w.units[0].clone();
            let city = w.cities[0].clone();
            w.units.clear();
            w.cities.clear();
            w.human_mask = header[1] as u8;
            w.diplomacy = [[0; 8]; 8];
            w.ai_state.attitudes = [[0; 8]; 8];
            w.map.terrain.fill(1);
            w.map.landmass.fill(1);
            let mut text = String::from("@UNITS\n");
            for (c, row) in parts[1].split_whitespace().enumerate() {
                let v: Vec<u32> = row.split(',').map(|x| x.parse().unwrap()).collect();
                text.push_str(&format!(
                    "Type{c},nil,0,1.,0,{}a,1d,1h,1f,1,0,{},nil,000000000000000\n",
                    v[1], v[2]
                ));
                for _ in 0..v[0] {
                    let mut u = unit.clone();
                    u.id = 1;
                    u.owner = c as u8;
                    u.type_id = c as u8;
                    u.x = 0;
                    u.y = 0;
                    w.units.push(u);
                }
                for _ in 0..v[3] {
                    let mut x = city.clone();
                    x.city_id = 1;
                    x.owner = c as u16;
                    x.x = 0;
                    x.y = 0;
                    x.map_id = 0;
                    w.cities.push(x);
                }
                w.diplomacy[header[0]][c] = v[4];
                w.ai_state.attitudes[header[0]][c] = v[5] as u8;
            }
            // Deleted and invalid slots must not inflate the native aggregates.
            let mut dead = unit.clone();
            dead.id = 0;
            w.units.push(dead);
            let mut invalid = unit;
            invalid.x = 60000;
            w.units.push(invalid);
            w.refresh_city_statistics();
            w.refresh_landmass_flags(&tot_kit::parse_rules(&text));
            let expected: u8 = parts[2].trim().parse().unwrap();
            assert_eq!(w.ai_state.landmass_flags[header[0]][1], expected, "{line}");
            assert_eq!(w.ai_state.landmass_flags[header[0]][0], 0);
            assert_eq!(w.ai_state.landmass_flags[header[0]][255], 0);
            cases += 1;
        }
        assert_eq!(cases, 512);
    }

    #[test]
    fn native_city_statistics_vectors() {
        let mut cases = 0;
        for line in include_str!("../../../docs/evidence/city-statistics-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let sections: Vec<_> = line.split('|').collect();
            let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
            let template = w.cities[0].clone();
            w.cities.clear();
            w.map.stored_w = 300;
            w.map.stored_h = 2;
            w.map.terrain = vec![1; 600];
            w.map.landmass = vec![0; 600];
            for (i, city) in sections[0].split_whitespace().enumerate() {
                let v: Vec<u16> = city.split(',').map(|s| s.parse().unwrap()).collect();
                let mut c = template.clone();
                c.city_id = i as u32 + 1;
                c.x = 2 * i as u16;
                c.y = 0;
                c.owner = 1;
                c.size = v[0];
                c.map_id = 0;
                w.map.landmass[i] = v[1] as u8;
                w.map.terrain[i] = if v[2] != 0 { 10 } else { 1 };
                w.cities.push(c);
            }
            // The enclosing original loop excludes dead/invalid records.
            let mut deleted = template.clone();
            deleted.city_id = 0;
            w.cities.push(deleted);
            let mut invalid = template;
            invalid.x = 60000;
            w.cities.push(invalid);
            w.refresh_city_statistics();
            let expected: Vec<i16> = sections[1]
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            assert_eq!(
                [w.city_counts[1], w.ai_state.population[1]],
                expected[..],
                "{line}"
            );
            let mut land = [0; 256];
            for pair in sections[2].split_whitespace() {
                let p: Vec<usize> = pair.split(',').map(|s| s.parse().unwrap()).collect();
                land[p[0]] = p[1] as u8;
            }
            assert_eq!(w.ai_state.city_landmasses[1], land, "{line}");
            cases += 1;
        }
        assert_eq!(cases, 128);
    }
}
