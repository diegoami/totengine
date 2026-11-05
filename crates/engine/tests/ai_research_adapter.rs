use engine::ai_research::{ValueContext, ValueRules};
use engine::newgame::NewGameConfig;

fn rules() -> tot_kit::Rules {
    let mut text = include_str!("fixtures/campaign-rules.txt").to_string();
    text.push_str("\n@ENDWONDER\n");
    text.push_str(&"nil\n".repeat(28));
    text.push_str("@LEADERS\n");
    text.push_str(&"First, Second, 0, 1, 0, Group, Own, 0, 0, -1\n".repeat(21));
    tot_kit::parse_rules(&text)
}

#[test]
fn adapted_fresh_world_filters_unavailable_slots_and_group_flags() {
    let mut rules = rules();
    // A disabled advance annotation retains the same reference slot.
    rules.sections.get_mut("CIVILIZE").unwrap().comments[1] = "*T01".into();
    rules.sections.get_mut("CIVILIZE").unwrap().rows[2][3] = "t01".into();
    let tables = ValueRules::from_rules(&rules).unwrap();
    assert_eq!(tables.advances[2].prerequisites, [1, -1]);
    let mut world = engine::newgame::generate(NewGameConfig::default(), &rules).unwrap();
    world.research_settings.flags = 8;
    world.game_flags = 4;
    let context = ValueContext::from_world(&world, &rules, 1).unwrap();
    assert_eq!(context.game_flags, 8, "scoring uses save @660, not @652");
    assert_eq!(context.leader_modifier, -1);
    let candidates = world.native_research_candidates(&rules, 1).unwrap();
    assert!(candidates[0].eligible);
    assert!(!candidates[2].eligible, "prerequisite still unknown");
    assert!(candidates[12..].iter().all(|c| !c.eligible));
    let mut groups = tot_kit::parse_rules("@CIVILIZE2\n0\n@LEADERS2\n10000000\n");
    rules.sections.append(&mut groups.sections);
    assert!(
        !world.native_research_candidates(&rules, 1).unwrap()[0].eligible,
        "native selector rejects flag 1 although the player gate permits it"
    );
    assert!(world.observe_research(&rules, 1).available.contains(&0));
}

#[test]
fn malformed_scoring_inputs_fail_instead_of_guessing() {
    let original = rules();
    assert!(ValueRules::from_rules(&original).is_ok());
    let mut bad = original.clone();
    bad.sections.get_mut("UNITS").unwrap().rows[0][12] = "unknown".into();
    assert!(ValueRules::from_rules(&bad)
        .err()
        .unwrap()
        .contains("unresolved"));
    let mut bad = original.clone();
    bad.sections.get_mut("UNITS").unwrap().rows[0].pop();
    bad.sections.get_mut("UNITS").unwrap().rows[0].pop();
    assert!(ValueRules::from_rules(&bad).is_err());
    let mut bad = original.clone();
    bad.sections.get_mut("CIVILIZE").unwrap().rows[0][1] = "128".into();
    assert!(ValueRules::from_rules(&bad).is_err());
    let mut bad = original.clone();
    bad.sections.remove("ENDWONDER");
    assert!(ValueRules::from_rules(&bad).is_err());
    let mut footer = original.clone();
    footer
        .sections
        .get_mut("UNITS")
        .unwrap()
        .rows
        .push(vec!["Text-only trailing annotation".into()]);
    assert!(ValueRules::from_rules(&footer).is_ok());
    let mut world = engine::newgame::generate(NewGameConfig::default(), &original).unwrap();
    world.ai_state.leader_ids[1] = 21;
    assert!(ValueContext::from_world(&world, &original, 1).is_err());
    assert!(ValueContext::from_world(&world, &original, 0).is_err());
}
