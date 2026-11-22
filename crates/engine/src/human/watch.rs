//! Drives a watched seat through the same validated commands as a person.
use super::{Command, Decision, HumanSession, HumanView, Origin, TalksReply};
use crate::ai::{AiV1, BuiltInAi};
use crate::campaign::AiPolicy;
use crate::player::{CityName, Order, OrderResult, Player, ACTION_CAP};

pub struct SeatWatcher {
    player: Box<dyn Player>,
    turn: Option<i16>,
    attempts: usize,
    rejected: bool,
}

impl HumanSession {
    pub fn watch_seat(&self) -> Result<SeatWatcher, String> {
        let seed = match &self.origin {
            Origin::Generated(config) => u64::from(config.gameplay_seed),
            Origin::Scenario(origin) => origin.seed,
        };
        let player: Box<dyn Player> = match self.policy {
            AiPolicy::BuiltIn1 => Box::new(BuiltInAi::new(&self.game.rules, seed, self.human)?),
            AiPolicy::AiV1 => Box::new(AiV1::new(&self.game.rules, seed, self.human)),
        };
        Ok(SeatWatcher {
            player,
            turn: None,
            attempts: 0,
            rejected: false,
        })
    }
}

impl SeatWatcher {
    pub fn record_rejection(&mut self) {
        self.rejected = true;
    }

    /// Derive one action entirely from what the watched player is allowed to see.
    pub fn command(&mut self, view: &HumanView) -> Result<Command, &'static str> {
        let mut observation = view.observation.clone();
        if self.turn != Some(observation.turn) {
            self.turn = Some(observation.turn);
            self.attempts = 0;
            self.rejected = false;
        }
        if std::mem::take(&mut self.rejected) {
            observation.last_result = Some(OrderResult::Rejected("Watched order refused"));
        }
        if let Some(decision) = &view.decision {
            return match decision {
                Decision::Research { id, available, .. } => {
                    observation.research.available.clone_from(available);
                    observation.research.target = None;
                    let advance = match self.player.act(&observation) {
                        Order::SetResearch { advance } if available.contains(&advance) => advance,
                        _ => *available.first().ok_or("No research action is available")?,
                    };
                    Ok(Command::Research {
                        decision: *id,
                        advance,
                    })
                }
                Decision::Founding { id, .. } => {
                    let name = (1..=observation.cities.len() + 1)
                        .map(|index| format!("City {index}"))
                        .find(|name| !observation.cities.iter().any(|city| city.name == *name))
                        .ok_or("No city name is available")?;
                    Ok(Command::NameCity {
                        decision: *id,
                        name: CityName::new(&name)?,
                    })
                }
                Decision::Government { id, eligible } => Ok(Command::Government {
                    decision: *id,
                    government: *eligible
                        .first()
                        .ok_or("No government choice is available")?,
                }),
                Decision::Rates { id, opening, .. } => Ok(Command::ReviewRates {
                    decision: *id,
                    tax: opening[0],
                    science: opening[2],
                }),
                Decision::Talks { id, .. } => Ok(Command::ReplyToTalks {
                    decision: *id,
                    reply: TalksReply::Leave,
                }),
            };
        }
        if self.attempts >= ACTION_CAP {
            return Ok(Command::EndTurn);
        }
        self.attempts += 1;
        // A running human research project is not a replaceable AI order.
        observation.research.available = observation.research.target.into_iter().collect();
        let unit = |slot| {
            view.units
                .iter()
                .find(|e| e.slot == slot)
                .copied()
                .ok_or("Watched unit is unavailable")
        };
        Ok(match self.player.act(&observation) {
            Order::Move { unit: slot, to } => Command::Move {
                unit: unit(slot)?,
                to,
            },
            Order::OccupyCity { unit: slot, city } => Command::Occupy {
                unit: unit(slot)?,
                city: view
                    .known_cities
                    .iter()
                    .find(|e| e.slot == city)
                    .copied()
                    .ok_or("Watched city is unknown")?,
            },
            Order::FoundCity { unit: slot } => Command::BeginFound { unit: unit(slot)? },
            Order::SetProduction { city, item } => Command::Production {
                city: view
                    .cities
                    .iter()
                    .find(|e| e.slot == city)
                    .copied()
                    .ok_or("Watched city is unavailable")?,
                item,
            },
            Order::EndTurn => Command::EndTurn,
            _ => return Err("The watched policy requested an unsupported action"),
        })
    }
}
