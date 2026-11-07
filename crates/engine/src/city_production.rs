//! City-production arithmetic from unpatched original execution.
//! See city-production-v1.json for fragment boundaries and UI substitutions.
//! Quotes validate owned surface-city transactions and captured switch budgets.
//! Native arithmetic evidence alone does not establish full dialog/window parity.
use crate::WorldState;
use tot_kit::Rules;

/// Engine interaction state, not a field imported from native saves. Native
/// city-window code captures the initial production class/shield budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProductionContext {
    pub civ: u8,
    pub city: usize,
    pub city_id: u32,
    pub initial_item: i8,
    pub initial_shields: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductionChangeQuote {
    pub city_id: u32,
    pub item: i8,
    pub current_item: i8,
    pub current_shields: u16,
    pub initial_item: i8,
    pub initial_shields: i16,
    pub retained: u16,
    pub difficulty: u8,
    pub penalty: u8,
    pub target_cost: Option<u16>,
}

impl ProductionChangeQuote {
    pub(crate) fn encode(self) -> String {
        format!(
            "{} {} {} {} {} {} {} {} {} {}",
            self.city_id,
            self.item,
            self.current_item,
            self.current_shields,
            self.initial_item,
            self.initial_shields,
            self.retained,
            self.difficulty,
            self.penalty,
            self.target_cost.map_or(-1, i32::from)
        )
    }
    pub(crate) fn decode(fields: &[&str]) -> Option<Self> {
        let [city_id, item, current_item, current_shields, initial_item, initial_shields, retained, difficulty, penalty, target_cost] =
            fields
        else {
            return None;
        };
        let cost: i32 = target_cost.parse().ok()?;
        Some(Self {
            city_id: city_id.parse().ok()?,
            item: item.parse().ok()?,
            current_item: current_item.parse().ok()?,
            current_shields: current_shields.parse().ok()?,
            initial_item: initial_item.parse().ok()?,
            initial_shields: initial_shields.parse().ok()?,
            retained: retained.parse().ok()?,
            difficulty: difficulty.parse().ok()?,
            penalty: penalty.parse().ok()?,
            target_cost: if cost == -1 {
                None
            } else {
                Some(u16::try_from(cost).ok()?)
            },
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HurryQuote {
    pub city_id: u32,
    pub item: i8,
    pub stored: u16,
    pub required: u16,
    pub price: i32,
    pub treasury: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaleQuote {
    pub city_id: u32,
    pub improvement: u8,
    pub income: i32,
    pub treasury: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CityProductionView {
    pub index: usize,
    pub item: i8,
    pub stored: u16,
    pub required: Option<u16>,
    pub remaining: Option<u16>,
    /// Estimate at current output; zero means funded for the production phase.
    pub turns: Option<u32>,
    pub net_shields: i32,
    pub hurry: Result<HurryQuote, &'static str>,
    pub improvements: Vec<(u8, Result<SaleQuote, &'static str>)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaleEffects {
    pub food_surplus: i32,
    pub net_shields: i32,
    pub science: i32,
    pub taxes: i32,
    pub luxuries: i32,
    pub happy: i32,
    pub unhappy: i32,
    pub upkeep: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SalePreview {
    pub quote: SaleQuote,
    pub before: SaleEffects,
    pub after: SaleEffects,
}

impl HurryQuote {
    pub(crate) fn encode(self) -> String {
        format!(
            "{} {} {} {} {} {}",
            self.city_id, self.item, self.stored, self.required, self.price, self.treasury
        )
    }
    pub(crate) fn decode(fields: &[&str]) -> Option<Self> {
        let [city_id, item, stored, required, price, treasury] = fields else {
            return None;
        };
        Some(Self {
            city_id: city_id.parse().ok()?,
            item: item.parse().ok()?,
            stored: stored.parse().ok()?,
            required: required.parse().ok()?,
            price: price.parse().ok()?,
            treasury: treasury.parse().ok()?,
        })
    }
}
impl SaleQuote {
    pub(crate) fn encode(self) -> String {
        format!(
            "{} {} {} {}",
            self.city_id, self.improvement, self.income, self.treasury
        )
    }
    pub(crate) fn decode(fields: &[&str]) -> Option<Self> {
        let [city_id, improvement, income, treasury] = fields else {
            return None;
        };
        Some(Self {
            city_id: city_id.parse().ok()?,
            improvement: improvement.parse().ok()?,
            income: income.parse().ok()?,
            treasury: treasury.parse().ok()?,
        })
    }
}

impl WorldState {
    fn production_cost(rules: &Rules, item: i8) -> Option<u16> {
        if item == -39 {
            return None;
        }
        let cost = if item >= 0 {
            rules.unit_cost(item as usize).filter(|&n| n <= 127)
        } else {
            let id = (-i32::from(item)) as usize;
            if id >= 68 {
                return None;
            }
            rules.improvement_cost(id).filter(|&n| n <= 255)
        }?;
        cost.checked_mul(u32::from(Self::production_rows_byte(rules).ok()?))
            .filter(|&n| n <= i16::MAX as u32)
            .map(|n| n as u16)
    }
    pub fn production_context_city(&self, civ: u8) -> Option<(usize, u32)> {
        let context = self.production_context?;
        if context.civ != civ
            || self.production_city(civ, context.city).is_err()
            || self.cities[context.city].city_id != context.city_id
        {
            return None;
        }
        Some((context.city, context.city_id))
    }

    pub fn quote_production_change(
        &self,
        rules: &Rules,
        civ: u8,
        index: usize,
        item: i8,
    ) -> Result<ProductionChangeQuote, &'static str> {
        self.production_city(civ, index)?;
        if !self.can_set_production(rules, civ, index, item) {
            return Err("cannot build");
        }
        let target_cost = Self::production_cost(rules, item);
        if item != -39 && target_cost.is_none() {
            return Err("unsupported production cost");
        }
        let city = &self.cities[index];
        let stored = i16::try_from(city.shield_box).map_err(|_| "unsupported signed shield box")?;
        let context = self
            .production_context
            .filter(|c| c.civ == civ && c.city == index && c.city_id == city.city_id)
            .unwrap_or(ProductionContext {
                civ,
                city: index,
                city_id: city.city_id,
                initial_item: city.building,
                initial_shields: stored,
            });
        let penalty = u8::try_from(rules.cosmic_row(18).unwrap_or(50))
            .map_err(|_| "unsupported production penalty")?;
        let retained = retained_shields(
            context.initial_item,
            item,
            context.initial_shields,
            self.difficulty,
            penalty,
        );
        if !(0..=i32::from(i16::MAX)).contains(&retained) {
            return Err("unsupported retained shield amount");
        }
        Ok(ProductionChangeQuote {
            city_id: city.city_id,
            item,
            current_item: city.building,
            current_shields: city.shield_box,
            initial_item: context.initial_item,
            initial_shields: context.initial_shields,
            retained: retained as u16,
            difficulty: self.difficulty,
            penalty,
            target_cost,
        })
    }

    pub fn change_quoted_production(
        &mut self,
        rules: &Rules,
        civ: u8,
        index: usize,
        expected: ProductionChangeQuote,
    ) -> Result<(), &'static str> {
        let current = self.quote_production_change(rules, civ, index, expected.item)?;
        if current != expected {
            return Err("stale production quote");
        }
        self.production_context = Some(ProductionContext {
            civ,
            city: index,
            city_id: current.city_id,
            initial_item: current.initial_item,
            initial_shields: current.initial_shields,
        });
        self.cities[index].building = current.item;
        self.cities[index].shield_box = current.retained;
        Ok(())
    }

    pub fn close_production_context(
        &mut self,
        civ: u8,
        index: usize,
        city_id: u32,
    ) -> Result<(), &'static str> {
        self.production_city(civ, index)?;
        if self.cities[index].city_id != city_id {
            return Err("stale city identity");
        }
        if let Some(c) = self.production_context {
            if c.civ != civ || c.city != index || c.city_id != city_id {
                return Err("production context belongs to another city");
            }
        }
        self.production_context = None;
        Ok(())
    }

    fn sale_effects(&self, rules: &Rules, civ: u8, index: usize) -> SaleEffects {
        let output = self.city_output(rules, index);
        let support = self.unit_support(rules, index);
        let finance = self.city_finance(rules, index);
        let mood = self.city_happiness_with_luxuries(rules, index, finance.luxuries);
        let food_eaten = i32::from(self.cities[index].size)
            * rules.cosmic_food_per_citizen() as i32
            + self
                .units
                .iter()
                .filter(|u| {
                    u.id != 0
                        && u.owner == civ
                        && u.home as usize == index
                        && rules.unit_role(u.type_id as usize) == Some(5)
                })
                .count() as i32
                * rules.cosmic_settler_food(self.governments[civ as usize] > 2) as i32;
        SaleEffects {
            food_surplus: output.food - food_eaten,
            net_shields: output.shields - support,
            science: finance.science,
            taxes: finance.taxes,
            luxuries: finance.luxuries,
            happy: mood.happy,
            unhappy: mood.unhappy,
            upkeep: (1..36)
                .filter(|&i| self.cities[index].has_building(i))
                .map(|i| self.improvement_upkeep(rules, civ, i))
                .sum(),
        }
    }

    /// One selected sale preview, evaluated on a clone; reads never mutate play.
    pub fn preview_sale(
        &self,
        rules: &Rules,
        civ: u8,
        index: usize,
        improvement: u8,
    ) -> Result<SalePreview, &'static str> {
        let quote = self.quote_sale(rules, civ, index, improvement)?;
        let before = self.sale_effects(rules, civ, index);
        let mut after = self.clone();
        after.sell_improvement(rules, civ, index, quote)?;
        Ok(SalePreview {
            quote,
            before,
            after: after.sale_effects(rules, civ, index),
        })
    }

    pub fn city_production_view(
        &self,
        rules: &Rules,
        civ: u8,
        index: usize,
    ) -> Option<CityProductionView> {
        self.production_city(civ, index).ok()?;
        let city = &self.cities[index];
        let required = if city.building == -39 {
            None
        } else {
            let cost = if city.building >= 0 {
                rules
                    .unit_cost(city.building as usize)
                    .filter(|&n| n <= 127)
            } else {
                let id = -i32::from(city.building) as usize;
                (id < 68).then(|| rules.improvement_cost(id)).flatten()
            };
            cost.and_then(|cost| {
                cost.checked_mul(u32::from(Self::production_rows_byte(rules).ok()?))
            })
            .filter(|&n| n <= i16::MAX as u32)
            .map(|n| n as u16)
        };
        let remaining = required.map(|cost| cost.saturating_sub(city.shield_box));
        let net_shields = if city.flags & 1 != 0 {
            0
        } else {
            (self.city_output(rules, index).shields - self.unit_support(rules, index)).max(0)
        };
        let turns = remaining.and_then(|n| {
            if n == 0 {
                Some(0)
            } else if net_shields > 0 {
                Some(u32::from(n).div_ceil(net_shields as u32))
            } else {
                None
            }
        });
        Some(CityProductionView {
            index,
            item: city.building,
            stored: city.shield_box,
            required,
            remaining,
            turns,
            net_shields,
            hurry: self.quote_hurry(rules, civ, index),
            improvements: (1..36)
                .filter(|&id| city.has_building(id as usize))
                .map(|id| (id, self.quote_sale(rules, civ, index, id)))
                .collect(),
        })
    }

    fn production_city(&self, civ: u8, index: usize) -> Result<(), &'static str> {
        let city = self.cities.get(index).ok_or("no such city")?;
        if city.city_id == 0 {
            return Err("no such city");
        }
        if civ >= 8 || city.owner & 15 != u16::from(civ) {
            return Err("not your city");
        }
        if city.map_id != 0 || self.human_mask & (1 << civ) == 0 {
            return Err("requires a human surface city");
        }
        Ok(())
    }

    fn production_rows_byte(rules: &Rules) -> Result<u8, &'static str> {
        u8::try_from(rules.cosmic_row(4).unwrap_or(10))
            .ok()
            .filter(|&n| n > 0)
            .ok_or("unsupported production shield rows")
    }

    /// Read-only quote. Treasury is shown even when it cannot cover the price.
    pub fn quote_hurry(
        &self,
        rules: &Rules,
        civ: u8,
        index: usize,
    ) -> Result<HurryQuote, &'static str> {
        self.production_city(civ, index)?;
        let city = &self.cities[index];
        if city.building == -39 {
            return Err("capitalization cannot be bought");
        }
        if city.building >= 0 && city.flags & 1 != 0 {
            return Err("units cannot be bought during disorder");
        }
        let cost = if city.building >= 0 {
            rules
                .unit_cost(city.building as usize)
                .filter(|&c| c <= 127)
        } else {
            let id = (-i32::from(city.building)) as usize;
            if id >= 68 {
                return Err("unsupported production item");
            }
            rules.improvement_cost(id)
        }
        .and_then(|c| u8::try_from(c).ok())
        .filter(|&c| c > 0)
        .ok_or("unsupported production cost")?;
        let rows = Self::production_rows_byte(rules)?;
        let stored = i16::try_from(city.shield_box).map_err(|_| "unsupported signed shield box")?;
        let required = u16::from(cost) * u16::from(rows);
        if required > i16::MAX as u16 {
            return Err("unsupported production shield cost");
        }
        if city.shield_box >= required {
            return Err("production is already funded");
        }
        let price =
            hurry_price(city.building, cost, rows, stored).ok_or("production cannot be bought")?;
        Ok(HurryQuote {
            city_id: city.city_id,
            item: city.building,
            stored: city.shield_box,
            required,
            price,
            treasury: self.treasury[civ as usize],
        })
    }

    /// Validated transaction: fill the box, without creating the item or ticking.
    pub fn buy_production(
        &mut self,
        rules: &Rules,
        civ: u8,
        index: usize,
        expected: HurryQuote,
    ) -> Result<(), &'static str> {
        let current = self.quote_hurry(rules, civ, index)?;
        if current != expected {
            return Err("stale purchase quote");
        }
        if current.treasury < current.price {
            return Err("not enough treasury");
        }
        self.treasury[civ as usize] -= current.price;
        self.cities[index].shield_box = current.required;
        self.production_context = Some(ProductionContext {
            civ,
            city: index,
            city_id: current.city_id,
            initial_item: current.item,
            initial_shields: current.required as i16,
        });
        Ok(())
    }

