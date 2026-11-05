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

    /// Completion dispatch `0x4c8f7d`: Darwin (slot 18) calls normal
    /// discovery twice. Keep required choices pending for the player
    /// instead of silently selecting advances in the turn kernel.
    pub(crate) fn wonder_completed(&mut self, civ: u8, wonder: usize) {
        if wonder == 18 {
            self.free_research[civ as usize] = self.free_research[civ as usize].saturating_add(2);
            if let Ok(target) = usize::try_from(self.research[civ as usize].target) {
                if self.grant_advance(civ, target) {
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
        w.wonder_completed(1, 18);
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
