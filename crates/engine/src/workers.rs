//! Original worker allocation (`0x4c2c14`) with supplied tile yields and
//! availability. World/map context and turn-phase integration remain separate.
#[derive(Debug, Clone)]
pub struct WorkerContext {
    pub size: u8,
    pub worked: u32,
    pub flags: u32,
    pub human: bool,
    /// Current turn plus city index (original AI reselection period).
    pub phase_turn: i32,
    pub processing: bool,
    pub food_each: i32,
    pub settlers: i32,
    pub settler_food: i32,
    pub support: i32,
    pub food_box: i16,
    pub unavailable: [u8; 20],
    pub yields: [[i32; 3]; 21],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerResult {
    pub worked: u32,
    pub flags: u32,
    pub food_box: i16,
    pub gross: [i32; 3],
    pub overassigned: bool,
}

fn add(gross: &mut [i32; 3], yields: [i32; 3]) {
    for (g, y) in gross.iter_mut().zip(yields) {
        *g += y;
    }
}

pub fn allocate(c: &WorkerContext) -> WorkerResult {
    let mut worked = c.worked;
    let mut flags = c.flags;
    let mut wanted = (worked >> 26) as i32;
    let size = c.size as i8 as i32;
    if !c.human {
        if flags & 1 != 0 {
            worked = 0;
        } else if c.processing && wanted > 0 {
            if flags & 0x4000 == 0 {
                wanted -= 1;
            } else {
                flags |= 0x8000;
            }
        }
    }
    if wanted == 0 {
        flags &= !0x8000;
    }
    let blocked_force = c.unavailable.iter().any(|b| b & 32 != 0);
    for (i, &b) in c.unavailable.iter().enumerate() {
        if b != 0 {
            worked &= !(1 << i);
        }
    }
    let mut unavailable: [bool; 21] = std::array::from_fn(|i| i < 20 && c.unavailable[i] != 0);
    let mut budget = size + 1;
    let mut selected = 0u32;
    let mut gross = [0; 3];
    let mut warning = false;
    let preserve = c.human || c.phase_turn & 3 != 0 || flags & 1 != 0 || blocked_force;
    let mut done = false;
    if preserve {
        let count = (worked & 0x1fffff).count_ones() as i32;
        if budget - count >= 0 {
            for k in std::iter::once(20).chain(0..20) {
                if worked & (1 << k) != 0 {
                    add(&mut gross, c.yields[k]);
                    selected |= 1 << k;
                    unavailable[k] = true;
                    budget -= 1;
                    if budget == 0 {
                        break;
                    }
                }
            }
        } else {
            warning = true;
        }
        if budget - wanted == 0 || budget == 0 {
            done = true;
        }
    }
    let eaten = size * c.food_each + c.settlers * c.settler_food;
    if !done {
        if budget > 0 && !unavailable[20] {
            selected |= 1 << 20;
            add(&mut gross, c.yields[20]);
            budget -= 1;
        }
        let divisor = c.food_each.max(1);
        loop {
            if (eaten <= gross[0] + budget / divisor && size >= 3) || budget - wanted <= 0 {
                break;
            }
            let mut best = None;
            let mut best_food = 0;
            let mut best_other = 0;
            for (i, &blocked) in unavailable.iter().enumerate().take(20) {
                if blocked {
                    continue;
                }
                let [f, s, t] = c.yields[i];
                if budget == 1 && gross[1] == 0 && s == 0 {
                    continue;
                }
                let other = 2 * s + t;
                if f > best_food || (f == best_food && other > best_other) {
                    best = Some(i);
                    best_food = f;
                    best_other = other;
                }
            }
            let Some(i) = best else {
                break;
            };
            unavailable[i] = true;
            selected |= 1 << i;
            add(&mut gross, c.yields[i]);
            budget -= 1;
        }
        if !c.human && flags & 1 != 0 && c.processing {
            flags |= 0x8000;
            wanted += 1;
        }
        while budget - wanted > 0 {
            let mut best = None;
            let mut best_score = 0;
            for (i, &blocked) in unavailable.iter().enumerate().take(20) {
                if blocked {
                    continue;
                }
                let y = c.yields[i];
                let score = (16 / (gross[0] - eaten).clamp(1, 99)) * y[0]
                    + (size * 3 / (gross[1] - c.support).clamp(1, 99)) * y[1]
                    + (size * 2 / gross[2].clamp(1, 99)) * y[2];
                if score > best_score {
                    best = Some(i);
                    best_score = score;
                }
            }
            let Some(i) = best else {
                break;
            };
            unavailable[i] = true;
            selected |= 1 << i;
            add(&mut gross, c.yields[i]);
            budget -= 1;
        }
    }
    let mut box_now = c.food_box;
    if eaten <= gross[0] {
        if box_now < 0 {
            box_now = 0;
        }
    } else if box_now == 0 {
        box_now = -1;
    }
    WorkerResult {
        worked: (budget as u32).wrapping_shl(26).wrapping_add(selected),
        flags,
        food_box: box_now,
        gross,
        overassigned: warning,
    }
}

/// Normalize the packed specialist slots after worker allocation (`0x4c34b6`).
/// Existing types survive; newly free citizens become entertainers.
pub fn normalize_specialists(mut specialists: u32, worked: u32) -> u32 {
    let count = (worked >> 26).min(16);
    for slot in 0..16 {
        let shift = 2 * slot;
        if slot >= count {
            specialists &= !(3 << shift);
        } else if specialists >> shift & 3 == 0 {
            specialists |= 1 << shift;
        }
    }
    specialists
}

impl crate::WorldState {
    /// Radius availability from original `0x4c109f` and `0x4c13fe`.
    /// Surface context only; terrain yield is evaluated separately.
    pub fn worker_availability(&self, rules: &tot_kit::Rules, city_index: usize) -> [u8; 20] {
        use crate::yields::{RADIUS_DX as DX, RADIUS_DY as DY};
        let Some(city) = self.cities.get(city_index) else {
            return [1; 20];
        };
        let civ = (city.owner & 15) as usize;
        let width = (2 * self.map.stored_w) as i32;
        let flat = self.game_flags & 0x8000 != 0;
        let mut blocked = [0; 20];
        if width == 0 || civ >= 8 || city.map_id != 0 {
            return [1; 20];
        }
        for k in 0..20 {
            let raw_x = city.x as i32 + DX[k];
            let y = city.y as i32 + DY[k];
            let x = if flat { raw_x } else { raw_x.rem_euclid(width) };
            let index = if x >= 0 && y >= 0 {
                self.map.stored_index(x as u16, y as u16)
            } else {
                None
            };
            let Some(i) = index else {
                blocked[k] |= 1;
                continue;
            };
            if self.map.visible[i] & (1 << civ) == 0 {
                blocked[k] |= 1;
                continue;
            }
            let mut stack: Vec<_> = self
                .units
                .iter()
                .enumerate()
                .filter(|(_, u)| u.id != 0 && (u.x, u.y) == (x as u16, y as u16))
                .collect();
            stack.sort_by_key(|&(i, u)| (u.stack_pos, i));
            for (_, u) in stack {
                let owner = (u.owner | if u.is_leader { 16 } else { 0 }) as usize;
                if owner == civ
                    || self.map.terrain[i] == 10
                    || self.diplomacy[civ].get(owner).is_some_and(|f| f & 8 != 0)
                {
                    break;
                }
                if rules.unit_attack(u.type_id as usize).unwrap_or(0) > 0 {
                    blocked[k] |= 4;
                    if owner < 8 && self.human_mask & (1 << owner) != 0 {
                        blocked[k] |= 32;
                    }
                    break;
                }
            }
            if self.map.improvements[i] & 2 != 0 {
                blocked[k] |= 8;
            }
        }
        for (other_index, other) in self.cities.iter().enumerate() {
            if other_index == city_index || other.city_id == 0 || other.map_id != city.map_id {
                continue;
            }
            if crate::placement::diamond_distance(
                width,
                flat,
                city.x as i32,
                city.y as i32,
                other.x as i32,
                other.y as i32,
            ) > 5
            {
                continue;
            }
            for j in 0..21 {
                if j != 20 && other.worked & (1 << j) == 0 {
                    continue;
                }
                let x = other.x as i32 + DX[j];
                let y = other.y as i32 + DY[j];
                if crate::placement::diamond_distance(
                    width,
                    flat,
                    city.x as i32,
                    city.y as i32,
                    x,
                    y,
                ) > 2
                {
                    continue;
                }
                // The original compares raw offset coordinates here, after
                // its wrapped distance check; do not normalize these deltas.
                for k in 0..20 {
                    if DX[k] == x - city.x as i32 && DY[k] == y - city.y as i32 {
                        blocked[k] |= 16;
                    }
                }
            }
        }
        blocked
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_worker_allocation_with_tile_boundary() {
        let mut cases = 0;
        for line in include_str!("../../../docs/evidence/worker-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let parts: Vec<_> = line.split('|').collect();
            let v: Vec<i64> = parts[0]
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            let expected: Vec<i64> = parts[1]
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            let c = WorkerContext {
                size: v[0] as u8,
                worked: v[1] as u32,
                flags: v[2] as u32,
                human: v[3] != 0,
                phase_turn: v[4] as i32,
                processing: v[5] != 0,
                food_each: v[6] as i32,
                settlers: v[7] as i32,
                settler_food: v[8] as i32,
                support: v[9] as i32,
                food_box: v[10] as i16,
                unavailable: std::array::from_fn(|i| v[11 + i] as u8),
                yields: std::array::from_fn(|i| std::array::from_fn(|k| v[31 + 3 * i + k] as i32)),
            };
            let r = allocate(&c);
            let actual = [
                r.worked as i64,
                r.flags as i64,
                r.food_box as i64,
                r.gross[0] as i64,
                r.gross[1] as i64,
                r.gross[2] as i64,
                i64::from(r.overassigned),
            ];
            assert_eq!(actual, expected[..], "{line}");
            cases += 1;
        }
        assert_eq!(cases, 1024);
    }
}
