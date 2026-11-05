//! Original unit-slot selection (`0x56a2d9..0x56a3d7`).
use crate::{Unit, WorldState};
use tot_kit::Rules;

impl WorldState {
    pub(crate) fn available_unit_slot(
        &self,
        rules: &Rules,
        owner: u8,
        type_id: usize,
    ) -> Option<usize> {
        let civ = owner as usize;
        if civ >= 8 || type_id >= rules.unit_names().len() {
            return None;
        }
        let free = self.units.iter().position(|u| u.id == 0);
        if free.is_none() {
            let military = self
                .units
                .iter()
                .filter(|u| {
                    u.id != 0
                        && u.owner == owner
                        && rules.unit_role(u.type_id as usize).unwrap_or(0) <= 4
                })
                .count();
            if self.units.len() >= 2048
                || military > 1948
                || (self.human_mask & (1 << civ) == 0 && self.units.len() >= 2040)
            {
                return None;
            }
        }
        Some(free.unwrap_or(self.units.len()))
    }

    /// Reuse the first deleted slot before appending. Capacity checks only
    /// apply to new slots; the original reserves the final eight for humans.
    /// Engine identities preserve the imported values and advance monotonically
    /// rather than reproducing the original loader's transient renumbering.
    pub(crate) fn allocate_unit(&mut self, rules: &Rules, mut unit: Unit) -> Option<usize> {
        let idx = self.available_unit_slot(rules, unit.owner, unit.type_id as usize)?;
        let max = self.units.iter().map(|u| u.id).max().unwrap_or(0);
        self.next_unit_id = self.next_unit_id.max(max.wrapping_add(1));
        unit.id = self.next_unit_id;
        self.next_unit_id = self.next_unit_id.wrapping_add(1);
        // Original creation inserts at the stack head (live hut fixture).
        for existing in self
            .units
            .iter_mut()
            .filter(|u| u.id != 0 && (u.x, u.y) == (unit.x, unit.y))
        {
            existing.stack_pos = existing.stack_pos.saturating_add(1);
        }
        unit.stack_pos = 0;
        if let Some(slot) = self.units.get_mut(idx) {
            *slot = unit;
        } else {
            self.units.push(unit);
        }
        Some(idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const RULES: &str = "@UNITS\nWarrior,nil,0,1.,0,1a,1d,1h,1f,1,0,1,nil,000000000000000\nSettler,nil,0,1.,0,0a,1d,1h,1f,1,0,5,nil,000000000000000\n";

    #[test]
    fn original_slot_decisions_and_wide_live_ids() {
        let rules = tot_kit::parse_rules(RULES);
        let mut cases = 0;
        for line in include_str!("../../../docs/evidence/unit-slot-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let v: Vec<i32> = line
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
            let template = w.units[0].clone();
            w.units = (0..v[0])
                .map(|i| Unit {
                    id: if i == v[3] { 0 } else { 65536 + i as u32 },
                    owner: 1,
                    type_id: if i < v[2] { 0 } else { 1 },
                    ..template.clone()
                })
                .collect();
            w.human_mask = if v[1] != 0 { 2 } else { 0 };
            assert_eq!(
                w.available_unit_slot(&rules, 1, 0).map_or(-1, |i| i as i32),
                v[4],
                "{line}"
            );
            cases += 1;
        }
        assert_eq!(cases, 124);
    }

    #[test]
    fn deleted_slots_reuse_indices_without_reusing_identities() {
        let rules = tot_kit::parse_rules(RULES);
        let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
        let mut unit = w.units[0].clone();
        unit.owner = 1;
        unit.type_id = 0;
        w.units = vec![unit.clone()];
        w.units[0].id = 65536;
        w.next_unit_id = 65537;
        assert_eq!(w.allocate_unit(&rules, unit.clone()), Some(1));
        assert_eq!(w.units[1].id, 65537);
        w.units[1].id = 0;
        assert_eq!(w.allocate_unit(&rules, unit), Some(1));
        assert_eq!(w.units[1].id, 65538);
        assert_eq!(w.units.len(), 2);
        let mut record = tot_kit::UnitRecord { raw: [0; 40] };
        record.raw[30..34].copy_from_slice(&65536u32.to_le_bytes());
        assert_eq!(record.id(), 65536);
        let mut city = tot_kit::CityRecord { raw: [0; 92] };
        city.raw[86..90].copy_from_slice(&65536u32.to_le_bytes());
        assert_eq!(city.city_id(), 65536);
    }
}