    pub fn quote_sale(
        &self,
        rules: &Rules,
        civ: u8,
        index: usize,
        improvement: u8,
    ) -> Result<SaleQuote, &'static str> {
        self.production_city(civ, index)?;
        let city = &self.cities[index];
        if !(2..36).contains(&improvement) {
            return Err("this is not a saleable improvement");
        }
        if !city.has_building(improvement as usize) {
            return Err("city does not have that improvement");
        }
        if city.flags & 4 != 0 {
            return Err("an improvement was already sold this turn");
        }
        let cost = rules
            .improvement_cost(improvement as usize)
            .and_then(|c| u8::try_from(c).ok())
            .ok_or("unsupported improvement cost")?;
        let income = sale_value(cost, Self::production_rows_byte(rules)?);
        self.treasury[civ as usize]
            .checked_add(income)
            .ok_or("treasury capacity reached")?;
        Ok(SaleQuote {
            city_id: city.city_id,
            improvement,
            income,
            treasury: self.treasury[civ as usize],
        })
    }

    pub fn sell_improvement(
        &mut self,
        rules: &Rules,
        civ: u8,
        index: usize,
        expected: SaleQuote,
    ) -> Result<(), &'static str> {
        let current = self.quote_sale(rules, civ, index, expected.improvement)?;
        if current != expected {
            return Err("stale sale quote");
        }
        self.treasury[civ as usize] += current.income;
        let id = current.improvement as usize;
        self.cities[index].buildings[id / 8] &= !(1 << (id % 8));
        self.cities[index].flags |= 4;
        self.refresh_city_totals(rules, index, false)
            .expect("validated surface city");
        Ok(())
    }
}

