//! City-count/population cache slice of the original per-civ statistics pass.
//! Native admitted-city accumulation: `0x507794..0x507908`; enclosing filters
//! reject deleted cities and invalid coordinates (`0x507491..0x50754c`).
use crate::WorldState;

impl WorldState {
    /// Refresh the own-city cache fields the arrival, hut and power routines
    /// read. Other AI statistics, including landmass strategy flags, remain
    /// separate. Non-surface cities contribute population/counts; their
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
