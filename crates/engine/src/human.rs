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
    /// A later client package must implement the required government choice.
    UnsupportedGovernment,
    TurnLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Research { id: u64, available: Vec<usize> },
    Founding { id: u64, unit: Entity },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Move { unit: Entity, to: (u16, u16) },
    Occupy { unit: Entity, city: Entity },
    Production { city: Entity, item: i8 },
    BeginFound { unit: Entity },
    NameCity { decision: u64, name: CityName },
    CancelFound { decision: u64 },
    Research { decision: u64, advance: usize },
    EndTurn,
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
}

/// Owns the game and opponent policies. Adapters cannot bypass recording or
/// supply their own Player for the human. Retry receipts live for this session;
/// changed payloads with a reused request ID are rejected, never reexecuted.
pub struct HumanSession {
    config: NewGameConfig,
    game: Game,
    human: u8,
    revision: u64,
    cursor: u8,
    actions: usize,
    last_ai: Option<OrderResult>,
    last_human: Option<OrderResult>,
    decision: Option<Decision>,
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
            config,
            game,
            human,
            revision: 0,
            cursor: 1,
            actions: 0,
            last_ai: None,
            last_human: None,
            decision: None,
            next_decision: 1,
            receipts: BTreeMap::new(),
            notices: Vec::new(),
            journal: Vec::new(),
        };
        session.require_research();
        Ok(session)
    }

    pub fn phase(&self) -> Phase {
        if !self.game.world.living_civs().contains(&self.human) {
            Phase::Eliminated
        } else if self.game.world.turn >= 30_001 {
            Phase::TurnLimit
        } else if self.cursor == self.human {
            if self.game.world.governments[self.human as usize] == 0 {
                Phase::UnsupportedGovernment
            } else {
                Phase::Human
            }
        } else {
            Phase::Opponents
        }
    }

    pub fn view(&self) -> HumanView {
        let observation =
            self.game
                .world
                .observe(&self.game.rules, self.human, self.last_human.clone());
        HumanView {
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
                    id: self.game.world.cities[c.index].city_id,
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

    fn require_research(&mut self) {
        if self.phase() != Phase::Human || self.decision.is_some() {
            return;
        }
        let research = self
            .game
            .world
            .observe_research(&self.game.rules, self.human);
        if (research.target.is_none() || research.free_advances > 0)
            && !research.available.is_empty()
        {
            let id = self.allocate_decision();
            self.decision = Some(Decision::Research {
                id,
                available: research.available,
            });
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
        let order = match (&self.decision, request.command) {
            (
                Some(Decision::Research { id, available }),
                Command::Research { decision, advance },
            ) if *id == decision && available.contains(&advance) => {
                Some(Order::SetResearch { advance })
            }
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
            (None, Command::Production { city, item }) => {
                self.city(city, true)?;
                Some(Order::SetProduction {
                    city: city.slot,
                    item,
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
            (None, Command::EndTurn) => Some(Order::EndTurn),
            (None, _) => return Err("no matching decision"),
        };
        let result = order
            .map(|o| self.game.submit_checked(self.human, o))
            .transpose()?;
        self.decision = new_decision.map(|unit| {
            let id = self.allocate_decision();
            Decision::Founding { id, unit }
        });
        if let Some(result) = &result {
            self.last_human = Some(result.clone());
        }
        if order == Some(Order::EndTurn) {
            self.next_civ();
        }
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
            steps += 1;
            self.revision += 1;
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
