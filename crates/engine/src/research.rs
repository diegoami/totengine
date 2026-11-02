//! Research tree + wonder table (T1-5).
//!
//! Built from the scenario's Rules.txt: `@CIVILIZE` rows are the advances
//! (their trailing comments are the alphabetically sorted three-letter
//! abbreviations used by prerequisite references), `@ENDWONDER` names the
//! wonders (with the advance that expires each), and wonder cost/
//! prerequisite come from the matching `@IMPROVE` row.
//!
//! The *scripted run* acceptance: starting from zero known advances, the
//! deterministic fixed-point research loop must reach every advance. An
//! unreachable advance (or an unresolvable prerequisite abbreviation) is
//! a parse/table defect, reported — never silently ignored.

use std::collections::HashMap;

use tot_kit::{Rules, WonderDef};

/// One advance with prerequisites resolved to indices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tech {
    pub name: String,
    pub abbrev: Option<String>,
    pub prereqs: [Option<usize>; 2],
    /// A prerequisite abbreviation did not resolve — the advance is
    /// treated as unresearchable (a dangling reference is a data defect,
    /// never silently "no prerequisite").
    pub defective: bool,
    pub epoch: Option<u8>,
    pub category: Option<u8>,
    pub ai_value: Option<i32>,
}

/// The research tree of one Rules.txt.
#[derive(Debug, Clone, Default)]
pub struct TechTree {
    pub techs: Vec<Tech>,
    /// Prerequisite abbreviations that resolved to nothing (data defect).
    pub unresolved_prereqs: Vec<String>,
    abbrev_to_id: HashMap<String, usize>,
}

impl TechTree {
    pub fn from_rules(rules: &Rules) -> Self {
        let defs = rules.tech_defs();
        let mut abbrev_to_id = HashMap::new();
        for (i, d) in defs.iter().enumerate() {
            if let Some(a) = &d.abbrev {
                abbrev_to_id.entry(a.clone()).or_insert(i);
            }
        }
        let mut unresolved = Vec::new();
        let techs = defs
            .into_iter()
            .map(|d| {
                let mut defective = false;
                let prereqs = d.prereqs.map(|p| {
                    let id = p.as_ref().and_then(|a| abbrev_to_id.get(a).copied());
                    if p.is_some() && id.is_none() {
                        defective = true;
                        if let Some(a) = &p {
                            unresolved.push(a.clone());
                        }
                    }
                    id
                });
                Tech {
                    name: d.name,
                    abbrev: d.abbrev,
                    prereqs,
                    defective,
                    epoch: d.epoch,
                    category: d.category,
                    ai_value: d.ai_value,
                }
            })
            .collect();
        TechTree {
            techs,
            unresolved_prereqs: unresolved,
            abbrev_to_id,
        }
    }

    pub fn by_abbrev(&self, abbrev: &str) -> Option<usize> {
        self.abbrev_to_id.get(abbrev).copied()
    }

    /// Can `id` be researched given the known set? (All prereqs known.)
    pub fn can_research(&self, id: usize, known: &[bool]) -> bool {
        match self.techs.get(id) {
            Some(t) => {
                !t.defective
                    && !known.get(id).copied().unwrap_or(false)
                    && t.prereqs.iter().all(|p| {
                        p.map(|pid| known.get(pid).copied().unwrap_or(false))
                            .unwrap_or(true)
                    })
            }
            None => false,
        }
    }

    /// Deterministic scripted run: repeatedly research the lowest-id
    /// researchable advance until nothing new can be researched. Returns
    /// the research order and the advances never reached.
    pub fn scripted_research(&self) -> (Vec<usize>, Vec<usize>) {
        let n = self.techs.len();
        let mut known = vec![false; n];
        let mut order = Vec::with_capacity(n);
        while let Some(next) = (0..n).find(|&i| self.can_research(i, &known)) {
            known[next] = true;
            order.push(next);
        }
        let unreached = (0..n).filter(|&i| !known[i]).collect();
        (order, unreached)
    }
}

/// Wonder table from one Rules.txt (name, cost/prereq, obsolescence),
/// with prerequisite/obsolescence advances resolved against the tree.
#[derive(Debug, Clone, Default)]
pub struct WonderTable {
    pub wonders: Vec<Wonder>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wonder {
    pub name: String,
    pub cost: Option<u32>,
    pub prereq: Option<usize>,
    pub obsolete_by: Option<usize>,
}

impl WonderTable {
    pub fn from_rules(rules: &Rules, tree: &TechTree) -> Self {
        let defs: Vec<WonderDef> = rules.wonder_defs();
        WonderTable {
            wonders: defs
                .into_iter()
                .map(|w| Wonder {
                    name: w.name,
                    cost: w.cost,
                    prereq: w.prereq.as_deref().and_then(|a| tree.by_abbrev(a)),
                    obsolete_by: w.obsolete_by.as_deref().and_then(|a| tree.by_abbrev(a)),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULES: &str = "\
@CIVILIZE\r\n\
Alphabet,           5, 1,  nil, nil, 0, 3    ; Alp\r\n\
Bronze Working,     6,-1,  nil, nil, 0, 4    ; Bro\r\n\
Code of Laws,       4, 1,  Alp, nil, 0, 2    ; CoL\r\n\
Writing,            4, 2,  Alp, nil, 0, 3    ; Wri\r\n\
Monarchy,           5, 1,  Cer, CoL, 0, 2    ; Mon\r\n\
@IMPROVE\r\n\
Barracks, 10, 1,  no,\r\n\
Pyramids, 20, 0,  Mas,\r\n\
@ENDWONDER\r\n\
nil,        ; Pyramids\r\n\
E1,         ; Hanging Gardens\r\n\
";

    #[test]
    fn tree_resolves_and_scripted_run_reaches_known_techs() {
        let rules = tot_kit::parse_rules(RULES);
        let tree = TechTree::from_rules(&rules);
        assert_eq!(tree.techs.len(), 5);
        assert_eq!(tree.by_abbrev("Alp"), Some(0));
        assert_eq!(tree.techs[4].prereqs[1], Some(2), "Monarchy needs CoL");
        // Cer is unresolved (not defined) — recorded, not swallowed.
        assert_eq!(tree.unresolved_prereqs, vec!["Cer"]);
        let (order, unreached) = tree.scripted_research();
        assert_eq!(order.len(), 4, "four techs are reachable");
        assert_eq!(unreached, vec![4], "Monarchy blocked by missing Cer");
        // deterministic
        assert_eq!(tree.scripted_research(), (order, unreached));
    }

    #[test]
    fn wonders_resolve_prereq_and_obsolescence() {
        let rules = tot_kit::parse_rules(RULES);
        let tree = TechTree::from_rules(&rules);
        let w = WonderTable::from_rules(&rules, &tree);
        assert_eq!(w.wonders.len(), 2);
        assert_eq!(w.wonders[0].name, "Pyramids");
        assert_eq!(w.wonders[0].prereq, None); // "Mas" not in the fixture
        assert_eq!(w.wonders[1].obsolete_by, None); // "E1" not in the fixture
    }
}
