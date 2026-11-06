//! Compare engine policies' initial build intentions on a supplied save.
use engine::player::{Order, Player};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let path = std::path::Path::new(&args[1]);
    let bytes = std::fs::read(path).unwrap();
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(path.parent().unwrap().join("Rules.txt")).unwrap(),
    ));
    let mut world = engine::boot(&tot_kit::parse_sav(&bytes).unwrap(), 106);
    if let Some(difficulty) = args.get(2) {
        world.difficulty = difficulty.parse().unwrap();
    }
    for builtin in [false, true] {
        let mut w = world.clone();
        for civ in 1..8 {
            let mut ai: Box<dyn Player> = if builtin {
                Box::new(engine::ai::BuiltInAi::new(&rules, 106, civ).unwrap())
            } else {
                Box::new(engine::ai::AiV1::new(&rules, 106, civ))
            };
            let mut last = None;
            for _ in 0..1000 {
                let order = ai.act(&w.observe(&rules, civ, last.take()));
                if !matches!(
                    order,
                    Order::SetResearch { .. } | Order::SetProduction { .. }
                ) {
                    break;
                }
                last = Some(w.apply_order(&rules, None, civ, order));
            }
        }
        for (i, c) in w.cities.iter().enumerate().filter(|(_, c)| c.city_id != 0) {
            println!(
                "{} {i} {} {} {}",
                if builtin { "builtin" } else { "ai-v1" },
                c.owner,
                c.size,
                c.building
            );
        }
    }
}
