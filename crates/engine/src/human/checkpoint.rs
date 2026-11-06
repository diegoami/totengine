//! Replay-based engine checkpoints. Original resource bytes stay local.
use super::*;

fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
    })
}

impl HumanSession {
    /// Reconstructs private opponent state, receipts, decisions and partial AI
    /// turns at O(recorded work) cost. `resources` is the adapter's canonical
    /// manifest identity. This is not a native SAV or a browser-visible payload.
    pub fn checkpoint(&self, resources: &str) -> String {
        let rules = tot_kit::emit_rules(&self.game.rules);
        let mut text = format!(
            "totengine-human 2\nengine {}\nsetup {}\nhuman {}\npolicy builtin-1\nrules {} {:016x}\nresources {} {:016x}\nworld {:016x}\nknowledge {:016x}\nlog {} {:016x}\n",
            env!("CARGO_PKG_VERSION"), self.config.descriptor(), self.human,
            rules.len(), fingerprint(rules.as_bytes()), resources.len(), fingerprint(resources.as_bytes()),
            self.game.world.checksum(), self.knowledge_fingerprint(), self.game.log().len(), fingerprint(self.game.log().as_bytes()),
        );
        for op in &self.journal {
            match op {
                Operation::Advance(n) => text.push_str(&format!("advance {n}\n")),
                Operation::Input(r) => {
                    text.push_str(&format!(
                        "input {} {} {} {}\n",
                        r.civ,
                        r.revision,
                        r.id,
                        r.command.encode()
                    ));
                }
            }
        }
        text.push_str("end\n");
        text
    }

    /// Builds a temporary replacement. Failure cannot mutate the active session.
    pub fn restore(rules: Rules, resources: &str, text: &str) -> Result<Self, String> {
        if text.len() > 16 * 1024 * 1024 {
            return Err("human checkpoint exceeds 16 MiB".into());
        }
        let mut lines = text.lines();
        if lines.next() != Some("totengine-human 2")
            || lines.next() != Some(&format!("engine {}", env!("CARGO_PKG_VERSION")))
        {
            return Err("unsupported human checkpoint or engine version".into());
        }
        let config = NewGameConfig::from_descriptor(
            lines
                .next()
                .and_then(|l| l.strip_prefix("setup "))
                .ok_or("missing setup")?,
        )?;
        let human = lines
            .next()
            .and_then(|l| l.strip_prefix("human "))
            .and_then(|n| n.parse().ok())
            .ok_or("invalid human owner")?;
        if lines.next() != Some("policy builtin-1") {
            return Err("unsupported human opponent policy".into());
        }
        let canonical_rules = tot_kit::emit_rules(&rules);
        if lines.next()
            != Some(&format!(
                "rules {} {:016x}",
                canonical_rules.len(),
                fingerprint(canonical_rules.as_bytes())
            ))
        {
            return Err("checkpoint Rules identity differs".into());
        }
        if lines.next()
            != Some(&format!(
                "resources {} {:016x}",
                resources.len(),
                fingerprint(resources.as_bytes())
            ))
        {
            return Err("checkpoint resource identity differs".into());
        }
        // Final world/log assertions are checked by canonical re-emission below.
        lines.next().ok_or("missing world fingerprint")?;
        lines.next().ok_or("missing knowledge fingerprint")?;
        lines.next().ok_or("missing order-log identity")?;
        let mut session = Self::generated(config, rules, human)?;
        let mut ended = false;
        for (count, line) in lines.by_ref().enumerate() {
            if count > 200_000 {
                return Err("too many checkpoint operations".into());
            }
            if line == "end" {
                ended = true;
                break;
            }
            if let Some(n) = line.strip_prefix("advance ") {
                let n: usize = n.parse().map_err(|_| "invalid advance budget")?;
                if !(1..=256).contains(&n) || session.advance(n) != n {
                    return Err("checkpoint opponent progression differs".into());
                }
            } else if let Some(rest) = line.strip_prefix("input ") {
                let mut fields = rest.splitn(4, ' ');
                let request = Request {
                    civ: fields
                        .next()
                        .and_then(|v| v.parse().ok())
                        .ok_or("invalid request owner")?,
                    revision: fields
                        .next()
                        .and_then(|v| v.parse().ok())
                        .ok_or("invalid request revision")?,
                    id: fields
                        .next()
                        .and_then(|v| v.parse().ok())
                        .ok_or("invalid request ID")?,
                    command: Command::decode(fields.next().ok_or("missing command")?)
                        .ok_or("invalid command")?,
                };
                session.submit(request)?;
            } else {
                return Err("invalid human checkpoint operation".into());
            }
        }
        if !ended || lines.next().is_some() || session.checkpoint(resources) != text {
            return Err("human checkpoint state/transcript differs from reconstruction".into());
        }
        Ok(session)
    }
}

impl Command {
    fn encode(self) -> String {
        match self {
            Self::Skip { unit } => format!("skip {} {}", unit.slot, unit.id),
            Self::Move { unit, to } => format!("move {} {} {} {}", unit.slot, unit.id, to.0, to.1),
            Self::Occupy { unit, city } => {
                format!("occupy {} {} {} {}", unit.slot, unit.id, city.slot, city.id)
            }
            Self::Production { city, item } => {
                format!("production {} {} {item}", city.slot, city.id)
            }
            Self::BeginFound { unit } => format!("found {} {}", unit.slot, unit.id),
            Self::NameCity { decision, name } => {
                let hex = name
                    .as_str()
                    .as_bytes()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>();
                format!("name {decision} {hex}")
            }
            Self::CancelFound { decision } => format!("cancel {decision}"),
            Self::Research { decision, advance } => format!("research {decision} {advance}"),
            Self::EndTurn => "end".into(),
        }
    }

    fn decode(text: &str) -> Option<Self> {
        let fields: Vec<_> = text.split_whitespace().collect();
        match fields.as_slice() {
            ["skip", slot, id] => Some(Self::Skip {
                unit: Entity {
                    slot: slot.parse().ok()?,
                    id: id.parse().ok()?,
                },
            }),
            ["move", slot, id, x, y] => Some(Self::Move {
                unit: Entity {
                    slot: slot.parse().ok()?,
                    id: id.parse().ok()?,
                },
                to: (x.parse().ok()?, y.parse().ok()?),
            }),
            ["occupy", slot, id, city, city_id] => Some(Self::Occupy {
                unit: Entity {
                    slot: slot.parse().ok()?,
                    id: id.parse().ok()?,
                },
                city: Entity {
                    slot: city.parse().ok()?,
                    id: city_id.parse().ok()?,
                },
            }),
            ["production", slot, id, item] => Some(Self::Production {
                city: Entity {
                    slot: slot.parse().ok()?,
                    id: id.parse().ok()?,
                },
                item: item.parse().ok()?,
            }),
            ["found", slot, id] => Some(Self::BeginFound {
                unit: Entity {
                    slot: slot.parse().ok()?,
                    id: id.parse().ok()?,
                },
            }),
            ["name", decision, name] => Some(Self::NameCity {
                decision: decision.parse().ok()?,
                name: CityName::from_hex(name)?,
            }),
            ["cancel", decision] => Some(Self::CancelFound {
                decision: decision.parse().ok()?,
            }),
            ["research", decision, advance] => Some(Self::Research {
                decision: decision.parse().ok()?,
                advance: advance.parse().ok()?,
            }),
            ["end"] => Some(Self::EndTurn),
            _ => None,
        }
    }
}
