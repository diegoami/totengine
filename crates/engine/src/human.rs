//! Shared resumable human turn driver. No window or transport owns scheduling.
//!
//! Generated surface games only. Research choices are delivered after the
//! atomic round; later cities bank science while the completed target is empty.
//! Views and polling never execute orders or consume either game RNG.
use std::collections::BTreeMap;

use tot_kit::Rules;

use crate::ai::BuiltInAi;
use crate::newgame::NewGameConfig;
use crate::player::{CityName, Game, Observation, Order, OrderResult, ACTION_CAP};
use crate::production::Built;

mod reports;
pub use reports::*;

/// Slot and identity together prevent acting on a reused entity slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entity {
    pub slot: usize,
    pub id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Human,
    Opponents,
    Eliminated,
    TurnLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// `default` is the first candidate on the path to the research goal
    /// (`0x49c8d4`), when a goal is set; a goal never skips the prompt.
    Research {
        id: u64,
        available: Vec<usize>,
        default: Option<usize>,
    },
    Founding {
        id: u64,
        unit: Entity,
    },
    /// Required after Anarchy ends its wait: choose one of the eligible
    /// governments (`0x5255c4` mode 0). There is no cancel.
    Government {
        id: u64,
        eligible: Vec<u8>,
    },
    /// Required after a government is set (the native setter opens the rate
    /// dialog for a human civ). `opening` is the split the dialog opens with,
    /// `[tax, luxury, science]`; commit with [`Command::ReviewRates`].
    Rates {
        id: u64,
        view: crate::rates::RatesView,
        opening: [u8; 3],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Worker {
        city: Entity,
        tile: u8,
    },
    Specialist {
        city: Entity,
        slot: u8,
    },
    Move {
        unit: Entity,
        to: (u16, u16),
    },
    Occupy {
        unit: Entity,
        city: Entity,
    },
    Production {
        city: Entity,
        item: i8,
    },
    QuotedProduction {
        city: Entity,
        quote: crate::city_production::ProductionChangeQuote,
    },
    CloseProduction {
        city: Entity,
    },
    Buy {
        city: Entity,
        quote: crate::city_production::HurryQuote,
    },
    Sell {
        city: Entity,
        quote: crate::city_production::SaleQuote,
    },
    BeginFound {
        unit: Entity,
    },
    NameCity {
        decision: u64,
        name: CityName,
    },
    CancelFound {
        decision: u64,
    },
    Research {
        decision: u64,
        advance: usize,
    },
    /// Commit science and tax shares in tenths; luxury is the remainder.
    Rates {
        science: u8,
        tax: u8,
    },
    /// Set or clear the research goal (`0x4a0886`). A goal only steers the
    /// default of the research prompt; it never changes a running project.
    Goal {
        advance: Option<usize>,
    },
    /// Start a voluntary revolution; Anarchy begins at once.
    Revolution,
    /// Resolve [`Decision::Government`].
    Government {
        decision: u64,
        government: u8,
    },
    /// Resolve [`Decision::Rates`] with one validated split.
    ReviewRates {
        decision: u64,
        science: u8,
        tax: u8,
    },
    EndTurn,
    Skip {
        unit: Entity,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    pub civ: u8,
    pub revision: u64,
    pub id: u64,
    pub command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub revision: u64,
    pub phase: Phase,
    pub decision: Option<Decision>,
    pub result: Option<OrderResult>,
}

/// Only the human's own round results are retained. No spectator event feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    Grew(usize),
    Starved(usize),
    CityDestroyed(usize),
    UnitDisbanded(usize),
    Production(usize, Built),
    Discovery(usize),
    Revolution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanView {
    pub revision: u64,
    pub phase: Phase,
    pub observation: Observation,
    pub decision: Option<Decision>,
    pub notices: Vec<Notice>,
    pub units: Vec<Entity>,
    pub cities: Vec<Entity>,
    /// Identities only for cities present in the filtered observation.
    pub known_cities: Vec<Entity>,
    pub production_choices: Vec<(Entity, Vec<i8>)>,
    /// (visible unit slot, current HP, maximum HP), filtered before transport.
    pub unit_health: Vec<(usize, u32, u32)>,
    pub city_citizens: Vec<crate::citizens::CityCitizens>,
    pub city_info: Vec<crate::city_info::CityInfo>,
    pub city_production: Vec<crate::city_production::CityProductionView>,
    pub production_context_city: Option<Entity>,
    /// The human civilization's committed shares and their cap.
    pub rates: Option<crate::rates::RatesView>,
    /// The research goal (one global value in the original), cleared when known.
    pub goal: Option<usize>,
    /// Whether a voluntary revolution may be started now.
    pub revolution_available: bool,
    /// Advances that may be set as the goal (not known, enabled, defined).
    pub goal_choices: Vec<usize>,
    /// While the human civ is in Anarchy: the turn counter at which the
    /// government chooser will next open.
    pub anarchy_choice_turn: Option<i16>,
    /// While a revolution may be started: the turn counter at which the
    /// government chooser would open if it began now.
    pub revolution_choice_turn: Option<i16>,
}

/// Owns the game and opponent policies. Adapters cannot bypass recording or
/// supply their own Player for the human. Retry receipts live for this session;
/// changed payloads with a reused request ID are rejected, never reexecuted.
pub struct HumanSession {
    view_cache: std::cell::RefCell<Option<(u64, HumanView)>>,
    known_tiles: Vec<Option<crate::player::TileView>>,
    known_cities: BTreeMap<usize, (u32, crate::player::CityView)>,
    config: NewGameConfig,
    game: Game,
    human: u8,
    revision: u64,
    cursor: u8,
    actions: usize,
    last_ai: Option<OrderResult>,
    last_human: Option<OrderResult>,
    decision: Option<Decision>,
    goal: Option<usize>,
    next_decision: u64,
    receipts: BTreeMap<u64, (Request, Response)>,
    notices: Vec<Notice>,
    journal: Vec<Operation>,
}

#[derive(Debug, Clone, Copy)]
enum Operation {
    Input(Request),
    Advance(usize),
}

impl HumanSession {
    /// Next unused canonical input ID, including after checkpoint restoration.
    pub fn next_request_id(&self) -> Result<u64, &'static str> {
        self.receipts
            .keys()
            .next_back()
            .copied()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("human request IDs exhausted")
    }
    pub fn generated(config: NewGameConfig, rules: Rules, human: u8) -> Result<Self, String> {
        if !(1..=config.civilizations).contains(&human) {
            return Err("human civilization is outside the generated setup".into());
        }
        let mut game = Game::generated_human(config, rules, human)?;
        for civ in 1..=config.civilizations {
            if civ != human {
                game.players[civ as usize] = Some(Box::new(BuiltInAi::new(
                    &game.rules,
                    u64::from(config.gameplay_seed),
                    civ,
                )?));
            }
        }
        let mut session = Self {
            known_tiles: vec![None; game.world.map.terrain.len()],
            known_cities: BTreeMap::new(),
            view_cache: std::cell::RefCell::new(None),
            config,
            game,
            human,
            revision: 0,
            cursor: 1,
            actions: 0,
            last_ai: None,
            last_human: None,
            decision: None,
            goal: None,
            next_decision: 1,
            receipts: BTreeMap::new(),
            notices: Vec::new(),
            journal: Vec::new(),
        };
        session.remember();
        session.begin_human_turn();
        session.require_research();
        Ok(session)
    }

    pub fn phase(&self) -> Phase {
        if !self.game.world.living_civs().contains(&self.human) {
            Phase::Eliminated
        } else if self.game.world.turn >= 30_001 {
            Phase::TurnLimit
        } else if self.cursor == self.human {
            Phase::Human
        } else {
            Phase::Opponents
        }
    }

    pub fn view(&self) -> HumanView {
        if let Some((_, view)) = self
            .view_cache
            .borrow()
            .as_ref()
            .filter(|(revision, _)| *revision == self.revision)
        {
            return view.clone();
        }
        let view = self.build_view();
        *self.view_cache.borrow_mut() = Some((self.revision, view.clone()));
        view
    }

    pub fn preview_sale(
        &self,
        city: Entity,
        improvement: u8,
    ) -> Result<crate::city_production::SalePreview, &'static str> {
        self.city(city, true)?;
        self.game
            .world
            .preview_sale(&self.game.rules, self.human, city.slot, improvement)
    }

    pub fn preview_production(
        &self,
        city: Entity,
        item: i8,
    ) -> Result<crate::city_production::ProductionChangeQuote, &'static str> {
        self.city(city, true)?;
        self.game
            .world
            .quote_production_change(&self.game.rules, self.human, city.slot, item)
    }

    /// An own-civilization report built from this session's filtered view only.
    /// No journal entry, no randomness, no mutation.
    pub fn report(&self, family: ReportFamily) -> Report {
        reports::build(family, &self.view(), &self.game.rules)
    }

    /// Immutable per-turn consequences of a candidate split for the human civ.
    /// No journal entry, no randomness; the quote never commits anything.
    pub fn preview_rates(
        &self,
        science: u8,
        tax: u8,
    ) -> Result<crate::rates::RatesQuote, &'static str> {
        self.game
            .world
            .rates_quote(&self.game.rules, self.human, science, tax)
    }

    fn build_view(&self) -> HumanView {
        let mut observation =
            self.game
                .world
                .observe(&self.game.rules, self.human, self.last_human.clone());
        observation.seen = self.known_tiles.clone();
        observation.cities.retain(|c| c.owner == self.human);
        observation.cities.extend(
            self.known_cities
                .values()
                .filter(|(_, c)| c.owner != self.human)
                .map(|(_, c)| c.clone()),
        );
        observation.cities.sort_by_key(|c| c.index);
        HumanView {
            goal: self.goal,
            goal_choices: self.game.world.goal_choices(&self.game.rules, self.human),
            anarchy_choice_turn: self
                .game
                .world
                .next_government_choice_turn(&self.game.rules, self.human),
            revolution_choice_turn: self
                .game
                .world
                .revolution_choice_turn(&self.game.rules, self.human),
            rates: self.game.world.rates_view(self.human),
            revolution_available: self.phase() == Phase::Human
                && self.decision.is_none()
                && self.game.world.can_start_revolution(self.human),
            production_context_city: self
                .game
                .world
                .production_context_city(self.human)
                .map(|(slot, id)| Entity { slot, id }),
            city_production: observation
                .cities
                .iter()
                .filter(|c| c.owner == self.human)
                .filter_map(|c| {
                    self.game
                        .world
                        .city_production_view(&self.game.rules, self.human, c.index)
                })
                .collect(),
            city_info: observation
                .cities
                .iter()
                .filter(|c| c.owner == self.human)
                .filter_map(|c| {
                    self.game
                        .world
                        .city_info(&self.game.rules, self.human, c.index)
                })
                .collect(),
            city_citizens: observation
                .cities
                .iter()
                .filter(|c| c.owner == self.human)
                .filter_map(|c| {
                    self.game
                        .world
                        .city_citizens(&self.game.rules, self.human, c.index)
                })
                .collect(),
            unit_health: observation
                .units
                .iter()
                .map(|u| {
                    let max = self
                        .game
                        .rules
                        .unit_hits(u.type_id as usize)
                        .unwrap_or(1)
                        .saturating_mul(10);
                    (
                        u.index,
                        max.saturating_sub(u32::from(self.game.world.units[u.index].damage)),
                        max,
                    )
                })
                .collect(),
            revision: self.revision,
            phase: self.phase(),
            decision: self.decision.clone(),
            notices: self.notices.clone(),
            units: self
                .game
                .world
                .units
                .iter()
                .enumerate()
                .filter(|(_, u)| u.id != 0 && u.owner == self.human)
                .map(|(slot, u)| Entity { slot, id: u.id })
                .collect(),
            cities: self
                .game
                .world
                .cities
                .iter()
                .enumerate()
                .filter(|(_, c)| c.city_id != 0 && c.owner & 15 == u16::from(self.human))
                .map(|(slot, c)| Entity {
                    slot,
                    id: c.city_id,
                })
                .collect(),
            known_cities: observation
                .cities
                .iter()
                .map(|c| Entity {
                    slot: c.index,
                    id: if c.owner == self.human {
                        self.game.world.cities[c.index].city_id
                    } else {
                        self.known_cities[&c.index].0
                    },
                })
                .collect(),
            production_choices: observation
                .cities
                .iter()
                .filter(|c| c.owner == self.human)
                .map(|c| {
                    let items = (-67..=79)
                        .filter(|&item| {
                            self.game.world.can_set_production(
                                &self.game.rules,
                                self.human,
                                c.index,
                                item,
                            )
                        })
                        .collect();
                    (
                        Entity {
                            slot: c.index,
                            id: self.game.world.cities[c.index].city_id,
                        },
                        items,
                    )
                })
                .collect(),
            observation,
        }
    }

    fn remember(&mut self) {
        let w = &self.game.world;
        let bit = 1u8 << self.human;
        let mut visible = vec![false; w.map.terrain.len()];
        for i in w.visible_indices(self.human) {
            visible[i] = true;
        }
        let mut newly_known = vec![false; visible.len()];
        for i in 0..visible.len() {
            newly_known[i] = self.known_tiles[i].is_none() && w.map.visible[i] & bit != 0;
            if visible[i] || newly_known[i] {
                self.known_tiles[i] = Some(crate::player::TileView {
                    terrain: w.map.terrain[i],
                    river: w.map.river[i],
                    special: w.map.special[i],
                    road: w.map.road[i],
                    rail: w.map.rail[i],
                });
            }
        }
        self.known_cities.retain(|index, (id, old)| {
            let Some(i) = w.map.stored_index(old.x, old.y) else {
                return false;
            };
            !visible[i]
                || w.cities
                    .get(*index)
                    .is_some_and(|c| c.city_id == *id && c.x == old.x && c.y == old.y)
        });
        for (index, c) in w.cities.iter().enumerate() {
            let Some(i) = w.map.stored_index(c.x, c.y) else {
                continue;
            };
            let was_own = self
                .known_cities
                .get(&index)
                .is_some_and(|(id, old)| *id == c.city_id && old.owner == self.human);
            if c.city_id == 0 {
                if was_own {
                    self.known_cities.remove(&index);
                }
                continue;
            }
            if c.owner & 15 == u16::from(self.human)
                || visible[i]
                || newly_known[i]
                || was_own
                || (c.known_to & bit != 0
                    && self
                        .known_cities
                        .get(&index)
                        .is_none_or(|(id, _)| *id != c.city_id))
            {
                self.known_cities.insert(
                    index,
                    (
                        c.city_id,
                        crate::player::CityView {
                            index,
                            name: c.name.clone(),
                            owner: (c.owner & 15) as u8,
                            x: c.x,
                            y: c.y,
                            size: c.size,
                            building: None,
                            shield_box: None,
                            output: None,
                        },
                    ),
                );
            }
        }
    }

    fn knowledge_fingerprint(&self) -> u64 {
        // Versioned checkpoint consistency, not an authenticity signature.
        format!("{:?} {:?}", self.known_tiles, self.known_cities)
            .bytes()
            .fold(0xcbf2_9ce4_8422_2325, |h, b| {
                (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
            })
    }

    fn require_research(&mut self) {
        // 0x4a1581: the goal is cleared once that advance is known.
        if let Some(goal) = self.goal {
            if !self
                .game
                .world
                .goal_is_valid(&self.game.rules, self.human, goal)
            {
                self.goal = None;
            }
        }
        if self.phase() != Phase::Human || self.decision.is_some() {
            return;
        }
        let research = self
            .game
            .world
            .observe_research(&self.game.rules, self.human);
        let available = self
            .game
            .world
            .human_research_candidates(&self.game.rules, self.human);
        if (research.target.is_none() || research.free_advances > 0) && !available.is_empty() {
            let id = self.allocate_decision();
            let default = self.goal.and_then(|goal| {
                available
                    .iter()
                    .copied()
                    .find(|&a| crate::WorldState::on_goal_path(&self.game.rules, goal, a))
            });
            self.decision = Some(Decision::Research {
                id,
                available,
                default,
            });
        }
    }

    /// The native government setter opens the rate dialog for a human civ whose
    /// new government is not Anarchy; the engine requires that review.
    fn open_rate_review(&mut self) {
        let Some(view) = self.game.world.rates_view(self.human) else {
            return;
        };
        let opening = crate::rates::open_normalize(view.cap, view.tax, view.science);
        let id = self.allocate_decision();
        self.decision = Some(Decision::Rates { id, view, opening });
    }

    /// The human civ's per-civ turn step (`0x5292d6`, called once when its turn
    /// begins after the round's turn increment): Anarchy asks for a government
    /// at the first turn counter that is a multiple of four, or at once with the
    /// Statue of Liberty.
    fn begin_human_turn(&mut self) {
        if self.phase() != Phase::Human || self.decision.is_some() {
            return;
        }
        if self
            .game
            .world
            .resolve_anarchy(&self.game.rules, self.human)
        {
            let eligible = self
                .game
                .world
                .eligible_governments(&self.game.rules, self.human);
            let id = self.allocate_decision();
            self.decision = Some(Decision::Government { id, eligible });
        }
    }

    fn allocate_decision(&mut self) -> u64 {
        let id = self.next_decision;
        self.next_decision += 1;
        id
    }

    fn unit(&self, entity: Entity) -> Result<(), &'static str> {
        self.game
            .world
            .units
            .get(entity.slot)
            .filter(|u| u.id != 0 && u.id == entity.id && u.owner == self.human)
            .map(|_| ())
            .ok_or("stale or foreign unit")
    }

    fn city(&self, entity: Entity, own: bool) -> Result<(), &'static str> {
        self.game
            .world
            .cities
            .get(entity.slot)
            .filter(|c| {
                c.city_id != 0
                    && c.city_id == entity.id
                    && (!own || c.owner & 15 == u16::from(self.human))
            })
            .map(|_| ())
            .ok_or("stale or foreign city")
    }

    pub fn submit(&mut self, request: Request) -> Result<Response, &'static str> {
        if request.civ != self.human {
            return Err("wrong human owner");
        }
        if let Some((old, response)) = self.receipts.get(&request.id) {
            return if *old == request {
                Ok(response.clone())
            } else {
                Err("request ID already used")
            };
        }
        if request.revision != self.revision {
            return Err("stale session revision");
        }
        if self.phase() != Phase::Human {
            return Err("not awaiting human input");
        }
        if self.receipts.len() >= 100_000 {
            return Err("session request limit reached");
        }
        let mut new_decision = None;
        let mut new_rates_review = false;
        let mut new_goal = None;
        let order = match (&self.decision, request.command) {
            (_, Command::Goal { advance }) => {
                if let Some(a) = advance {
                    if !self
                        .game
                        .world
                        .goal_is_valid(&self.game.rules, self.human, a)
                    {
                        return Err("not a valid research goal");
                    }
                }
                new_goal = Some(advance);
                None
            }
            (_, Command::CloseProduction { city }) => {
                self.city(city, true)?;
                Some(Order::CloseProduction {
                    city: city.slot,
                    city_id: city.id,
                })
            }
            (
                Some(Decision::Research { id, available, .. }),
                Command::Research { decision, advance },
            ) if *id == decision && available.contains(&advance) => {
                Some(Order::SetResearch { advance })
            }
            (
                Some(Decision::Government { id, eligible }),
                Command::Government {
                    decision,
                    government,
                },
            ) if *id == decision && eligible.contains(&government) => {
                new_rates_review = true;
                Some(Order::SetGovernment { government })
            }
            (
                Some(Decision::Rates { id, .. }),
                Command::ReviewRates {
                    decision,
                    science,
                    tax,
                },
            ) if *id == decision => Some(Order::SetRates { science, tax }),
            (Some(Decision::Founding { id, unit }), Command::NameCity { decision, name })
                if *id == decision =>
            {
                self.unit(*unit)?;
                Some(Order::FoundNamedCity {
                    unit: unit.slot,
                    name,
                })
            }
            (Some(Decision::Founding { id, .. }), Command::CancelFound { decision })
                if *id == decision =>
            {
                None
            }
            (Some(_), _) => return Err("required decision does not match"),
            (None, Command::Move { unit, to }) => {
                self.unit(unit)?;
                Some(Order::Move {
                    unit: unit.slot,
                    to,
                })
            }
            (None, Command::Skip { unit }) => {
                self.unit(unit)?;
                Some(Order::SkipUnit { unit: unit.slot })
            }
            (None, Command::Occupy { unit, city }) => {
                self.unit(unit)?;
                // Unknown foreign entities cannot be probed through this API.
                if !self
                    .view()
                    .observation
                    .cities
                    .iter()
                    .any(|c| c.index == city.slot)
                {
                    return Err("city is not known");
                }
                self.city(city, false)?;
                Some(Order::OccupyCity {
                    unit: unit.slot,
                    city: city.slot,
                })
            }
            (None, Command::Worker { city, tile }) => {
                self.city(city, true)?;
                Some(Order::ToggleWorker {
                    city: city.slot,
                    tile,
                })
            }
            (None, Command::Specialist { city, slot }) => {
                self.city(city, true)?;
                Some(Order::CycleSpecialist {
                    city: city.slot,
                    slot,
                })
            }
            (None, Command::Production { city, item }) => {
                self.city(city, true)?;
                let quote = self.game.world.quote_production_change(
                    &self.game.rules,
                    self.human,
                    city.slot,
                    item,
                )?;
                Some(Order::QuotedProduction {
                    city: city.slot,
                    quote,
                })
            }
            (None, Command::QuotedProduction { city, quote }) => {
                self.city(city, true)?;
                Some(Order::QuotedProduction {
                    city: city.slot,
                    quote,
                })
            }
            (None, Command::Buy { city, quote }) => {
                self.city(city, true)?;
                Some(Order::BuyProduction {
                    city: city.slot,
                    quote,
                })
            }
            (None, Command::Sell { city, quote }) => {
                self.city(city, true)?;
                Some(Order::SellImprovement {
                    city: city.slot,
                    quote,
                })
            }
            (None, Command::BeginFound { unit }) => {
                self.unit(unit)?;
                if let Some(why) = self
                    .game
                    .world
                    .found_city_problem(&self.game.rules, unit.slot)
                {
                    return Err(why);
                }
                new_decision = Some(unit);
                None
            }
            (None, Command::Rates { science, tax }) => Some(Order::SetRates { science, tax }),
            (None, Command::Revolution) => Some(Order::Revolution),
            (None, Command::EndTurn) => Some(Order::EndTurn),
            (None, _) => return Err("no matching decision"),
        };
        let result = order
            .map(|o| self.game.submit_checked(self.human, o))
            .transpose()?;
        if !matches!(
            request.command,
            Command::CloseProduction { .. } | Command::Goal { .. }
        ) {
            self.decision = new_decision.map(|unit| {
                let id = self.allocate_decision();
                Decision::Founding { id, unit }
            });
            if new_rates_review {
                self.open_rate_review();
            }
        }
        if let Some(goal) = new_goal {
            self.goal = goal;
            // A pending prompt keeps its candidates; only its default follows the goal.
            if let Some(Decision::Research {
                available, default, ..
            }) = &mut self.decision
            {
                *default = goal.and_then(|g| {
                    available
                        .iter()
                        .copied()
                        .find(|&a| crate::WorldState::on_goal_path(&self.game.rules, g, a))
                });
            }
        }
        if let Some(result) = &result {
            self.last_human = Some(result.clone());
        }
        if order == Some(Order::EndTurn) {
            self.next_civ();
        }
        self.remember();
        self.revision += 1;
        self.require_research();
        let response = Response {
            revision: self.revision,
            phase: self.phase(),
            decision: self.decision.clone(),
            result,
        };
        self.journal.push(Operation::Input(request));
        self.receipts
            .insert(request.id, (request, response.clone()));
        Ok(response)
    }

    fn next_civ(&mut self) {
        self.cursor += 1;
        self.actions = 0;
        self.last_ai = None;
    }

    /// Each work item is one AI order, skipped civilization, or atomic round.
    /// Clamped to 256 items. Calling while the human is thinking is a no-op.
    /// The round itself is not yet an interruptible city-phase continuation.
    pub fn advance(&mut self, budget: usize) -> usize {
        let mut steps = 0;
        while steps < budget.min(256) && self.phase() == Phase::Opponents {
            if self.cursor == 8 {
                self.close_round();
                self.cursor = 1;
                self.last_human = None;
            } else if !self.game.world.living_civs().contains(&self.cursor) {
                self.next_civ();
            } else if let Some(mut player) = self.game.players[self.cursor as usize].take() {
                let obs =
                    self.game
                        .world
                        .observe(&self.game.rules, self.cursor, self.last_ai.take());
                let order = player.act(&obs);
                self.last_ai = Some(self.game.submit(self.cursor, order));
                self.game.players[self.cursor as usize] = Some(player);
                self.actions += 1;
                if order == Order::EndTurn || self.actions >= ACTION_CAP {
                    self.next_civ();
                }
            } else {
                self.next_civ();
            }
            self.remember();
            steps += 1;
            self.revision += 1;
            if self.cursor == self.human {
                self.begin_human_turn();
            }
            self.require_research();
        }
        if steps > 0 {
            self.journal.push(Operation::Advance(steps));
        }
        steps
    }

    fn close_round(&mut self) {
        let cities: Vec<bool> = self
            .game
            .world
            .cities
            .iter()
            .map(|c| c.city_id != 0 && c.owner & 15 == u16::from(self.human))
            .collect();
        let units: Vec<bool> = self
            .game
            .world
            .units
            .iter()
            .map(|u| u.id != 0 && u.owner == self.human)
            .collect();
        let summary = self.game.close_turn();
        let own = |k: &usize| cities.get(*k).copied().unwrap_or(false);
        self.notices.clear();
        self.notices
            .extend(summary.grew.into_iter().filter(own).map(Notice::Grew));
        self.notices
            .extend(summary.famined.into_iter().filter(own).map(Notice::Starved));
        self.notices.extend(
            summary
                .destroyed_cities
                .into_iter()
                .filter(own)
                .map(Notice::CityDestroyed),
        );
        self.notices.extend(
            summary
                .disbanded_units
                .into_iter()
                .filter(|k| units.get(*k).copied().unwrap_or(false))
                .map(Notice::UnitDisbanded),
        );
        self.notices.extend(
            summary
                .built
                .into_iter()
                .filter(|(k, _)| own(k))
                .map(|(k, b)| Notice::Production(k, b)),
        );
        self.notices.extend(
            summary
                .discoveries
                .into_iter()
                .filter(|(c, _)| *c == self.human)
                .map(|(_, a)| Notice::Discovery(a)),
        );
        if summary.revolutions.contains(&self.human) {
            self.notices.push(Notice::Revolution);
        }
    }
}

mod checkpoint;

#[cfg(test)]
mod tests;
