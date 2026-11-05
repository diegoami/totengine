//! Wonder ownership and completion. Original table: 28 city indices;
//! -1 unbuilt, -2 destroyed. Effect IDs follow physical Rules row order.

use crate::{research::TechTree, WorldState};
use tot_kit::Rules;

impl WorldState {
    /// `0x444a28`: a wonder stops working when any civ knows its
    /// obsoleting advance. Missing ENDWONDER entries have no expiry.
    pub fn active_wonder_city(&self, rules: &Rules, wonder: usize) -> Option<usize> {
        let index = usize::try_from(*self.wonders.get(wonder)?).ok()?;
        self.cities.get(index).filter(|c| c.city_id != 0)?;
        let tree = TechTree::from_rules(rules);
        if let Some(raw) = rules
            .section("ENDWONDER")
            .and_then(|s| s.rows.get(wonder))
            .and_then(|r| r.first())
        {
            if let Some(tech) = tree.by_abbrev(raw) {
                if tech != 89 && (1..8).any(|c| self.techs[c][tech / 8] & (1 << (tech % 8)) != 0) {
                    return None;
                }
            }
        }
        Some(index)
    }

    pub fn has_active_wonder(&self, rules: &Rules, civ: u8, wonder: usize) -> bool {
        self.active_wonder_city(rules, wonder)
            .is_some_and(|i| (self.cities[i].owner & 15) as u8 == civ)
    }

    /// Leonardo (0x49d4f9): upgrade each owned live unit to the last
    /// qualifying Rules type, preserving spent moves/damage and clearing veteran.
    pub fn upgrade_obsolete_units(&mut self, rules: &Rules, civ: u8) -> Vec<usize> {
        if civ >= 8 || !self.has_active_wonder(rules, civ, 14) {
            return Vec::new();
        }
        let tree = TechTree::from_rules(rules);
        let known = |tech: usize| {
            self.techs[civ as usize]
                .get(tech / 8)
                .is_some_and(|b| b & (1 << (tech % 8)) != 0)
        };
        let mut upgrades = Vec::new();
        for (index, unit) in self.units.iter().enumerate() {
            if unit.id == 0 || unit.owner != civ {
                continue;
            }
            let old = unit.type_id as usize;
            let mut until = rules.unit_until_raw(old).and_then(|p| tree.by_abbrev(p));
            if rules.unit_role(old) == Some(1)
                && rules.unit_defense(old).unwrap_or(0) < rules.unit_defense(7).unwrap_or(0)
                && known(35)
            {
                until = Some(35);
            }
            let Some(until) = until.filter(|&t| known(t)) else {
                continue;
            };
            let candidate = (0..rules.unit_names().len().min(80)).rev().find(|&t| {
                rules.unit_civ_mask(t) & (1 << civ) != 0
                    && rules.unit_prereq_raw(t).and_then(|p| tree.by_abbrev(p)) == Some(until)
                    && rules.unit_role(t) == rules.unit_role(old)
                    && rules.unit_cost(t).unwrap_or(0) >= rules.unit_cost(old).unwrap_or(0)
            });
            if let Some(t) = candidate {
                upgrades.push((index, t));
            }
        }
        for &(index, t) in &upgrades {
            let unit = &mut self.units[index];
            let spent = (rules.unit_move(unit.type_id as usize).unwrap_or(0) * 3)
                .saturating_sub(unit.moves_left);
            unit.type_id = t as u8;
            unit.moves_left = (rules.unit_move(t).unwrap_or(0) * 3).saturating_sub(spent);
            unit.veteran = false;
        }
        upgrades.into_iter().map(|(i, _)| i).collect()
    }