/// Native production classes: units, ordinary improvements, special projects.
pub fn category(item: i8) -> u8 {
    if item >= 0 {
        0
    } else if item > -36 {
        1
    } else {
        2
    }
}

/// Retained shields relative to the captured starting category and shield box.
/// `0x4c0ee6` captures that context; `0x4c0f37` computes the candidate value.
/// Callers must retain the authoritative context, not accept a client's budget.
pub fn retained_shields(
    initial_item: i8,
    new_item: i8,
    stored: i16,
    difficulty: u8,
    penalty: u8,
) -> i32 {
    let initial = i32::from(stored);
    if difficulty == 0 || category(initial_item) == category(new_item) {
        initial
    } else {
        // Native multiplication explicitly zero-extends the low shield word.
        initial - i32::from(stored as u16) * i32::from(penalty) / 100
    }
}

/// Original buy-price fragment. Capitalization (-39) has no purchase option.
/// Inputs are the native Rules bytes and signed stored shield word.
pub fn hurry_price(item: i8, cost: u8, rows: u8, stored: i16) -> Option<i32> {
    if item == -39 {
        return None;
    }
    let cost = if item >= 0 {
        i32::from(cost as i8)
    } else {
        i32::from(cost)
    };
    let remaining = (cost * i32::from(rows) - i32::from(stored)).clamp(0, 999);
    let price = if item >= 0 {
        remaining * 2 + remaining * remaining / 20
    } else if item <= -36 {
        remaining * 4
    } else {
        remaining * 2
    };
    Some(price * if stored == 0 { 2 } else { 1 })
}

