//! T1-5 acceptance against the real install (skips without CIVTOT_HOME):
//! the parsed advance tree and wonder table must match the classic Civ2
//! tree, and the scripted research run must reach every advance.

use engine::research::{TechTree, WonderTable};

fn global_rules() -> Option<tot_kit::Rules> {
    let home = std::env::var_os("CIVTOT_HOME")?;
    let path = std::path::PathBuf::from(home).join("Rules.txt");
    let text = std::fs::read_to_string(path).ok()?;
    Some(tot_kit::parse_rules(&text))
}

#[test]
fn real_tech_tree_matches_classic_and_is_fully_reachable() {
    let Some(rules) = global_rules() else {
        eprintln!("skipping: CIVTOT_HOME/Rules.txt not present");
        return;
    };
    let tree = TechTree::from_rules(&rules);
    assert_eq!(tree.techs.len(), 100, "base ToT has 100 advances");
    assert!(
        tree.unresolved_prereqs.is_empty(),
        "dangling prereqs: {:?}",
        tree.unresolved_prereqs
    );

    let id = |abbrev: &str| {
        tree.by_abbrev(abbrev)
            .unwrap_or_else(|| panic!("abbrev {abbrev} missing"))
    };
    // classic Civ2 prerequisites (cross-checked against the documented
    // column layout: AiVal, Modifier, Preq1, Preq2, Epoch, Category)
    let by_name = |name: &str| {
        tree.techs
            .iter()
            .position(|t| t.name == name)
            .unwrap_or_else(|| panic!("tech {name} missing"))
    };
    let prereqs = |name: &str| tree.techs[by_name(name)].prereqs;
    assert_eq!(prereqs("Alphabet"), [None, None]);
    assert_eq!(prereqs("Bronze Working"), [None, None]);
    assert_eq!(prereqs("Writing"), [Some(id("Alp")), None]);
    assert_eq!(prereqs("Code of Laws"), [Some(id("Alp")), None]);
    assert_eq!(prereqs("Currency"), [Some(id("Bro")), None]);
    assert_eq!(
        prereqs("Iron Working"),
        [Some(id("Bro")), Some(id("War"))],
        "Bronze Working + Warrior Code"
    );
    assert_eq!(
        prereqs("Monarchy"),
        [Some(id("Cer")), Some(id("CoL"))],
        "Ceremonial Burial + Code of Laws"
    );
    assert_eq!(
        prereqs("Trade"),
        [Some(id("Cur")), Some(id("CoL"))],
        "Currency + Code of Laws"
    );
    // epochs: 0 Ancient, 1 Renaissance; categories: 3 Academic, 1 Economic
    assert_eq!(tree.techs[by_name("Alphabet")].epoch, Some(0));
    assert_eq!(
        tree.techs[by_name("Trade")].epoch,
        Some(0),
        "Trade is Ancient in this table"
    );
    assert_eq!(tree.techs[by_name("Alphabet")].category, Some(3));
    assert_eq!(tree.techs[by_name("Trade")].category, Some(1));

    // scripted run: fixed-point research reaches every advance
    let (order, unreached) = tree.scripted_research();
    assert!(
        unreached.is_empty(),
        "unreachable advances: {:?}",
        unreached
            .iter()
            .map(|&i| &tree.techs[i].name)
            .collect::<Vec<_>>()
    );
    assert_eq!(order.len(), tree.techs.len());
    assert!(order.len() <= 100, "one discovery per step at most");
}

#[test]
fn real_wonder_table_has_28_wonders_with_classic_obsolescence() {
    let Some(rules) = global_rules() else {
        eprintln!("skipping: CIVTOT_HOME/Rules.txt not present");
        return;
    };
    let tree = TechTree::from_rules(&rules);
    let wonders = WonderTable::from_rules(&rules, &tree);
    assert_eq!(
        wonders.wonders.len(),
        28,
        "28 wonder slots — matching the save's 28 i16 entries at 906"
    );
    let w = |name: &str| {
        wonders
            .wonders
            .iter()
            .find(|w| w.name == name)
            .unwrap_or_else(|| panic!("wonder {name} missing"))
    };
    // classic: Pyramids never expires, costs 20 rows (200 shields),
    // needs Masonry; Great Library expires with Education (E1); Great
    // Wall with Metallurgy (Met); Hanging Gardens with Railroad (RR).
    assert_eq!(w("Pyramids").obsolete_by, None);
    assert_eq!(w("Pyramids").cost, Some(20));
    assert_eq!(w("Pyramids").prereq, tree.by_abbrev("Mas"));
    assert_eq!(w("Great Library").obsolete_by, tree.by_abbrev("E1"));
    assert_eq!(w("Great Wall").obsolete_by, tree.by_abbrev("Met"));
    assert_eq!(w("Hanging Gardens").obsolete_by, tree.by_abbrev("RR"));
    assert_eq!(w("Apollo Program").obsolete_by, None);
    assert_eq!(w("SETI Program").obsolete_by, None);
}
