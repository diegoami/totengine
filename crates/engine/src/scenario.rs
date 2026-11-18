//! Experimental scenario entry (decision 0008, owner 2026-11-18).
//!
//! A scenario or classic save is booted into a fresh engine turn for human
//! play. This is **not** admission under `docs/scenario-entry-contract.md`:
//! [`preflight`] lists every gap the engine knows about so the player sees it
//! before starting, and files the decoders cannot frame are rejected.
use crate::WorldState;
use tot_kit::{EventFile, Rules};

/// Lowercase hex of `bytes` (checkpoint-safe encoding of free text).
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Inverse of [`hex`]; `None` on odd length or a non-hex digit.
pub fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

impl WorldState {
    /// The civilization still has a unit or a city.
    pub fn civ_alive(&self, civ: u8) -> bool {
        self.units.iter().any(|u| u.id != 0 && u.owner == civ)
            || self
                .cities
                .iter()
                .any(|c| c.city_id != 0 && (c.owner & 0x0F) as u8 == civ)
    }
}

/// One playable civilization of a scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioCiv {
    pub civ: u8,
    pub name: String,
    pub units: usize,
    pub cities: usize,
    /// The file marks this civilization as human-played.
    pub human_in_file: bool,
}

/// What the player sees before an experimental scenario starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preflight {
    pub turn: i16,
    pub year: i16,
    pub width: usize,
    pub height: usize,
    pub civs: Vec<ScenarioCiv>,
    /// Plain-language gaps, most important first.
    pub gaps: Vec<String>,
}

/// Triggers the engine fires today (turn loop, battles, captures).
pub const FIRED_TRIGGERS: [&str; 5] = [
    "TURN",
    "TURNINTERVAL",
    "RANDOMTURN",
    "UNITKILLED",
    "CITYTAKEN",
];
/// Actions that change the game today.
pub const APPLIED_ACTIONS: [&str; 3] = ["CHANGEMONEY", "CREATEUNIT", "JUSTONCE"];

fn summarize(list: &mut Vec<(String, usize)>, key: &str) {
    match list.iter_mut().find(|(k, _)| k == key) {
        Some((_, n)) => *n += 1,
        None => list.push((key.to_string(), 1)),
    }
}

