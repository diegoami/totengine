//! City income and upkeep (`0x4c9d58`, `0x4ca5ae`, `0x4ca6e0`).
//! The original city-turn caller charges upkeep only to human-mask civs.

use crate::WorldState;
use tot_kit::Rules;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CityAccount {
    pub income: i32,
    pub maintenance: i32,
    pub sold: Vec<usize>,
}

/// Original per-improvement upkeep, after resolving the Barracks techs.
pub fn upkeep(
    base: i32,
    improvement: usize,
    difficulty: u8,
    barracks_techs: [bool; 2],
    smith: bool,
    government: u8,
) -> i32 {
    let mut cost = base;
    if improvement == 2 {
        if difficulty < 2 && cost != 0 {
            cost -= 1;
        }
        cost += barracks_techs.into_iter().filter(|v| *v).count() as i32;
    }
    if cost == 1 && smith {
        cost = 0;
    }
    if government == 4 && [4, 11, 14].contains(&improvement) {
        cost = 0;
    }
    cost
}

impl WorldState {
    pub fn improvement_upkeep(&self, rules: &Rules, civ: u8, improvement: usize) -> i32 {
        let c = civ as usize;
        if c >= 8 {
            return 0;
        }
        let knows = |t: usize| c != 0 && t != 89 && self.techs[c][t / 8] & (1 << (t % 8)) != 0;
        // Rules loader 0x413146 marks a tech unavailable when its first
        // prerequisite is `no`. The Barracks lookup follows that disabled
        // entry's first prerequisite (-2), for which 0x49c820 returns false.
        let second_barracks_tech = rules
            .section("CIVILIZE")
            .and_then(|s| s.rows.get(53))
            .and_then(|r| r.get(3))
            .is_some_and(|p| p != "no")
            && knows(53);
        upkeep(
            rules.improvement_upkeep(improvement).unwrap_or(0) as u8 as i32,
            improvement,
            self.difficulty,
            [knows(35), second_barracks_tech],
            self.has_active_wonder(rules, civ, 17),
            self.governments[c],
        )
    }

    /// Credit this evaluation's taxes, then charge existing improvements
    /// in ID order. Unaffordable upkeep clamps the balance to zero, removes
    /// that improvement and refunds its full production cost, as original
    /// 0x4ca7b8..0x4ca85d does. The UI notification has no engine effect.
    pub fn process_city_treasury(&mut self, rules: &Rules, idx: usize, taxes: i32) -> CityAccount {
        let Some(city) = self.cities.get(idx) else {
            return CityAccount::default();
        };
        let civ = (city.owner & 15) as usize;
        if self.governments[civ] == 0 {
            return CityAccount::default();
        }
        let income = if city.happy >= city.unhappy { taxes } else { 0 };
        self.treasury[civ] = self.treasury[civ].wrapping_add(income);
        let mut account = CityAccount {
            income,
            ..CityAccount::default()
        };
        if self.human_mask & (1 << civ) == 0 {
            return account;
        }
        let rows = self.shield_rows(rules, civ as u8);
        for building in 1..36 {
            if !self.cities[idx].has_building(building) {
                continue;
            }
            let cost = self.improvement_upkeep(rules, civ as u8, building);
            account.maintenance += cost;
            self.treasury[civ] = self.treasury[civ].wrapping_sub(cost);
            if self.treasury[civ] < 0 {
                self.cities[idx].buildings[building / 8] &= !(1 << (building % 8));
                self.treasury[civ] =
                    (rules.improvement_cost(building).unwrap_or(0) as i32).wrapping_mul(rows);
                account.sold.push(building);
            }
        }
        account
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_upkeep_vectors() {
        let mut count = 0;
        for line in include_str!("../../../docs/evidence/maintenance-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let v: Vec<i32> = line
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            assert_eq!(
                upkeep(
                    v[0],
                    v[1] as usize,
                    v[2] as u8,
                    [v[3] != 0, v[4] != 0],
                    v[5] != 0,
                    v[6] as u8
                ),
                v[7],
                "{line}"
            );
            count += 1;
        }
        assert_eq!(count, 128);
    }
    #[test]
    fn maintenance_exemptions_and_bankruptcy_order() {
        let rules = tot_kit::parse_rules("@IMPROVE\nNone,0,0,nil\nPalace,10,0,nil\nBarracks,4,1,nil\nGranary,6,1,nil\nTemple,4,1,nil\n");
        let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
        w.wonders = [-1; 28];
        w.techs = [[0; 13]; 8];
        w.governments[1] = 2;
        w.difficulty = 3;
        w.human_mask = 2;
        w.cities[1].owner = 1;
        w.cities[1].happy = 0;
        w.cities[1].unhappy = 1;
        w.cities[1].buildings = [0b11100, 0, 0, 0, 0];
        w.treasury[1] = 0;
        let account = w.process_city_treasury(&rules, 1, 50);
        assert_eq!(
            (account.income, account.maintenance, account.sold),
            (0, 3, vec![2])
        );
        assert_eq!(w.treasury[1], 38); // 40 refund pays the next two buildings.
        assert!(!w.cities[1].has_building(2));
        w.human_mask = 0;
        w.cities[1].unhappy = 0;
        let account = w.process_city_treasury(&rules, 1, 5);
        assert_eq!((account.income, account.maintenance), (5, 0));
        assert_eq!(w.treasury[1], 43);
        w.governments[1] = 0;
        assert_eq!(
            w.process_city_treasury(&rules, 1, 50),
            CityAccount::default()
        );
        assert_eq!(w.treasury[1], 43);
    }

    #[test]
    fn turns_credit_taxes_and_charge_upkeep_once() {
        let rules = tot_kit::parse_rules("@IMPROVE\nNone,0,0,nil\nPalace,10,0,nil\nBarracks,4,0,nil\nGranary,6,0,nil\nTemple,4,1,nil\n");
        let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
        for c in &mut w.cities {
            c.gross_food = 0;
        }
        w.wonders = [-1; 28];
        w.techs = [[0; 13]; 8];
        w.human_mask = 2;
        w.governments[1] = 2;
        w.difficulty = 3;
        w.treasury[1] = 0;
        let c = &mut w.cities[1];
        c.owner = 1;
        c.size = 1;
        c.gross_food = 2;
        c.specialists = 2;
        c.worked = 0;
        c.building = -39;
        c.buildings = [1 << 4, 0, 0, 0, 0];
        c.flags = 0;
        for expected in [2, 4] {
            let report = w.run_turn(&rules, None);
            assert_eq!(w.treasury[1], expected);
            assert_eq!(report.city_accounts.len(), 1);
            assert_eq!(
                (
                    report.city_accounts[0].1.income,
                    report.city_accounts[0].1.maintenance
                ),
                (3, 1)
            );
        }
    }
}
