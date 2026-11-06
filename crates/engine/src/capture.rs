//! Surface military occupation. Loot, building attrition and non-barbarian
//! population loss are native kernels; see capture-kernels.json. The enclosing
//! engine transition omits native capital relocation/civil war, technology
//! plunder, diplomacy notifications and civilization respawn. It is not a port
//! of the complete 0x53c765 capture dispatcher.
use crate::{combat::CrtRand, WorldState};
use tot_kit::Rules;

/// Native 0x53adc6, with positive signed-byte size and a nonnegative cache.
pub fn loot(size: u16, treasury: i32, population: i16) -> i32 {
    let size = i32::from(size.clamp(1, 127));
    let population = i32::from(population.max(0)) + 1;
    let result = if 32000 / size > treasury {
        size.wrapping_mul(treasury) / population
    } else {
        (treasury / population).wrapping_mul(size)
    };
    if result < 0 {
        32000
    } else {
        result
    }
}

/// Native attrition phase, after mandatory removal of improvements 1/4/7/11.
/// A single CRT draw chooses one alternating bit mask for all five bytes.
pub fn building_attrition(buildings: &mut [u8; 5], mode: u8, recapture: bool, rng: &mut CrtRand) {
    if mode == 0 && !recapture {
        let mask = 0xaa >> (rng.rand() % 2);
        for byte in buildings {
            *byte &= mask as u8;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PopulationInput {
    pub size: u16,
    pub recapture: bool,
    pub old_owner: u8,
    pub new_owner: u8,
    pub humans: u8,
    pub known_to: u8,
    pub new_rank: u8,
    pub reference_rank: u8,
}

/// Native military population phase for a non-barbarian conqueror. A size-one
/// AI city may survive according to human knowledge and the saved power ranks.
pub fn remaining_population(c: PopulationInput, rng: &mut CrtRand) -> u16 {
    if c.recapture && c.size <= 1 {
        return c.size;
    }
    let lose = if c.size > 1
        || c.humans & ((1 << c.old_owner) | (1 << c.new_owner)) != 0
        || c.humans & c.known_to != 0
        || c.new_rank == 7
    {
        true
    } else {
        c.reference_rank < c.new_rank && !rng.rand().is_multiple_of(2)
    };
    c.size.saturating_sub(u16::from(lose))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Occupation {
    pub city: usize,
    pub old_owner: u8,
    pub new_owner: u8,
    pub razed: bool,
    pub plunder: i32,
}

impl WorldState {
    /// Occupy an adjacent undefended surface city with a land combat unit.
    /// Validation precedes movement and all RNG/state changes. The explicit
    /// order preserves the meaning of pre-v0.10 `move` logs.
    pub fn occupy_city(
        &mut self,
        rules: &Rules,
        unit: usize,
        city: usize,
    ) -> Result<Occupation, &'static str> {
        let u = self
            .units
            .get(unit)
            .filter(|u| u.id != 0)
            .ok_or("no such unit")?;
        let c = self
            .cities
            .get(city)
            .filter(|c| c.city_id != 0 && c.map_id == 0)
            .ok_or("no such surface city")?;
        let owner = u.owner;
        let old_owner = (c.owner & 15) as u8;
        if !(1..8).contains(&owner) || old_owner >= 8 || old_owner == owner {
            return Err("not a foreign city");
        }
        if rules.unit_domain(u.type_id as usize) != Some(0)
            || rules.unit_attack(u.type_id as usize).unwrap_or(0) == 0
            || rules.unit_role(u.type_id as usize).is_none_or(|r| r > 1)
        {
            return Err("not an occupation unit");
        }
        if u.moves_left == 0 || !crate::movement::is_adjacent((u.x, u.y), (c.x, c.y)) {
            return Err("city not reachable");
        }
        if self
            .units
            .iter()
            .any(|u| u.id != 0 && u.owner != owner && (u.x, u.y) == (c.x, c.y))
        {
            return Err("city defended");
        }
        let (x, y, size, recapture, known_to) =
            (c.x, c.y, c.size, c.original_owner == owner, c.known_to);
        // Preflight terrain before mutating; move_unit charges the ordinary
        // terrain/ZOC cost and cannot fight because the target was checked.
        if self.map.terrain_at(x, y).is_none_or(|t| t == 10) {
            return Err("city terrain refused");
        }
        if !self.move_unit(rules, unit, x, y).moved {
            return Err("occupation move refused");
        }
        let plunder = loot(
            size,
            self.treasury[old_owner as usize],
            self.ai_state.population[old_owner as usize],
        )
        .clamp(0, self.treasury[old_owner as usize].max(0));
        self.treasury[old_owner as usize] = self.treasury[old_owner as usize].wrapping_sub(plunder);
        self.treasury[owner as usize] = self.treasury[owner as usize].wrapping_add(plunder);
        let c = &mut self.cities[city];
        c.shield_box = 0;
        c.flags &= !0x3b;
        if !recapture {
            for improvement in [1, 4, 7, 11] {
                c.buildings[improvement / 8] &= !(1 << (improvement % 8));
            }
        }
        building_attrition(&mut c.buildings, 0, recapture, &mut self.combat_rng);
        let reference = usize::from(self.ai_state.power_reference).min(7);
        let population = remaining_population(
            PopulationInput {
                size,
                recapture,
                old_owner,
                new_owner: owner,
                humans: self.human_mask,
                known_to,
                new_rank: self.ai_state.power_ranks[owner as usize],
                reference_rank: self.ai_state.power_ranks[reference],
            },
            &mut self.combat_rng,
        );
        let razed = population == 0;
        if razed {
            self.destroy_surface_city(rules, city);
        } else {
            self.displace_city_units(rules, city, Some(old_owner));
            let c = &mut self.cities[city];
            c.owner = u16::from(owner);
            c.original_owner = old_owner;
            c.size = population;
            c.building = 0; // the new owner's policy chooses through SetProduction
            c.flags |= crate::founding::CITY_REFRESH;
            self.upgrade_obsolete_units(rules, owner);
        }
        self.refresh_city_statistics();
        Ok(Occupation {
            city,
            old_owner,
            new_owner: owner,
            razed,
            plunder,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_capture_kernels() {
        let mut counts = [0; 3];
        for line in include_str!("../../../docs/evidence/capture-kernel-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let (kind, rest) = line.split_once(' ').unwrap();
            let v: Vec<i64> = rest
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            match kind {
                "loot" => {
                    assert_eq!(
                        loot(v[0] as u16, v[1] as i32, v[2] as i16),
                        v[3] as i32,
                        "{line}"
                    );
                    counts[0] += 1;
                }
                "buildings" => {
                    let mut buildings = std::array::from_fn(|i| v[3 + i] as u8);
                    let mut rng = CrtRand::new(v[2] as u32);
                    building_attrition(&mut buildings, v[0] as u8, v[1] != 0, &mut rng);
                    assert_eq!(buildings, std::array::from_fn(|i| v[8 + i] as u8), "{line}");
                    assert_eq!(rng.state(), v[13] as u32, "{line}");
                    counts[1] += 1;
                }
                "population" => {
                    let mut rng = CrtRand::new(v[5] as u32);
                    let c = PopulationInput {
                        size: v[0] as u16,
                        recapture: v[1] != 0,
                        humans: v[2] as u8,
                        known_to: v[3] as u8,
                        new_rank: v[4] as u8,
                        old_owner: 5,
                        new_owner: 6,
                        reference_rank: 3,
                    };
                    assert_eq!(remaining_population(c, &mut rng), v[6] as u16, "{line}");
                    assert_eq!(rng.state(), v[7] as u32, "{line}");
                    counts[2] += 1;
                }
                _ => panic!("unknown vector {line}"),
            }
        }
        assert_eq!(counts, [75, 192, 576]);
    }
}
