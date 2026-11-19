//! #329: scenario Rules that rename advances (skips without CIVTOT_HOME).
//! References resolve through the original's fixed slot identifiers, not
//! the row comments, so the built-in AI reads every installed scenario.

use engine::research::TechTree;

const SCENARIOS: [&str; 5] = [
    "Scenario/FallRome",
    "Scenario/Hannibal",
    "Scenario/Roman Civil Wars",
    "Scenario/ToT_AE-1200BC",
    "Scenario/ToT_Constantine",
];

fn rules(home: &std::path::Path, dir: &str) -> Option<tot_kit::Rules> {
    let raw = std::fs::read(home.join(dir).join("Rules.txt")).ok()?;
    Some(tot_kit::parse_rules(&tot_kit::latin1(&raw)))
}

#[test]
fn builtin_ai_reads_every_installed_scenario() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(std::path::PathBuf::from) else {
        eprintln!("skipping: CIVTOT_HOME not set");
        return;
    };
    let mut seen = 0;
    for dir in SCENARIOS {
        let Some(rules) = rules(&home, dir) else {
            continue;
        };
        seen += 1;
        if let Err(e) = engine::ai::BuiltInAi::new(&rules, 0, 1) {
            panic!("{dir}: {e}");
        }
        let tree = TechTree::from_rules(&rules);
        assert!(
            tree.unresolved_prereqs.is_empty(),
            "{dir}: {:?}",
            tree.unresolved_prereqs
        );
    }
    if seen == 0 {
        eprintln!("skipping: no scenarios installed");
    }
}

#[test]
fn renamed_rows_resolve_by_slot_not_comment() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(std::path::PathBuf::from) else {
        eprintln!("skipping: CIVTOT_HOME not set");
        return;
    };
    // Roman Civil Wars: Astrology needs Mys and Mat; Mathematics' comment
    // is `Mat;50`. AE 1200 BC: Alphabet needs AFl, the slot-0 row
    // "Pictographs" commented `AFl Advanced Flight`.
    if let Some(r) = rules(&home, "Scenario/Roman Civil Wars") {
        let tree = TechTree::from_rules(&r);
        let ast = tree
            .techs
            .iter()
            .position(|t| t.name == "Astrology")
            .unwrap();
        let mat = tree
            .techs
            .iter()
            .position(|t| t.name == "Mathematics")
            .unwrap();
        assert_eq!(mat, 49);
        assert_eq!(tree.techs[ast].prereqs[1], Some(mat));
        assert!(!tree.techs[ast].defective);
    }
    if let Some(r) = rules(&home, "Scenario/ToT_AE-1200BC") {
        let tree = TechTree::from_rules(&r);
        let alp = tree
            .techs
            .iter()
            .position(|t| t.name == "Alphabet")
            .unwrap();
        assert_eq!(tree.techs[alp].prereqs[0], Some(0));
        assert_eq!(tree.techs[0].name, "Pictographs");
    }
}
