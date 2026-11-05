//! Recurring trade-route arrows (`0x4c46bf..0x4c4890`).
//!
//! Partners supply cached pre-route trade, avoiding recursive city totals.
//! Route creation, delivery rewards and supply/demand refresh remain open.

use crate::WorldState;
use tot_kit::Rules;

/// Original arithmetic, with the connection factor already including
/// airports and the source city's Superhighways bonus. Negative commodity
/// IDs contribute zero; domestic routes are halved after all bonuses.
pub fn route_income(source: i16, partner: i16, commodity: i8, domestic: bool, factor: i32) -> i32 {
    let base = (source as i32 + partner as i32 + 4) >> 3;
    let value = if commodity < 0 {
        0
    } else {
        base + ((base * factor) >> 1)
    };
    if domestic {
        value >> 1
    } else {
        value
    }
}

impl WorldState {
    /// Individual route arrows before the city's final corruption pass.
    /// `source_trade` is current trade after the first corruption pass;
    /// partner trade comes from the last city evaluation, as in the original.
    pub fn trade_route_income(&self, rules: &Rules, idx: usize, source_trade: i16) -> [i32; 3] {
        let Some(c) = self.cities.get(idx) else {
            return [0; 3];
        };
        std::array::from_fn(|k| {
            if k >= c.route_count.clamp(0, 3) as usize || c.route_commodities[k] < 0 {
                return 0;
            }
            let Ok(partner) = usize::try_from(c.route_partners[k]) else {
                return 0;
            };
            let Some(p) = self.cities.get(partner).filter(|p| p.city_id != 0) else {
                return 0;
            };
            // The decoded path helper supports the surface map only.
            if c.map_id != 0 || p.map_id != 0 {
                return 0;
            }
            let mut factor =
                self.road_connection(rules, (c.owner & 15) as u8, (c.x, c.y), (p.x, p.y));
            if c.has_building(32) && p.has_building(32) {
                factor = factor.max(1);
            }
            factor += c.has_building(25) as i32;
            route_income(
                source_trade,
                p.route_trade,
                c.route_commodities[k],
                c.owner == p.owner,
                factor,
            )
        })
    }

    /// First corruption pass, cached in the original city record at +32.
    pub fn pre_route_trade(&self, rules: &Rules, idx: usize) -> i16 {
        let Some(c) = self.cities.get(idx) else {
            return 0;
        };
        let trade = self.worked_yield(rules, idx)[2];
        let gov = self
            .governments
            .get((c.owner & 15) as usize)
            .copied()
            .unwrap_or(0);
        let corruption = if gov == 4 || gov == 6 {
            0
        } else {
            self.corruption(rules, idx, trade, false)
        };
        (trade - corruption) as i16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_partners_multiple_slots_and_finance() {
        let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
        w.wonders = [-1; 28];
        w.units.clear();
        w.map = crate::MapState {
            stored_w: 32,
            stored_h: 16,
            terrain: vec![1; 512],
            landmass: vec![1; 512],
            landmass_values: vec![100; 256],
            site_bits: vec![0; 512],
            fertility: vec![0; 512],
            special: vec![false; 512],
            river: vec![false; 512],
            road: vec![true; 512],
            rail: vec![false; 512],
            improvements: vec![16; 512],
            owner: vec![15; 512],
            visible: vec![255; 512],
            resource_seed: 0,
            huts_enabled: false,
        };
        w.game_flags = 0;
        w.governments[1] = 6;
        w.science_rates[1] = 5;
        w.tax_rates[1] = 5;
        for (k, c) in w.cities.iter_mut().take(2).enumerate() {
            c.x = 8 + 8 * k as u16;
            c.y = 8;
            c.owner = 1;
            c.map_id = 0;
            c.city_id = k as u32 + 1;
            c.worked = 0;
            c.buildings = [0; 5];
            c.specialists = 0;
            c.building = 0;
            c.route_trade = if k == 0 { 999 } else { 100 };
            // Mutual routes must read caches rather than recurse.
            c.route_count = 3;
            c.route_partners = [1 - k as i16; 3];
            c.route_commodities = [0, -1, 15];
        }
        let rules = tot_kit::parse_rules("@UNITS\na,no,0\nb,no,0\nc,no,0\n");
        // Current source is zero, stale source cache 999 must not be used.
        // (0+100+4)/8 = 13; road gives 19; domestic gives 9 per slot.
        assert_eq!(w.trade_route_income(&rules, 0, 0), [9, 0, 9]);
        assert_eq!(w.city_output(&rules, 0).trade, 18);
        let finance = w.city_finance(&rules, 0);
        assert_eq!((finance.science, finance.taxes), (9, 9));
        w.cities[1].route_trade = 60;
        assert_eq!(w.trade_route_income(&rules, 0, 0), [6, 0, 6]);
        w.cities[0].route_partners = [-1, 30000, 1];
        w.cities[0].route_commodities = [0; 3];
        assert_eq!(w.trade_route_income(&rules, 0, 0), [0, 0, 6]);
        w.cities[1].city_id = 0;
        assert_eq!(w.trade_route_income(&rules, 0, 0), [0; 3]);
    }

    #[test]
    fn original_x86_vectors() {
        let vectors = include_str!("../../../docs/evidence/trade-route-vectors.txt");
        let mut count = 0;
        for line in vectors.lines().filter(|l| !l.starts_with('#')) {
            let v: Vec<i32> = line
                .split_whitespace()
                .map(|x| x.parse().unwrap())
                .collect();
            let factor = v[4].max(v[5]) + v[6];
            assert_eq!(
                route_income(v[0] as i16, v[1] as i16, v[2] as i8, v[3] != 0, factor),
                v[7],
                "{line}"
            );
            count += 1;
        }
        assert_eq!(count, 288);
    }
}
