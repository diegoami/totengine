//! Headless demo: boot FallRome from the original install, fight one
//! battle, then run turns and narrate what the engine does.
//!
//!     CIVTOT_HOME=/path/to/CIVTOT cargo run -q -p engine --example headless_demo [turns]

use engine::effects::Applied;
use engine::events::{Effect, EventEngine};

fn main() {
    let home = std::path::PathBuf::from(std::env::var("CIVTOT_HOME").expect("set CIVTOT_HOME"));
    let turns: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(20);
    let dir = home.join("FallRome");
    // scenario text files are Latin-1, like the save's names
    let lossy = |f: &str| tot_kit::latin1(&std::fs::read(dir.join(f)).unwrap());
    let sav = tot_kit::parse_sav_file(&dir.join("ROMFALL.scn")).unwrap();
    let rules = tot_kit::parse_rules(&lossy("Rules.txt"));
    let mut events = EventEngine::new(tot_kit::parse_events(&lossy("Events.txt")).unwrap());
    let names = rules.unit_names();
    let uname = |t: u8| names.get(t as usize).map_or("?", |n| n.trim()).to_string();

    let mut w = engine::boot(&sav, 2026);
    w.start_turn(&rules);
    println!(
        "== FallRome booted headless: turn {}, {} units, {} cities",
        w.turn,
        w.units.len(),
        w.cities.len()
    );
    for c in 1..=7u8 {
        let n = w.cities.iter().filter(|x| x.owner == c as u16).count();
        println!(
            "   {:<13} {:>3} cities, treasury {:>5}",
            w.civ_name(c),
            n,
            w.treasury[c as usize]
        );
    }

    // one battle: the West Roman Legion at (21,31) attacks NE into (22,30)
    if let Some(l) = w
        .units
        .iter()
        .position(|u| (u.x, u.y, u.owner) == (21, 31, 1))
    {
        let (out, fired) = w.move_unit_with_events(&rules, &mut events, l, 22, 30);
        if let Some(b) = out.battle {
            println!(
                "\n== Battle: {} {} (A {}) vs {} {} (D {}): {} rounds, {} wins; {} dies",
                w.civ_name(b.attacker.owner),
                uname(b.attacker.type_id),
                b.attack,
                w.civ_name(b.defender.owner),
                uname(b.defender.type_id),
                b.defense,
                b.outcome.rounds,
                if b.outcome.attacker_won {
                    "attacker"
                } else {
                    "defender"
                },
                uname(b.killed.type_id)
            );
            println!("   UNITKILLED events fired: {}", fired.len());
        }
    }

    println!("\n== Running {turns} turns");
    for _ in 0..turns {
        let s = w.run_turn(&rules, Some(&mut events));
        let mut lines = Vec::new();
        for &k in &s.grew {
            let c = &w.cities[k];
            lines.push(format!("{} grows to {}", c.name, c.size));
        }
        for (e, a) in &s.events {
            match (e, a) {
                (Effect::Text(t), _) => lines.push(format!(
                    "event: \"{}\"",
                    t.lines().next().unwrap_or("").trim()
                )),
                (
                    Effect::CreateUnit {
                        unit_name, owner, ..
                    },
                    Applied::Created { at, .. },
                ) => lines.push(format!("  -> {owner} get a {unit_name} at {at:?}")),
                (Effect::ChangeMoney { receiver, amount }, Applied::Money { treasury, .. }) => {
                    lines.push(format!("  -> {receiver} {amount:+} gold (now {treasury})"))
                }
                _ => {}
            }
        }
        if !lines.is_empty() {
            println!("turn {:>3}: {}", s.turn, lines.join("; "));
        }
    }
    println!(
        "\n== After {turns} turns: turn {}, {} units, checksum {:016x}",
        w.turn,
        w.units.len(),
        w.checksum()
    );
}
