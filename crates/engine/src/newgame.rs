//! Fresh generated surface games (#91). No reference save is used.
//!
//! Initialization policy v1 is deliberately explicit: all players AI, one
//! unhomed land settler each, Despotism, zero treasury/advances, 60/40 science/
//! tax, no improvements/cities. Names are engine labels. Land/sea components
//! are engine-defined regions, not a port of original landmass assignment.
//! Start placement uses the decoded original sampler after deriving its
//! inputs; fertility is an engine estimate from Rules food, not native parity.
use tot_kit::{CalendarSettings, ResearchSettings, ResearchState, Rules, SavedAiState};

use crate::combat::CrtRand;
use crate::placement::{place_starts, StartMap};
use crate::worldgen::{self, WorldConfig};
use crate::{Civ, MapState, Unit, WorldState};

pub const INITIALIZATION_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewGameConfig {
    pub world: WorldConfig,
    pub gameplay_seed: u32,
    pub civilizations: u8,
    pub difficulty: u8,
}

impl Default for NewGameConfig {
    fn default() -> Self {
        Self {
            world: WorldConfig::default(),
            gameplay_seed: 1,
            civilizations: 5,
            difficulty: 2,
        }
    }
}

impl NewGameConfig {
    pub fn descriptor(self) -> String {
        format!(
            "{} {} {} {} {} {} {} {} {}",
            INITIALIZATION_VERSION,
            worldgen::ALGORITHM_VERSION,
            self.world.width,
            self.world.height,
            self.world.land_percent,
            self.world.seed,
            self.gameplay_seed,
            self.civilizations,
            self.difficulty
        )
    }

    pub fn from_descriptor(text: &str) -> Result<Self, String> {
        let fields: Vec<_> = text.split_whitespace().collect();
        if fields.len() != 9
            || fields[0] != INITIALIZATION_VERSION.to_string()
            || fields[1] != worldgen::ALGORITHM_VERSION.to_string()
        {
            return Err("unsupported generated-game setup version".into());
        }
        let bad = || "invalid generated-game setup".to_string();
        Ok(Self {
            world: WorldConfig {
                width: fields[2].parse().map_err(|_| bad())?,
                height: fields[3].parse().map_err(|_| bad())?,
                land_percent: fields[4].parse().map_err(|_| bad())?,
                seed: fields[5].parse().map_err(|_| bad())?,
            },
            gameplay_seed: fields[6].parse().map_err(|_| bad())?,
            civilizations: fields[7].parse().map_err(|_| bad())?,
            difficulty: fields[8].parse().map_err(|_| bad())?,
        })
    }
}

