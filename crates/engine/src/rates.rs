//! Human tax/luxury/science shares: legal caps, the slider redistribution rule
//! and the validated change (#194/#195).
//!
//! Evidence: docs/evidence/README.md §Rates, revolution and government selection.
//! Cap `0x40966b` and redistribution `0x409704` were executed natively
//! (docs/evidence/rates-government-vectors.txt). Shares are tenths: tax,
//! luxury and science always total 10, and each is at most the government's cap.
//! The engine validates committed shares only; lock state and the draft belong
//! to the client, which may use [`redistribute`] to reproduce the original
//! slider behavior.
use tot_kit::Rules;

use crate::WorldState;

/// Which share a slider edits. Dialog order is tax, luxury, science.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slider {
    Tax,
    Luxury,
    Science,
}

impl Slider {
    fn index(self) -> usize {
        match self {
            Slider::Tax => 0,
            Slider::Luxury => 1,
            Slider::Science => 2,
        }
    }
}

/// Committed shares and the limit that applies to each of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RatesView {
    pub government: u8,
    /// Per-share cap in tenths (`0x40966b`).
    pub cap: u8,
    pub tax: u8,
    pub luxury: u8,
    pub science: u8,
}

/// Highest share, in tenths, the government allows for each of tax, luxury and
/// science. Anarchy shares Despotism's limit in the original executable.
pub fn max_rate(government: u8) -> u8 {
    match government {
        0 | 1 => 6,
        2 => 7,
        3..=5 => 8,
        _ => 10,
    }
}

/// Reason a requested split is not legal, or `Ok(luxury)`.
pub fn check_rates(government: u8, science: u8, tax: u8) -> Result<u8, &'static str> {
    let cap = max_rate(government);
    let Some(luxury) = 10u8.checked_sub(science.saturating_add(tax)) else {
        return Err("science and tax exceed 100%");
    };
    if science > cap || tax > cap || luxury > cap {
        return Err("rate exceeds the government limit");
    }
    Ok(luxury)
}

/// The original slider rule (`0x409704` through the three slider handlers).
///
/// `current` and `remembered` are `[tax, luxury, science]`. `remembered` is the
/// legal split the dialog opened with; it is not updated by moves. The moved
/// slider is clamped to `[0, cap]` and the other two follow in the cycle
/// tax→science→luxury→tax. Locked shares do not change unless every unlocked
/// share is exhausted, so with both others locked the move is effectively
/// refused. Returns `current` unchanged if `current` is not a legal split.
pub fn redistribute(
    cap: u8,
    current: [u8; 3],
    remembered: [u8; 3],
    locks: [bool; 3],
    moved: Slider,
    value: u8,
) -> [u8; 3] {
    let cap = i32::from(cap);
    if current.iter().map(|&v| i32::from(v)).sum::<i32>() != 10
        || current.iter().any(|&v| i32::from(v) > cap)
    {
        return current;
    }
    let xi = moved.index();
    let (yi, zi) = match xi {
        0 => (2, 1),
        2 => (1, 0),
        _ => (0, 2),
    };
    let mut x = i32::from(value).min(cap);
    let mut y = i32::from(current[yi]);
    let mut z = i32::from(current[zi]);
    let w = i32::from(remembered[yi]);
    let (ly, lz) = (locks[yi], locks[zi]);
    while x + y + z < 10 {
        if y < cap && w > y && !ly {
            y += 1;
        } else if z < cap && !lz {
            z += 1;
        } else if y < cap && !ly {
            y += 1;
        } else {
            x += 1;
        }
    }
    while x + y + z > 10 {
        if y != 0 && !ly {
            y -= 1;
        } else if z != 0 && !lz {
            z -= 1;
        } else {
            x -= 1;
        }
    }
    let mut out = [0u8; 3];
    out[xi] = x.clamp(0, 10) as u8;
    out[yi] = y.clamp(0, 10) as u8;
    out[zi] = z.clamp(0, 10) as u8;
    out
}

impl WorldState {
    pub fn rates_view(&self, civ: u8) -> Option<RatesView> {
        let c = civ as usize;
        let government = *self.governments.get(c)?;
        let (science, tax) = (*self.science_rates.get(c)?, *self.tax_rates.get(c)?);
        let luxury = 10u8.saturating_sub(science.saturating_add(tax));
        Some(RatesView {
            government,
            cap: max_rate(government),
            tax,
            luxury,
            science,
        })
    }

