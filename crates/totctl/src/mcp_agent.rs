//! Experimental MCP agent play (#256): an MCP client plays one civilization
//! of a headless generated campaign against built-in (`builtin-1`) opponents.
//!
//! Owner decision 2026-11-12 (decision 0008, "MCP follow-up decisions"):
//! allowed now for **headless, generated-campaign, experimental** play only.
//! No client games, scenarios or delegated actions.
//!
//! The agent drives an [`engine::human::HumanSession`], the same resumable,
//! validated driver the playable clients use. It therefore sees only that
//! civilization's filtered view, and every command passes the engine's normal
//! ownership, revision, request-ID and legality checks. A repeated request ID
//! returns the recorded response instead of executing twice. The session's
//! checkpoint and `.totrec` recording replay the game without the agent.

use engine::human::{Command, Decision, Entity, HumanSession, Phase, Request};
use serde::Deserialize;
use serde_json::{json, Value};

/// Resource identity bound by headless agent checkpoints: no artwork or
/// installation is involved, only the Rules (bound separately).
pub(crate) const AGENT_RESOURCES: &str = "totengine-headless-agent 1\n";
/// Commands an agent may submit per turn before only `end_turn` is accepted,
/// the same per-turn bound the built-in players get (`ACTION_CAP`).
pub(crate) const ACTION_BUDGET: usize = engine::player::ACTION_CAP;
/// Opponent steps run after `end_turn` before giving up (a safety bound).
const SETTLE_STEPS: usize = 2_000_000;

pub(crate) struct AgentGame {
    session: HumanSession,
    rules: tot_kit::Rules,
    /// Free-text identity the agent declared, recorded in exports.
    agent: String,
    civ: u8,
    turn: i16,
    actions_this_turn: usize,
}

impl AgentGame {
    pub(crate) fn start(
        config: engine::newgame::NewGameConfig,
        rules: tot_kit::Rules,
        civ: u8,
        agent: String,
        resume_checkpoint: Option<&str>,
    ) -> Result<Self, String> {
        if agent.is_empty() || agent.len() > 80 || agent.chars().any(char::is_control) {
            return Err("agent must be a non-empty single-line name of at most 80 bytes".into());
        }
        let session = match resume_checkpoint {
            Some(text) => HumanSession::restore(rules.clone(), AGENT_RESOURCES, text)?,
            None => HumanSession::generated(config, rules.clone(), civ)?,
        };
        let civ = session.view().observation.civ;
        let mut game = AgentGame {
            session,
            rules,
            agent,
            civ,
            turn: 0,
            actions_this_turn: 0,
        };
        game.settle()?;
        Ok(game)
    }

    /// Run opponents until the agent must act or the game ends.
    fn settle(&mut self) -> Result<(), String> {
        let mut steps = 0;
        while self.session.phase() == Phase::Opponents {
            let done = self.session.advance(256);
            steps += done;
            if done == 0 || steps > SETTLE_STEPS {
                return Err("opponents did not return control".into());
            }
        }
        let turn = self.session.view().observation.turn;
        if turn != self.turn {
            self.turn = turn;
            self.actions_this_turn = 0;
        }
        Ok(())
    }

    pub(crate) fn submit(
        &mut self,
        revision: u64,
        request_id: u64,
        command: &Value,
    ) -> Result<Value, String> {
        let command = parse_command(command)?;
        if command != Command::EndTurn && self.actions_this_turn >= ACTION_BUDGET {
            return Err(format!(
                "action budget of {ACTION_BUDGET} commands this turn is spent; submit end_turn"
            ));
        }
        let response = self
            .session
            .submit(Request {
                civ: self.civ,
                revision,
                id: request_id,
                command,
            })
            .map_err(|e| format!("rejected: {e}"))?;
        self.actions_this_turn += 1;
        let accepted = !matches!(
            response.result,
            Some(engine::player::OrderResult::Rejected(_))
        );
        let result = response.result.map(|r| r.to_string());
        let ended = command == Command::EndTurn;
        if ended {
            self.settle()?;
        }
        let view = self.session.view();
        Ok(json!({
            "accepted": accepted,
            "result": result,
            "revision": view.revision,
            "phase": phase_name(view.phase),
            "turn": view.observation.turn,
            "decision": view.decision.as_ref().map(|d| decision_json(d, &self.rules)),
            "actions_left": ACTION_BUDGET.saturating_sub(self.actions_this_turn),
        }))
    }

