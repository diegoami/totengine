//! Original executable outputs from supplied install Rules and saved positions.
use engine::ai_research::{choose, value, Context, ValueContext, ValueRules};
use engine::combat::CrtRand;

#[test]
fn parsed_rules_and_world_research_match_native_saved_inputs() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        return;
    };
    let home = std::path::PathBuf::from(home);
    let mut count = 0;
    for line in include_str!("fixtures/ai-research-adapter.txt").lines() {
        if line.starts_with('#') {
            continue;
        }
        let parts: Vec<_> = line.split(" | ").collect();
        let header: Vec<_> = parts[0].split_whitespace().collect();
        let path = home.join(header[0]);
        let civ = header[1].parse::<u8>().unwrap();
        let saved = tot_kit::parse_sav(&std::fs::read(&path).unwrap()).unwrap();
        let rules = tot_kit::parse_rules(&tot_kit::latin1(
            &std::fs::read(path.parent().unwrap().join("Rules.txt")).unwrap(),
        ));
        let world = engine::boot(&saved, 106);
        let before = world.checksum();
        let tables = ValueRules::from_rules(&rules).unwrap();
        let context = ValueContext::from_world(&world, &rules, civ).unwrap();
        assert_eq!(context.sea_units, header[2].parse::<i16>().unwrap());
        let expected: Vec<i32> = parts[1]
            .split_whitespace()
            .map(|n| n.parse().unwrap())
            .collect();
        let eligible: Vec<_> = parts[2].split_whitespace().map(|n| n == "1").collect();
        assert_eq!(expected.len(), 100);
        assert_eq!(eligible.len(), 100);
        let candidates = world.native_research_candidates(&rules, civ).unwrap();
        for id in 0..100 {
            assert_eq!(
                value(&tables, &context, id),
                Some(expected[id]),
                "{} civ {civ} advance {id}",
                header[0]
            );
            assert_eq!(candidates[id].value, expected[id]);
            assert_eq!(
                candidates[id].eligible, eligible[id],
                "{} civ {civ} gate {id}",
                header[0]
            );
        }
        let choices: Vec<i64> = parts[3]
            .split_whitespace()
            .map(|n| n.parse().unwrap())
            .collect();
        assert_eq!(choices.len(), 24);
        for choice in choices.as_chunks::<4>().0 {
            let mut rng = CrtRand::new(choice[0] as u32);
            let winner = choose(
                &candidates,
                Context {
                    human: world.human_mask & (1 << civ) != 0,
                    difficulty: choice[1] as u8,
                    acquired: world.research[civ as usize].acquired,
                },
                &mut rng,
            );
            assert_eq!(
                winner.map_or(-1, |v| v as i64),
                choice[2],
                "{} civ {civ} difficulty {}",
                header[0],
                choice[1]
            );
            assert_eq!(i64::from(rng.state()), choice[3]);
        }
        assert_eq!(
            world.checksum(),
            before,
            "mechanics adapter must be read-only"
        );
        count += 1;
    }
    assert_eq!(count, 14);
}
