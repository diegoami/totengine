//! Event interpreter (T1-7): fires parsed scenario events against game
//! state and returns the effects the loop must apply.
//!
//! Scope (from the FallRome corpus — 64 events): triggers
//! SCENARIOLOADED / UNITKILLED / CITYTAKEN / NEGOTIATION / RANDOMTURN,
//! actions TEXT / CREATEUNIT / CHANGEMONEY / PLAYCDTRACK / JUSTONCE /
//! DONTPLAYWONDERS. Matching follows Macro.txt: `ANYBODY` is a wildcard,
//! every other name must match exactly (case-insensitive).
//! `JUSTONCE` marks the event as fired (a one-shot latch).

use std::collections::HashSet;

use tot_kit::{EventFile, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Macro.txt: create `count` units (default 1) at the FIRST valid
    /// location of `locations`; later entries are fallbacks tried in order
    /// (`Randomize` picks one at random — not modelled).
    CreateUnit {
        unit_name: String,
        owner: String,
        veteran: bool,
        count: u32,
        locations: Vec<(i32, i32)>,
    },
    ChangeMoney(i32),
    GiveTechnology(String),
    Text(String),
    PlayTrack(i32),
    DontPlayWonders,
}

/// Context for one trigger evaluation.
#[derive(Debug, Clone, Default)]
pub struct Ctx {
    pub unit: Option<String>,
    pub attacker: Option<String>,
    pub defender: Option<String>,
    pub talker: Option<String>,
    pub listener: Option<String>,
    pub denominator: Option<i32>,
}

pub struct EventEngine {
    pub file: EventFile,
    fired: HashSet<usize>,
    /// Texts emitted by the last fire() — also returned as effects.
    pub log: Vec<String>,
}

fn word_of(v: Option<&Value>) -> Option<&str> {
    v.and_then(|v| v.word())
}

/// ANYBODY / ANYONE match everything; otherwise names must match
/// **exactly**, case-insensitively — Macro.txt (shipped with the game):
/// "unit types, civilization names, and so on must match the
/// corresponding names in the rules.txt file exactly". (Prefix matching
/// would fire FallRome's "Theodoric II dies" event when Theodoric I dies.)
fn matches(param: Option<&str>, actual: Option<&str>) -> bool {
    let Some(p) = param else { return true };
    if p.eq_ignore_ascii_case("ANYBODY") || p.eq_ignore_ascii_case("ANYONE") {
        return true;
    }
    let Some(a) = actual else { return false };
    p.trim().eq_ignore_ascii_case(a.trim())
}

impl EventEngine {
    pub fn new(file: EventFile) -> Self {
        EventEngine { file, fired: HashSet::new(), log: Vec::new() }
    }

    /// Evaluate every event with this trigger keyword; returns effects.
    /// JUSTONCE events fire at most once; non-JUSTONCE events re-arm.
    pub fn fire(&mut self, trigger: &str, ctx: &Ctx) -> Vec<Effect> {
        let mut effects = Vec::new();
        for (i, e) in self.file.events.iter().enumerate() {
            if !e.trigger.keyword.eq_ignore_ascii_case(trigger) {
                continue;
            }
            if self.fired.contains(&i) {
                continue;
            }
            let t = &e.trigger;
            if !matches(word_of(t.param("unit")), ctx.unit.as_deref())
                || !matches(word_of(t.param("attacker")), ctx.attacker.as_deref())
                || !matches(word_of(t.param("defender")), ctx.defender.as_deref())
                || !matches(word_of(t.param("talker")), ctx.talker.as_deref())
                || !matches(word_of(t.param("listener")), ctx.listener.as_deref())
            {
                continue;
            }
            // RANDOMTURN: fire with probability 1/denominator
            if e.trigger.keyword.eq_ignore_ascii_case("RANDOMTURN") {
                let denom = t.param("denominator").and_then(|v| v.number()).unwrap_or(2);
                // caller decides via ctx.denominator == Some(roll); here we
                // treat Some(n) as "roll succeeded" only when n == denom
                if ctx.denominator != Some(denom) {
                    continue;
                }
            }
            let justonce = e.actions.iter().any(|a| a.keyword == "JUSTONCE");
            for a in &e.actions {
                match a.keyword.as_str() {
                    "TEXT" => {
                        if let Some(Value::Text(s)) = a.args.first() {
                            effects.push(Effect::Text(s.clone()));
                            self.log.push(s.clone());
                        }
                    }
                    "CREATEUNIT" => {
                        let unit = word_of(a.param("unit")).unwrap_or("").to_string();
                        let owner = word_of(a.param("owner")).unwrap_or("ANYBODY").to_string();
                        let veteran = word_of(a.param("veteran"))
                            .map(|v| v.eq_ignore_ascii_case("yes"))
                            .unwrap_or(false);
                        let count = a
                            .param("count")
                            .and_then(|v| v.number())
                            .map(|n| n.clamp(1, 255) as u32)
                            .unwrap_or(1);
                        if let Some(Value::Points(pts)) =
                            a.args.iter().find(|v| matches!(v, Value::Points(_)))
                        {
                            effects.push(Effect::CreateUnit {
                                unit_name: unit,
                                owner,
                                veteran,
                                count,
                                locations: pts.clone(),
                            });
                        }
                    }
                    "CHANGEMONEY" => {
                        if let Some(n) = a.param("amount").and_then(|v| v.number()) {
                            effects.push(Effect::ChangeMoney(n));
                        }
                    }
                    "PLAYCDTRACK" => {
                        if let Some(n) = a.args.first().and_then(|v| v.number()) {
                            effects.push(Effect::PlayTrack(n));
                        }
                    }
                    "DONTPLAYWONDERS" => effects.push(Effect::DontPlayWonders),
                    "GIVETECHNOLOGY" => {
                        if let Some(t) = word_of(a.param("tech")) {
                            effects.push(Effect::GiveTechnology(t.to_string()));
                        }
                    }
                    _ => {} // JUSTONCE and unknown actions carry no effect
                }
            }
            if justonce {
                self.fired.insert(i);
            }
        }
        effects
    }