    /// The agent's filtered view: own state, legal choices and what it sees.
    pub(crate) fn view_json(&self) -> Value {
        let view = self.session.view();
        let o = &view.observation;
        let unit_names = self.rules.unit_names();
        let own_unit = |slot: usize| view.units.iter().find(|e| e.slot == slot);
        let own_city = |slot: usize| view.cities.iter().find(|e| e.slot == slot);
        let units: Vec<Value> = o
            .units
            .iter()
            .filter_map(|u| {
                own_unit(u.index).map(|e| {
                    json!({
                        "unit": entity(e),
                        "type": u.type_id,
                        "name": unit_names.get(usize::from(u.type_id)).copied().unwrap_or("?"),
                        "x": u.x, "y": u.y,
                        "moves_left": u.moves_left,
                        "veteran": u.veteran,
                    })
                })
            })
            .collect();
        let cities: Vec<Value> = o
            .cities
            .iter()
            .filter_map(|c| {
                own_city(c.index).map(|e| {
                    let choices = view
                        .production_choices
                        .iter()
                        .find(|(city, _)| city == e)
                        .map(|(_, items)| {
                            items
                                .iter()
                                .map(|&i| json!({"item": i, "name": self.item_name(i)}))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    json!({
                        "city": entity(e),
                        "name": c.name,
                        "x": c.x, "y": c.y,
                        "size": c.size,
                        "building": c.building.map(|i| json!({"item": i, "name": self.item_name(i)})),
                        "shield_box": c.shield_box,
                        "production_choices": choices,
                    })
                })
            })
            .collect();
        let foreign_units: Vec<Value> = o
            .units
            .iter()
            .filter(|u| u.owner != self.civ)
            .map(|u| {
                json!({
                    "owner": u.owner, "type": u.type_id,
                    "name": unit_names.get(usize::from(u.type_id)).copied().unwrap_or("?"),
                    "x": u.x, "y": u.y,
                })
            })
            .collect();
        let foreign_cities: Vec<Value> = o
            .cities
            .iter()
            .filter(|c| c.owner != self.civ)
            .map(|c| json!({"owner": c.owner, "name": c.name, "x": c.x, "y": c.y, "size": c.size}))
            .collect();
        json!({
            "experimental": "headless MCP agent play (#256); filtered view of this civilization only",
            "civ": self.civ,
            "revision": view.revision,
            "phase": phase_name(view.phase),
            "turn": o.turn,
            "treasury": o.treasury,
            "government": o.government,
            "research": {
                "target": o.research.target.map(|a| json!({"advance": a, "name": self.advance_name(a)})),
                "progress": o.research.progress,
                "cost": o.research.cost,
            },
            "known_advances": known_advances(&o.techs),
            "rates": view.rates.map(|r| json!({"tax": r.tax, "luxury": r.luxury, "science": r.science, "cap": r.cap})),
            "goal": view.goal,
            "revolution_available": view.revolution_available,
            "decision": view.decision.as_ref().map(|d| decision_json(d, &self.rules)),
            "notices": view.notices.iter().map(|n| format!("{n:?}")).collect::<Vec<_>>(),
            "units": units,
            "cities": cities,
            "visible_foreign_units": foreign_units,
            "known_foreign_cities": foreign_cities,
            "map": {"width": o.w2, "height": o.h},
            "actions_left": ACTION_BUDGET.saturating_sub(self.actions_this_turn),
        })
    }

    /// Seen tiles around a point: `[x, y, terrain, river, road, rail]`.
    pub(crate) fn map_json(&self, x: u16, y: u16, radius: u16) -> Result<Value, String> {
        if radius > 10 {
            return Err("radius must be at most 10".into());
        }
        let view = self.session.view();
        let o = &view.observation;
        let terrain = self
            .rules
            .section("TERRAIN")
            .map(|s| {
                s.rows
                    .iter()
                    .take(11)
                    .filter_map(|r| r.first().cloned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut tiles = Vec::new();
        for ty in y.saturating_sub(radius)..=y.saturating_add(radius).min(o.h.saturating_sub(1)) {
            for tx in x.saturating_sub(radius * 2)..=x.saturating_add(radius * 2) {
                // Logical x and y share parity; `w2` is the logical width.
                if (tx + ty) % 2 != 0 || tx >= o.w2 {
                    continue;
                }
                let index = usize::from(ty) * usize::from(o.w2 / 2) + usize::from(tx / 2);
                if let Some(Some(t)) = o.seen.get(index) {
                    tiles.push(json!([tx, ty, t.terrain, t.river, t.road, t.rail]));
                }
            }
        }
        Ok(json!({
            "terrain_names": terrain,
            "tiles": tiles,
            "note": "only tiles this civilization has seen; [x, y, terrain, river, road, rail]",
        }))
    }

    pub(crate) fn checkpoint(&self) -> String {
        self.session.checkpoint(AGENT_RESOURCES)
    }

    pub(crate) fn recording(&self) -> Result<String, String> {
        tot_record::Bundle::human(
            &self.checkpoint(),
            &self.rules,
            AGENT_RESOURCES,
            tot_record::StopReason::Stopped,
            &format!("totctl mcp agent (experimental, #256): {}", self.agent),
        )?
        .encode()
    }

    fn item_name(&self, item: i8) -> String {
        if item >= 0 {
            self.rules
                .unit_names()
                .get(item as usize)
                .map_or_else(|| format!("Unit {item}"), |s| s.to_string())
        } else {
            self.rules
                .section("IMPROVE")
                .and_then(|s| s.rows.get((-i32::from(item)) as usize))
                .and_then(|r| r.first())
                .cloned()
                .unwrap_or_else(|| format!("Improvement {}", -i32::from(item)))
        }
    }

    fn advance_name(&self, id: usize) -> String {
        self.rules
            .tech_defs()
            .get(id)
            .map_or_else(|| format!("Advance {id}"), |t| t.name.clone())
    }
}

fn entity(e: &Entity) -> Value {
    json!({"slot": e.slot, "id": e.id})
}

fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Human => "your-turn",
        Phase::Opponents => "opponents",
        Phase::Eliminated => "eliminated",
        Phase::TurnLimit => "turn-limit",
    }
}

fn known_advances(techs: &[u8; 13]) -> Vec<usize> {
    (0..techs.len() * 8)
        .filter(|&a| techs[a / 8] & (1 << (a % 8)) != 0)
        .collect()
}

fn decision_json(decision: &Decision, rules: &tot_kit::Rules) -> Value {
    let name = |a: usize| {
        rules
            .tech_defs()
            .get(a)
            .map_or_else(|| format!("Advance {a}"), |t| t.name.clone())
    };
    match decision {
        Decision::Research {
            id,
            available,
            default,
        } => json!({
            "type": "research", "decision": id,
            "available": available.iter().map(|&a| json!({"advance": a, "name": name(a)})).collect::<Vec<_>>(),
            "default": default,
            "answer_with": {"type": "research", "decision": id, "advance": "<one of available>"},
        }),
        Decision::Founding { id, unit } => json!({
            "type": "founding", "decision": id, "unit": entity(unit),
            "answer_with": {"type": "name_city", "decision": id, "name": "<city name>"},
        }),
        Decision::Government { id, eligible } => json!({
            "type": "government", "decision": id, "eligible": eligible,
            "answer_with": {"type": "government", "decision": id, "government": "<one of eligible>"},
        }),
        Decision::Rates { id, view, opening } => json!({
            "type": "rates", "decision": id,
            "cap": view.cap, "opening": {"tax": opening[0], "luxury": opening[1], "science": opening[2]},
            "answer_with": {"type": "review_rates", "decision": id, "science": "<tenths>", "tax": "<tenths>"},
        }),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntityArg {
    slot: usize,
    id: u32,
}

impl From<EntityArg> for Entity {
    fn from(e: EntityArg) -> Self {
        Entity {
            slot: e.slot,
            id: e.id,
        }
    }
}

/// The JSON command schema an agent submits. Purchases, sales and quoted
/// production switches need engine quotes and are not offered yet.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum CommandArg {
    Move { unit: EntityArg, to: (u16, u16) },
    Occupy { unit: EntityArg, city: EntityArg },
    Skip { unit: EntityArg },
    Found { unit: EntityArg },
    NameCity { decision: u64, name: String },
    CancelFound { decision: u64 },
    Production { city: EntityArg, item: i8 },
    Worker { city: EntityArg, tile: u8 },
    Specialist { city: EntityArg, slot: u8 },
    Research { decision: u64, advance: usize },
    Goal { advance: Option<usize> },
    Rates { science: u8, tax: u8 },
    // Empty struct variants (not unit variants) so that unknown fields are
    // rejected: serde ignores `deny_unknown_fields` for internally tagged
    // unit variants.
    Revolution {},
    Government { decision: u64, government: u8 },
    ReviewRates { decision: u64, science: u8, tax: u8 },
    EndTurn {},
}

fn parse_command(value: &Value) -> Result<Command, String> {
    let arg: CommandArg =
        serde_json::from_value(value.clone()).map_err(|e| format!("invalid command: {e}"))?;
    Ok(match arg {
        CommandArg::Move { unit, to } => Command::Move {
            unit: unit.into(),
            to,
        },
        CommandArg::Occupy { unit, city } => Command::Occupy {
            unit: unit.into(),
            city: city.into(),
        },
        CommandArg::Skip { unit } => Command::Skip { unit: unit.into() },
        CommandArg::Found { unit } => Command::BeginFound { unit: unit.into() },
        CommandArg::NameCity { decision, name } => Command::NameCity {
            decision,
            name: engine::player::CityName::new(&name).map_err(str::to_string)?,
        },
        CommandArg::CancelFound { decision } => Command::CancelFound { decision },
        CommandArg::Production { city, item } => Command::Production {
            city: city.into(),
            item,
        },
        CommandArg::Worker { city, tile } => Command::Worker {
            city: city.into(),
            tile,
        },
        CommandArg::Specialist { city, slot } => Command::Specialist {
            city: city.into(),
            slot,
        },
        CommandArg::Research { decision, advance } => Command::Research { decision, advance },
        CommandArg::Goal { advance } => Command::Goal { advance },
        CommandArg::Rates { science, tax } => Command::Rates { science, tax },
        CommandArg::Revolution {} => Command::Revolution,
        CommandArg::Government {
            decision,
            government,
        } => Command::Government {
            decision,
            government,
        },
        CommandArg::ReviewRates {
            decision,
            science,
            tax,
        } => Command::ReviewRates {
            decision,
            science,
            tax,
        },
        CommandArg::EndTurn {} => Command::EndTurn,
    })
}
