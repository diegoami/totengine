//! The headless turn loop (v0.2.0): what one game turn does to the world.
//!
//! [`WorldState::run_turn`] chains the modelled subsystems:
//!
//! 1. **Cities** grow or starve via [`crate::cities::CityEcon::turn`]. The
//!    model is oracle-replayed (evidence §T1-4 growth replay). Gross food is
//!    the save's value held constant, because tile yields are not modelled
//!    yet. Settlers are the role-5 ("settle") units homed in the city.
//!    Cities with gross food 0 are skipped: the original leaves young cities
//!    half-initialised until their first processing (evidence §T1-4).
//! 2. **Production** ([`WorldState::production_step`]): shields fill
//!    the box; units and improvements complete (evidence §T1-4
//!    production).
//! 3. **Turn events**: TURN, TURNINTERVAL and RANDOMTURN fire against the
//!    new turn number, and their effects are applied. RANDOMTURN rolls
//!    `next_below(n) == 0` on the kernel's generator, once per event in file
//!    order.
//! 4. The turn counter advances and movement resets for every unit.
//!
//! **The order of these steps, and which generator draws the RANDOMTURN
//! rolls, are the engine's choice, not decoded from the original.** The
//! loop is deterministic: the same save, seed and rules give the same
//! fingerprints (the 50-turn golden, `docs/evidence/fallrome-50turn.txt`).

use tot_kit::Rules;

use crate::cities::{CityEcon, CityRules};
use crate::effects::Applied;
use crate::events::{Ctx, Effect, EventEngine};
use crate::rng::GameRng;
use crate::WorldState;

/// What one [`WorldState::run_turn`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TurnSummary {
    /// The turn number after advancing.
    pub turn: i16,
    /// Cities that grew / starved this turn (indices).
    pub grew: Vec<usize>,
    pub famined: Vec<usize>,
    /// Turn events fired, with what applying them did.
    pub events: Vec<(Effect, Applied)>,
    /// What city production completed: (city index, item).
    pub built: Vec<(usize, crate::production::Built)>,
    /// (civ, advance) discoveries from this turn's city science.
    pub discoveries: Vec<(u8, usize)>,
}

/// AI role 5 = settle (Rules @UNITS field 11).
const ROLE_SETTLE: u8 = 5;

