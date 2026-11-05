//! Headless round loop. Surface cities use the decoded growth, allocation,
//! collection, production, support and end-of-city mood sequence in
//! `crate::city_phase`; each civ's cities run in descending slot order.
//!
//! Civ scheduling (0..7, after player orders), initial transient city context,
//! and turn-event dispatch are engine policy. This is not a complete original
//! round/PRNG replay: non-surface maps, sea cargo, pollution/settler-job planning
//! and the original AI's build choices remain separate.

use tot_kit::Rules;

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
    pub destroyed_cities: Vec<usize>,
    pub disbanded_units: Vec<usize>,
    pub revolutions: Vec<u8>,
    /// Turn events fired, with what applying them did.
    pub events: Vec<(Effect, Applied)>,
    /// What city production completed: (city index, item).
    pub built: Vec<(usize, crate::production::Built)>,
    /// (civ, advance) discoveries from this turn's city science.
    pub discoveries: Vec<(u8, usize)>,
    pub city_accounts: Vec<(usize, crate::treasury::CityAccount)>,
    /// New unit slots from the original round-end barbarian arrival phase.
    pub barbarian_arrivals: Vec<usize>,
}

impl WorldState {
    /// Run one full turn; see the module docs for what it covers.
    pub fn run_turn(&mut self, rules: &Rules, events: Option<&mut EventEngine>) -> TurnSummary {
        let mut sum = TurnSummary::default();
        let mut food_routes = crate::city_phase::FoodRouteState::default();
        // 0x470aa5..0x470b69: descending city slots within each civ/map.
        // The headless round uses civ order 0..7 and the retained surface.
        for civ in 0..8 {
            for k in (0..self.cities.len()).rev() {
                if self.cities[k].owner & 15 != civ {
                    continue;
                }
                let Some(city) = self.run_city_phase(rules, k, &mut food_routes) else {
                    continue;
                };
                if city.grew {
                    sum.grew.push(k);
                }
                if city.famined {
                    sum.famined.push(k);
                }
                if city.destroyed {
                    sum.destroyed_cities.push(k);
                    continue;
                }
                if let Some(built) = city.built {
                    sum.built.push((k, built));
                }
                if let Some(discovery) = city.discovery {
                    sum.discoveries.push(discovery);
                }
                sum.disbanded_units.extend(city.disbanded);
                if city.revolution {
                    sum.revolutions.push(civ as u8);
                }
                sum.city_accounts.push((k, city.account));
            }
        }

        self.refresh_city_statistics();
        sum.barbarian_arrivals = self.barbarian_arrivals(rules);
        if self.turn != 0 {
            self.refresh_power();
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

    /// Growth reads the starting box; current food is collected afterwards.
    #[test]
    fn growth_precedes_current_food_collection() {
        let mut w = crate::boot(&synthetic_save(), 7);
        let rules = tot_kit::parse_rules("@TERRAIN\nDesert,1,1,0,0,0\nPlains,1,1,4,0,0\n");
        w.human_mask = 2;
        w.governments[1] = 2;
        let c = &mut w.cities[1];
        c.city_id = 1;
        c.food_box = 19;
        c.worked = (1 << 20) | (1 << 26);
        let first = w.run_turn(&rules, None);
        assert!(!first.grew.contains(&1));
        assert_eq!((w.cities[1].size, w.cities[1].food_box), (1, 21));
        let second = w.run_turn(&rules, None);
        assert!(second.grew.contains(&1));
        assert_eq!((w.cities[1].size, w.cities[1].food_box), (2, 0));
    }
}
