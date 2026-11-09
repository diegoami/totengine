//! Governments: forced anarchy from city unrest (`0x52588e -> 0x52522f`),
//! voluntary revolution, native eligibility (`0x525468`), the government
//! setter and anarchy resolution (`0x5292d6`) for #195.
//!
//! Evidence: docs/evidence/README.md §Rates, revolution and government
//! selection. Eligibility and anarchy resolution are checked against native
//! vectors (docs/evidence/rates-government-vectors.txt); the revolution
//! command and setter effects are static decode.
use crate::WorldState;
use tot_kit::Rules;

/// Advance that enables each government (`0x525468`), indexed by government.
/// Anarchy, Despotism and ids above 6 need none.
const GOVERNMENT_ADVANCE: [Option<usize>; 7] =
    [None, None, Some(54), Some(15), Some(31), Some(71), Some(21)];
/// Statue of Liberty is wonder 19 (improvement 59).
const STATUE_OF_LIBERTY: usize = 19;
/// Fundamentalism's advance record (+17 enabled byte, `0x5d0c91`).
const FUNDAMENTALISM_ADVANCE: usize = 31;

/// Native `0x525468`: may `government` be chosen? Anarchy, Despotism and any
/// id of 7 or more are always allowed; the other five need their advance or an
/// active Statue of Liberty, and Fundamentalism also needs its advance record
/// enabled by the loaded rules.
pub fn government_eligible(
    government: u8,
    knows_advance: bool,
    statue_of_liberty: bool,
    fundamentalism_enabled: bool,
) -> bool {
    if government as usize >= GOVERNMENT_ADVANCE.len()
        || GOVERNMENT_ADVANCE[government as usize].is_none()
    {
        return true;
    }
    if !(knows_advance || statue_of_liberty) {
        return false;
    }
    government != 4 || fundamentalism_enabled
}

/// Result of one `0x5292d6` pass for a civ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnarchyStep {
    /// Civ flags after the pass (bits `0x48` are always cleared).
    pub flags: u16,
    /// The government chooser opens (`0x52588e(civ, 1)`).
    pub open_chooser: bool,
}

/// Native `0x5292d6`, all 528 executed combinations. A civ in Anarchy:
///
/// * human, turn a multiple of four: the chooser opens unless flag bit `0x1` is
///   set, which is then only consumed (also with the Statue of Liberty);
/// * human, other turns: with the Statue of Liberty the chooser opens and bit
///   `0x1` is cleared; without it nothing happens and bit `0x1` stays;
/// * AI: opens on a multiple of four or with the Statue; bit `0x1` is untouched.
pub fn anarchy_resolution(
    human: bool,
    government: u8,
    turn: i16,
    flags: u16,
    statue_of_liberty: bool,
) -> AnarchyStep {
    let mut flags = flags & !0x48;
    let mut open_chooser = false;
    if government == 0 {
        let aligned = turn & 3 == 0;
        if !human {
            open_chooser = aligned || statue_of_liberty;
        } else if aligned {
            if flags & 1 != 0 {
                flags &= !1;
            } else {
                open_chooser = true;
            }
        } else if statue_of_liberty {
            flags &= !1;
            open_chooser = true;
        }
    }
    AnarchyStep {
        flags,
        open_chooser,
    }
}

impl WorldState {
    /// Governments the civ may choose from the chooser (`0x5255c4` mode 0, ids 1..=6).
    pub fn eligible_governments(&self, rules: &Rules, civ: u8) -> Vec<u8> {
        let c = civ as usize;
        if c >= 8 {
            return Vec::new();
        }
        let statue = self.has_active_wonder(rules, civ, STATUE_OF_LIBERTY);
        let fundamentalism = rules
            .tech_defs()
            .get(FUNDAMENTALISM_ADVANCE)
            .is_some_and(|a| a.prereqs[0].is_some());
        (1..=6u8)
            .filter(|&g| {
                let knows = GOVERNMENT_ADVANCE[g as usize]
                    .is_some_and(|a| self.techs[c][a / 8] & (1 << (a % 8)) != 0);
                government_eligible(g, knows, statue, fundamentalism)
            })
            .collect()
    }

    /// Whether the human civ may start a voluntary revolution now: it must not
    /// already be in Anarchy. The scenario restriction (`0x607640 & 0x80` with
    /// `0x5f5e50 & 0x10`) is not decoded and is not enforced.
    pub fn can_start_revolution(&self, civ: u8) -> bool {
        let c = civ as usize;
        c < 8 && self.human_mask & (1 << civ) != 0 && self.governments[c] != 0
    }