pub fn generate(config: NewGameConfig, rules: &Rules) -> Result<WorldState, String> {
    if !(2..=7).contains(&config.civilizations) || config.difficulty > 5 {
        return Err("new game requires 2..7 civilizations and difficulty 0..5".into());
    }
    if !rules.terrain_semantic_ok() {
        return Err("generated surface games require the standard 11 base terrain IDs".into());
    }
    // The permissive Rules reader retains trailing annotation rows (including
    // one after Original's 80 unit definitions). Only extra actual unit rows
    // exceed our classic capacity; a text-only footer is not a unit type.
    if (80..rules.unit_names().len())
        .any(|t| rules.unit_domain(t).is_some() && rules.unit_role(t).is_some())
        || rules
            .section("CIVILIZE")
            .is_some_and(|s| s.rows.len() > 100)
    {
        return Err("generated games support at most 80 unit types and 100 advances".into());
    }
    let mask = ((1u16 << (config.civilizations + 1)) - 2) as u8;
    let settler = (0..rules.unit_names().len().min(80))
        .find(|&t| {
            rules.unit_role(t) == Some(crate::founding::ROLE_SETTLE)
                && rules.unit_domain(t) == Some(0)
                && matches!(rules.unit_prereq_raw(t), None | Some("nil"))
                && rules.unit_civ_mask(t) & mask == mask
                && rules.unit_move(t).is_some_and(|m| m > 0)
        })
        .ok_or("Rules need an initially available land settler for all active civs")?;
    let generated = worldgen::generate(config.world)?;
    let n = generated.terrain.len();
    let mut landmass = vec![0u8; n];
    let mut values = vec![0i16; 256];
    let mut fertile_land = vec![0i16; 256];
    let mut next_region = 1usize;
    for i in 0..n {
        if landmass[i] != 0 {
            continue;
        }
        if next_region >= 255 {
            return Err("generated world exceeds 254 engine land/sea regions".into());
        }
        let sea = generated.terrain[i] == 10;
        let mut stack = vec![i];
        landmass[i] = next_region as u8;
        while let Some(at) = stack.pop() {
            if sea {
                values[next_region] += 1;
            } else if matches!(generated.terrain[at], 1 | 2) {
                fertile_land[next_region] += 1;
            }
            for j in worldgen::neighbors(config.world, at) {
                if landmass[j] == 0 && (generated.terrain[j] == 10) == sea {
                    landmass[j] = next_region as u8;
                    stack.push(j);
                }
            }
        }
        next_region += 1;
    }
    let fertility: Vec<u8> = (0..n)
        .map(|i| {
            if generated.terrain[i] == 10 {
                return 0;
            }
            let food = |j: usize| {
                i64::from(
                    rules
                        .terrain_yield(generated.terrain[j] as usize, 0)
                        .unwrap_or(0)
                        .max(0),
                )
            };
            let adjacent: i64 = worldgen::neighbors(config.world, i).map(food).sum();
            (food(i) * 4 + adjacent / 4).clamp(1, 15) as u8
        })
        .collect();
    let mut start_map = StartMap {
        w2: config.world.width as i32,
        h: config.world.height as i32,
        seed: config.world.seed as u16,
        flat: false,
        huts: true,
        tiles: (0..n)
            .map(|i| {
                [
                    generated.terrain[i] | (u8::from(generated.river[i]) << 7),
                    0,
                    0,
                    landmass[i],
                    0,
                    0xf0 | fertility[i],
                ]
            })
            .collect(),
        landmass_value: fertile_land,
    };
    // A separate stream: generator draws cannot change placement or gameplay.
    let mut placement_rng = CrtRand::new(config.gameplay_seed);
    let starts = place_starts(&mut start_map, 0, mask, &mut placement_rng);
    let mut units = Vec::new();
    for start in starts {
        let i = start.y as usize * (config.world.width as usize / 2) + start.x as usize / 2;
        // The original's late placement relaxations can accept ocean. An
        // unsupported setup must fail explicitly rather than boot invalid units.
        if generated.terrain[i] == 10
            || units
                .iter()
                .any(|u: &Unit| u.x == start.x as u16 && u.y == start.y as u16)
        {
            return Err("placement could not find distinct land starts; change setup/seed".into());
        }
        units.push(Unit {
            type_id: settler as u8,
            owner: start.civ,
            x: start.x as u16,
            y: start.y as u16,
            is_leader: false,
            moves_left: 0,
            veteran: false,
            damage: 0,
            orders: 0xff,
            stack_pos: 0,
            home: 0xff,
            id: units.len() as u32 + 1,
        });
    }
    let map = MapState {
        stored_w: config.world.width as usize / 2,
        stored_h: config.world.height as usize,
        terrain: generated.terrain,
        landmass,
        landmass_values: values,
        site_bits: vec![0; n],
        fertility,
        special: vec![false; n],
        river: generated.river,
        road: vec![false; n],
        rail: vec![false; n],
        improvements: start_map.tiles.iter().map(|t| t[1]).collect(),
        visible: vec![0; n],
        known_improvements: std::array::from_fn(|_| vec![0; n]),
        owner: start_map.tiles.iter().map(|t| t[5] >> 4).collect(),
        resource_seed: config.world.seed as u16,
        huts_enabled: true,
    };
    let mut world = WorldState {
        production_context: None,
        turn: 1,
        year: -4000,
        difficulty: config.difficulty,
        rivals: config.civilizations - 1,
        seed_word: 0,
        game_flags: 0x10,
        human_mask: 0,
        power_order: std::array::from_fn(|i| i as u8),
        game_mode: 0,
        ai_state: SavedAiState {
            alive_mask: mask,
            ..SavedAiState::default()
        },
        calendar: Some(CalendarSettings::default()),
        city_counts: [0; 8],
        wonders: [-1; 28],
        treasury: [0; 8],
        governments: [1; 8],
        science_rates: [6; 8],
        tax_rates: [4; 8],
        diplomacy: [[0; 8]; 8],
        research: [ResearchState::default(); 8],
        free_research: [0; 8],
        research_settings: ResearchSettings {
            advance_count: rules
                .section("CIVILIZE")
                .map(|s| {
                    s.rows
                        .iter()
                        .take(100)
                        .filter(|r| {
                            r.get(3).is_some_and(|p| p != "no")
                                && r.get(4).is_some_and(|p| p != "no")
                        })
                        .count() as i16
                })
                .unwrap_or(0),
            ..ResearchSettings::default()
        },
        techs: [[0; 13]; 8],
        civs: (1..=config.civilizations)
            .map(|id| Civ {
                id,
                names: vec![format!("Civilization {id}")],
                plural: format!("Civilization {id}"),
                unit_count: 1,
            })
            .collect(),
        next_unit_id: units.len() as u32 + 1,
        units,
        cities: Vec::new(),
        next_city_id: 1,
        map,
        rng: crate::rng::Lcg64::new(config.gameplay_seed),
        combat_rng: CrtRand::new(config.gameplay_seed),
        rng_seed: config.gameplay_seed as u64,
    };
    world.refresh_city_statistics();
    world.refresh_power();
    world.refresh_landmass_flags(rules);
    Ok(world)
}
