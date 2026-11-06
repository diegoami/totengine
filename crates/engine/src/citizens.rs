//! Immediate city citizen edits. Native transition evidence: citizen-edits.json.
use crate::{player::TileView, yields, WorldState};
use tot_kit::Rules;

/// Raw original tile-click transition, before allocation/totals refresh.
pub fn toggle_worker(worked: u32, tile: u8, blocked: bool) -> Result<u32, &'static str> {
    if tile > 20 {
        return Err("invalid city radius tile");
    }
    if tile == 20 {
        return Ok(0); // Centre click requests automatic allocation.
    }
    let bit = 1u32 << tile;
    if worked & bit != 0 {
        if worked >> 26 == 63 {
            return Err("specialist count capacity reached");
        }
        Ok((worked & !bit) + (1 << 26))
    } else if blocked {
        Err("city tile is unavailable")
    } else if worked >> 26 == 0 {
        Ok(0) // Native empty-tile click without a spare citizen reassigns automatically.
    } else {
        Ok((worked | bit) - (1 << 26))
    }
}

/// Original specialist cycling after the caller validates the active slot.
pub fn cycle_role(size: u16, packed: u32, slot: u8) -> Result<u32, &'static str> {
    if slot >= 16 {
        return Err("only the first 16 specialist roles are editable");
    }
    let shift = 2 * slot;
    let old = (packed >> shift) & 3;
    if old == 0 {
        return Err("specialist slot is unused");
    }
    let next = if size < 5 {
        if old == 1 {
            return Err("tax collectors and scientists require population five");
        }
        1
    } else {
        old % 3 + 1
    };
    Ok((packed & !(3 << shift)) | (next << shift))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadiusTile {
    pub position: Option<(u16, u16)>,
    pub terrain: Option<TileView>,
    pub worked: bool,
    /// Legal availability only; no foreign city/worker identities are exposed.
    pub available: bool,
    pub yields: [i32; 3],
}

/// Additional detail for an own surface city only. Reading never refreshes caches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CityCitizens {
    pub index: usize,
    pub radius: [RadiusTile; 21],
    /// Packed roles 1 entertainer / 2 tax collector / 3 scientist. Slots above
    /// the native sixteen stored roles are fixed entertainers.
    pub specialists: Vec<u8>,
    pub food_box: i16,
    pub growth_threshold: i32,
    pub support: i32,
}

impl WorldState {
    pub fn city_citizens(&self, rules: &Rules, civ: u8, index: usize) -> Option<CityCitizens> {
        let c = self.cities.get(index)?;
        if civ >= 8 || c.city_id == 0 || c.owner & 15 != u16::from(civ) || c.map_id != 0 {
            return None;
        }
        let blocked = self.worker_availability(rules, index);
        let width = (self.map.stored_w * 2) as i32;
        let flat = self.game_flags & 0x8000 != 0;
        let radius = std::array::from_fn(|k| {
            let raw_x = i32::from(c.x) + yields::RADIUS_DX[k];
            let y = i32::from(c.y) + yields::RADIUS_DY[k];
            let x = if flat || width == 0 {
                raw_x
            } else {
                raw_x.rem_euclid(width)
            };
            let position = (x >= 0 && y >= 0).then_some((x as u16, y as u16));
            let tile = position.and_then(|(x, y)| self.map.stored_index(x, y));
            let known = tile.filter(|&i| self.map.visible[i] & (1 << civ) != 0);
            RadiusTile {
                position: position.filter(|_| tile.is_some()),
                terrain: known.map(|i| TileView {
                    terrain: self.map.terrain[i],
                    river: self.map.river[i],
                    special: self.map.special[i],
                    road: self.map.road[i],
                    rail: self.map.rail[i],
                }),
                worked: c.worked & (1 << k) != 0,
                available: known.is_some() && (k == 20 || blocked[k] == 0),
                yields: if known.is_some() {
                    self.tile_yield(rules, index, k)
                } else {
                    [0; 3]
                },
            }
        });
        Some(CityCitizens {
            index,
            radius,
            specialists: (0..c.worked >> 26)
                .map(|slot| {
                    if slot < 16 {
                        ((c.specialists >> (2 * slot)) & 3) as u8
                    } else {
                        1
                    }
                })
                .collect(),
            food_box: c.food_box as i16,
            growth_threshold: self
                .city_growth_context(rules, index)
                .map_or(0, |g| (i32::from(c.size) + 1) * g.rows),
            support: self.unit_support(rules, index),
        })
    }

    pub fn edit_citizen(
        &mut self,
        rules: &Rules,
        civ: u8,
        index: usize,
        slot: u8,
        specialist: bool,
    ) -> Result<(), &'static str> {
        let c = self.cities.get(index).ok_or("no such city")?;
        if c.city_id == 0 || c.owner & 15 != u16::from(civ) {
            return Err("not your city");
        }
        if civ >= 8
            || self.human_mask & (1 << civ) == 0
            || c.map_id != 0
            || !(1..=127).contains(&c.size)
        {
            return Err("citizen editing requires a supported human surface city");
        }
        // Validate before the first mutation; rejected input is completely inert.
        if specialist {
            if u32::from(slot) >= c.worked >> 26 {
                return Err("specialist slot is unused");
            }
            let packed = cycle_role(c.size, c.specialists, slot)?;
            self.cities[index].specialists = packed;
        } else {
            let blocked =
                slot < 20 && self.worker_availability(rules, index)[usize::from(slot)] != 0;
            let worked = toggle_worker(c.worked, slot, blocked)?;
            self.cities[index].worked = worked;
        }
        self.refresh_city_totals(rules, index, false)
            .expect("validated live surface city");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_packed_citizen_edits() {
        let mut cases = 0;
        for line in include_str!("../../../docs/evidence/citizen-edit-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let v = line
                .split_whitespace()
                .map(|s| s.parse::<u32>().unwrap())
                .collect::<Vec<_>>();
            let (worked, packed) = if v[0] == 0 {
                (
                    toggle_worker(v[2], v[4] as u8, v[5] != 0).unwrap_or(v[2]),
                    v[3],
                )
            } else {
                (
                    v[2],
                    cycle_role(v[1] as u16, v[3], v[4] as u8).unwrap_or(v[3]),
                )
            };
            assert_eq!((worked, packed), (v[6], v[7]), "{line}");
            if v[0] == 1 {
                assert_eq!(
                    u32::from(cycle_role(v[1] as u16, v[3], v[4] as u8).is_err()),
                    v[8]
                );
            }
            cases += 1;
        }
        assert_eq!(cases, 1548);
    }
}
