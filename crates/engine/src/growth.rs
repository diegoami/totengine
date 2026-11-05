//! Native growth preparation, before collection and worker assignment (#13).
//! This replaces the earlier observed-series approximation once the complete
//! city phase is integrated. It is not yet the turn loop's growth path.
use crate::production::production_rows;

/// Original food-box rows (`0x4c19e0`): the late adjustment starts strictly
/// after turn 200, unlike the shield function's inclusive threshold.
#[allow(clippy::too_many_arguments)] // Independently executed native inputs.
pub fn food_rows(
    cosmic: i32,
    difficulty: u8,
    humans: u8,
    civ: u8,
    turn: i16,
    leader: u8,
    wonders: [i16; 2],
    masks: [u8; 3],
) -> i32 {
    production_rows(
        cosmic,
        difficulty,
        humans,
        civ,
        turn.saturating_sub(1),
        leader,
        wonders,
        masks,
    )
}

#[derive(Debug, Clone, Copy)]
pub struct GrowthContext {
    pub size: u8,
    pub food_box: i16,
    pub rows: i32,
    pub cosmic_rows: u8,
    pub granary: bool,
    pub pyramids: bool,
    pub aqueduct: bool,
    pub sewer: bool,
    pub aqueduct_limit: u8,
    pub sewer_limit: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrowthResult {
    pub size: u8,
    pub food_box: i16,
    pub blocked_by: Option<u8>,
}

/// `0x4c5cdb..0x4c5dd2` plus the blocked-growth size rollback at `0x4c5de9`.
/// The caller must handle negative boxes through the starvation branch first.
/// Food collection happens later in `0x4cb1da..0x4cb23d`.
pub fn prepare_growth(c: GrowthContext) -> GrowthResult {
    let mut result = GrowthResult {
        size: c.size,
        food_box: c.food_box,
        blocked_by: None,
    };
    if c.food_box < 0 || (c.size as i8 as i32 + 1) * c.rows > c.food_box as i32 {
        return result;
    }
    let blocked = if c.size as i8 as i32 >= c.aqueduct_limit as i32 && !c.aqueduct {
        Some(9)
    } else if c.size as i8 as i32 >= c.sewer_limit as i32 && !c.sewer {
        Some(23)
    } else {
        None
    };
    let next = c.size.wrapping_add(1);
    result.food_box = if c.granary || c.pyramids {
        ((next as i8 as i32 + 1) * (c.cosmic_rows as i32 / 2)) as i16
    } else {
        0
    };
    result.size = if blocked.is_some() { c.size } else { next };
    result.blocked_by = blocked;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_food_rows() {
        let mut cases = 0;
        for line in include_str!("../../../docs/evidence/food-row-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let v: Vec<i32> = line
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            assert_eq!(
                food_rows(
                    v[0],
                    v[1] as u8,
                    v[2] as u8,
                    v[3] as u8,
                    v[4] as i16,
                    v[5] as u8,
                    [v[6] as i16, v[7] as i16],
                    [v[8] as u8, v[9] as u8, v[10] as u8]
                ),
                v[14],
                "{line}"
            );
            cases += 1;
        }
        assert_eq!(cases, 1024);
    }

    #[test]
    fn original_growth_threshold_granary_and_building_limits() {
        let mut cases = 0;
        for line in include_str!("../../../docs/evidence/city-growth-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let v: Vec<i32> = line
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            let result = prepare_growth(GrowthContext {
                size: v[0] as u8,
                food_box: v[1] as i16,
                rows: v[2],
                cosmic_rows: v[3] as u8,
                granary: v[4] & 1 != 0,
                aqueduct: v[4] & 2 != 0,
                sewer: v[4] & 4 != 0,
                pyramids: v[5] != 0,
                aqueduct_limit: v[6] as u8,
                sewer_limit: v[7] as u8,
            });
            assert_eq!(
                (
                    result.size as i32,
                    result.food_box as i32,
                    result.blocked_by.unwrap_or(0) as i32
                ),
                (v[8], v[9], v[10]),
                "{line}"
            );
            cases += 1;
        }
        assert_eq!(cases, 1024);
    }
}