fn counted(list: &[(String, usize)]) -> String {
    list.iter()
        .map(|(k, n)| {
            if *n == 1 {
                k.clone()
            } else {
                format!("{k} ×{n}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Event gaps: triggers that never fire and actions that do nothing.
pub fn event_gaps(events: &EventFile) -> Vec<String> {
    let mut triggers = Vec::new();
    let mut actions = Vec::new();
    let mut texts = 0;
    let mut give_tech = 0;
    let mut create_unit = 0;
    for e in &events.events {
        let t = e.trigger.keyword.to_ascii_uppercase();
        if !FIRED_TRIGGERS.contains(&t.as_str()) {
            summarize(&mut triggers, &t);
        }
        for a in &e.actions {
            let k = a.keyword.to_ascii_uppercase();
            match k.as_str() {
                "TEXT" => texts += 1,
                "GIVETECHNOLOGY" => give_tech += 1,
                "CREATEUNIT" => create_unit += 1,
                k if APPLIED_ACTIONS.contains(&k) => {}
                // A file argument (PLAYWAVEFILE's sound) read as a keyword.
                k if k.contains('.') => {}
                // CREATEUNIT's coordinate block.
                "LOCATIONS" | "ENDLOCATIONS" => {}
                "PLAYCDTRACK" | "DONTPLAYWONDERS" => {}
                _ => summarize(&mut actions, &k),
            }
        }
    }
    let mut gaps = Vec::new();
    if !triggers.is_empty() {
        gaps.push(format!(
            "Events that never fire (trigger not supported): {}",
            counted(&triggers)
        ));
    }
    if !actions.is_empty() {
        gaps.push(format!(
            "Event actions that do nothing yet: {}",
            counted(&actions)
        ));
    }
    if give_tech > 0 {
        gaps.push(format!(
            "GIVETECHNOLOGY is skipped ({give_tech} action{})",
            if give_tech == 1 { "" } else { "s" }
        ));
    }
    if create_unit > 0 {
        gaps.push(format!(
            "CREATEUNIT placement is approximate ({create_unit} action{})",
            if create_unit == 1 { "" } else { "s" }
        ));
    }
    if texts > 0 {
        gaps.push(format!(
            "Event messages are applied silently; their text is not shown yet ({texts})"
        ));
    }
    if !events.unknown_keywords.is_empty() {
        let mut k = events.unknown_keywords.clone();
        k.sort();
        k.dedup();
        gaps.push(format!("Unrecognized event keywords: {}", k.join(", ")));
    }
    gaps
}

/// Inspect scenario bytes for experimental entry. `events` is the parsed
/// Events.txt beside it (if any), `events_unreadable` that one exists but did
/// not parse, `lua` that Lua scripts sit beside it.
pub fn preflight(
    save: &[u8],
    rules: &Rules,
    events: Option<&EventFile>,
    events_unreadable: bool,
    lua: bool,
) -> Result<Preflight, String> {
    let sav = tot_kit::parse_sav(save).map_err(|e| {
        if save.get(10..12) == Some(&[0x33, 0][..]) {
            format!("This file cannot be decoded yet ({e}); its ToTPP-extended (0x33) layout is not supported")
        } else {
            format!("This file cannot be decoded yet ({e})")
        }
    })?;
    let world = crate::boot(&sav, 0);
    let civs: Vec<ScenarioCiv> = (1..8u8)
        .filter(|&c| world.civ_alive(c))
        .map(|c| ScenarioCiv {
            civ: c,
            name: world
                .civs
                .get(usize::from(c) - 1)
                .map(|v| {
                    if v.plural.trim().is_empty() {
                        v.names.first().cloned().unwrap_or_default()
                    } else {
                        v.plural.clone()
                    }
                })
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| format!("Civilization {c}")),
            units: world
                .units
                .iter()
                .filter(|u| u.id != 0 && u.owner == c)
                .count(),
            cities: world
                .cities
                .iter()
                .filter(|k| k.city_id != 0 && (k.owner & 0x0F) as u8 == c)
                .count(),
            human_in_file: world.human_mask & (1 << c) != 0,
        })
        .collect();
    if civs.is_empty() {
        return Err("No civilization has units or cities in this file".into());
    }
    let mut gaps = vec![
        "Starts a fresh engine turn: every unit gets its full movement again".to_string(),
        "Opponents are played by the engine AI, not the original AI's saved plans".to_string(),
    ];
    if sav.maps.len() > 1 {
        // Live records only: dead slots carry fill in their map byte.
        let units = sav
            .units
            .iter()
            .filter(|u| u.map_id() != 0 && u.id() != 0)
            .count();
        let cities = sav
            .cities
            .iter()
            .filter(|c| c.map_id() != 0 && c.city_id() != 0)
            .count();
        gaps.insert(
            0,
            format!(
                "Only the first of {} maps is played: {units} unit{} and {cities} cit{} on the other maps are left out",
                sav.maps.len(),
                if units == 1 { "" } else { "s" },
                if cities == 1 { "y" } else { "ies" },
            ),
        );
    }
    if lua {
        gaps.push("Lua scripts beside the scenario are not run".into());
    }
    if events_unreadable {
        gaps.push("Events.txt is present but could not be read; no events run".into());
    }
    if let Some(e) = events {
        gaps.extend(event_gaps(e));
    }
    let unknown_units = world
        .units
        .iter()
        .filter(|u| u.id != 0 && usize::from(u.type_id) >= rules.unit_names().len())
        .count();
    if unknown_units > 0 {
        gaps.push(format!(
            "{unknown_units} units have types missing from the Rules and cannot move"
        ));
    }
    gaps.push(
        "Diplomacy, spaceflight and other unbuilt features behave as in generated games".into(),
    );
    Ok(Preflight {
        turn: world.turn,
        year: world.year,
        width: world.map.stored_w,
        height: world.map.stored_h,
        civs,
        gaps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips_and_rejects_garbage() {
        let path = "Scenario/Roman Civil Wars/RomCivWars.SCN";
        assert_eq!(unhex(&hex(path.as_bytes())).unwrap(), path.as_bytes());
        assert!(unhex("abc").is_none());
        assert!(unhex("zz").is_none());
    }

    #[test]
    fn event_gaps_name_unsupported_triggers_and_actions() {
        let file = tot_kit::parse_events(
            "@BEGINEVENTS\n@IF\nTURN\nturn=1\n@THEN\nTEXT\nHello\nENDTEXT\n@ENDIF\n\
             @IF\nNEGOTIATION\ntalker=A\ntalkertype=Human\nlistener=B\nlistenertype=Human\n@THEN\nMAKEAGGRESSION\nwho=A\nwhom=B\n@ENDIF\n@ENDEVENTS\n",
        )
        .unwrap();
        let gaps = event_gaps(&file);
        assert!(gaps.iter().any(|g| g.contains("NEGOTIATION")), "{gaps:?}");
        assert!(
            gaps.iter().any(|g| g.contains("MAKEAGGRESSION")),
            "{gaps:?}"
        );
        assert!(
            gaps.iter().any(|g| g.contains("text is not shown")),
            "{gaps:?}"
        );
        assert!(!gaps.iter().any(|g| g.contains("TURN,")), "{gaps:?}");
    }
}
