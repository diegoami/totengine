//! City-turn economics (T1-4): food storage, growth, production.
//!
//! Model proven against oracle saves (a peaceful standard game driven
//! turn-by-turn through the original; evidence §T1-4):
//!
//! - Each turn a city collects gross food F (from worked tiles — given
//!   here as an input; tile-yield computation is a later layer).
//! - Citizens eat `food_per_citizen × size` (standard Rules 2, FallRome 3
//!   via `@COSMIC`); each supported settler adds its own food cost.
//! - `food_box += surplus`; when the box exceeds the growth capacity
//!   `(rows − 1) × (size + 1)` (18 at size 1, 27 at size 2) the city grows
//!   and the box restarts at **this turn's surplus** — not the overflow
//!   (Utica, turn 41: 28 + 1 = 29 at size 1 → size 2, box 1, where the
//!   overflow would be 11).
//! - The std1 lab series (`docs/evidence/t1-4-growth-series.tsv`, read in
//!   the game's record framing, settlers counted from unit home cities)
//!   replays with this model: 126 transitions checked, 125 exact (one
//!   known mismatch, Ur); see the tests.
//! - The shield box (`+= net shields`, reset on production completion)
//!   lives in the save at u16@30 but production items/completions are
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
    /// (the save's u8@82), `settlers` the supported-settler count.
    pub fn turn(&mut self, gross_food: u32, settlers: u32, rules: &CityRules) -> CityTurn {
        let eaten = rules.food_per_citizen * self.size as u32 + rules.settler_food * settlers;
        let surplus = gross_food as i32 - eaten as i32;
        let cap = rules.growth_cap(self.size);
        let mut result = CityTurn {
            surplus_food: surplus,
            grew: false,
            famined: false,
        };
        if surplus >= 0 {
            self.food_box += surplus as u32;
            if self.food_box > cap {
                self.food_box = surplus as u32;
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

    /// Utica, std1 lab (true framing): turn 41 size 1, box 28 (left
    /// over from a settler shrink), F 4, one settler → surplus 1 → 29 >
    /// 18 → size 2 with box 1 (the turn's surplus; the overflow would be
    /// 11). Oracle turn 42: size 2, box 1.
    #[test]
    fn growth_restarts_the_box_at_the_surplus() {
        let rules = CityRules {
            food_per_citizen: 2,
            settler_food: 1,
            food_box_rows: 10,
        };
        let mut u = CityEcon {
            size: 1,
            food_box: 28,
            shield_box: 0,
        };
        let t = u.turn(4, 1, &rules);
        assert!(t.grew);
        assert_eq!((u.size, u.food_box), (2, 1));
    }

    /// A box exactly at capacity persists (LB fb 18@t22, Carthage 18@t22,
    /// Madrid 18@t36 — all sat there a full turn without growing).
    #[test]
    fn box_at_exact_capacity_persists() {
        let rules = CityRules {
            food_per_citizen: 2,
            settler_food: 1,
            food_box_rows: 10,
        };
        let mut c = CityEcon {
            size: 1,
            food_box: 16,
            shield_box: 0,
        };
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
        let rules = CityRules {
            food_per_citizen: 2,
            settler_food: 1,
            food_box_rows: 10,
        };
        let mut c = CityEcon {
            size: 2,
            food_box: 25,
            shield_box: 0,
        };
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
        let rules = CityRules {
            food_per_citizen: 3,
            settler_food: 2,
            food_box_rows: 10,
        };
        let mut r = CityEcon {
            size: 15,
            food_box: 6,
            shield_box: 6,
        };
        let t = r.turn(45, 0, &rules);
        assert_eq!(t.surplus_food, 0);
        assert!(!t.grew && !t.famined);
        assert_eq!(r.food_box, 6);
    }

    /// Replay of the whole std1 series (evidence TSV): between two saves
    /// `dt` turns apart, step the model `dt` times with the first save's
    /// food and settlers. Pairs where the settler count changes (a
    /// settler built or disbanded mid-span) or where the city grows before
    /// the last step (its worked tiles, hence F, change) are skipped;
    /// everything else must match size and box exactly.
    #[test]
    fn std1_series_replays() {
        let rules = CityRules {
            food_per_citizen: 2, // standard @COSMIC
            settler_food: 1,     // "Settlers eat (govt <= Monarchy)"
            food_box_rows: 10,
        };
        let tsv = include_str!("../../../docs/evidence/t1-4-growth-series.tsv");
        // (city, size, food box, gross food, settlers)
        type Row<'a> = (&'a str, u16, u32, u32, u32);
        let mut rows: Vec<(i16, Vec<Row>)> = Vec::new();
        for line in tsv.lines().filter(|l| !l.starts_with('#')) {
            let f: Vec<&str> = line.split('\t').collect();
            let turn: i16 = f[0].parse().unwrap();
            let num = |i: usize| f[i].parse::<u32>().unwrap();
            if rows.last().map(|r| r.0) != Some(turn) {
                rows.push((turn, Vec::new()));
            }
            let c = (f[1], num(2) as u16, num(3), num(4), num(5));
            let row = &mut rows.last_mut().unwrap().1;
            // the lab harvested turn 4 four times; keep the first
            if !row.iter().any(|r| r.0 == c.0) {
                row.push(c);
            }
        }
        let (mut checked, mut grew, mut mismatches) = (0, 0, Vec::new());
        for pair in rows.windows(2) {
            let ((t0, a), (t1, b)) = (&pair[0], &pair[1]);
            for &(name, size, fb, food, settlers) in a {
                let Some(&(_, size1, fb1, _, settlers1)) = b.iter().find(|c| c.0 == name) else {
                    continue;
                };
                if food == 0 || settlers != settlers1 {
                    continue; // unprocessed young city / settler change
                }
                let mut c = CityEcon {
                    size,
                    food_box: fb,
                    shield_box: 0,
                };
                let dt = (t1 - t0) as usize;
                let mut early_growth = false;
                for step in 0..dt {
                    if c.turn(food, settlers, &rules).grew && step + 1 < dt {
                        early_growth = true;
                        break;
                    }
                }
                if early_growth {
                    continue;
                }
                checked += 1;
                grew += usize::from(c.size != size);
                if (c.size, c.food_box) != (size1, fb1) {
                    mismatches.push((*t0, name, (c.size, c.food_box), (size1, fb1)));
                }
            }
        }
        // Ur, turn 40→41: predicted box 2, observed 1 (its F moved 4→5
        // across the growth; a worker change the saves cannot show).
        assert_eq!(mismatches, [(40, "Ur", (2, 2), (2, 1))], "{mismatches:?}");
        // 126 checked transitions (10 of them growths); all but Ur match,
        // so 125 exact, 9 of the growths exact (reviewer finding #68)
        assert_eq!((checked, grew), (126, 10));
        assert_eq!(checked - mismatches.len(), 125);
    }
}
