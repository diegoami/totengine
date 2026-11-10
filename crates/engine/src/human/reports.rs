//! Own-civilization report views (#201): City status (F1), Defense (F2) and
//! Trade (F5). Every row is derived only from the human's filtered
//! [`HumanView`] and the loaded rules, so a report cannot expose foreign hidden
//! data, and building one never touches game state, the journal or any RNG.
//!
//! Field sets follow the inspected original reports (docs/ui/discovery.md U05,
//! U23): city identity/outputs/item/progress; unit statistics with active and
//! in-production counts; per-city tax, science and improvement maintenance.
//! Layout, sorting and the F-keys are client presentation. Attitude (F4) lists
//! citizen moods per city and Wonders (F7) the wonders this civilization hosts;
//! foreign, ranking, score, demographics and spaceship reports are not part of
//! this slice. Wonder expiry is not shown: it depends on every civilization's
//! knowledge, whose visibility to the player is unverified.
use super::{Entity, HumanView};
use tot_kit::Rules;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFamily {
    CityStatus,
    Defense,
    Trade,
    Attitude,
    Wonders,
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
pub struct AttitudeRow {
    pub city: Entity,
    pub name: String,
    pub size: u16,
    pub happy: i32,
    pub content: i32,
    pub unhappy: i32,
    pub entertainers: i32,
    pub tax_collectors: i32,
    pub scientists: i32,
    pub disorder: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WonderRow {
    /// Physical `@ENDWONDER` row id.
    pub wonder: u8,
    pub name: String,
    pub city: Entity,
    pub city_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Report {
    CityStatus(Vec<CityStatusRow>),
    Defense(Vec<DefenseRow>),
    Trade {
        rows: Vec<TradeRow>,
        totals: TradeTotals,
    },
    Attitude(Vec<AttitudeRow>),
    Wonders(Vec<WonderRow>),
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

/// Citizen moods per own city. Content citizens are the remainder after the
/// happy, unhappy and specialist citizens (never negative).
pub fn attitude(view: &HumanView) -> Vec<AttitudeRow> {
    let o = &view.observation;
    let mut rows = Vec::new();
    for city in o.cities.iter().filter(|c| c.owner == o.civ) {
        let (Some(entity), Some(e), Some(c)) = (
            city_entity(view, city.index),
            o.city_economy.iter().find(|e| e.index == city.index),
            view.city_citizens.iter().find(|c| c.index == city.index),
        ) else {
            continue;
        };
        let role = |r: u8| c.specialists.iter().filter(|&&s| s == r).count() as i32;
        let (entertainers, tax_collectors, scientists) = (role(1), role(2), role(3));
        let specialists = entertainers + tax_collectors + scientists;
        rows.push(AttitudeRow {
            city: entity,
            name: city.name.clone(),
            size: city.size,
            happy: e.happy,
            content: (i32::from(city.size) - specialists - e.happy - e.unhappy).max(0),
            unhappy: e.unhappy,
            entertainers,
            tax_collectors,
            scientists,
            disorder: e.unhappy > e.happy,
        });
    }
    rows.sort_by_key(|r| r.city.slot);
    rows
}

/// Own wonders in ascending wonder-row order, with their host city.
pub fn wonders(view: &HumanView, rules: &Rules) -> Vec<WonderRow> {
    let o = &view.observation;
    let names = rules.section("ENDWONDER").map(|s| &s.comments);
    let mut rows = Vec::new();
    for (id, host) in o.own_wonders.iter().enumerate() {
        let Some(slot) = host else { continue };
        let (Some(entity), Some(city)) = (
            city_entity(view, *slot),
            o.cities
                .iter()
                .find(|c| c.index == *slot && c.owner == o.civ),
        ) else {
            continue;
        };
        rows.push(WonderRow {
            wonder: id as u8,
            name: names
                .and_then(|n| n.get(id))
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| format!("Wonder {id}")),
            city: entity,
            city_name: city.name.clone(),
        });
    }
    rows
}

pub fn build(family: ReportFamily, view: &HumanView, rules: &Rules) -> Report {
    match family {
        ReportFamily::CityStatus => Report::CityStatus(city_status(view)),
        ReportFamily::Defense => Report::Defense(defense(view, rules)),
        ReportFamily::Trade => {
            let (rows, totals) = trade(view);
            Report::Trade { rows, totals }
        }
        ReportFamily::Attitude => Report::Attitude(attitude(view)),
        ReportFamily::Wonders => Report::Wonders(wonders(view, rules)),
    }
}
