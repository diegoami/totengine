//! Numeric outputs of complete original valuation/selection; no callee stubs.
use engine::ai_research::{
    choose, value, AdvanceValue, Candidate, Context, ValueCity, ValueContext, ValueRules,
};
use engine::combat::CrtRand;

#[test]
fn complete_native_values_and_selection_match() {
    let mut cases = 0;
    for line in include_str!("fixtures/ai-research-value.txt").lines() {
        if line.starts_with('#') {
            continue;
        }
        let parts: Vec<_> = line.split(" | ").collect();
        let mut input = parts[0]
            .split_whitespace()
            .map(|n| n.parse::<i64>().unwrap());
        let mut next = || input.next().expect("context field missing");
        let civ = next() as u8;
        let human_mask = next() as u8;
        let game_flags = next() as u16;
        let scenario_flags = next() as u16;
        let leader_id = next() as i16;
        let leader_modifier = next() as i8;
        let sea_units = next() as i16;
        let advance_76_mask = next() as u8;
        let rules = ValueRules {
            advances: std::array::from_fn(|_| AdvanceValue {
                base: next() as i8,
                modifier: next() as i8,
                prerequisites: [next() as i8, next() as i8],
            }),
            unit_prerequisites: std::array::from_fn(|_| next() as i8),
            improvement_prerequisites: std::array::from_fn(|_| next() as i8),
            wonder_expiry: std::array::from_fn(|_| next() as i8),
            growth_limits: [next() as u8, next() as u8],
        };
        let acquired = std::array::from_fn(|_| next() as u8);
        let reputations = std::array::from_fn(|_| next() as u8);
        let treaties = std::array::from_fn(|_| next() as u32);
        let mut city_landmasses = [[0; 256]; 8];
        for lands in &mut city_landmasses {
            for land in [1, 2, 254, 255] {
                lands[land] = next() as u8;
            }
        }
        let tech_owner_masks = std::array::from_fn(|_| next() as u8);
        assert_eq!(advance_76_mask, tech_owner_masks[76]);
        let city_count = next();
        let cities = (0..city_count)
            .map(|_| ValueCity {
                owner: next() as u8,
                size: next() as i8,
                live: next() != 0,
            })
            .collect();
        let wonder_cities = std::array::from_fn(|_| next() as i16);
        assert!(input.next().is_none());
        let state = ValueContext {
            civ,
            human_mask,
            game_flags,
            scenario_flags,
            leader_id,
            leader_modifier,
            acquired,
            reputations,
            treaties,
            city_landmasses,
            tech_owner_masks,
            sea_units,
            cities,
            wonder_cities,
        };
        let expected: Vec<i32> = parts[1]
            .split_whitespace()
            .map(|n| n.parse().unwrap())
            .collect();
        assert_eq!(expected.len(), 100);
        let eligibility: Vec<_> = parts[2].split_whitespace().map(|n| n == "1").collect();
        assert_eq!(eligibility.len(), 100);
        let candidates = std::array::from_fn(|id| {
            let score = value(&rules, &state, id).unwrap();
            assert_eq!(score, expected[id], "context {cases}, advance {id}");
            Candidate {
                eligible: eligibility[id],
                value: score,
            }
        });
        let selection: Vec<i64> = parts[3]
            .split_whitespace()
            .map(|n| n.parse().unwrap())
            .collect();
        let mut rng = CrtRand::new(selection[0] as u32);
        let winner = choose(
            &candidates,
            Context {
                human: human_mask & (1 << civ) != 0,
                difficulty: selection[1] as u8,
                acquired: acquired[civ as usize],
            },
            &mut rng,
        );
        assert_eq!(
            winner.map_or(-1, |v| v as i64),
            selection[2],
            "context {cases}"
        );
        assert_eq!(i64::from(rng.state()), selection[3], "context {cases}");
        cases += 1;
    }
    assert_eq!(cases, 128);
}
