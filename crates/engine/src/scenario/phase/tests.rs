use super::*;

fn round_code(value: RoundEntry) -> i64 {
    match value {
        RoundEntry::TurnStart => 0,
        RoundEntry::RestartSetup => 1,
        RoundEntry::HumanPreparation => 2,
        RoundEntry::OpponentDispatch => 3,
    }
}

fn science_code(value: ScienceBoundary) -> i64 {
    match value {
        ScienceBoundary::Return => 0,
        ScienceBoundary::ProjectCost => 1,
        ScienceBoundary::AdvisorBeforeChoice => 2,
        ScienceBoundary::ChooseProject => 3,
    }
}

#[test]
fn kernels_match_original_loader_round_guard_and_science_vectors() {
    let mut counts = [0; 4];
    for line in include_str!("../../../../../docs/evidence/import-phase-vectors.txt").lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let (input, output) = line.split_once('|').unwrap();
        let mut fields = input.split_whitespace();
        let kind = fields.next().unwrap();
        let input: Vec<i64> = fields.map(|v| v.parse().unwrap()).collect();
        let output: Vec<i64> = output
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        match kind {
            "L" => {
                assert_eq!(
                    output,
                    [input[0], 0, 1, input[2], input[3], input[4], 0, 0, 0],
                    "{line}"
                );
                counts[0] += 1;
            }
            "R" => {
                assert_eq!(
                    [round_code(round_entry(
                        input[0] != 0,
                        input[2] != 0,
                        input[3] & (1 << input[1]) != 0
                    ))],
                    output.as_slice(),
                    "{line}"
                );
                counts[1] += 1;
            }
            "S" => {
                assert!((1..8).contains(&input[0]));
                let state = ResearchState {
                    progress: input[1].try_into().unwrap(),
                    target: input[2].try_into().unwrap(),
                    acquired: input[6].try_into().unwrap(),
                    ..Default::default()
                };
                let result = science_entry(
                    state,
                    input[3].try_into().unwrap(),
                    input[4] != 0,
                    input[5] != 0,
                );
                assert_eq!(
                    [i64::from(result.progress), science_code(result.boundary)],
                    output.as_slice(),
                    "{line}"
                );
                counts[2] += 1;
            }
            "P" => {
                assert_eq!(output, [-1, input[1], input[2]], "{line}");
                counts[3] += 1;
            }
            _ => panic!("unknown native vector {line}"),
        }
    }
    assert_eq!(counts, [98, 280, 2016, 1]);
}

fn context() -> (SaveFile, Rules) {
    let mut save = crate::tests_support::synthetic_save();
    save.multiplayer = 0;
    save.player_civ = 1;
    save.active_civ = 1;
    save.human_mask = 2;
    save.civs_mask = 2;
    save.research[1].target = -1;
    let rules = tot_kit::parse_rules("@COSMIC\n3 ; Road movement multiplier\n");
    (save, rules)
}

#[test]
fn saved_context_check_is_stricter_than_experimental_resume_without_mutation() {
    let (save, rules) = context();
    let before = save.clone();
    assert_eq!(
        saved_human_entry(&save, &rules, 1).unwrap(),
        RoundEntry::HumanPreparation
    );
    assert_eq!(save, before);
    assert!(saved_human_entry(&save, &rules, 0).is_err());
    assert!(saved_human_entry(&save, &rules, 8).is_err());
    let mut other_active = save.clone();
    other_active.active_civ = 2;
    assert!(super::super::resume_blocker(&other_active, &rules, 1).is_none());
    assert!(saved_human_entry(&other_active, &rules, 1)
        .unwrap_err()
        .contains("active civilization"));
    let unknown = tot_kit::parse_rules("@COSMIC\n3\n");
    assert!(super::super::resume_blocker(&save, &unknown, 1).is_none());
    assert!(saved_human_entry(&save, &unknown, 1)
        .unwrap_err()
        .contains("explicitly decoded"));
}

#[test]
fn unsupported_saved_contexts_return_reasons() {
    type Edit = fn(&mut SaveFile);
    let cases: [(Edit, &str); 8] = [
        (|s| s.version = 0x33, "classic Original"),
        (|s| s.game_mode = 1, "classic Original"),
        (|s| s.map_header.raw[7] = 1, "surface context"),
        (|s| s.multiplayer = 1, "multiplayer"),
        (|s| s.research_settings.flags |= 0x80, "scenario files"),
        (|s| s.human_mask = 6, "human mask"),
        (|s| s.civs_mask = 0, "live active"),
        (|s| s.research[1].target = -2, "research target"),
    ];
    for (edit, reason) in cases {
        let (mut save, rules) = context();
        edit(&mut save);
        let error = saved_human_entry(&save, &rules, 1).unwrap_err();
        assert!(error.contains(reason), "{error}");
    }
    let (mut save, rules) = context();
    for unit in &mut save.units {
        unit.raw[30..34].fill(0);
    }
    for city in &mut save.cities {
        city.raw[86..90].fill(0);
    }
    assert!(saved_human_entry(&save, &rules, 1)
        .unwrap_err()
        .contains("no live units or cities"));
}

#[test]
fn installed_first_candidate_matches_the_measured_round_and_science_boundaries() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        return;
    };
    let home = std::path::PathBuf::from(home);
    let save = tot_kit::parse_sav_file(&home.join("Original/Ph_Auto2.SAV")).unwrap();
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(home.join("Original/Rules.txt")).unwrap(),
    ));
    assert_eq!(
        saved_human_entry(&save, &rules, 4).unwrap(),
        RoundEntry::HumanPreparation
    );
    let state = save.research[4];
    assert_eq!((state.progress, state.target), (0, -1));
    assert_eq!(
        science_entry(state, 0, true, false),
        ScienceEntry {
            progress: 0,
            boundary: ScienceBoundary::Return
        }
    );
    assert_eq!(
        science_entry(state, 1, true, false),
        ScienceEntry {
            progress: 1,
            boundary: ScienceBoundary::ChooseProject
        }
    );
}
