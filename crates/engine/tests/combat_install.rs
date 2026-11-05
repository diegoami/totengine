//! T1-6 acceptance against the real install (skips without CIVTOT_HOME):
//! boot ROMFALL.scn, seed the combat generator with the CRT state read
//! from the live original before oracle battle 2, and attack exactly as in
//! that battle — the engine must reproduce the original's strengths,
//! outcome, end state and choice of defender (evidence §T1-6 combat,
//! docs/evidence/t1-6-combat-oracle.json).

use engine::combat::CrtRand;

fn romfall() -> Option<(engine::WorldState, tot_kit::Rules)> {
    let home = std::path::PathBuf::from(std::env::var_os("CIVTOT_HOME")?);
    let sav = tot_kit::parse_sav_file(&home.join("FallRome/ROMFALL.scn")).ok()?;
    // FallRome's Rules.txt is not strictly UTF-8.
    let bytes = std::fs::read(home.join("FallRome/Rules.txt")).ok()?;
    let rules = tot_kit::parse_rules(&String::from_utf8_lossy(&bytes));
    Some((engine::boot(&sav, 1), rules))
}

fn find(w: &engine::WorldState, x: u16, y: u16, owner: u8) -> Vec<usize> {
    (0..w.units.len())
        .filter(|&i| {
            let u = &w.units[i];
            u.id != 0 && (u.x, u.y, u.owner) == (x, y, owner)
        })
        .collect()
}

#[test]
fn oracle_battle_2_through_move_unit() {
    let Some((mut w, rules)) = romfall() else {
        eprintln!("skipping: CIVTOT_HOME/FallRome not present");
        return;
    };
    let plurals: Vec<_> = (1..=7).map(|c| w.civ_name(c)).collect();
    assert_eq!(
        plurals,
        [
            "West Romans",
            "Anglo-Saxons",
            "Francs",
            "Vandals",
            "Visigoths",
            "Byzantines",
            "Huns"
        ]
    );
    w.start_turn(&rules);
    let legion = find(&w, 21, 31, 1);
    assert_eq!(legion.len(), 1);
    let legion = legion[0];
    assert_eq!(w.units[legion].type_id, 2, "WRE Legion");
    assert!(w.units[legion].veteran);
    let horsemen = find(&w, 22, 30, 0);
    assert_eq!(horsemen.len(), 2, "two B. Horsemen at (22,30)");
    // The save's stack list is 689 -> 510 (game indices = engine
    // indices); the original picked 510 (the later one).
    let expected_defender = w.units[510].clone();
    assert_eq!((expected_defender.x, expected_defender.y), (22, 30));

    w.combat_rng = CrtRand::new(0x3a69_6ba7);
    let before = w.units.len();
    let home = std::path::PathBuf::from(std::env::var_os("CIVTOT_HOME").unwrap());
    let src = String::from_utf8_lossy(&std::fs::read(home.join("FallRome/Events.txt")).unwrap())
        .into_owned();
    let mut eng = engine::events::EventEngine::new(tot_kit::parse_events(&src).unwrap());
    let treasury = w.treasury;
    let (out, fired) = w.move_unit_with_events(&rules, &mut eng, legion, 22, 30);
    // the kill went through the real Events.txt: no FallRome event for a
    // B. Horsemen, so nothing fires and the treasuries stand
    assert!(fired.is_empty(), "{fired:?}");
    assert_eq!(w.treasury, treasury);
    assert!(!out.moved, "an attack does not move the attacker");
    let b = out.battle.expect("moving onto barbarians is an attack");
    assert_eq!((b.attack, b.defense), (96, 54), "live-frame values");
    assert_eq!(b.outcome.rounds, 11);
    assert_eq!(
        (b.outcome.attacker_damage, b.outcome.defender_damage),
        (1, 10)
    );
    assert!(b.outcome.attacker_won);
    assert_eq!(w.combat_rng.state(), 0xa925_748c, "oracle CRT end state");
    assert_eq!(b.killed, expected_defender_after(&expected_defender));
    // Only the defender dies; its stack-mate survives (oracle).
    assert_eq!(w.units.len(), before, "casualties retain stable unit slots");
    assert_eq!(w.units[510].id, 0);
    assert_eq!(w.units[legion].id, b.attacker.id);
    assert_eq!(find(&w, 22, 30, 0).len(), 1);
    // The attacker keeps its tile, takes its damage, spends one point.
    let l = find(&w, 21, 31, 1)[0];
    assert_eq!(w.units[l].damage, 1);
    assert_eq!(w.units[l].moves_left, 0, "1-move Legion spent its point");

    // T1-7 bridge: the kill as the UNITKILLED context, fired against the
    // real FallRome Events.txt.
    let ctx = b.unitkilled_ctx(&w, &rules);
    assert_eq!(ctx.unit.as_deref(), Some("B. Horsemen"));
    assert_eq!(ctx.attacker.as_deref(), Some("West Romans"));
    assert_eq!(ctx.defender.as_deref(), Some("Barbarians"));
    assert!(
        eng.fire("UNITKILLED", &ctx).is_empty(),
        "no FallRome event for a horseman"
    );
    // The same kill of a Merchant is the "ransacked a merchant caravan"
    // event: +50 gold for the West Romans.
    let merchant = engine::events::Ctx {
        unit: Some("Merchant".into()),
        ..ctx
    };
    let eff = eng.fire("UNITKILLED", &merchant);
    assert!(
        eff.contains(&engine::events::Effect::ChangeMoney {
            receiver: "West Romans".into(),
            amount: 50
        }),
        "{eff:?}"
    );
}

fn expected_defender_after(d: &engine::Unit) -> engine::Unit {
    engine::Unit {
        damage: 10,
        // The post-damage snapshot uses original 0x568da2's land minimum.
        // This allowance is checked by movement vectors, not the battle capture.
        moves_left: 3,
        ..d.clone()
    }
}