    pub fn fired_count(&self) -> usize {
        self.fired.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tot_kit::parse_events;

    fn fallrome_events() -> Option<EventFile> {
        let p = std::env::var_os("CIVTOT_HOME")?;
        let src = std::fs::read_to_string(
            std::path::Path::new(&p).join("Scenario/FallRome/Events.txt")).ok()?;
        parse_events(&src).ok()
    }

    #[test]
    fn scenario_loaded_fires_track_only() {
        let f = parse_events(
            "@IF\nSCENARIOLOADED\n@THEN\nPLAYCDTRACK\n11\nDONTPLAYWONDERS\n@ENDIF",
        )
        .unwrap();
        let mut eng = EventEngine::new(f);
        let eff = eng.fire("SCENARIOLOADED", &Ctx::default());
        assert_eq!(eff, vec![Effect::PlayTrack(11), Effect::DontPlayWonders]);
    }

    #[test]
    fn theodoric_killed_creates_successor_once() {
        let Some(f) = fallrome_events() else {
            eprintln!("skipping: no FallRome Events.txt");
            return;
        };
        let mut eng = EventEngine::new(f);
        let ctx = Ctx {
            unit: Some("Theodoric I".into()),
            attacker: Some("Huns".into()),
            defender: Some("Visigoths".into()),
            ..Default::default()
        };
        let eff = eng.fire("UNITKILLED", &ctx);
        let creates: Vec<_> = eff
            .iter()
            .filter(|e| matches!(e, Effect::CreateUnit { .. }))
            .collect();
        // ONE create (Macro.txt: first valid location, the rest are
        // fallbacks) of one veteran Theodoric II, six candidate tiles.
        assert_eq!(creates.len(), 1, "{creates:?}");
        match creates[0] {
            Effect::CreateUnit { unit_name, owner, veteran, count, locations } => {
                assert_eq!((unit_name.as_str(), owner.as_str()), ("Theodoric II", "Visigoths"));
                assert!(*veteran);
                assert_eq!(*count, 1);
                assert_eq!(locations.len(), 6);
                assert_eq!(locations[0], (24, 60));
            }
            _ => unreachable!(),
        }
        assert!(eff.iter().any(|e| matches!(e, Effect::Text(t) if t.contains("Theodoric II"))));
        // JUSTONCE: a second kill of the same king does not re-fire
        let again = eng.fire("UNITKILLED", &ctx);
        assert!(again.is_empty(), "JUSTONCE latch held");
    }

    #[test]
    fn exact_and_anybody_matching() {
        let f = parse_events(
            "@IF\nUNITKILLED\nunit=Theodoric I\ndefender=west romans\nattacker=ANYBODY\n@THEN\nTEXT\nhit\nENDTEXT\n@ENDIF",
        )
        .unwrap();
        let mut eng = EventEngine::new(f);
        let ctx = Ctx {
            unit: Some("Theodoric I".into()),
            attacker: Some("Francs".into()),
            defender: Some("West Romans".into()),
            ..Default::default()
        };
        assert!(!eng.fire("UNITKILLED", &ctx).is_empty(), "exact (any case) + ANYBODY");
        for unit in ["Theodoric II", "Theodoric"] {
            let wrong = Ctx { unit: Some(unit.into()), ..ctx.clone() };
            assert!(eng.fire("UNITKILLED", &wrong).is_empty(), "{unit} is not Theodoric I");
        }
        let wrong = Ctx { defender: Some("West".into()), ..ctx };
        assert!(eng.fire("UNITKILLED", &wrong).is_empty(), "no prefix match");
    }
}
