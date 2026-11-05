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

/// Original transient food-route inputs consumed by growth/starvation.
/// The headless round starts with no preceding city calculation.
#[derive(Debug, Clone, Copy)]
pub struct FoodRouteState {
    pub outgoing: i32,
    pub partner: i16,
}
impl Default for FoodRouteState {
    fn default() -> Self {
        Self {
            outgoing: 0,
            partner: -1,
        }
    }
}
impl FoodRouteState {
    pub fn refresh(&mut self, city: &crate::City) {
        self.outgoing = 0;
        self.partner = -1;
        for k in 0..city.route_count.clamp(0, 3) as usize {
            if city.route_commodities[k] < 0 {
                self.outgoing += 1;
                self.partner = city.route_partners[k];
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct CityStep {
    pub grew: bool,
    pub famined: bool,
    pub destroyed: bool,
    pub built: Option<crate::production::Built>,
    pub disbanded: Vec<usize>,
    pub discovery: Option<(u8, usize)>,
    pub account: crate::treasury::CityAccount,
    pub revolution: bool,
}

impl WorldState {
    /// Surface city phase: prepare growth, refresh/collect, produce, enforce
    /// support, refresh again, then resolve mood, science and upkeep.
    /// AI build choices remain player policy; settler-job planning, pollution
    /// and sea cargo are not part of this city-economics boundary.
    pub fn run_city_phase(
        &mut self,
        rules: &Rules,
        index: usize,
        food_routes: &mut FoodRouteState,
    ) -> Option<CityStep> {
        let city = self.cities.get(index)?;
        if city.city_id == 0 || city.map_id != 0 {
            return None;
        }
        let mut step = CityStep::default();
        let civ = (city.owner & 15) as u8;
        self.cities[index].flags &= 0xdfbeffbb;
        let context = self.city_growth_context(rules, index)?;
        if context.food_box < 0 {
            step.famined = true;
            let settler = self.units.iter().position(|u| {
                u.id != 0
                    && u.home as usize == index
                    && rules.unit_role(u.type_id as usize) == Some(5)
            });
            match crate::growth::famine_action(
                self.human_mask,
                civ,
                self.cities[index].known_to,
                food_routes.outgoing,
                food_routes.partner,
                settler,
            ) {
                crate::growth::FamineAction::CancelFoodRoutes(partner) => {
                    let c = &mut self.cities[index];
                    if crate::growth::cancel_food_routes(
                        &mut c.route_count,
                        &mut c.route_commodities,
                        &mut c.route_partners,
                        partner,
                    ) {
                        c.flags |= crate::founding::CITY_REFRESH;
                    }
                }
                crate::growth::FamineAction::RemoveSettler(unit) => {
                    self.remove_land_unit(unit);
                    step.disbanded.push(unit);
                }
                crate::growth::FamineAction::RetainSettler => (),
                crate::growth::FamineAction::LosePopulation => {
                    self.cities[index].size =
                        (self.cities[index].size as u8).wrapping_sub(1) as u16;
                    if self.cities[index].size as u8 as i8 <= 0 {
                        step.destroyed = self.destroy_surface_city(rules, index);
                        return Some(step);
                    }
                }
            }
            self.cities[index].food_box = 0;
        } else {
            let growth = crate::growth::prepare_growth(context);
            step.grew = growth.size > context.size;
            self.cities[index].size = growth.size as u16;
            self.cities[index].food_box = growth.food_box as u16;
        }
        let pre = self.refresh_city_totals(rules, index, false)?;
        food_routes.refresh(&self.cities[index]);
        let surplus = pre.output.food
            - self.cities[index].size as u8 as i8 as i32 * rules.cosmic_food_per_citizen() as i32
            - pre.settlers * pre.settler_food;
        self.cities[index].food_box = (self.cities[index].food_box as i16 as i32 + surplus) as u16;
        let rows = self.shield_rows(rules, civ);
        step.built = self.production_step(rules, index);
        step.disbanded
            .extend(self.disband_city_units(rules, index, pre.output.shields, rows));
        let post = self.refresh_city_totals(rules, index, true)?;
        food_routes.refresh(&self.cities[index]);
        let city = &self.cities[index];
        let transition = crate::happiness::transition(&crate::happiness::MoodTransitionInput {
            size: city.size as u8,
            owner: civ,
            government: self.governments[civ as usize],
            human: self.human_mask & (1 << civ) != 0,
            flags: city.flags,
            happy: post.mood.happy,
            unhappy: post.mood.unhappy,
            food: post.output.food,
            eaten: city.size as u8 as i8 as i32 * rules.cosmic_food_per_citizen() as i32
                + post.settlers * post.settler_food,
            net_shields: post.output.shields - post.support,
            surplus,
            taxes: post.finance.taxes,
            scenario: self.research_settings.flags & 128 != 0,
            scenario_flags: self.ai_state.scenario_flags.unwrap_or(0),
            growth_blocked: self
                .city_growth_context(rules, index)?
                .blocking_improvement()
                .is_some(),
        });
        step.grew |= transition.size > city.size as u8;
        self.cities[index].size = transition.size as u16;
        self.cities[index].flags = transition.flags;
        if transition.revolution {
            self.ai_state.civ_flags[civ as usize] &= !1;
            step.revolution = self.force_human_anarchy(rules, civ);
            self.ai_state.civ_flags[civ as usize] |= 1;
            if let Some(last) = self
                .cities
                .iter()
                .rfind(|c| c.city_id != 0 && c.map_id == 0 && c.owner & 15 == civ as u16)
            {
                food_routes.refresh(last);
            }
        }
        // 0x4ca000: anarchy contributes no research. The Chieftain human
        // bonus doubles science toward an advance known by another civ.
        // The late spaceship-related AI bonus remains separate.
        if self.governments[civ as usize] != 0 {
            let mut science = post.finance.science;
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
            step.discovery = self.add_research(rules, civ, science).map(|t| (civ, t));
        }
        step.account = self.process_city_treasury(rules, index, post.finance.taxes);
        Some(step)
    }
}
