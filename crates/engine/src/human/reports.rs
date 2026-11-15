//! Own-civilization report views (#201): City status (F1), Defense (F2) and
//! Trade (F5). Every row is derived only from the human's filtered
//! [`HumanView`] and the loaded rules, so a report cannot expose foreign hidden
//! data, and building one never touches game state, the journal or any RNG.
//!
//! Field sets follow the inspected original reports (docs/ui/discovery.md U05,
//! U23): city identity/outputs/item/progress; unit statistics with active and
//! in-production counts; per-city tax, science and improvement maintenance.
//! Layout, sorting and the F-keys are client presentation. Attitude (F4) lists
//! citizen moods per city and Wonders (F7) the wonders this civilization hosts.
//! Wonder expiry is not shown: it depends on every civilization's
//! knowledge, whose visibility to the player is unverified.
//!
//! Ranked cities (F8), Score (F9) and Demographics (F11) are the original's
//! *public* reports (U23/U24): like the power ranking, the original shows them
//! for every civilization, independent of the player's exploration. They are
//! therefore built from the world's public figures through
//! [`HumanSession::any_report`], never from private foreign state, and they are
//! as inert as the own-civ views. Foreign/intelligence (F3) needs the diplomacy
//! model and spaceship (F12) remains unscoped.
use super::{Entity, HumanSession, HumanView};
use tot_kit::Rules;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFamily {
    CityStatus,
    Defense,
    Trade,
    Attitude,
    Wonders,
    RankedCities,
    Score,
    Demographics,
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

/// One of the world's largest cities (F8). Public in the original (U23):
/// foreign entries appear regardless of this player's exploration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedCityRow {
    /// 1-based world rank by size.
    pub rank: u8,
    pub name: String,
    pub civ: u8,
    pub size: u16,
    /// Wonders this city hosts.
    pub wonders: u32,
}

/// One civilization's public power score (F9, U24). The score is the engine's
/// original-decoded power score; the components are the standard-mode terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreRow {
    pub civ: u8,
    pub alive: bool,
    pub score: i32,
    /// Standard-mode component: acquired advances (×3).
    pub advances: i32,
    /// Standard-mode component: population (×8).
    pub population: i32,
    /// Standard-mode component: treasury (÷32).
    pub treasury: i32,
}

/// One demographics category (F11, U23): your value, the best civilization's
/// value and your rank among the living civilizations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemographicsRow {
    pub category: &'static str,
    pub own: i64,
    pub best: i64,
    pub best_civ: u8,
    /// 1-based; civilizations with a strictly greater value rank ahead.
    pub rank: u8,
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
    RankedCities(Vec<RankedCityRow>),
    Score {
        rows: Vec<ScoreRow>,
        /// Scenario objectives mode (scenario research flag 128): scores count
        /// objectives/wonders instead of the standard components.
        scenario: bool,
    },
    Demographics(Vec<DemographicsRow>),
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
        // The public families route through `any_report`, which has the world.
        family
        @ (ReportFamily::RankedCities | ReportFamily::Score | ReportFamily::Demographics) => {
            unreachable!("public family {family:?} needs the world, not the filtered view")
        }
    }
}

/// The world's five largest surface cities, ties in ascending slot order.
fn ranked_cities(world: &crate::WorldState) -> Vec<RankedCityRow> {
    let mut all: Vec<_> = world
        .cities
        .iter()
        .enumerate()
        .filter(|(_, c)| c.city_id != 0 && c.map_id == 0)
        .collect();
    all.sort_by(|a, b| b.1.size.cmp(&a.1.size).then(a.0.cmp(&b.0)));
    all.truncate(5);
    all.into_iter()
        .enumerate()
        .map(|(i, (slot, c))| RankedCityRow {
            rank: i as u8 + 1,
            name: c.name.clone(),
            civ: (c.owner & 15) as u8,
            size: c.size,
            wonders: world.wonders.iter().filter(|&&w| w == slot as i16).count() as u32,
        })
        .collect()
}

/// Public power scores with the standard-mode components per living civ.
fn score(world: &crate::WorldState) -> (Vec<ScoreRow>, bool) {
    let scenario = world.research_settings.flags & 128 != 0;
    let scores = world.power_scores().unwrap_or([0; 8]);
    let rows = (1u8..8)
        .filter(|c| world.ai_state.alive_mask & (1 << c) != 0)
        .map(|c| ScoreRow {
            civ: c,
            alive: true,
            score: scores[usize::from(c)],
            advances: i32::from(world.research[usize::from(c)].acquired),
            population: i32::from(world.ai_state.population[usize::from(c)]),
            treasury: world.treasury[usize::from(c)],
        })
        .collect();
    (rows, scenario)
}

/// One row per category: this civilization's value, the best living civ's
/// value and this civilization's rank among the living.
fn demographics(world: &crate::WorldState, human: u8) -> Vec<DemographicsRow> {
    let civs: Vec<u8> = (1u8..8)
        .filter(|c| world.ai_state.alive_mask & (1 << c) != 0)
        .collect();
    let value = |category: &str, c: u8| -> i64 {
        match category {
            "Population" => i64::from(world.ai_state.population[usize::from(c)]),
            "Cities" => world
                .cities
                .iter()
                .filter(|c2| c2.city_id != 0 && c2.map_id == 0 && c2.owner & 15 == u16::from(c))
                .count() as i64,
            "Military" => world
                .units
                .iter()
                .filter(|u| u.id != 0 && u.owner == c)
                .count() as i64,
            "Advances" => i64::from(world.research[usize::from(c)].acquired),
            _ => i64::from(world.treasury[usize::from(c)]),
        }
    };
    ["Population", "Cities", "Military", "Advances", "Treasury"]
        .into_iter()
        .map(|category| {
            let own = value(category, human);
            let (best, best_civ) = civs
                .iter()
                .map(|&c| (value(category, c), c))
                .max_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)))
                .unwrap_or((0, human));
            let ahead = civs.iter().filter(|&&c| value(category, c) > own).count() as u8;
            DemographicsRow {
                category,
                own,
                best,
                best_civ,
                rank: ahead + 1,
            }
        })
        .collect()
}

impl HumanSession {
    /// Every report family. The own-civilization families (F1/F2/F4/F5/F7)
    /// come from the filtered view like [`HumanSession::report`]; the public
    /// families (F8/F9/F11) are built from the world's public figures, which
    /// the original reports for every civilization. All families are equally
    /// inert: no journal entry, no randomness, no revision bump.
    pub fn any_report(&self, family: ReportFamily) -> Report {
        match family {
            ReportFamily::RankedCities => Report::RankedCities(ranked_cities(&self.game.world)),
            ReportFamily::Score => {
                let (rows, scenario) = score(&self.game.world);
                Report::Score { rows, scenario }
            }
            ReportFamily::Demographics => {
                Report::Demographics(demographics(&self.game.world, self.human))
            }
            own => build(own, &self.view(), &self.game.rules),
        }
    }
}
