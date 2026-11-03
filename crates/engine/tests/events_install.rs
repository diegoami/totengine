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

    // Tyras taken from the Huns: the tech gift is reported, not applied
    let tyras = Ctx {
        attacker: Some("Byzantines".into()),
        defender: Some("Huns".into()),
        city: Some("Tyras".into()),
        ..Default::default()
    };
    let out = w.fire_event(&rules, &mut events, "CITYTAKEN", &tyras);
    assert!(out.iter().any(
        |(e, a)| matches!(e, Effect::GiveTechnology { technology: 99, .. })
            && matches!(a, Applied::Skipped(_))
    ));
}
