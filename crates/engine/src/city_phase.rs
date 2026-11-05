//! City-total refresh boundary, following original `0x4c5268`.
//! Growth, collection, production and income are separate caller phases.
use crate::{finance::CityFinance, happiness::Happiness, workers, yields::CityOutput, WorldState};
use tot_kit::Rules;

#[derive(Debug, Clone, Copy)]
pub struct CityTotals {
    pub output: CityOutput,
    pub finance: CityFinance,
    pub mood: Happiness,
    pub support: i32,
    pub settlers: i32,
    pub settler_food: i32,
}

impl WorldState {
    /// Reassign workers and refresh the city's cached totals. `processing`
    /// selects the original post-production worker adjustment (`0x5d75ac`).
    /// Surface single-player context; this does not update disorder flags.
    pub fn refresh_city_totals(
        &mut self,
        rules: &Rules,
        index: usize,
        processing: bool,
    ) -> Option<CityTotals> {
        let city = self.cities.get(index)?;
        if city.city_id == 0 || city.map_id != 0 {
            return None;
        }
        let owner = (city.owner & 15) as u8;
        let settlers = self
            .units
            .iter()
            .filter(|u| {
                u.id != 0
                    && u.owner == owner
                    && u.home as usize == index
                    && rules.unit_role(u.type_id as usize) == Some(5)
            })
            .count() as i32;
        let support = self.unit_support(rules, index);
        let settler_food = rules.cosmic_settler_food(self.governments[owner as usize] > 2) as i32;
        let result = workers::allocate(&workers::WorkerContext {
            size: city.size as u8,
            worked: city.worked,
            flags: city.flags,
            human: self.human_mask & (1 << owner) != 0,
            phase_turn: self.turn as i32 + index as i32,
            processing,
            food_each: rules.cosmic_food_per_citizen() as i32,
            settlers,
            settler_food,
            support,
            food_box: city.food_box as i16,
            unavailable: self.worker_availability(rules, index),
            yields: std::array::from_fn(|k| self.tile_yield(rules, index, k)),
        });
        let city = &mut self.cities[index];
        city.worked = result.worked;
        city.flags = result.flags;
        city.food_box = result.food_box as u16;
        city.specialists = workers::normalize_specialists(city.specialists, city.worked);
        self.cities[index].route_trade = self.pre_route_trade(rules, index);
        let output = self.city_output(rules, index);
        let finance = self.split_city_trade(rules, index, output.trade, output.shields - support);
        let mood = self.city_happiness_with_luxuries(rules, index, finance.luxuries);
        let city = &mut self.cities[index];
        city.gross_food = output.food as u8;
        city.gross_shields = output.shields as u8;
        city.gross_trade = output.trade as u16;
        city.net_science = finance.science as u16;
        city.net_taxes = finance.taxes as u16;
        city.happy = mood.happy as u8;
        city.unhappy = mood.unhappy as u8;
        city.flags &= !crate::founding::CITY_REFRESH;
        Some(CityTotals {
            output,
            finance,
            mood,
            support,
            settlers,
            settler_food,
        })
    }
}
