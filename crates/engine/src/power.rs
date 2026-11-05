//! Original power ranking (`0x46de84..0x46e354`), excluding its diplomacy tail.
use crate::WorldState;

fn rank(mut scores: [i32; 8], alive: u8, human: u8) -> ([u8; 8], [u8; 8], u8, u8) {
    let mut ranks = [0; 8];
    for step in 1..8 {
        let mut best = -1;
        let mut civ = 0;
        for (c, &score) in scores.iter().enumerate().skip(1) {
            if score > best {
                best = score;
                civ = c;
            }
        }
        ranks[civ] = 8 - step;
        scores[civ] = -1;
    }
    let mut order = [0; 8];
    for civ in 1..8 {
        if alive & (1 << civ) != 0 {
            order[ranks[civ] as usize] = civ as u8;
        }
    }
    let strong = (0..8)
        .rev()
        .find(|&r| human & (1 << order[r]) != 0)
        .unwrap_or(0) as u8;
    let weak = (0..8).find(|&r| human & (1 << order[r]) != 0).unwrap_or(0) as u8;
    (ranks, order, strong, weak)
}

impl WorldState {
    /// Return the original score per civ. Its unit-strength sum is computed
    /// in the executable but never added to the score before sorting.
    pub fn power_scores(&self) -> Option<[i32; 8]> {
        let scenario = self.research_settings.flags & 128 != 0;
        let flags = if scenario {
            self.ai_state.scenario_flags?
        } else {
            0
        };
        Some(std::array::from_fn(|c| {
            if c == 0 || self.ai_state.alive_mask & (1 << c) == 0 {
                return 0;
            }
            if scenario && flags & 2 != 0 {
                self.cities
                    .iter()
                    .enumerate()
                    .filter(|(_, city)| {
                        city.city_id != 0 && city.map_id == 0 && city.owner == c as u16
                    })
                    .map(|(i, city)| {
                        let objectives = i32::from(city.flags & 0x04000000 != 0);
                        let wonders = if flags & 4 != 0 {
                            self.wonders.iter().filter(|&&idx| idx == i as i16).count() as i32
                        } else {
                            0
                        };
                        2 * (objectives + wonders)
                    })
                    .sum()
            } else {
                3 * self.research[c].acquired as i32
                    + 8 * self.ai_state.population[c] as i32
                    + (self.treasury[c] >> 5)
            }
        }))
    }

    /// Round-close ranking refresh. The original reference bytes store rank
    /// indices, which cost/hut consumers subsequently use literally.
    pub fn refresh_power(&mut self) {
        let Some(scores) = self.power_scores() else {
            return;
        };
        let (ranks, order, strong, weak) = rank(scores, self.ai_state.alive_mask, self.human_mask);
        self.ai_state.power_ranks = ranks;
        self.power_order = order;
        self.research_settings.reference_civ = strong;
        self.ai_state.power_reference = weak;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn original_scores_ranks_and_rank_references() {
        let mut cases = 0;
        for line in include_str!("../../../docs/evidence/power-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let parts: Vec<_> = line.split('|').collect();
            let v: Vec<i32> = parts[0]
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            let expected: Vec<i32> = parts[1]
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
            w.ai_state.alive_mask = v[0] as u8;
            w.human_mask = v[1] as u8;
            w.research_settings.flags = v[2] as u16;
            w.ai_state.scenario_flags = Some(v[3] as u16);
            w.cities.truncate(7);
            for c in 1..8 {
                let p = 4 + (c - 1) * 3;
                w.research[c].acquired = v[p] as u8;
                w.ai_state.population[c] = v[p + 1] as i16;
                w.treasury[c] = v[p + 2];
                let city = &mut w.cities[c - 1];
                city.city_id = c as u32;
                city.map_id = 0;
                city.owner = c as u16;
                city.flags = v[25 + c - 1] as u32;
            }
            for (i, value) in w.wonders.iter_mut().enumerate() {
                *value = v[32 + i] as i16;
            }
            let scores = w.power_scores().unwrap();
            assert_eq!(&scores[1..], &expected[..7], "{line}");
            w.refresh_power();
            assert_eq!(
                w.ai_state.power_ranks.map(i32::from),
                expected[7..15],
                "{line}"
            );
            assert_eq!(w.power_order.map(i32::from), expected[15..23], "{line}");
            assert_eq!(
                [
                    w.research_settings.reference_civ as i32,
                    w.ai_state.power_reference as i32
                ],
                expected[23..25],
                "{line}"
            );
            cases += 1;
        }
        assert_eq!(cases, 256);
    }
}