    /// Validate and commit new shares. A no-op split is accepted and changes
    /// nothing; a changed split refreshes the civ's own surface cities in
    /// ascending order (the original commit runs a city pass only on change).
    pub fn set_rates(
        &mut self,
        rules: &Rules,
        civ: u8,
        science: u8,
        tax: u8,
    ) -> Result<(), &'static str> {
        let c = civ as usize;
        let government = *self.governments.get(c).ok_or("unknown civilization")?;
        check_rates(government, science, tax)?;
        if self.science_rates[c] == science && self.tax_rates[c] == tax {
            return Ok(());
        }
        self.science_rates[c] = science;
        self.tax_rates[c] = tax;
        for k in 0..self.cities.len() {
            let city = &self.cities[k];
            if city.city_id != 0 && city.map_id == 0 && city.owner & 15 == u16::from(civ) {
                self.refresh_city_totals(rules, k, false);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VECTORS: &str = include_str!("../../../docs/evidence/rates-government-vectors.txt");

    fn section(name: &str) -> Vec<Vec<i32>> {
        let mut inside = false;
        let mut rows = Vec::new();
        for line in VECTORS.lines() {
            if let Some(header) = line.strip_prefix("# ") {
                inside = header.starts_with(name);
            } else if inside && !line.is_empty() {
                rows.push(
                    line.split_whitespace()
                        .filter(|t| *t != "|")
                        .map(|t| t.parse().unwrap())
                        .collect(),
                );
            }
        }
        rows
    }

    #[test]
    fn caps_match_all_256_native_government_bytes() {
        let rows = section("cap:");
        assert_eq!(rows.len(), 256);
        for r in rows {
            assert_eq!(i32::from(max_rate(r[0] as u8)), r[1], "government {}", r[0]);
        }
    }

    #[test]
    fn redistribution_matches_native_vectors() {
        let rows = section("normalize:");
        assert_eq!(rows.len(), 621);
        for r in rows {
            let (cap, moved, value) = (r[0] as u8, r[1], r[2] as u8);
            let current = [r[3] as u8, r[4] as u8, r[5] as u8];
            let locks = [r[6] & 1 != 0, r[6] & 2 != 0, r[6] & 4 != 0];
            let moved = match moved {
                0 => Slider::Tax,
                1 => Slider::Luxury,
                _ => Slider::Science,
            };
            let out = redistribute(cap, current, current, locks, moved, value);
            assert_eq!(
                out.map(i32::from),
                [r[7], r[8], r[9]],
                "cap {cap} {moved:?} {value} from {current:?} locks {locks:?}"
            );
        }
    }

    #[test]
    fn remembered_opening_value_matches_native_vectors() {
        let rows = section("remembered:");
        assert_eq!(rows.len(), 414);
        let mut differs = 0;
        for r in rows {
            let (cap, value) = (r[0] as u8, r[2] as u8);
            let current = [r[3] as u8, r[4] as u8, r[5] as u8];
            let locks = [r[6] & 1 != 0, r[6] & 2 != 0, r[6] & 4 != 0];
            let remembered = [r[7] as u8, r[8] as u8, r[9] as u8];
            let moved = match r[1] {
                0 => Slider::Tax,
                1 => Slider::Luxury,
                _ => Slider::Science,
            };
            let out = redistribute(cap, current, remembered, locks, moved, value);
            assert_eq!(
                out.map(i32::from),
                [r[10], r[11], r[12]],
                "cap {cap} {moved:?} {value} from {current:?} opened {remembered:?} locks {locks:?}"
            );
            if out != redistribute(cap, current, current, locks, moved, value) {
                differs += 1;
            }
        }
        // the remembered value must actually change outcomes in the corpus
        assert!(
            differs >= 20,
            "only {differs} vectors depend on the opening value"
        );
    }

    #[test]
    fn legality_uses_each_share_and_the_total() {
        assert_eq!(check_rates(1, 6, 4), Ok(0));
        assert_eq!(
            check_rates(1, 7, 3),
            Err("rate exceeds the government limit")
        );
        assert_eq!(check_rates(1, 2, 2), Ok(6));
        assert_eq!(
            check_rates(1, 1, 2),
            Err("rate exceeds the government limit")
        );
        assert_eq!(check_rates(2, 7, 3), Ok(0));
        assert_eq!(check_rates(6, 10, 0), Ok(0));
        assert_eq!(check_rates(6, 6, 5), Err("science and tax exceed 100%"));
    }

    #[test]
    fn illegal_or_unlegal_starting_split_is_left_alone() {
        assert_eq!(
            redistribute(6, [7, 3, 0], [7, 3, 0], [false; 3], Slider::Tax, 2),
            [7, 3, 0]
        );
        assert_eq!(
            redistribute(6, [4, 0, 5], [4, 0, 5], [false; 3], Slider::Tax, 2),
            [4, 0, 5]
        );
    }
}
