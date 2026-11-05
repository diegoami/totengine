//! Round-end surface barbarian arrivals, decoded from `0x46e6cd`.
//! Native comparison vectors isolate successful allocation and UI boundaries;
//! they do not establish complete original-game turn parity.
use crate::WorldState;
use tot_kit::Rules;

impl WorldState {
    fn barbarian_roll(&mut self, n: i32) -> i32 {
        if n > 1 {
            self.combat_rng.rand() as i32 % n
        } else {
            0
        }
    }

    fn barbarian_tech(&self, tech: usize, human_only: bool) -> bool {
        self.techs.iter().enumerate().any(|(c, bits)| {
            (!human_only || self.human_mask & (1 << c) != 0)
                && bits
                    .get(tech / 8)
                    .is_some_and(|b| b & (1 << (tech % 8)) != 0)
        })
    }

    fn arrival_occupied(&self, x: u16, y: u16) -> bool {
        self.units.iter().any(|u| u.id != 0 && (u.x, u.y) == (x, y))
    }

    /// `0x53adc6`: cached per-city share of its owner's treasury.
    fn arrival_chance(&self, city: usize) -> i32 {
        let c = &self.cities[city];
        let size = c.size as i8 as i32;
        let owner = (c.owner & 15) as usize;
        let Some(&money) = self.treasury.get(owner) else {
            return 0;
        };
        let denominator = self.ai_state.population[owner] as i32 + 1;
        if size == 0 || denominator == 0 {
            return 0;
        }
        let value = if 32000 / size > money {
            size.wrapping_mul(money) / denominator
        } else {
            (money / denominator).wrapping_mul(size)
        };
        if value < 0 {
            32000
        } else {
            value
        }
    }

    fn hostile_neighbour(&self, x: u16, y: u16) -> bool {
        let width = 2 * self.map.stored_w as i32;
        let origin_water = self
            .map
            .stored_index(x, y)
            .is_some_and(|i| self.map.terrain[i] == 10);
        [
            (1, -1),
            (2, 0),
            (1, 1),
            (0, 2),
            (-1, 1),
            (-2, 0),
            (-1, -1),
            (0, -2),
        ]
        .into_iter()
        .any(|(dx, dy)| {
            let mut nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if self.game_flags & 0x8000 == 0 {
                nx = nx.rem_euclid(width);
            }
            if nx < 0 || ny < 0 {
                return false;
            }
            let Some(i) = self.map.stored_index(nx as u16, ny as u16) else {
                return false;
            };
            if (self.map.terrain[i] == 10) != origin_water {
                return false;
            }
            self.units.iter().any(|u| {
                u.id != 0
                    && u.owner != 0
                    && (u.x, u.y) == (nx as u16, ny as u16)
                    && self.diplomacy[0]
                        .get(u.owner as usize)
                        .is_some_and(|f| f & 8 == 0)
            })
        })
    }

