//! T1-7 acceptance against the real install (skips without CIVTOT_HOME):
//! FallRome's own Events.txt fired against the booted ROMFALL.scn, with
//! the effects applied to the world state.

use engine::effects::Applied;
use engine::events::{Ctx, Effect, EventEngine};

struct Fixture {
    w: engine::WorldState,
    rules: tot_kit::Rules,
    events: EventEngine,
}

fn fallrome() -> Option<Fixture> {
    let home = std::path::PathBuf::from(std::env::var_os("CIVTOT_HOME")?);
    let sav = tot_kit::parse_sav_file(&home.join("FallRome/ROMFALL.scn")).ok()?;
    let lossy = |f: &str| {
        std::fs::read(home.join("FallRome").join(f))
            .ok()
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    };
    let rules = tot_kit::parse_rules(&lossy("Rules.txt")?);
    let events = EventEngine::new(tot_kit::parse_events(&lossy("Events.txt")?).ok()?);
    Some(Fixture {
        w: engine::boot(&sav, 1),
        rules,
        events,
    })
}

#[test]
fn treasuries_boot_from_the_civ_blocks() {
    let Some(f) = fallrome() else {
        eprintln!("skipping: CIVTOT_HOME/FallRome not present");
        return;
    };
    // barbarians, WRE … Huns as stored in ROMFALL.scn
    assert_eq!(f.w.treasury, [4050, 10000, 200, 100, 100, 200, 10000, 208]);
}

#[test]
fn fallrome_events_change_the_world() {
    let Some(Fixture {
        mut w,
        rules,
        mut events,
    }) = fallrome()
    else {
        eprintln!("skipping: CIVTOT_HOME/FallRome not present");
        return;
    };
    // SCENARIOLOADED: music only
    let out = w.fire_event(&rules, &mut events, "SCENARIOLOADED", &Ctx::default());
    assert!(out.iter().all(|(_, a)| *a == Applied::NoState), "{out:?}");

    // A West Roman unit kills a Merchant: +50 gold
    let merchant = Ctx {
        unit: Some("Merchant".into()),
        attacker: Some("West Romans".into()),
        defender: Some("Vandals".into()),
        ..Default::default()
    };
    let out = w.fire_event(&rules, &mut events, "UNITKILLED", &merchant);
    assert!(out.contains(&(
        Effect::ChangeMoney {
            receiver: "West Romans".into(),
            amount: 50
        },
        Applied::Money {
            civ: 1,
            treasury: 10050
        }
    )));
    assert_eq!(w.treasury[1], 10050);

    // Theodoric I dies: one veteran Theodoric II for the Visigoths at the
    // first valid location
    let before = w.units.len();
    let king = Ctx {
        unit: Some("Theodoric I".into()),
        attacker: Some("West Romans".into()),
        defender: Some("Visigoths".into()),
        ..Default::default()
    };
    let out = w.fire_event(&rules, &mut events, "UNITKILLED", &king);
    let created: Vec<_> = out
        .iter()
        .filter_map(|(_, a)| match a {
            Applied::Created { first, count, at } => Some((*first, *count, *at)),
            _ => None,
        })
        .collect();
    assert_eq!(created, [(before, 1, (24, 60))], "{out:?}");
    let u = &w.units[before];
    assert_eq!(
        rules.unit_names()[u.type_id as usize].trim(),
        "Theodoric II"
    );
    assert_eq!((u.owner, u.veteran), (5, true));
    // JUSTONCE
    let again = w.fire_event(&rules, &mut events, "UNITKILLED", &king);
    assert!(again.is_empty());
    assert_eq!(w.units.len(), before + 1);

    // Tyras taken from the Huns: GIVETECHNOLOGY receiver=TRIGGERATTACKER
    // gives the attacker advance 99 ("Mixing") through the shared grant (#327).
    let tyras = Ctx {
        attacker: Some("Byzantines".into()),
        defender: Some("Huns".into()),
        city: Some("Tyras".into()),
        ..Default::default()
    };
    let byz = w.civ_by_name("Byzantines").unwrap();
    let knows = |w: &engine::WorldState| w.techs[byz as usize][99 / 8] & (1 << (99 % 8)) != 0;
    assert!(!knows(&w));
    let out = w.fire_event(&rules, &mut events, "CITYTAKEN", &tyras);
    assert!(
        out.iter().any(
            |(e, a)| matches!(e, Effect::GiveTechnology { technology: 99, .. })
                && *a
                    == Applied::Advance {
                        civ: byz,
                        advance: 99
                    }
        ),
        "{out:?}"
    );
    assert!(knows(&w));
}

