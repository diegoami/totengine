//! Retained outputs of the unmodified original selector/CRT, with the three
//! explicitly supplied callee boundaries documented in the evidence artifact.
use engine::ai_research::{choose, Candidate, Context};
use engine::combat::CrtRand;

#[test]
fn native_selection_winner_and_randomness_match() {
    let mut count = 0;
    for line in include_str!("fixtures/ai-research-choice.txt").lines() {
        if line.starts_with('#') {
            continue;
        }
        let (context, entries) = line.split_once(';').unwrap();
        let values: Vec<i64> = context
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        let candidates: Vec<_> = entries
            .split_whitespace()
            .map(|entry| {
                let fields: Vec<i32> = entry.split(':').map(|v| v.parse().unwrap()).collect();
                Candidate {
                    eligible: fields[0] != 0 && fields[1] != 0 && fields[2] == 0,
                    value: fields[3],
                }
            })
            .collect();
        let mut rng = CrtRand::new(values[3] as u32);
        let actual = choose(
            &candidates.try_into().unwrap(),
            Context {
                human: values[0] != 0,
                difficulty: values[1] as u8,
                acquired: values[2] as u8,
            },
            &mut rng,
        );
        assert_eq!(actual.map_or(-1, |v| v as i64), values[4], "case {count}");
        assert_eq!(i64::from(rng.state()), values[5], "case {count}");
        assert_eq!(rng.draws(), values[6] as u64, "case {count}");
        count += 1;
    }
    assert_eq!(count, 128);
}
