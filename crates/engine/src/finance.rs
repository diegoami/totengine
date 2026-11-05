//! Science/tax split (#87), decoded from unpatched civ2.bak `0x4c3f0c`.
//! Evidence: docs/evidence/README.md §Science and tax split. These are
//! city outputs; civ research accumulation and maintenance payments are
//! separate work. No treasury income is invented here.
use tot_kit::Rules;

use crate::{research::TechTree, WorldState};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CityFinance {
    pub luxuries: i32,
    pub science: i32,
    pub taxes: i32,
}

impl WorldState {
    fn finance_knows(&self, civ: usize, tech: usize) -> bool {
        // 0x49c820 excludes barbarians, the reserved advance 89, and 100+.
        civ > 0
            && tech < 100
            && tech != 89
            && self
                .techs
                .get(civ)
                .is_some_and(|b| b[tech / 8] & (1 << (tech % 8)) != 0)
    }

    /// A wonder's city while it is active. `0x444a28` suppresses the
    /// wonder once any player knows its obsoleting advance. Use physical
    /// @ENDWONDER row IDs, including repeated scenario labels such as '*'.
    fn finance_wonder(&self, rules: &Rules, wonder: usize) -> Option<usize> {
        let city = *self.wonders.get(wonder)?;
        if city < 0 || city as usize >= self.cities.len() {
            return None;
        }
        let raw = rules
            .section("ENDWONDER")
            .and_then(|s| s.rows.get(wonder))
            .and_then(|r| r.first());
        if let Some(abbrev) = raw.filter(|s| s.as_str() != "nil" && s.as_str() != "no") {
            if let Some(tech) = TechTree::from_rules(rules).by_abbrev(abbrev) {
                if (1..8).any(|civ| self.finance_knows(civ, tech)) {
                    return None;
                }
            }
        }
        Some(city as usize)
    }

    /// Split trade after routes and corruption, with already computed net
    /// shields (after support) for Capitalization. This boundary also lets
    /// saved totals test finance independently of still-open route/waste
    /// calculations. Rounding and operation order match `0x4c3f0c`.
    /// The network-game bookkeeping branch is not modelled.
    pub fn split_city_trade(
        &self,
        rules: &Rules,
        idx: usize,
        trade: i32,
        net_shields: i32,
    ) -> CityFinance {
        let Some(city) = self.cities.get(idx) else {
            return CityFinance::default();
        };
        let civ = (city.owner & 15) as usize;
        let Some(&government) = self.governments.get(civ) else {
            return CityFinance::default();
        };
        let fundamentalist = government == 4;
        let science_rate = self.science_rates[civ] as i32;
        let tax_rate = self.tax_rates[civ] as i32;
        let trade = trade.max(0);
        let mut luxuries = ((trade * (10 - science_rate - tax_rate) + 4) / 10).clamp(0, trade);
        let science_rate = if fundamentalist {
            science_rate.min(rules.cosmic_row(21).unwrap_or(5) as i32)
        } else {
            science_rate
        };
        let mut science = ((trade * science_rate + 4) / 10).clamp(0, trade - luxuries);
        let mut taxes = trade - science - luxuries;
        if fundamentalist && self.human_mask & (1 << civ) == 0 {
            science += luxuries;
            luxuries = 0;
        }
        let count = |kind| {
            (0..16)
                .filter(|k| city.specialists >> (2 * k) & 3 == kind)
                .count() as i32
        };
        luxuries += 2 * count(1);
        taxes += 3 * count(2);
        science += 3 * count(3);
        if fundamentalist {
            science -= science * rules.cosmic_row(17).unwrap_or(50) as i32 / 100;
        }
        let knows = |t| self.finance_knows(civ, t) as i32;
        let has = |i| city.has_building(i) as i32;
        let wonder = |n| self.finance_wonder(rules, n);
        let holds = |n| wonder(n).is_some_and(|i| self.cities[i].owner & 15 == civ as u16);
        // Temple/Cathedral/Colosseum and Bach contributions; these become
        // tithes only under Fundamentalism. The helper is 0x444a61.
        let mut tithe = if has(4) != 0 {
            (knows(56) + knows(9)) * if holds(5) { 2 } else { 1 }
        } else {
            0
        };
        if has(14) != 0 {
            tithe += 3 + knows(24);
        }
        if knows(55) != 0 && (has(11) != 0 || holds(10)) {
            tithe += 2 + knows(15) + knows(82);
        }
        if holds(15) {
            tithe += 2;
        }
        if fundamentalist {
            taxes += tithe;
        }
        let multiplier = has(5) + has(10) + has(22);
        luxuries += luxuries * multiplier / 2;
        taxes += taxes * multiplier / 2;
        let multiplier = has(6) + has(12) + (has(26) != 0 || holds(26)) as i32;
        science += science * multiplier / if wonder(16) == Some(idx) { 1 } else { 2 };
        if wonder(11) == Some(idx) {
            science *= 2;
        }
        // Normal totals pass (mode 0), after multipliers. The UI's
        // alternate Capitalization preview modes do not run in this API.
        if city.building == -39 {
            taxes += net_shields.max(0);
        }
        CityFinance {
            luxuries,
            science,
            taxes,
        }
    }