    /// Run the original surface arrival phase using the gameplay CRT.
    /// Call before incrementing the turn, as the original round close does.
    pub fn barbarian_arrivals(&mut self, rules: &Rules) -> Vec<usize> {
        let mut created = Vec::new();
        let level = (self.seed_word & 255) as i32;
        let turn = self.turn as i32;
        let difficulty = self.difficulty as i32;
        if !(1..=3).contains(&level) || turn == 0 {
            return created;
        }
        let minimum = if difficulty >= 5 {
            (3 - level) * 3 + 15
        } else {
            ((3 - level) * 3 + 30) * (5 - difficulty)
        };
        if turn < minimum {
            return created;
        }
        let tree = crate::research::TechTree::from_rules(rules);
        let advanced = rules
            .unit_prereq_raw(22)
            .and_then(|p| tree.by_abbrev(p))
            .is_some_and(|t| self.barbarian_tech(t, false));
        let fantasy = self.game_mode == 2;
        let gunpowder_rules = self.game_flags & 1 != 0;
        let width = (self.map.stored_w * 2) as i32;
        let height = self.map.stored_h as i32;
        if width == 0 || height == 0 {
            return created;
        }
        if (turn + 1) & (if level == 1 { 15 } else { 7 }) == 0 && !advanced {
            let mut site = None;
            for attempt in 0..1001 {
                let mut x = self.barbarian_roll(width);
                let y = 3 + self.barbarian_roll(height - 6);
                x = (x & !1) + (y & 1);
                if attempt == 1000 {
                    return created;
                }
                if let Some(i) = self.map.stored_index(x as u16, y as u16) {
                    if self.map.terrain[i] == 10
                        && !self.arrival_occupied(x as u16, y as u16)
                        && self
                            .map
                            .landmass_values
                            .get(self.map.landmass[i] as usize)
                            .is_some_and(|&v| v >= 16)
                    {
                        site = Some((x as u16, y as u16));
                        break;
                    }
                }
            }
            if let Some((x, y)) = site {
                let count = (turn / (250 - 50 * level) + 1).clamp(1, 5) + 1 + i32::from(level == 3);
                let boat = if fantasy || self.barbarian_tech(45, false) {
                    35
                } else if self.barbarian_tech(57, false) {
                    33
                } else {
                    32
                };
                let mut passenger = if gunpowder_rules { 4 } else { 2 };
                if self.barbarian_tech(39, true) {
                    passenger = 5;
                }
                if fantasy {
                    passenger = 51;
                }
                if self.barbarian_tech(51, false) || (level == 3 && self.barbarian_tech(11, false))
                {
                    passenger = if x & 1 != 0 && gunpowder_rules {
                        if fantasy {
                            17
                        } else {
                            18
                        }
                    } else if fantasy {
                        49
                    } else {
                        19
                    };
                }
                if self.barbarian_tech(81, true) && gunpowder_rules {
                    passenger = if fantasy { 38 } else { 20 };
                }
                let first = created.len();
                for i in (0..count).rev() {
                    let t = if i == count - 1 { 46 } else { passenger };
                    if let Some(idx) = self.spawn_unit(rules, t, 0, x, y) {
                        self.units[idx].orders = 3;
                        created.push(idx);
                    }
                }
                if let Some(idx) = self.spawn_unit(rules, boat, 0, x, y) {
                    self.units[idx].veteran = self.combat_rng.rand().is_multiple_of(2);
                    created.push(idx);
                } else {
                    for idx in created.drain(first..) {
                        self.units[idx].id = 0;
                    }
                }
            }
        }
        // The original counts all surface slots, then advances past the
        // chosen slot before validating it. Preserve that indexing behavior.
        let mut choice =
            self.barbarian_roll(self.cities.iter().filter(|c| c.map_id == 0).count() as i32);
        let mut city = 0;
        while choice >= 0 && city < self.cities.len() {
            if self.cities[city].map_id == 0 {
                choice -= 1;
            }
            city += 1;
        }
        let Some(c) = self.cities.get(city) else {
            return created;
        };
        if c.city_id == 0 || c.map_id != 0 || self.human_mask & (1 << (c.owner & 7)) == 0 {
            return created;
        }
        let chance = self.arrival_chance(city);
        if self.combat_rng.rand() as i32 % 100 >= chance {
            return created;
        }
        let mut site = None;
        for attempt in 1..=50 * level {
            let dx = 2 * (self.combat_rng.rand() as i32 % 13) - 6;
            let mut x = self.cities[city].x as i32 + dx;
            if self.game_flags & 0x8000 == 0 {
                x = x.rem_euclid(width);
            }
            let y = self.cities[city].y as i32 + self.combat_rng.rand() as i32 % 13 - 6;
            x = (x & !1) + (y & 1);
            let index = if x >= 0 && y >= 0 {
                self.map.stored_index(x as u16, y as u16)
            } else {
                None
            };
            let mut distance = 0;
            if index.is_some() {
                let Some((nearest, d)) = self.nearest_city(x as u16, y as u16) else {
                    return created;
                };
                city = nearest;
                distance = d;
            }
            if attempt >= 50 * level {
                return created;
            }
            let Some(i) = index else {
                continue;
            };
            let (x, y) = (x as u16, y as u16);
            let impassable = rules
                .section("TERRAIN")
                .and_then(|s| s.rows.get(self.map.terrain[i] as usize))
                .and_then(|r| r.get(15))
                .is_some_and(|v| v.eq_ignore_ascii_case("yes"));
            let c = &self.cities[city];
            let city_landmass = self
                .map
                .stored_index(c.x, c.y)
                .map(|k| self.map.landmass[k]);
            if distance >= 5
                && !self.hostile_neighbour(x, y)
                && self.map.terrain[i] != 10
                && !impassable
                && !self.arrival_occupied(x, y)
                && city_landmass == Some(self.map.landmass[i])
            {
                site = Some((x, y));
                break;
            }
        }
        let Some((x, y)) = site else {
            return created;
        };
        // Switch cases 1 and 2 fall through to case 3 in the original.
        let size = self.cities[city].size as i8 as i32;
        let mut count = self.barbarian_roll(size);
        if level == 3 && turn >= 150 && self.barbarian_roll(10 - difficulty) == 0 {
            count += size / 2 + self.barbarian_roll(size / 2);
        }
        count = count.clamp(2, 99);
        let mut first = if fantasy { 17 } else { 16 };
        let mut second = if fantasy { 17 } else { 15 };
        if self.combat_rng.rand().is_multiple_of(3) {
            std::mem::swap(&mut first, &mut second);
        }
        if self.barbarian_tech(35, false) && gunpowder_rules {
            first = if fantasy { 51 } else { 4 };
            second = if fantasy { 49 } else { 17 };
        }
        if self.barbarian_tech(37, false) {
            first = if fantasy { 25 } else { 24 };
            second = if fantasy { 40 } else { 7 };
            if advanced {
                second = if fantasy { 40 } else { 11 };
                if gunpowder_rules {
                    first = 25;
                    if self.barbarian_tech(34, false) {
                        let reference = self.research_settings.reference_civ as usize;
                        let c = &self.cities[city];
                        if self.governments.get(reference).is_some_and(|&g| g >= 5)
                            && c.owner == c.original_owner as u16
                        {
                            second = if fantasy { 49 } else { 8 };
                        } else {
                            first = if fantasy { 38 } else { 21 };
                            second = if fantasy { 38 } else { 9 };
                        }
                    }
                }
            }
        }
        for i in (0..=count).rev() {
            let t = if i == count {
                46
            } else if i & 4 != 0 {
                first
            } else {
                second
            };
            if let Some(idx) = self.spawn_unit(rules, t, 0, x, y) {
                self.units[idx].veteran = t == if fantasy { 49 } else { 8 };
                created.push(idx);
            }
        }
        created
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;
    #[test]
    fn original_arrival_requests_and_final_crt() {
        let mut text = String::from("@CIVILIZE\n");
        for t in 0..100 {
            writeln!(text, "Tech{t},5,1,nil,nil,0,0 ; T{t}").unwrap();
        }
        text.push_str("@UNITS\n");
        for t in 0..80 {
            let prereq = if t == 22 { "T50" } else { "nil" };
            writeln!(
                text,
                "Unit{t},nil,0,1.,0,1a,1d,1h,1f,1,0,1,{prereq},000000000000000"
            )
            .unwrap();
        }
        text.push_str("@TERRAIN\n");
        for t in 0..11 {
            writeln!(text, "Terrain{t},1,0,0,0,0,0,0,0,0,0,0,0,0,0,no").unwrap();
        }
        let rules = tot_kit::parse_rules(&text);
        let mut cases = 0;
        for line in include_str!("../../../docs/evidence/barbarian-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let sections: Vec<_> = line.split('|').collect();
            let v: Vec<i64> = sections[0]
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            let mut w = crate::boot(&crate::tests_support::synthetic_save(), v[0] as u64);
            w.turn = v[1] as i16;
            w.seed_word = v[2] as u16;
            w.difficulty = v[3] as u8;
            w.game_mode = v[4] as u8;
            w.game_flags = v[5] as u16;
            w.human_mask = v[11] as u8;
            w.units.clear();
            w.cities.truncate(2);
            for (i, c) in w.cities.iter_mut().enumerate() {
                c.x = 20 + 12 * i as u16;
                c.y = 24;
                c.map_id = 0;
                c.owner = 1;
                c.size = v[8] as u16;
                c.original_owner = v[13] as u8;
                c.city_id = i as u32 + 1;
            }
            w.treasury[1] = v[9] as i32;
            w.ai_state.population[1] = v[10] as i16;
            w.research_settings.reference_civ = 1;
            w.governments[1] = v[12] as u8;
            w.techs = [[0; 13]; 8];
            for (i, t) in [11, 34, 35, 37, 39, 45, 50, 51, 57, 81]
                .into_iter()
                .enumerate()
            {
                if v[6] & (1 << i) != 0 {
                    w.techs[1][t / 8] |= 1 << (t % 8);
                }
            }
            w.map.stored_w = 32;
            w.map.stored_h = 48;
            w.map.terrain = (0..1536)
                .map(|i| {
                    if v[7] == 1 || (v[7] == 2 && i / 32 < 20) {
                        10
                    } else {
                        1
                    }
                })
                .collect();
            w.map.landmass = vec![1; 1536];
            w.map.landmass_values = vec![100; 256];
            let indices = w.barbarian_arrivals(&rules);
            let actual: Vec<_> = indices
                .iter()
                .map(|&i| {
                    let u = &w.units[i];
                    format!(
                        "{},{},{},{},{}",
                        u.type_id,
                        u.x,
                        u.y,
                        u.orders,
                        u8::from(u.veteran)
                    )
                })
                .collect();
            assert_eq!(actual.join(" "), sections[2].trim(), "{line}");
            assert_eq!(
                w.combat_rng.state().to_string(),
                sections[1].trim(),
                "{line}"
            );
            cases += 1;
        }
        assert_eq!(cases, 1024);
    }
}
