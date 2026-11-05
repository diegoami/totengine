//! Forced human anarchy from city unrest (`0x52588e -> 0x52522f`).
//! Player-selected revolutions and subsequent government choice are separate.
use crate::WorldState;
use tot_kit::Rules;

impl WorldState {
    /// Human-mask forced anarchy. A changed government clears incoming
    /// treaty bit 0x10 and refreshes own surface cities in ascending order.
    /// Returns whether the government changed; caller handles civic riot flags.
    pub fn force_human_anarchy(&mut self, rules: &Rules, civ: u8) -> bool {
        let c = civ as usize;
        if c >= 8 || self.human_mask & (1 << civ) == 0 || self.governments[c] == 0 {
            return false;
        }
        self.governments[c] = 0;
        for incoming in &mut self.diplomacy {
            incoming[c] &= !0x10;
        }
        for k in 0..self.cities.len() {
            let city = &self.cities[k];
            if city.city_id == 0 || city.map_id != 0 || city.owner & 15 != civ as u16 {
                continue;
            }
            self.refresh_city_totals(rules, k, false);
            // Fanatics -> Riflemen when leaving Fundamentalism. This is
            // the original's literal unit-index switch, also in scenarios.
            if self.cities[k].building == 8 {
                self.cities[k].building = 11;
            }
        }
        true
    }
}
