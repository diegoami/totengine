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
    /// The starting date as the original computes it (#330); `None` when
    /// the calendar is not located.
    pub date: Option<(i32, crate::calendar::DateStyle)>,
    pub width: usize,
    pub height: usize,
    pub civs: Vec<ScenarioCiv>,
    /// Plain-language gaps, most important first.
    pub gaps: Vec<String>,
}

/// Triggers the engine fires today (turn loop, battles, captures).
pub const FIRED_TRIGGERS: [&str; 8] = [
    "TURN",
    "TURNINTERVAL",
    "RANDOMTURN",
    "UNITKILLED",
    "CITYTAKEN",
    "CHECKFLAG",
    "RECEIVEDTECHNOLOGY",
    "SCENARIOLOADED",
];
/// Actions and modifiers that change the game today.
pub const APPLIED_ACTIONS: [&str; 14] = [
    "CHANGEMONEY",
    "CREATEUNIT",
    "GIVETECHNOLOGY",
    "TAKETECHNOLOGY",
    "CHANGETERRAIN",
    "MAKEAGGRESSION",
    "DELAY",
    "JUSTONCE",
    "FLAG",
    "CONTINUOUS",
    "RANDOMIZE",
    "INCAPITAL",
    "COLLAPSE",
    "NOCOLLAPSE",
];

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
    // Triggers whose game event the engine cannot produce yet.
    let mut missing_feature = Vec::new();
    let mut actions = Vec::new();
    let mut create_unit = 0;
    let mut flag_tech = 0;
    let mut delay_random = 0;
    let mut other_map = 0;
    let mut aggression = 0;
    let mut media = 0;
    let mut no_schism = 0;
    for e in &events.events {
        // Both halves of an @AND event must fire.
        for half in std::iter::once(&e.trigger).chain(&e.and) {
            let t = half.keyword.to_ascii_uppercase();
            if !FIRED_TRIGGERS.contains(&t.as_str()) {
                let list = match t.as_str() {
                    "BRIBEUNIT" | "NEGOTIATION" => &mut missing_feature,
                    _ => &mut triggers,
                };
                summarize(list, &t);
            } else if t == "CHECKFLAG" && half.param("technology").is_some() {
                flag_tech += 1;
            }
        }
        for (n, a) in e.actions.iter().enumerate() {
            let k = a.keyword.to_ascii_uppercase();
            let next = e.actions.get(n + 1).map(|c| c.keyword.as_str());
            if k == "DELAY" && next == Some("RANDOMIZE") {
                delay_random += 1;
            }
            if k == "CHANGETERRAIN" && a.param("map").and_then(|v| v.number()).unwrap_or(0) != 0 {
                other_map += 1;
            }
            if k == "MAKEAGGRESSION" {
                aggression += 1;
            }
            match k.as_str() {
                // Shown to the player since #326.
                "TEXT" => {}
                "CREATEUNIT" => create_unit += 1,
                k if APPLIED_ACTIONS.contains(&k) => {}
                // A file argument (PLAYWAVEFILE's sound) read as a keyword.
                k if k.contains('.') => {}
                // CREATEUNIT's coordinate block.
                "LOCATIONS" | "ENDLOCATIONS" => {}
                "PLAYCDTRACK" | "DONTPLAYWONDERS" => {}
                "PLAYWAVEFILE" | "PLAYAVIFILE" => media += 1,
                "NOSCHISM" => no_schism += 1,
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
    if !missing_feature.is_empty() {
        gaps.push(format!(
            "Events that never fire because units cannot be bribed and there is no diplomacy yet: {}",
            counted(&missing_feature)
        ));
    }
    if !actions.is_empty() {
        gaps.push(format!(
            "Event actions that do nothing yet: {}",
            counted(&actions)
        ));
    }
    if flag_tech > 0 {
        gaps.push(format!(
            "CHECKFLAG with technology= never fires ({flag_tech} event{})",
            if flag_tech == 1 { "" } else { "s" }
        ));
    }
    let plural = |n: usize| if n == 1 { "" } else { "s" };
    if delay_random > 0 {
        gaps.push(format!(
            "DELAY with Randomize waits the full delay ({delay_random} event{})",
            plural(delay_random)
        ));
    }
    if other_map > 0 {
        gaps.push(format!(
            "CHANGETERRAIN on maps other than 0 does nothing ({other_map} action{})",
            plural(other_map)
        ));
    }
    if aggression > 0 {
        gaps.push(format!(
            "MAKEAGGRESSION declares war, but a human party's treaty-break consequences are not modelled ({aggression} action{})",
            plural(aggression)
        ));
    }
    if media > 0 {
        gaps.push(format!(
            "Event sounds and videos are not played ({media} action{})",
            plural(media)
        ));
    }
    if no_schism > 0 {
        gaps.push(format!(
            "NOSCHISM has nothing to prevent: the engine never splits a civilization in civil war ({no_schism} action{})",
            plural(no_schism)
        ));
    }
    if create_unit > 0 {
        gaps.push(format!(
            "CREATEUNIT placement skips the original's impassable-terrain test ({create_unit} action{})",
            if create_unit == 1 { "" } else { "s" }
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
        format!("This file cannot be decoded yet ({e}); multi-map and extended scenarios are not supported")
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
    if world.date().is_none() {
        gaps.push(
            "The scenario calendar is not decoded for this file; dates show as turn numbers".into(),
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
        date: world.date(),
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
        assert!(
            gaps.iter()
                .any(|g| g.contains("no diplomacy") && g.contains("NEGOTIATION")),
            "{gaps:?}"
        );
        // Sounds and NOSCHISM say why they change nothing (#328).
        let media = event_gaps(
            &tot_kit::parse_events(
                "@IF\nTURN\nturn=1\n@THEN\nPLAYWAVEFILE\nx.wav\nNOSCHISM\nwhom=A\n@ENDIF\n",
            )
            .unwrap(),
        );
        assert!(
            media.iter().any(|g| g.starts_with("Event sounds")),
            "{media:?}"
        );
        assert!(
            media.iter().any(|g| g.starts_with("NOSCHISM has nothing")),
            "{media:?}"
        );
        assert!(
            !media.iter().any(|g| g.contains("do nothing yet")),
            "{media:?}"
        );
        assert!(
            gaps.iter().any(|g| g.contains("MAKEAGGRESSION")),
            "{gaps:?}"
        );
        // Flags (#328): FLAG/CHECKFLAG are supported; an @AND half is
        // still reported when its trigger is not.
        let file = tot_kit::parse_events(
            "@INITFLAG\n@IF\nCHECKFLAG\nwho=A\nflag=0\nstate=on\n@AND\nBRIBEUNIT\n\
             who=A\nwhom=B\nunittype=1\n@THEN\nFLAG\ncontinuous\nflag=1\nstate=on\n@ENDIF\n",
        )
        .unwrap();
        let flag_gaps = event_gaps(&file);
        assert!(
            flag_gaps.iter().any(|g| g.contains("BRIBEUNIT")),
            "{flag_gaps:?}"
        );
        assert!(
            !flag_gaps
                .iter()
                .any(|g| g.contains("FLAG") || g.contains("CONTINUOUS")),
            "{flag_gaps:?}"
        );
        // TEXT boxes are shown (#326): no gap for them.
        assert!(!gaps.iter().any(|g| g.contains("TEXT")), "{gaps:?}");
        assert!(!gaps.iter().any(|g| g.contains("TURN,")), "{gaps:?}");
    }
}