/// Original single-improvement refund amount; eligibility is a separate check.
pub fn sale_value(cost: u8, rows: u8) -> i32 {
    i32::from(cost) * i32::from(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::GameRng;

    fn authored_city(cost: u8, rows: u8) -> (WorldState, Rules) {
        let mut text = format!("@COSMIC\n3\n2\n2\n10\n{rows}\n@UNITS\n");
        for i in 0..2 {
            text.push_str(&format!(
                "Unit{i},nil,0,1.,0,1a,1d,1h,1f,{cost},0,0,nil,000000000000000\n"
            ));
        }
        text.push_str("@IMPROVE\n");
        for i in 0..68 {
            text.push_str(&format!("Building{i},{cost},0,nil\n"));
        }
        let rules = tot_kit::parse_rules(&text);
        let mut world = crate::boot(&crate::tests_support::synthetic_save(), 1);
        world.human_mask = 2;
        world.wonders = [-1; 28];
        world.units.clear();
        world.cities[1].city_id = 1;
        world.cities[1].owner = 1;
        world.cities[1].map_id = 0;
        world.cities[1].size = 1;
        world.cities[1].flags = 0;
        world.cities[1].buildings = [0; 5];
        (world, rules)
    }
    #[test]
    fn original_purchase_sale_and_retention_vectors() {
        let mut count = 0;
        let mut sequence: Option<(i64, WorldState, Rules)> = None;
        for line in include_str!("../../../docs/evidence/city-production-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let mut fields = line.split_whitespace();
            let kind = fields.next().unwrap();
            let v: Vec<i64> = fields.map(|s| s.parse().unwrap()).collect();
            let got: i64 = match kind {
                "H" => hurry_price(v[0] as i8, v[1] as u8, v[2] as u8, v[3] as i16).unwrap_or(-1)
                    as i64,
                "P" => {
                    retained_shields(v[0] as i8, v[1] as i8, v[2] as i16, v[3] as u8, v[4] as u8)
                        as i64
                }
                "S" => sale_value(v[0] as u8, v[1] as u8) as i64,
                "B" => {
                    let (mut w, rules) = authored_city(v[1] as u8, v[2] as u8);
                    w.cities[1].building = v[0] as i8;
                    w.cities[1].shield_box = v[3] as u16;
                    w.treasury[1] = v[4] as i32;
                    let unchanged = (
                        w.turn,
                        w.rng.state(),
                        w.combat_rng.state(),
                        w.units.len(),
                        w.cities[1].building,
                    );
                    let quote = w.quote_hurry(&rules, 1, 1).unwrap();
                    assert_eq!(i64::from(quote.price), v[5]);
                    w.buy_production(&rules, 1, 1, quote).unwrap();
                    assert_eq!(i64::from(w.treasury[1]), v[6]);
                    assert_eq!(
                        unchanged,
                        (
                            w.turn,
                            w.rng.state(),
                            w.combat_rng.state(),
                            w.units.len(),
                            w.cities[1].building
                        )
                    );
                    assert_eq!(i64::from(w.cities[1].shield_box), v[7]);
                    let ctx = w.production_context.unwrap();
                    assert_eq!(i64::from(category(ctx.initial_item)), v[8]);
                    i64::from(ctx.initial_shields)
                }
                "C" => {
                    let (mut w, rules) = authored_city(v[1] as u8, v[2] as u8);
                    w.cities[1].buildings = [255; 5];
                    w.treasury[1] = v[3] as i32;
                    let quote = w.quote_sale(&rules, 1, 1, v[0] as u8).unwrap();
                    assert_eq!(i64::from(quote.income), v[4]);
                    w.sell_improvement(&rules, 1, 1, quote).unwrap();
                    assert_eq!(i64::from(w.treasury[1]), v[5]);
                    let mask = w.cities[1]
                        .buildings
                        .iter()
                        .enumerate()
                        .fold(0i64, |n, (i, b)| n | (i64::from(*b) << (8 * i)));
                    assert_eq!(mask, v[6]);
                    i64::from(w.cities[1].flags & 4)
                }
                "Q" => {
                    let (mut w, rules) = authored_city(v[1] as u8, v[2] as u8);
                    w.cities[1].building = v[0] as i8;
                    w.cities[1].shield_box = v[3] as u16;
                    w.cities[1].flags = v[4] as u32;
                    w.treasury[1] = v[5] as i32;
                    let before = w.checksum();
                    let accepted = match w.quote_hurry(&rules, 1, 1) {
                        Ok(q) => {
                            assert_eq!(i64::from(q.price), v[6]);
                            w.buy_production(&rules, 1, 1, q).is_ok()
                        }
                        Err(_) => false,
                    };
                    if !accepted {
                        assert_eq!(before, w.checksum());
                    }
                    i64::from(accepted)
                }
                "L" => {
                    let (mut w, rules) = authored_city(4, 10);
                    w.cities[1].buildings = [255; 5];
                    w.cities[1].flags = v[0] as u32;
                    let before = w.checksum();
                    let allowed = w.quote_sale(&rules, 1, 1, 4).is_ok();
                    assert_eq!(before, w.checksum());
                    i64::from(allowed)
                }
                "K" => {
                    if v[1] == 0 {
                        let (mut w, mut rules) = authored_city(4, 10);
                        let cosmic = rules.sections.get_mut("COSMIC").unwrap();
                        cosmic.rows.resize(19, vec!["0".into()]);
                        cosmic.rows[18] = vec![v[5].to_string()];
                        w.cities[1].building = v[2] as i8;
                        w.cities[1].shield_box = v[3] as u16;
                        w.difficulty = v[4] as u8;
                        sequence = Some((v[0], w, rules));
                    }
                    let (id, w, rules) = sequence.as_mut().unwrap();
                    assert_eq!(*id, v[0]);
                    let before = w.checksum();
                    let quote = w.quote_production_change(rules, 1, 1, v[6] as i8).unwrap();
                    assert_eq!(
                        before,
                        w.checksum(),
                        "quote must not capture or mutate budget"
                    );
                    assert!(w
                        .change_quoted_production(
                            rules,
                            1,
                            1,
                            ProductionChangeQuote {
                                initial_shields: quote.initial_shields + 1,
                                ..quote
                            }
                        )
                        .is_err());
                    assert_eq!(before, w.checksum());
                    w.change_quoted_production(rules, 1, 1, quote).unwrap();
                    assert_eq!(i64::from(w.cities[1].building), v[7]);
                    i64::from(w.cities[1].shield_box)
                }
                _ => panic!("unknown native vector {line}"),
            };
            assert_eq!(got, *v.last().unwrap(), "{line}");
            count += 1;
        }
        assert_eq!(count, 4091);
    }

    #[test]
    fn production_context_restores_budget_closes_and_rebases_after_purchase() {
        let (mut w, rules) = authored_city(4, 10);
        w.difficulty = 2;
        w.treasury[1] = 1000;
        w.cities[1].building = 0;
        w.cities[1].shield_box = 51;
        let initial = w.checksum();
        let quote = w.quote_production_change(&rules, 1, 1, -4).unwrap();
        assert_eq!(quote.retained, 26);
        assert_eq!(quote.target_cost, Some(40));
        assert_eq!(initial, w.checksum());
        w.change_quoted_production(&rules, 1, 1, quote).unwrap();
        let active = w.checksum();
        assert_ne!(active, initial);
        assert_eq!(
            w.quote_production_change(&rules, 1, 1, 0).unwrap().retained,
            51
        );
        let q = w.quote_production_change(&rules, 1, 1, -5).unwrap();
        assert_eq!(
            q.retained, 26,
            "cross-category changes reuse the initial budget"
        );
        w.change_quoted_production(&rules, 1, 1, q).unwrap();
        assert!(w.close_production_context(1, 1, 999).is_err());
        w.close_production_context(1, 1, 1).unwrap();
        assert_eq!(
            w.quote_production_change(&rules, 1, 1, 0).unwrap().retained,
            13,
            "reopening captures the current budget"
        );
        let buy = w.quote_hurry(&rules, 1, 1).unwrap();
        w.buy_production(&rules, 1, 1, buy).unwrap();
        assert_eq!(
            w.quote_production_change(&rules, 1, 1, 0).unwrap().retained,
            20,
            "buying rebases to the filled box"
        );
        let context = w.production_context.unwrap();
        let base = w.checksum();
        for field in 0..5 {
            let mut other = context;
            match field {
                0 => other.civ ^= 1,
                1 => other.city += 1,
                2 => other.city_id += 1,
                3 => other.initial_item += 1,
                _ => other.initial_shields += 1,
            };
            w.production_context = Some(other);
            assert_ne!(w.checksum(), base);
        }
        w.production_context = Some(context);
        let quote = w.quote_production_change(&rules, 1, 1, 0).unwrap();
        w.cities[1].city_id += 1;
        let before = w.checksum();
        assert!(w.change_quoted_production(&rules, 1, 1, quote).is_err());
        assert_eq!(before, w.checksum());
    }

    #[test]
    fn quotes_are_reads_and_rejections_are_atomic() {
        let (mut w, rules) = authored_city(4, 10);
        w.cities[1].building = 0;
        w.cities[1].shield_box = 1;
        w.treasury[1] = 0;
        let before = w.checksum();
        let poor = w.quote_hurry(&rules, 1, 1).unwrap();
        assert_eq!(before, w.checksum());
        assert_eq!(
            w.buy_production(&rules, 1, 1, poor),
            Err("not enough treasury")
        );
        assert_eq!(before, w.checksum());
        w.treasury[1] = 1000;
        let before = w.checksum();
        assert_eq!(
            w.buy_production(&rules, 1, 1, poor),
            Err("stale purchase quote")
        );
        assert!(w.quote_hurry(&rules, 2, 1).is_err());
        assert_eq!(before, w.checksum());
        w.cities[1].flags |= 1;
        assert!(w.quote_hurry(&rules, 1, 1).is_err());
        w.cities[1].building = -4;
        assert!(
            w.quote_hurry(&rules, 1, 1).is_ok(),
            "improvements can be bought during disorder"
        );
        w.cities[1].building = -39;
        assert!(w.quote_hurry(&rules, 1, 1).is_err());
        w.cities[1].buildings = [255; 5];
        assert!(w.quote_sale(&rules, 1, 1, 1).is_err());
        assert!(w.quote_sale(&rules, 1, 1, 40).is_err());
        let quote = w.quote_sale(&rules, 1, 1, 4).unwrap();
        let before = w.checksum();
        let preview = w.preview_sale(&rules, 1, 1, 4).unwrap();
        assert_eq!(before, w.checksum(), "preview only evaluates a clone");
        assert!(w
            .sell_improvement(
                &rules,
                1,
                1,
                SaleQuote {
                    income: quote.income + 1,
                    ..quote
                }
            )
            .is_err());
        assert_eq!(before, w.checksum());
        w.sell_improvement(&rules, 1, 1, quote).unwrap();
        assert_eq!(preview.after, w.sale_effects(&rules, 1, 1));
        let before = w.checksum();
        assert!(w.quote_sale(&rules, 1, 1, 5).is_err());
        assert!(w.sell_improvement(&rules, 1, 1, quote).is_err());
        assert_eq!(before, w.checksum());
    }
}
