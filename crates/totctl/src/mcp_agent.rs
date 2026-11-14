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
    /// One summary of the agent's civilization per turn reached (#263).
    history: Vec<Value>,
    stats: CommandStats,
}

/// What happened to the agent's submissions (#263).
#[derive(Default)]
struct CommandStats {
    submitted: usize,
    accepted: usize,
    /// Orders the engine executed as "nothing changed", by reason.
    engine_rejected: std::collections::BTreeMap<String, usize>,
    /// Submissions refused before reaching the game, by message.
    refused: std::collections::BTreeMap<String, usize>,
    /// Turns in which the action budget ran out.
    budget_exhausted_turns: std::collections::BTreeSet<i16>,
}

impl CommandStats {
    fn json(&self) -> Value {
        json!({
            "submitted": self.submitted,
            "accepted": self.accepted,
            "engine_rejected": self.engine_rejected,
            "refused": self.refused,
            "budget_exhausted_turns": self.budget_exhausted_turns,
        })
    }
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
            history: Vec::new(),
            stats: CommandStats::default(),
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
            self.history.push(self.summary());
        }
        Ok(())
    }

    /// The agent civilization's state at the start of its turn.
    fn summary(&self) -> Value {
        let view = self.session.view();
        let o = &view.observation;
        let own: Vec<_> = o.cities.iter().filter(|c| c.owner == self.civ).collect();
        json!({
            "turn": o.turn,
            "cities": own.len(),
            "population": own.iter().map(|c| u32::from(c.size)).sum::<u32>(),
            "units": o.units.iter().filter(|u| u.owner == self.civ).count(),
            "treasury": o.treasury,
            "advances": known_advances(&o.techs).len(),
            "research": o.research.target,
            "government": o.government,
        })
    }

    /// Turn-by-turn summary and command statistics. Observed state only; no
    /// strength claim.
    pub(crate) fn report_json(&self) -> Value {
        report(
            &self.agent,
            self.civ,
            i64::from(self.turn),
            &self.history,
            self.stats.json(),
        )
    }

    /// Preview a production switch: the engine quote the `production`
    /// command would apply now (retained shields and penalty).
    pub(crate) fn quote_production(&self, city: &Value, item: i8) -> Result<Value, String> {
        let city: EntityArg =
            serde_json::from_value(city.clone()).map_err(|e| format!("invalid city: {e}"))?;
        let q = self
            .session
            .preview_production(city.into(), item)
            .map_err(|e| format!("no quote: {e}"))?;
        Ok(json!({
            "item": q.item, "name": self.item_name(q.item),
            "current_item": q.current_item, "current_shields": q.current_shields,
            "retained_shields": q.retained, "penalty_percent": q.penalty,
            "target_cost": q.target_cost,
            "note": "submit {\"type\": \"production\", \"city\": ..., \"item\": ...} to switch at this quote",
        }))
    }

    pub(crate) fn submit(
        &mut self,
        revision: u64,
        request_id: u64,
        command: &Value,
    ) -> Result<Value, String> {
        self.stats.submitted += 1;
        let outcome = self.submit_inner(revision, request_id, command);
        if let Err(message) = &outcome {
            *self.stats.refused.entry(message.clone()).or_default() += 1;
        }
        outcome
    }

    fn submit_inner(
        &mut self,
        revision: u64,
        request_id: u64,
        command: &Value,
    ) -> Result<Value, String> {
        let command = self.resolve(parse_command(command)?)?;
        if command != Command::EndTurn && self.actions_this_turn >= ACTION_BUDGET {
            self.stats.budget_exhausted_turns.insert(self.turn);
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
        match &response.result {
            Some(engine::player::OrderResult::Rejected(reason)) => {
                *self
                    .stats
                    .engine_rejected
                    .entry(reason.to_string())
                    .or_default() += 1;
            }
            _ => self.stats.accepted += 1,
        }
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

    /// Turn quote-bound commands into engine commands. The agent passes the
    /// price or income it saw in `agent_view`; the command executes only if
    /// that is still the current quote (quotes are immutable, as in the
    /// clients).
    fn resolve(&self, parsed: Parsed) -> Result<Command, String> {
        let production = |city: &Entity| {
            self.session
                .view()
                .city_production
                .into_iter()
                .find(|p| p.index == city.slot)
                .ok_or_else(|| "not one of your cities".to_string())
        };
        match parsed {
            Parsed::Direct(command) => Ok(command),
            Parsed::Buy { city, price } => {
                let quote = production(&city)?
                    .hurry
                    .map_err(|e| format!("cannot buy: {e}"))?;
                if quote.price != price {
                    return Err(format!(
                        "quote changed: buying now costs {}, not {price}",
                        quote.price
                    ));
                }
                Ok(Command::Buy { city, quote })
            }
            Parsed::Sell {
                city,
                improvement,
                income,
            } => {
                let quote = production(&city)?
                    .improvements
                    .into_iter()
                    .find(|(id, _)| *id == improvement)
                    .ok_or("the city has no such improvement")?
                    .1
                    .map_err(|e| format!("cannot sell: {e}"))?;
                if quote.income != income {
                    return Err(format!(
                        "quote changed: selling now yields {}, not {income}",
                        quote.income
                    ));
                }
                Ok(Command::Sell { city, quote })
            }
        }
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
                    let production = view.city_production.iter().find(|p| p.index == e.slot);
                    let buy = production.map(|p| match &p.hurry {
                        Ok(q) => json!({"price": q.price, "stored": q.stored, "required": q.required}),
                        Err(why) => json!({"unavailable": why}),
                    });
                    let sell: Vec<Value> = production
                        .map(|p| {
                            p.improvements
                                .iter()
                                .map(|(id, q)| match q {
                                    Ok(q) => json!({"improvement": id, "name": self.improvement_name(*id), "income": q.income}),
                                    Err(why) => json!({"improvement": id, "name": self.improvement_name(*id), "unavailable": why}),
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    json!({
                        "city": entity(e),
                        "name": c.name,
                        "x": c.x, "y": c.y,
                        "size": c.size,
                        "building": c.building.map(|i| json!({"item": i, "name": self.item_name(i)})),
                        "shield_box": c.shield_box,
                        "remaining_shields": production.and_then(|p| p.remaining),
                        "turns_to_complete": production.and_then(|p| p.turns),
                        "net_shields": production.map(|p| p.net_shields),
                        "production_choices": choices,
                        "buy": buy,
                        "sell": sell,
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
            "production_context_city": view.production_context_city.as_ref().map(entity),
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

    /// A `.totrec` recording. Its inert presentation stream carries the
    /// per-turn summaries and command statistics as agent-reported notes
    /// (not verified by replay), so a report can be read back from the file.
    pub(crate) fn recording(&self) -> Result<String, String> {
        let mut bundle = tot_record::Bundle::human(
            &self.checkpoint(),
            &self.rules,
            AGENT_RESOURCES,
            tot_record::StopReason::Stopped,
            &format!("{AGENT_PRODUCER}{}", self.agent),
        )?;
        let mut notes = String::new();
        for summary in &self.history {
            let mut entry = summary.clone();
            entry["round"] = summary["turn"].clone();
            entry["kind"] = json!("agent-summary");
            notes.push_str(&entry.to_string());
            notes.push('\n');
        }
        let mut stats = self.stats.json();
        stats["round"] = json!(self.turn.max(0));
        stats["kind"] = json!("agent-commands");
        notes.push_str(&stats.to_string());
        notes.push('\n');
        bundle.presentation = Some(notes);
        bundle.encode()
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

    fn improvement_name(&self, id: u8) -> String {
        self.rules
            .section("IMPROVE")
            .and_then(|s| s.rows.get(usize::from(id)))
            .and_then(|r| r.first())
            .cloned()
            .unwrap_or_else(|| format!("Improvement {id}"))
    }

    fn advance_name(&self, id: usize) -> String {
        self.rules
            .tech_defs()
            .get(id)
            .map_or_else(|| format!("Advance {id}"), |t| t.name.clone())
    }
}

/// Prefix of the recording producer for agent games.
pub(crate) const AGENT_PRODUCER: &str = "totctl mcp agent (experimental, #256): ";

fn report(agent: &str, civ: u8, turn: i64, history: &[Value], commands: Value) -> Value {
    json!({
        "agent": agent,
        "civ": civ,
        "turn": turn,
        "history": history,
        "commands": commands,
        "note": "observed state of the agent's civilization; no playing-strength claim",
    })
}

/// The same report, read back from an agent recording's presentation notes.
pub(crate) fn report_from_recording(bundle: &tot_record::Bundle) -> Result<Value, String> {
    let agent = bundle
        .manifest
        .producer
        .strip_prefix(AGENT_PRODUCER)
        .ok_or("not an agent recording")?;
    let civ = bundle.manifest.human.ok_or("not an agent recording")?;
    let mut history = Vec::new();
    let mut commands = None;
    for line in bundle.presentation.as_deref().unwrap_or_default().lines() {
        let mut entry: Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
        let kind = entry["kind"].as_str().unwrap_or_default().to_string();
        let object = entry.as_object_mut().ok_or("invalid note")?;
        object.remove("kind");
        object.remove("round");
        match kind.as_str() {
            "agent-summary" => history.push(entry),
            "agent-commands" => commands = Some(entry),
            _ => {}
        }
    }
    Ok(report(
        agent,
        civ,
        i64::from(bundle.manifest.progress),
        &history,
        commands.ok_or("recording has no agent command statistics")?,
    ))
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
    Move {
        unit: EntityArg,
        to: (u16, u16),
    },
    Occupy {
        unit: EntityArg,
        city: EntityArg,
    },
    Skip {
        unit: EntityArg,
    },
    Found {
        unit: EntityArg,
    },
    NameCity {
        decision: u64,
        name: String,
    },
    CancelFound {
        decision: u64,
    },
    Production {
        city: EntityArg,
        item: i8,
    },
    Worker {
        city: EntityArg,
        tile: u8,
    },
    Specialist {
        city: EntityArg,
        slot: u8,
    },
    Buy {
        city: EntityArg,
        price: i32,
    },
    Sell {
        city: EntityArg,
        improvement: u8,
        income: i32,
    },
    CloseProduction {
        city: EntityArg,
    },
    Research {
        decision: u64,
        advance: usize,
    },
    Goal {
        advance: Option<usize>,
    },
    Rates {
        science: u8,
        tax: u8,
    },
    // Empty struct variants (not unit variants) so that unknown fields are
    // rejected: serde ignores `deny_unknown_fields` for internally tagged
    // unit variants.
    Revolution {},
    Government {
        decision: u64,
        government: u8,
    },
    ReviewRates {
        decision: u64,
        science: u8,
        tax: u8,
    },
    EndTurn {},
}

/// A parsed command; quote-bound ones are resolved against the current view.
enum Parsed {
    Direct(Command),
    Buy {
        city: Entity,
        price: i32,
    },
    Sell {
        city: Entity,
        improvement: u8,
        income: i32,
    },
}

fn parse_command(value: &Value) -> Result<Parsed, String> {
    let arg: CommandArg =
        serde_json::from_value(value.clone()).map_err(|e| format!("invalid command: {e}"))?;
    Ok(Parsed::Direct(match arg {
        CommandArg::Buy { city, price } => {
            return Ok(Parsed::Buy {
                city: city.into(),
                price,
            })
        }
        CommandArg::Sell {
            city,
            improvement,
            income,
        } => {
            return Ok(Parsed::Sell {
                city: city.into(),
                improvement,
                income,
            })
        }
        CommandArg::CloseProduction { city } => Command::CloseProduction { city: city.into() },
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
    }))
}