impl WorldState {
    /// Run one full turn; see the module docs for what it covers.
    pub fn run_turn(&mut self, rules: &Rules, events: Option<&mut EventEngine>) -> TurnSummary {
        let mut sum = TurnSummary::default();
        let cr = CityRules::from_rules(rules);
        for k in 0..self.cities.len() {
            let c = &self.cities[k];
            if c.gross_food == 0 {
                continue;
            }
            let settlers = self
                .units
                .iter()
                .filter(|u| {
                    u.home as usize == k && rules.unit_role(u.type_id as usize) == Some(ROLE_SETTLE)
                })
                .count() as u32;
            let mut econ = CityEcon {
                size: c.size,
                food_box: c.food_box as u32,
                shield_box: c.shield_box as u32,
            };
            let t = econ.turn(c.gross_food as u32, settlers, &cr);
            let c = &mut self.cities[k];
            c.size = econ.size;
            c.food_box = econ.food_box.min(u16::MAX as u32) as u16;
            if t.grew {
                sum.grew.push(k);
            }
            if t.famined {
                sum.famined.push(k);
            }
        }
        // production (evidence §T1-4 production); cities with no food
        // are the half-initialised young cities the growth step skips too
        for k in 0..self.cities.len() {
            if self.cities[k].gross_food == 0 {
                continue;
            }
            // City finance (#87): evaluate before this turn's build is
            // completed, so a new Library affects the next totals pass.
            // Route-inclusive trade remains #85; do not turn these city
            // taxes into treasury income without maintenance accounting.
            let finance = self.city_finance(rules, k);
            self.cities[k].net_science = finance.science.clamp(0, u16::MAX as i32) as u16;
            self.cities[k].net_taxes = finance.taxes.clamp(0, u16::MAX as i32) as u16;
            let civ = (self.cities[k].owner & 15) as u8;
            // 0x4ca000: anarchy contributes no research. The tutorial's
            // Chieftain human bonus doubles science toward an advance
            // already discovered by another player. The late spaceship-
            // related AI doubling branch is not modelled yet.
            if self.governments.get(civ as usize).is_some_and(|&g| g != 0) {
                let mut science = finance.science;
                let target = self.research[civ as usize].target;
                if self.difficulty == 0
                    && self.human_mask & (1 << civ) != 0
                    && (0..100).contains(&target)
                    && target != 89
                    && self
                        .techs
                        .iter()
                        .any(|b| b[target as usize / 8] & (1 << (target as usize % 8)) != 0)
                {
                    science *= 2;
                }
                if let Some(advance) = self.add_research(rules, civ, science) {
                    sum.discoveries.push((civ, advance));
                }
            }
            // the totals pass (0x4c45ff -> 0x430232) clears create_city's
            // refresh-pending bit (evidence §City founding)
            self.cities[k].flags &= !crate::founding::CITY_REFRESH;
            if let Some(b) = self.production_step(rules, k) {
                sum.built.push((k, b));
            }
        }

        self.advance_turn();
        if let Some(ev) = events {
            let ctx = Ctx {
                turn: Some(self.turn as i32),
                ..Default::default()
            };
            let rng = &mut self.rng;
            let mut roll = |n: i32| n > 0 && rng.next_below(n as u32) == 0;
            let mut effects = ev.fire_with("TURN", &ctx, &mut roll);
            effects.extend(ev.fire_with("TURNINTERVAL", &ctx, &mut roll));
            effects.extend(ev.fire_with("RANDOMTURN", &ctx, &mut roll));
            let applied = self.apply_effects(rules, &effects);
            sum.events = effects.into_iter().zip(applied).collect();
        }
        self.start_turn(rules);
        sum.turn = self.turn;
        sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_support::synthetic_save;

    const RULES: &str = "@COSMIC\n3 ; road\n2 ; trireme\n2 ; food each citizen eats\n\
10 ; rows in food box\n10 ; rows in shield box\n1 ; settlers eat (low)\n2 ; settlers eat (high)\n\
@UNITS\nSettlers, nil, 0, 1., 0, 0a, 1d, 1h, 1f, 3, 0, 5, nil, 000000000000000\n\
Warriors, nil, 0, 1., 0, 1a, 1d, 1h, 1f, 1, 0, 1, nil, 000000000000000\n\
@TERRAIN\nDesert, 1, 1, 0, 1, 0\nGrassland, 1, 1, 2, 0, 0\n";

    const EVENTS: &str = "@IF\nRANDOMTURN\ndenominator=2\n@THEN\nCREATEUNIT\nunit=Warriors\n\
owner=Testers\nveteran=no\nhomecity=NONE\nlocations\n4,2\nendlocations\n@ENDIF\n\
@IF\nTURN\nturn=10\n@THEN\nCHANGEMONEY\nreceiver=Testers\namount=7\n@ENDIF\n";

    fn run(seed: u64, turns: usize) -> (Vec<u64>, WorldState) {
        let rules = tot_kit::parse_rules(RULES);
        let mut ev = EventEngine::new(tot_kit::parse_events(EVENTS).unwrap());
        let mut w = crate::boot(&synthetic_save(), seed);
        w.start_turn(&rules);
        let mut sums = vec![w.checksum()];
        for _ in 0..turns {
            w.run_turn(&rules, Some(&mut ev));
            sums.push(w.checksum());
        }
        (sums, w)
    }

    /// Asset-free 50-turn run (the CI half of T1-9): same seed ⇒ identical
    /// fingerprints every turn; a different seed changes the RANDOMTURN
    /// rolls and therefore the world.
    #[test]
    fn fifty_turns_are_deterministic() {
        let (a, wa) = run(7, 50);
        let (b, _) = run(7, 50);
        assert_eq!(a, b);
        let (c, _) = run(8, 50);
        assert_ne!(a.last(), c.last(), "seed reaches the RANDOMTURN rolls");
        assert_eq!(wa.turn, 51);
        // TURN turn=10: +7 once
        assert_eq!(wa.treasury[1], 7);
        // RANDOMTURN 1-in-2 over 50 turns created Warriors at (4,2)
        let created = wa.units.iter().filter(|u| (u.x, u.y) == (4, 2)).count();
        assert!((10..=40).contains(&created), "{created}");
    }

    /// The city step: City0 feeds a settler (surplus 1), the others
    /// gain 2 a turn; 18 is the size-1 capacity, growth strictly above.
    #[test]
    fn cities_grow_on_schedule() {
        let (_, w) = run(7, 10);
        // City1..7: +2 a turn from 0 → 20 > 18 at turn 10 → size 2, box 2
        assert!(w.cities[1..].iter().all(|c| (c.size, c.food_box) == (2, 2)));
        // City0 with its settler: +1 a turn → box 10, still size 1
        assert_eq!((w.cities[0].size, w.cities[0].food_box), (1, 10));
    }
}
