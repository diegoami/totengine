//! Own-civilization report views (#201): City status (F1), Defense (F2) and
//! Trade (F5). Every row is derived only from the human's filtered
//! [`HumanView`] and the loaded rules, so a report cannot expose foreign hidden
//! data, and building one never touches game state, the journal or any RNG.
//!
//! Field sets follow the inspected original reports (docs/ui/discovery.md U05,
//! U23): city identity/outputs/item/progress; unit statistics with active and
//! in-production counts; per-city tax, science and improvement maintenance.
//! Layout, sorting and the F-keys are client presentation. Wonders, attitude,
//! foreign, ranking, score, demographics and spaceship reports are not part
//! of this slice.
use super::{Entity, HumanView};
use tot_kit::Rules;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFamily {
    CityStatus,
    Defense,
    Trade,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CityStatusRow {
    pub city: Entity,
    pub name: String,
    pub size: u16,
    pub food_surplus: i32,
    pub shields: i32,
    pub science: i32,
    pub taxes: i32,
    pub luxuries: i32,
    pub happy: i32,
    pub unhappy: i32,
    /// Unhappy citizens outnumber the happy ones.
    pub disorder: bool,
    /// Build item: unit id when non-negative, improvement `-id` otherwise.
    pub item: i8,
    pub stored: u16,
    pub required: Option<u16>,
    pub turns: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefenseRow {
    pub unit_type: u8,
    pub attack: u32,
    pub defense: u32,
    pub hits: u32,
    pub firepower: u32,
    /// The rules' movement value for the type, unmodified.
    pub moves: u32,
    pub active: u32,
    /// Own cities currently building this unit type.
    pub building: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeRow {
    pub city: Entity,
    pub name: String,
    /// Taxes the city credits per turn: none in disorder or Anarchy.
    pub taxes: i32,
    pub science: i32,
    pub luxuries: i32,
    pub upkeep: i32,
    pub improvements: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeTotals {
    pub taxes: i32,
    pub science: i32,
    pub luxuries: i32,
    pub upkeep: i32,
    pub treasury: i32,
}

impl TradeTotals {
    pub fn net(&self) -> i32 {
        self.taxes - self.upkeep
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Report {
    CityStatus(Vec<CityStatusRow>),
    Defense(Vec<DefenseRow>),
    Trade {
        rows: Vec<TradeRow>,
        totals: TradeTotals,
    },
}

fn improvement_count(buildings: &[u8; 5]) -> u32 {
    (1..36usize)
        .filter(|b| buildings[b / 8] & (1 << (b % 8)) != 0)
        .count() as u32
}

fn city_entity(view: &HumanView, slot: usize) -> Option<Entity> {
    view.cities.iter().copied().find(|e| e.slot == slot)
}

/// Rows in ascending city-slot order; clients sort for presentation.
pub fn city_status(view: &HumanView) -> Vec<CityStatusRow> {
    let o = &view.observation;
    let mut rows = Vec::new();
    for city in o.cities.iter().filter(|c| c.owner == o.civ) {
        let (Some(entity), Some(e)) = (
            city_entity(view, city.index),
            o.city_economy.iter().find(|e| e.index == city.index),
        ) else {
            continue;
        };
        let production = view.city_production.iter().find(|p| p.index == city.index);
        rows.push(CityStatusRow {
            city: entity,
            name: city.name.clone(),
            size: city.size,
            food_surplus: e.food_surplus,
            shields: e.net_shields,
            science: e.finance.science,
            taxes: e.finance.taxes,
            luxuries: e.finance.luxuries,
            happy: e.happy,
            unhappy: e.unhappy,
            disorder: e.unhappy > e.happy,
            item: city.building.unwrap_or(0),
            stored: city.shield_box.unwrap_or(0),
            required: production.and_then(|p| p.required),
            turns: production.and_then(|p| p.turns),
        });
    }
    rows.sort_by_key(|r| r.city.slot);
    rows
}

/// One row per own unit type that is active or in production, ascending type id.
pub fn defense(view: &HumanView, rules: &Rules) -> Vec<DefenseRow> {
    let o = &view.observation;
    let mut counts: std::collections::BTreeMap<u8, (u32, u32)> = Default::default();
    for u in o.units.iter().filter(|u| u.owner == o.civ) {
        counts.entry(u.type_id).or_default().0 += 1;
    }
    for c in o.cities.iter().filter(|c| c.owner == o.civ) {
        if let Some(item) = c.building.filter(|i| *i >= 0) {
            counts.entry(item as u8).or_default().1 += 1;
        }
    }
    counts
        .into_iter()
        .map(|(t, (active, building))| DefenseRow {
            unit_type: t,
            attack: rules.unit_attack(t as usize).unwrap_or(0),
            defense: rules.unit_defense(t as usize).unwrap_or(0),
            hits: rules.unit_hits(t as usize).unwrap_or(0),
            firepower: rules.unit_firepower(t as usize).unwrap_or(0),
            moves: rules.unit_move(t as usize).unwrap_or(0),
            active,
            building,
        })
        .collect()
}

pub fn trade(view: &HumanView) -> (Vec<TradeRow>, TradeTotals) {
    let o = &view.observation;
    let mut rows = Vec::new();
    let mut totals = TradeTotals {
        taxes: 0,
        science: 0,
        luxuries: 0,
        upkeep: 0,
        treasury: o.treasury,
    };
    for city in o.cities.iter().filter(|c| c.owner == o.civ) {
        let (Some(entity), Some(e)) = (
            city_entity(view, city.index),
            o.city_economy.iter().find(|e| e.index == city.index),
        ) else {
            continue;
        };
        // The treasury credits taxes only outside Anarchy and without disorder.
        let credited = if o.government != 0 && e.unhappy <= e.happy {
            e.finance.taxes
        } else {
            0
        };
        totals.taxes += credited;
        totals.science += e.finance.science;
        totals.luxuries += e.finance.luxuries;
        totals.upkeep += e.upkeep;
        rows.push(TradeRow {
            city: entity,
            name: city.name.clone(),
            taxes: credited,
            science: e.finance.science,
            luxuries: e.finance.luxuries,
            upkeep: e.upkeep,
            improvements: improvement_count(&e.buildings),
        });
    }
    rows.sort_by_key(|r| r.city.slot);
    (rows, totals)
}

pub fn build(family: ReportFamily, view: &HumanView, rules: &Rules) -> Report {
    match family {
        ReportFamily::CityStatus => Report::CityStatus(city_status(view)),
        ReportFamily::Defense => Report::Defense(defense(view, rules)),
        ReportFamily::Trade => {
            let (rows, totals) = trade(view);
            Report::Trade { rows, totals }
        }
    }
}