    /// Finance from the current worked tiles, building multipliers,
    /// recurring routes, corruption and unit support.
    pub fn city_finance(&self, rules: &Rules, idx: usize) -> CityFinance {
        let output = self.city_output(rules, idx);
        self.split_city_trade(
            rules,
            idx,
            output.trade,
            (output.shields - self.unit_support(rules, idx)).max(0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> WorldState {
        let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
        w.wonders = [-1; 28];
        w.cities[1].buildings = [0; 5];
        w.cities[1].specialists = 0;
        w.cities[1].building = 0;
        w.science_rates[1] = 5;
        w.tax_rates[1] = 5;
        w
    }
    fn building(w: &mut WorldState, id: usize) {
        w.cities[1].buildings[id / 8] |= 1 << (id % 8);
    }

    #[test]
    fn rounding_remainder_and_specialists_precede_multipliers() {
        let rules = Rules::default();
        let mut w = world();
        assert_eq!(
            w.split_city_trade(&rules, 1, 1, 0),
            CityFinance {
                luxuries: 0,
                science: 0,
                taxes: 1
            }
        );
        w.science_rates[1] = 6;
        w.tax_rates[1] = 4;
        assert_eq!(w.split_city_trade(&rules, 1, 1, 0).science, 1);
        w.cities[1].specialists = 1 | (2 << 2) | (3 << 4);
        building(&mut w, 5); // marketplace
        building(&mut w, 6); // library
        assert_eq!(
            w.split_city_trade(&rules, 1, 0, 0),
            CityFinance {
                luxuries: 3,
                science: 4,
                taxes: 4
            }
        );
        w.cities[1].building = -39;
        assert_eq!(
            w.split_city_trade(&rules, 1, 0, 7).taxes,
            11,
            "Capitalization after marketplace, already net of support"
        );
    }

    #[test]
    fn science_wonders_multiply_in_order_and_expire_globally() {
        let mut w = world();
        building(&mut w, 6);
        building(&mut w, 12);
        w.wonders[16] = 1; // Newton: doubles the improvement bonus, not the total
        w.wonders[11] = 1; // Copernicus: doubles the resulting total
        assert_eq!(w.split_city_trade(&Rules::default(), 1, 12, 0).science, 36);
        w.wonders[16] = -1;
        assert_eq!(w.split_city_trade(&Rules::default(), 1, 12, 0).science, 24);
        // Physical row IDs must survive repeated scenario display labels.
        let rows = (0..28)
            .map(|i| format!("{}, ; *\n", if i == 11 { "Alp" } else { "nil" }))
            .collect::<String>();
        let rules = tot_kit::parse_rules(&format!(
            "@CIVILIZE\nAlphabet, 5, 1, nil, nil, 0, 3 ; Alp\n@ENDWONDER\n{rows}"
        ));
        w.techs[2][0] |= 1;
        assert_eq!(
            w.split_city_trade(&rules, 1, 12, 0).science,
            12,
            "foreign discovery obsoletes Copernicus"
        );
    }

    #[test]
    fn fundamentalism_caps_science_then_adds_specialists_and_tithes() {
        let mut w = world();
        w.governments[1] = 4;
        w.science_rates[1] = 6;
        w.tax_rates[1] = 2; // 20% luxuries, calculated before science cap
        w.cities[1].specialists = 3; // +3 science before penalty
        w.human_mask = 2;
        building(&mut w, 14); // cathedral => 3 tithe, +1 with tech 24
        w.techs[1][3] |= 1;
        assert_eq!(
            w.split_city_trade(&Rules::default(), 1, 10, 0),
            CityFinance {
                luxuries: 2,
                science: 4,
                taxes: 7
            }
        );
        w.human_mask = 0;
        assert_eq!(
            w.split_city_trade(&Rules::default(), 1, 10, 0),
            CityFinance {
                luxuries: 0,
                science: 5,
                taxes: 7
            }
        );
    }

    #[test]
    fn turns_refresh_finance_before_completing_a_library() {
        let mut w = world();
        let improvements = (0..7)
            .map(|i| format!("Building {i}, 1, 0, nil\n"))
            .collect::<String>();
        let rules = tot_kit::parse_rules(&format!("@IMPROVE\n{improvements}"));
        w.cities[1].specialists = 3; // scientist supplies 3, with no tile trade
        w.cities[1].building = -6;
        w.cities[1].shield_box = 100;
        w.cities[1].gross_food = 2;
        w.cities[1].net_science = 99;
        w.cities[1].net_taxes = 99;
        let treasury = w.treasury;
        w.run_turn(&rules, None);
        assert!(w.cities[1].has_building(6));
        assert_eq!((w.cities[1].net_science, w.cities[1].net_taxes), (3, 0));
        w.cities[1].gross_food = 2;
        w.run_turn(&rules, None);
        assert_eq!(w.cities[1].net_science, 4);
        assert_eq!(
            w.treasury, treasury,
            "maintenance/income accounting remains separate"
        );
    }
}
