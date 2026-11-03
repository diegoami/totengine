//! City-turn economics (T1-4): food storage, growth, production.
//!
//! Model proven against oracle saves (a peaceful standard game driven
//! turn-by-turn through the original; evidence §T1-4):
//!
//! - Each turn a city collects gross food F (from worked tiles — given
//!   here as an input; tile-yield computation is a later layer).
//! - Citizens eat `food_per_citizen × size` (standard Rules 2, FallRome 3
//!   via `@COSMIC`); each supported settler adds its own food cost.
//! - `food_box += surplus`; if the box reaches the growth capacity the
//!   city grows and the box carries the remainder.
//!   Capacities measured turn-by-turn at five growth events: **20 at
//!   size 1** (three cities), **27 at size 2** (two cities) — i.e.
//!   `20 + 7·(size−1)`. Size ≥ 3 is extrapolation pending samples.
//! - The shield box (`+= net shields`, reset on production completion)
//!   lives in the save at u16@88 but production items/completions are
//!   not modelled yet (item location open).

use tot_kit::Rules;

/// Growth-related constants, from `@COSMIC` + measured capacities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CityRules {
    /// `@COSMIC` row 3: food each citizen eats per turn.
    pub food_per_citizen: u32,
    /// Food eaten per supported settler (government-dependent row).
    pub settler_food: u32,
    /// `@COSMIC` food-box rows (10 in both rules files examined).
    pub food_box_rows: u32,
}

impl CityRules {
    pub fn from_rules(rules: &Rules) -> Self {
        CityRules {
            food_per_citizen: rules.cosmic_food_per_citizen(),
            settler_food: rules.cosmic_settler_food(false),
            food_box_rows: rules.cosmic_food_box_rows(),
        }
    }

    /// Food-box capacity at a given size. The `@COSMIC` comment
    /// ("rows * city_size+1 = box") is garbled; the measured rule across
    /// seven growth events (standard game, three cities, turn-by-turn
    /// samples) is **(rows − 1) × (size + 1)** — 18 at size 1, 27 at
    /// size 2 — with growth strictly *above* the capacity (a box sitting
    /// exactly at 18/27 persists; several such samples exist).
    /// Size ≥ 3 is extrapolation pending samples.
    pub fn growth_cap(&self, size: u16) -> u32 {
        (self.food_box_rows.saturating_sub(1)) * (size as u32 + 1)
    }
}

/// The mutable per-city economic state (what the save's u16@86/@88 hold).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CityEcon {
    pub size: u16,
    pub food_box: u32,
    pub shield_box: u32,
}

/// What one city-turn produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CityTurn {
    pub surplus_food: i32,
    pub grew: bool,
    /// True when famine emptied the box (shrink behaviour unverified —
    /// the size is left unchanged and the flag returned instead).
    pub famined: bool,
}

impl CityEcon {
    /// Advance one turn. `gross_food` is the city's food output this turn
    /// (the save's u8@48), `settlers` the supported-settler count.
    pub fn turn(&mut self, gross_food: u32, settlers: u32, rules: &CityRules) -> CityTurn {
        let eaten = rules.food_per_citizen * self.size as u32
            + rules.settler_food * settlers;
        let surplus = gross_food as i32 - eaten as i32;
        let cap = rules.growth_cap(self.size);
        let mut result = CityTurn { surplus_food: surplus, grew: false, famined: false };
        if surplus >= 0 {
            self.food_box += surplus as u32;
            if self.food_box > cap {
                self.food_box -= cap;
                self.size += 1;
                result.grew = true;
            }
        } else {
            let deficit = (-surplus) as u32;
            if self.food_box > deficit {
                self.food_box -= deficit;
            } else {
                self.food_box = 0;
                result.famined = true;
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Madrid, standard game: size 1, F=6 (surplus +4) — box 18 sits at
    /// capacity (no growth at exactly 18 — measured t36), then 18+4 = 22
    /// > 18 → grows, box 4; size 2 with F=6 (+2): 6, 8, 10 …
    /// Oracle: fb 18@t36, grew (@90 3→4) by t38 with fb 4, then 6/8/10.
    #[test]
    fn madrid_growth_size1() {
        let rules = CityRules { food_per_citizen: 2, settler_food: 1, food_box_rows: 10 };
        let mut m = CityEcon { size: 1, food_box: 18, shield_box: 0 };
        let t = m.turn(6, 0, &rules); // +4: 22 > 18 → grow, carry 4
        assert!(t.grew);
        assert_eq!((m.size, m.food_box), (2, 4));
        m.turn(6, 0, &rules);
        assert_eq!(m.food_box, 6);
        m.turn(6, 0, &rules);
        assert_eq!(m.food_box, 8);
    }

    /// A box exactly at capacity persists (LB fb 18@t22, Carthage 18@t22,
    /// Madrid 18@t36 — all sat there a full turn without growing).
    #[test]
    fn box_at_exact_capacity_persists() {
        let rules = CityRules { food_per_citizen: 2, settler_food: 1, food_box_rows: 10 };
        let mut c = CityEcon { size: 1, food_box: 16, shield_box: 0 };
        let t = c.turn(4, 0, &rules); // +2 → 18, not > 18
        assert!(!t.grew);
        assert_eq!(c.food_box, 18);
        c.turn(4, 0, &rules); // +2 → 20 > 18 → grow, carry 2
        assert_eq!((c.size, c.food_box), (2, 2));
    }

    /// Carthage/Little Bighorn at size 2, F=6 (+2/turn): 27 persists,
    /// 27+2 = 29 > 27 → grow, carry 2. Oracle t40 fb 27 → t41 fb 2.
    #[test]
    fn carthage_lb_growth_size2() {
        let rules = CityRules { food_per_citizen: 2, settler_food: 1, food_box_rows: 10 };
        let mut c = CityEcon { size: 2, food_box: 25, shield_box: 0 };
        c.turn(6, 0, &rules);
        assert_eq!((c.size, c.food_box), (2, 27));
        let t = c.turn(6, 0, &rules);
        assert!(t.grew);
        assert_eq!((c.size, c.food_box), (3, 2));
    }

    /// FallRome Roma: 15 citizens × 3 food = 45 eaten, F=45 → surplus 0,
    /// box frozen — the city screen's "Net Food: 0" (oracle 2026-11-03).
    #[test]
    fn roma_fallrome_zero_surplus() {
        let rules = CityRules { food_per_citizen: 3, settler_food: 2, food_box_rows: 10 };
        let mut r = CityEcon { size: 15, food_box: 6, shield_box: 6 };
        let t = r.turn(45, 0, &rules);
        assert_eq!(t.surplus_food, 0);
        assert!(!t.grew && !t.famined);
        assert_eq!(r.food_box, 6);
    }
}