    /// Native setter `0x52522f` for a human civ: stores the government, clears
    /// incoming treaty bit 0x10 for Anarchy, refreshes own surface cities in
    /// ascending order and maps queued Fanatics to Riflemen outside
    /// Fundamentalism. Returns whether the government changed.
    pub fn set_government(&mut self, rules: &Rules, civ: u8, government: u8) -> bool {
        let c = civ as usize;
        if c >= 8 || self.governments[c] == government {
            return false;
        }
        self.governments[c] = government;
        if government == 0 {
            for incoming in &mut self.diplomacy {
                incoming[c] &= !0x10;
            }
        }
        for k in 0..self.cities.len() {
            let city = &self.cities[k];
            if city.city_id == 0 || city.map_id != 0 || city.owner & 15 != civ as u16 {
                continue;
            }
            self.refresh_city_totals(rules, k, false);
            // Fanatics -> Riflemen when leaving Fundamentalism. This is
            // the original's literal unit-index switch, also in scenarios.
            if government != 4 && self.cities[k].building == 8 {
                self.cities[k].building = 11;
            }
        }
        true
    }

    /// Voluntary revolution (`0x40bde1` after the Yes answer): Anarchy starts
    /// at once and civ flag 0x1 is not set, unlike the city-unrest path.
    pub fn start_revolution(&mut self, rules: &Rules, civ: u8) -> Result<(), &'static str> {
        if !self.can_start_revolution(civ) {
            return Err("revolution is not available");
        }
        self.set_government(rules, civ, 0);
        Ok(())
    }

    /// One `0x5292d6` pass for `civ` at the per-civ turn step: consumes flag
    /// bits as the original does and reports whether the chooser must open.
    pub fn resolve_anarchy(&mut self, rules: &Rules, civ: u8) -> bool {
        let c = civ as usize;
        if c >= 8 {
            return false;
        }
        let statue = self.has_active_wonder(rules, civ, STATUE_OF_LIBERTY);
        let step = anarchy_resolution(
            self.human_mask & (1 << civ) != 0,
            self.governments[c],
            self.turn,
            self.ai_state.civ_flags[c],
            statue,
        );
        self.ai_state.civ_flags[c] = step.flags;
        step.open_chooser
    }

    /// Human-mask forced anarchy. A changed government clears incoming
    /// treaty bit 0x10 and refreshes own surface cities in ascending order.
    /// Returns whether the government changed; caller handles civic riot flags.
    pub fn force_human_anarchy(&mut self, rules: &Rules, civ: u8) -> bool {
        let c = civ as usize;
        if c >= 8 || self.human_mask & (1 << civ) == 0 || self.governments[c] == 0 {
            return false;
        }
        self.set_government(rules, civ, 0)
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
                        .map(|t| t.parse().unwrap())
                        .collect(),
                );
            }
        }
        rows
    }

    #[test]
    fn eligibility_matches_native_vectors() {
        let rows = section("eligible:");
        assert_eq!(rows.len(), 144);
        for r in rows {
            // government, knows_own_advance, knows_other_government_advances,
            // statue_of_liberty, fundamentalism_flag, result
            let got = government_eligible(r[0] as u8, r[1] != 0, r[3] != 0, r[4] != 0);
            assert_eq!(i32::from(got), r[5], "{r:?}");
        }
    }

    #[test]
    fn anarchy_resolution_matches_native_vectors() {
        let rows = section("resolve:");
        assert_eq!(rows.len(), 528);
        for r in rows {
            // human, government, turn, flags_in, statue, flags_out, chooser_calls, ...
            let step =
                anarchy_resolution(r[0] != 0, r[1] as u8, r[2] as i16, r[3] as u16, r[4] != 0);
            assert_eq!(i32::from(step.flags), r[5], "{r:?}");
            assert_eq!(i32::from(step.open_chooser), r[6], "{r:?}");
        }
    }

    #[test]
    fn revolution_lasts_until_the_next_multiple_of_four() {
        let open: Vec<i16> = (41..53)
            .filter(|&t| anarchy_resolution(true, 0, t, 0, false).open_chooser)
            .collect();
        assert_eq!(open, [44, 48, 52]);
        // city-unrest anarchy sets flag 0x1: its first aligned turn is consumed
        let first = anarchy_resolution(true, 0, 44, 1, false);
        assert!(!first.open_chooser);
        assert_eq!(first.flags, 0);
        assert!(anarchy_resolution(true, 0, 48, first.flags, false).open_chooser);
    }
}