/// #328: Roman Civil Wars' flags. Each Gaulish city lost sets one
/// continuous Gaul flag; the turn-start CHECKFLAG pass gives the Gauls
/// advance 33 once six of the thirteen are set (Threshold=6).
#[test]
fn roman_civil_wars_gaul_flags_reach_the_threshold() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(std::path::PathBuf::from) else {
        eprintln!("skipping: CIVTOT_HOME not set");
        return;
    };
    let dir = home.join("Scenario/Roman Civil Wars");
    let Ok(sav) = tot_kit::parse_sav_file(&dir.join("RomCivWars.SCN")) else {
        eprintln!("skipping: Roman Civil Wars not installed");
        return;
    };
    let read = |f: &str| tot_kit::latin1(&std::fs::read(dir.join(f)).unwrap());
    let rules = tot_kit::parse_rules(&read("Rules.txt"));
    let file = tot_kit::parse_events(&read("Events.txt")).unwrap();
    assert!(file.init_flags);
    // Cities whose loss by the Gauls sets a flag, in file order.
    let cities: Vec<String> = file
        .events
        .iter()
        .filter(|e| {
            e.trigger.keyword == "CITYTAKEN"
                && e.trigger.param("defender").and_then(|v| v.word()) == Some("Gauls")
                && e.actions.iter().any(|a| {
                    a.keyword == "FLAG" && a.param("state").and_then(|v| v.word()) == Some("on")
                })
        })
        .filter_map(|e| {
            e.trigger
                .param("city")
                .and_then(|v| v.word())
                .map(String::from)
        })
        .collect();
    assert!(cities.len() >= 6, "{cities:?}");
    let mut w = engine::boot(&sav, 1);
    let gauls = w.civ_by_name("Gauls").expect("Gauls civ");
    let mut events = EventEngine::new(file);
    let turn = Ctx {
        turn: Some(i32::from(w.turn) + 1),
        ..Default::default()
    };
    let gives_33 = |effects: &[Effect]| {
        effects.iter().any(|e| {
            matches!(e, Effect::GiveTechnology { receiver, technology: 33 } if receiver == "Gauls")
        })
    };
    let mut never = |_| false;
    for (n, city) in cities.iter().take(6).enumerate() {
        let ctx = Ctx {
            attacker: Some("Roman Senate".into()),
            defender: Some("Gauls".into()),
            city: Some(city.clone()),
            ..Default::default()
        };
        w.fire_event(&rules, &mut events, "CITYTAKEN", &ctx);
        assert_eq!(
            events.flags[usize::from(gauls)].count_ones() as usize,
            n + 1
        );
        events.civ_names = w.event_civ_names();
        let fx = events.turn_start(&turn, &mut never);
        assert_eq!(gives_33(&fx), n == 5, "after {} captures: {fx:?}", n + 1);
        if n == 5 {
            let applied = w.apply_effects(&rules, &fx);
            assert!(
                applied.contains(&Applied::Advance {
                    civ: gauls,
                    advance: 33
                }),
                "{applied:?}"
            );
        }
    }
    // Continuous flags survive the pass; JUSTONCE keeps the gift single.
    assert_eq!(events.flags[usize::from(gauls)].count_ones(), 6);
    assert!(!gives_33(&events.turn_start(&turn, &mut never)));
}