    /// Original one-off completion dispatch, excluding notifications.
    /// Darwin keeps its required research choices pending for the player.
    pub(crate) fn wonder_completed(&mut self, rules: &Rules, civ: u8, wonder: usize) {
        if wonder == 20 && self.ai_state.civ_flags[civ as usize] & 16 == 0 {
            self.ai_state.civ_flags[civ as usize] |= 16;
            self.ai_state.reputation[civ as usize] >>= 1;
            for other in 1..8 {
                if other == civ as usize {
                    continue;
                }
                self.ai_state.attitudes[other][civ as usize] =
                    (self.ai_state.attitudes[other][civ as usize] as i32 - 25).clamp(0, 100) as u8;
                self.diplomacy[other][civ as usize] &= !16;
            }
        }
        if wonder == 25 {
            self.map.visible.fill(255);
            for cache in &mut self.map.known_improvements {
                *cache = self.map.improvements.clone();
            }
            for city in self.cities.iter_mut().filter(|c| c.city_id != 0) {
                city.known_to = 255;
                city.known_sizes = [city.size as u8; 8];
            }
        }
        if wonder == 23 {
            for reputation in &mut self.ai_state.reputation[1..] {
                *reputation = (*reputation as u16).div_ceil(2).min(6) as u8;
            }
        }
        if wonder == 14 {
            self.upgrade_obsolete_units(rules, civ);
        }
        if wonder == 18 {
            self.free_research[civ as usize] = self.free_research[civ as usize].saturating_add(2);
            if let Ok(target) = usize::try_from(self.research[civ as usize].target) {
                if self.grant_advance_with_rules(rules, civ, target) {
                    self.research[civ as usize].progress = 0;
                    self.free_research[civ as usize] -= 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        player::{Order, OrderResult},
        production::Built,
    };

    #[test]
    fn original_leonardo_upgrade_vectors() {
        use std::fmt::Write;
        let mut cases = 0;
        for line in include_str!("../../../docs/evidence/leonardo-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let parts: Vec<_> = line.split('|').collect();
            let fields: Vec<_> = parts[0].split_whitespace().collect();
            let v: Vec<u32> = fields[..8].iter().map(|s| s.parse().unwrap()).collect();
            let result: Vec<u32> = parts[1]
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            let rows: Vec<Vec<i32>> = fields[8..]
                .iter()
                .map(|row| row.split(',').map(|s| s.parse().unwrap()).collect())
                .collect();
            let mut text = String::from("@CIVILIZE\n");
            for t in 0..100 {
                writeln!(text, "Tech{t},1,0,nil,nil,0,0 ; T{t}").unwrap();
            }
            text.push_str("@UNITS\n");
            let tech = |t: i32| {
                if t == -1 {
                    "nil".to_string()
                } else if t == -2 {
                    "no".to_string()
                } else {
                    format!("T{t}")
                }
            };
            for (t, row) in rows.iter().enumerate() {
                writeln!(
                    text,
                    "Unit{t},{},0,1.,0,1a,{}d,1h,1f,{},0,{},{},000000000000000",
                    tech(row[0]),
                    row[2],
                    row[3],
                    row[1],
                    tech(row[4])
                )
                .unwrap();
            }
            text.push_str("@UNITS_ADVANCED\n");
            for row in &rows {
                writeln!(text, "{:08b},00000000,0,0,0,0,0", row[5]).unwrap();
            }
            let rules = tot_kit::parse_rules(&text);
            let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
            w.techs = [[0; 13]; 8];
            w.wonders = [-1; 28];
            if v[0] != 0 {
                w.wonders[14] = 0;
            }
            w.cities[0].city_id = 1;
            w.cities[0].owner = 1;
            for (i, t) in [0, 1, 2, 3, 4, 5, 35].into_iter().enumerate() {
                if v[1] & (1 << i) != 0 {
                    w.techs[1][t / 8] |= 1 << (t % 8);
                }
            }
            w.units.truncate(1);
            let u = &mut w.units[0];
            u.type_id = v[2] as u8;
            u.owner = v[3] as u8;
            u.id = v[4];
            u.veteran = v[5] & 8192 != 0;
            u.damage = v[6] as u8;
            u.moves_left = 3u32.saturating_sub(v[7]);
            w.upgrade_obsolete_units(&rules, 1);
            let u = &w.units[0];
            assert_eq!(
                [
                    u.type_id as u32,
                    u32::from(u.veteran) * 8192,
                    u.damage as u32
                ],
                result[..3],
                "{line}"
            );
            assert_eq!(u.moves_left, 3u32.saturating_sub(result[3]), "{line}");
            cases += 1;
        }
        assert_eq!(cases, 256);
    }

    #[test]
    fn original_statue_and_manhattan_mutations() {
        let rules = rules();
        let mut cases = 0;
        for line in include_str!("../../../docs/evidence/diplomatic-wonder-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let parts: Vec<_> = line.split('|').collect();
            let v: Vec<u32> = parts[0]
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            let expected: Vec<u32> = parts[1]
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
            let civ = v[1] as usize;
            w.ai_state.civ_flags[civ] = v[2] as u16;
            for c in 0..8 {
                w.ai_state.reputation[c] = v[3 + c] as u8;
                w.ai_state.attitudes[c][civ] = v[11 + c] as u8;
                w.diplomacy[c][civ] = v[19 + c];
            }
            w.wonder_completed(&rules, civ as u8, if v[0] == 0 { 20 } else { 23 });
            assert_eq!(w.ai_state.civ_flags[civ] as u32, expected[0], "{line}");
            assert_eq!(
                w.ai_state.reputation.map(u32::from),
                expected[1..9],
                "{line}"
            );
            assert_eq!(
                std::array::from_fn::<_, 8, _>(|c| w.ai_state.attitudes[c][civ] as u32),
                expected[9..17],
                "{line}"
            );
            assert_eq!(
                std::array::from_fn::<_, 8, _>(|c| w.diplomacy[c][civ]),
                expected[17..25],
                "{line}"
            );
            cases += 1;
        }
        assert_eq!(cases, 256);
    }

    #[test]
    fn original_apollo_visibility_and_city_knowledge() {
        let rules = rules();
        let mut cases = 0;
        for line in include_str!("../../../docs/evidence/apollo-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let parts: Vec<_> = line.split('|').collect();
            let v: Vec<u8> = parts[0]
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            let expected: Vec<u8> = parts[1]
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
            w.map.improvements = v[..32].iter().step_by(2).copied().collect();
            w.map.visible = v[1..32].iter().step_by(2).copied().collect();
            w.map.known_improvements = std::array::from_fn(|_| vec![13; 16]);
            w.cities.truncate(3);
            for (i, c) in w.cities.iter_mut().enumerate() {
                c.city_id = u32::from(i != 2);
                c.size = v[32 + i] as u16;
                c.known_to = 0;
                c.known_sizes = [0; 8];
            }
            w.wonder_completed(&rules, 1, 25);
            assert_eq!(w.map.visible, expected[..16], "{line}");
            for cache in &w.map.known_improvements {
                assert_eq!(cache, &expected[16..32], "{line}");
            }
            for (i, c) in w.cities.iter().enumerate() {
                assert_eq!(c.known_to, expected[32 + 9 * i], "{line}");
                assert_eq!(c.known_sizes, expected[33 + 9 * i..41 + 9 * i], "{line}");
            }
            cases += 1;
        }
        assert_eq!(cases, 32);
    }

    fn rules() -> Rules {
        let mut text = String::from("@CIVILIZE\nOne,1,0,nil,nil,0,0 ; One\nTwo,1,0,One,nil,0,0 ; Two\nThree,1,0,Two,nil,0,0 ; Thr\n@IMPROVE\n");
        for id in 0..68 {
            text.push_str(&format!("Building{id},2,0,nil\n"));
        }
        tot_kit::parse_rules(&text)
    }

    #[test]
    fn all_wonders_register_once_and_discard_overflow() {
        let rules = rules();
        for slot in 0..28 {
            let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
            w.human_mask = 0xff;
            w.wonders = [-1; 28];
            w.cities[1].flags = 1;
            w.cities[1].building = -(40 + slot as i8);
            w.cities[1].shield_box = 19;
            assert_eq!(w.production_step(&rules, 1), None);
            w.cities[1].shield_box = 25;
            assert_eq!(
                w.production_step(&rules, 1),
                Some(Built::Wonder(slot as u8))
            );
            assert_eq!(w.wonders[slot], 1);
            assert_eq!(w.cities[1].shield_box, 0);
            assert_eq!(w.cities[1].flags & 0x100, 0x100);
            w.cities[2].building = -(40 + slot as i8);
            w.cities[2].shield_box = 100;
            assert_eq!(w.production_step(&rules, 2), None);
            assert_eq!(w.wonders[slot], 1);
            w.wonders[slot] = -2;
            assert_eq!(w.production_step(&rules, 2), None);
            assert_eq!(w.wonders[slot], -2);
        }
    }

    #[test]
    fn darwin_grants_target_and_waits_for_next_player_choice() {
        let rules = rules();
        let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
        w.techs = [[0; 13]; 8];
        w.research[1].target = 0;
        w.research[1].progress = 5;
        w.wonder_completed(&rules, 1, 18);
        assert_eq!(w.techs[1][0], 1);
        assert_eq!(w.free_research[1], 1);
        assert_eq!(w.research[1].target, -1);
        assert_eq!(w.research[1].progress, 0);
        assert_eq!(
            w.apply_order(&rules, None, 1, Order::SetResearch { advance: 1 }),
            OrderResult::ResearchSet
        );
        assert_eq!(w.techs[1][0], 3);
        assert_eq!(w.free_research[1], 0);
        assert_eq!(w.research[1].target, -1);
        assert_eq!(
            w.apply_order(&rules, None, 1, Order::SetResearch { advance: 2 }),
            OrderResult::ResearchSet
        );
        assert_eq!(w.research[1].target, 2);
        assert_eq!(w.techs[1][0], 3);
    }

    #[test]
    fn completed_wonder_effect_expires_for_every_owner() {
        let rules = tot_kit::parse_rules("@CIVILIZE\nOne,1,0,nil,nil,0,0 ; One\n@ENDWONDER\nOne\n");
        let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
        w.wonders = [-1; 28];
        w.techs = [[0; 13]; 8];
        w.wonders[0] = 1;
        w.cities[1].city_id = 1;
        assert_eq!(w.active_wonder_city(&rules, 0), Some(1));
        w.grant_advance(2, 0);
        assert_eq!(w.active_wonder_city(&rules, 0), None);
        w.wonders[0] = -2;
        assert_eq!(w.active_wonder_city(&rules, 0), None);
    }

    #[test]
    fn players_can_order_unbuilt_wonders_only() {
        let rules = rules();
        let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
        w.wonders = [-1; 28];
        w.cities[1].city_id = 1;
        w.cities[1].owner = 1;
        assert_eq!(
            w.apply_order(&rules, None, 1, Order::SetProduction { city: 1, item: -40 }),
            OrderResult::ProductionSet
        );
        w.wonders[0] = 2;
        assert_eq!(
            w.apply_order(&rules, None, 1, Order::SetProduction { city: 1, item: -40 }),
            OrderResult::Rejected("cannot build")
        );
        w.wonders[0] = -2;
        assert_eq!(
            w.apply_order(&rules, None, 1, Order::SetProduction { city: 1, item: -40 }),
            OrderResult::Rejected("cannot build")
        );
        assert_eq!(
            w.apply_order(&rules, None, 1, Order::SetProduction { city: 1, item: -68 }),
            OrderResult::Rejected("cannot build")
        );
    }
}
