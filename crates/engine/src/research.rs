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

/// The player's own research state and prerequisite-valid choices. The
/// engine exposes every such choice; the original UI's rotating subset
/// and ToTPP technology groups are not modelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchView {
    pub free_advances: u8,
    pub target: Option<usize>,
    pub progress: i16,
    /// None when the source scenario's cost settings are unsupported.
    pub cost: Option<i32>,
    pub available: Vec<usize>,
}

impl crate::WorldState {
    /// Original cost routine `civ2.bak 0x4a1ae0`, including integer truncation
    /// at each stage. See evidence §Per-civ research. The comparison slot
    /// is the saved @964 value; the engine does not yet update rankings.
    pub fn research_cost(&self, rules: &Rules, civ: u8) -> Option<i32> {
        let civ = civ as usize;
        if !(1..8).contains(&civ) {
            return None;
        }
        let settings = self.research_settings;
        let count = |c: usize| {
            self.research
                .get(c)
                .map(|r| r.acquired as i32 + r.adjustment as i32)
        };
        let raw = count(civ)?;
        let n = raw.max(1);
        let d = (self.difficulty as i32).min(4);
        let human = self.human_mask & (1 << civ) != 0;
        let mut factor = if human { 2 * d + 6 } else { 14 - d };
        let scenario = settings.flags & 0x80 != 0;
        let (paradigm, skip) = if scenario {
            let (p, s) = settings.scenario?;
            (p as i32, s != 0)
        } else {
            (rules.cosmic_row(11).unwrap_or(10) as i32, false)
        };
        if !skip {
            let reference = count(settings.reference_civ as usize)?;
            if reference > n {
                if self.difficulty != 0 {
                    factor -= 1;
                }
                if self.difficulty == 5 && reference > n + 4 && self.turn > 150 {
                    factor -= 1;
                }
            } else {
                factor += (n - reference) / 3;
            }
            if n >= 20 {
                factor += (n - self.turn as i32 / 8).clamp(0, 6);
            }
        }
        factor = factor * paradigm / 10;
        let mut bonus = (factor * 3) >> 2;
        if raw < 20 {
            bonus = bonus * raw / 20;
        }
        factor += bonus;
        if settings.advance_count > 67 {
            factor = factor * 67 / settings.advance_count as i32;
        }
        if settings.flags & 4 != 0 {
            factor = factor * 5 / 4;
        }
        if settings.flags & 8 != 0 {
            factor = factor * 4 / 5;
        }
        if human {
            factor = factor.max(11 - n);
        }
        let cost = factor * n;
        Some(if (1..=32000).contains(&cost) {
            cost
        } else {
            32000
        })
    }

    /// Original leader-group flag, loaded from CIVILIZE2/LEADERS2.
    /// 0 permits hut scrolls/research; 1 blocks scrolls; 2 blocks research.
    pub(crate) fn tech_group_flag(&self, rules: &Rules, civ: u8, tech: usize) -> u8 {
        let group = rules
            .section("CIVILIZE2")
            .and_then(|s| s.rows.get(tech))
            .and_then(|r| r.first())
            .and_then(|v| v.parse::<usize>().ok());
        let Some(group) = group.filter(|g| *g < 8) else {
            return 0;
        };
        self.ai_state
            .leader_ids
            .get(civ as usize)
            .and_then(|&id| usize::try_from(id).ok())
            .and_then(|i| rules.section("LEADERS2").and_then(|s| s.rows.get(i)))
            .and_then(|r| r.first())
            .and_then(|v| v.chars().nth(group))
            .and_then(|c| c.to_digit(10))
            .unwrap_or(0) as u8
    }

    pub fn observe_research(&self, rules: &Rules, civ: u8) -> ResearchView {
        let Some(state) = self.research.get(civ as usize) else {
            return ResearchView {
                free_advances: 0,
                target: None,
                progress: 0,
                cost: None,
                available: Vec::new(),
            };
        };
        let tree = TechTree::from_rules(rules);
        let known: Vec<bool> = (0..tree.techs.len())
            .map(|t| {
                self.techs[civ as usize]
                    .get(t / 8)
                    .is_some_and(|b| b & (1 << (t % 8)) != 0)
            })
            .collect();
        let available = if self.research_cost(rules, civ).is_some() {
            (0..tree.techs.len().min(100))
                .filter(|&id| {
                    let t = &tree.techs[id];
                    // Keep the historical structural-tree walk separate from
                    // runtime legality: a 'no' prerequisite disables research.
                    let disabled = rules
                        .section("CIVILIZE")
                        .and_then(|s| s.rows.get(id))
                        .is_some_and(|r| {
                            r.get(3).is_some_and(|p| p == "no")
                                || r.get(4).is_some_and(|p| p == "no")
                        });
                    !disabled
                        && self.tech_group_flag(rules, civ, id) != 2
                        && !t.defective
                        && (id == 89 || !known[id])
                        && t.prereqs
                            .iter()
                            .all(|p| p.is_none_or(|k| known.get(k).copied().unwrap_or(false)))
                })
                .collect()
        } else {
            Vec::new()
        };
        ResearchView {
            free_advances: self.free_research[civ as usize],
            target: usize::try_from(state.target).ok(),
            progress: state.progress,
            cost: self.research_cost(rules, civ),
            available,
        }
    }

    /// Shared normal/future advance grant, including hut scrolls. Mirrors
    /// `0x49dedf`'s bit/count/target bookkeeping; its UI, diplomacy, first-
    /// discovery rewards and automatic follow-on effects remain separate.
    pub fn grant_advance(&mut self, civ: u8, advance: usize) -> bool {
        let c = civ as usize;
        if !(1..8).contains(&c) || advance >= 100 {
            return false;
        }
        if advance != 89 {
            let bit = 1 << (advance % 8);
            if self.techs[c][advance / 8] & bit != 0 {
                return false;
            }
            self.techs[c][advance / 8] |= bit;
        } else {
            self.research[c].future = self.research[c].future.wrapping_add(1);
        }
        if self.turn != 0 {
            self.research[c].acquired = self.research[c].acquired.wrapping_add(1);
        }
        if self.research[c].target == advance as i16 {
            self.research[c].target = -1;
        }
        true
    }

    /// Normal discovery bookkeeping followed by Rules-dependent effects
    /// (original 0x49ebad invokes Leonardo after the advance grant).
    pub fn grant_advance_with_rules(&mut self, rules: &Rules, civ: u8, advance: usize) -> bool {
        if !self.grant_advance(civ, advance) {
            return false;
        }
        self.upgrade_obsolete_units(rules, civ);
        true
    }

    /// Add positive science (`0x4a1ebf`) using the signed 16-bit saved
    /// accumulator. A completed target discards overflow and clears the
    /// target (`0x4a154b`/`0x4a1565`). The player chooses its next target
    /// during its next order phase; science received meanwhile is banked.
    pub fn add_research(&mut self, rules: &Rules, civ: u8, science: i32) -> Option<usize> {
        if science <= 0 {
            return None;
        }
        let cost = self.research_cost(rules, civ)?;
        let c = civ as usize;
        self.research[c].progress = self.research[c].progress.wrapping_add(science as i16);
        let target = usize::try_from(self.research[c].target).ok()?;
        if target >= rules.tech_defs().len().min(100) || (self.research[c].progress as i32) < cost {
            return None;
        }
        self.research[c].progress = 0;
        self.research[c].target = -1;
        self.grant_advance_with_rules(rules, civ, target)
            .then_some(target)
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

#[cfg(test)]
mod accounting_tests {
    use crate::player::{Game, Order, OrderResult};

    const RULES: &str = "@CIVILIZE\nAlphabet, 8, 1, nil, nil, 0, 3 ; Alp\nBronze, 6, 1, nil, nil, 0, 0 ; Bro\nWriting, 7, 1, Alp, nil, 0, 3 ; Wri\nDisabled, 99, 1, no, nil, 0, 0 ; Dis\n";
    fn game() -> Game {
        Game::new(
            "test.sav",
            &crate::tests_support::synthetic_save_bytes(),
            3,
            tot_kit::parse_rules(RULES),
            None,
        )
        .unwrap()
    }

    #[test]
    fn costs_match_128_cases_executed_from_original_x86() {
        let mut world = game().world;
        let vectors = include_str!("../../../docs/evidence/research-cost-vectors.txt");
        let mut count = 0;
        for line in vectors
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
        {
            let v: Vec<i32> = line
                .split_whitespace()
                .map(|x| x.parse().unwrap())
                .collect();
            assert_eq!(v.len(), 11);
            world.difficulty = v[0] as u8;
            world.human_mask = 2 * v[1] as u8;
            world.research[1].acquired = v[2] as u8;
            world.research[1].adjustment = v[3] as u8;
            world.research[2].acquired = v[4] as u8;
            world.research[2].adjustment = 0;
            world.turn = v[5] as i16;
            world.research_settings = tot_kit::ResearchSettings {
                advance_count: v[6] as i16,
                flags: v[7] as u16,
                reference_civ: 2,
                scenario: Some((v[8] as i16, v[9] as i16)),
            };
            let cosmic = (0..12)
                .map(|i| format!("{}\n", if i == 11 { v[8] } else { 10 }))
                .collect::<String>();
            let rules = tot_kit::parse_rules(&format!("@COSMIC\n{cosmic}"));
            assert_eq!(world.research_cost(&rules, 1), Some(v[10]), "{line}");
            count += 1;
        }
        assert_eq!(count, 128);
    }

    #[test]
    fn research_orders_validate_and_discovery_resets_overflow() {
        let mut g = game();
        g.world.research[1].target = -1;
        let before = g.world.checksum();
        for (civ, advance) in [(0, 0), (8, 0), (1, 2), (1, 3), (1, usize::MAX)] {
            assert_eq!(
                g.world
                    .apply_order(&g.rules, None, civ, Order::SetResearch { advance }),
                OrderResult::Rejected("cannot research")
            );
            assert_eq!(g.world.checksum(), before);
        }
        assert_eq!(
            g.world
                .apply_order(&g.rules, None, 1, Order::SetResearch { advance: 0 }),
            OrderResult::ResearchSet
        );
        let cost = g.world.research_cost(&g.rules, 1).unwrap();
        assert_eq!(g.world.add_research(&g.rules, 1, cost - 1), None);
        assert_eq!(g.world.add_research(&g.rules, 1, 10), Some(0));
        assert_eq!(g.world.research[1].progress, 0, "overflow is discarded");
        assert_eq!(g.world.research[1].target, -1);
        assert_eq!(g.world.research[1].acquired, 1);
        assert!(g.world.techs[1][0] & 1 != 0);
        let view = g.world.observe(&g.rules, 1, None);
        assert!(!view.research.available.contains(&0));
        assert!(view.research.available.contains(&2), "Writing unlocked");
        assert_eq!(g.world.add_research(&g.rules, 1, 3), None);
        assert_eq!(
            g.world.research[1].progress, 3,
            "science is banked pending a new order"
        );
    }

    #[test]
    fn grants_keep_hut_and_future_bookkeeping_consistent() {
        let mut g = game();
        g.world.research[1].target = 0;
        g.world.research[1].progress = 4;
        assert!(g.world.grant_advance(1, 0));
        assert_eq!(
            g.world.research[1].progress, 4,
            "external grant preserves progress"
        );
        assert_eq!(g.world.research[1].target, -1);
        assert!(!g.world.grant_advance(1, 0));
        assert_eq!(g.world.research[1].acquired, 1);
        assert!(g.world.grant_advance(1, 89));
        assert!(g.world.grant_advance(1, 89));
        assert_eq!(g.world.research[1].future, 2);
        assert_eq!(g.world.research[1].acquired, 3);
        assert_eq!(g.world.techs[1][89 / 8] & (1 << (89 % 8)), 0);
    }

    #[test]
    fn unknown_scenario_cost_never_uses_a_fabricated_default() {
        let mut g = game();
        g.world.research_settings.flags = 0x80;
        g.world.research_settings.scenario = None;
        let before = g.world.checksum();
        assert_eq!(g.world.research_cost(&g.rules, 1), None);
        assert!(g.world.observe_research(&g.rules, 1).available.is_empty());
        assert_eq!(g.world.add_research(&g.rules, 1, 100), None);
        assert_eq!(g.world.checksum(), before);
    }

    #[test]
    fn research_ai_logs_orders_and_replays_discoveries() {
        let save = crate::tests_support::synthetic_save_bytes();
        let rules_text = format!("{RULES}@TERRAIN\nDesert, 1, 1, 0, 0, 1\nPlains, 1, 1, 1, 1, 1\nGrassland, 1, 1, 2, 1, 1\n");
        // Seed the fixture with an actual research-producing civ, so this
        // exercises discoveries through Game, not direct world edits that
        // would disappear from canonical replay.
        let mut save = save;
        let fixture = tot_kit::parse_sav(&save).unwrap();
        for city in 0..fixture.cities.len() {
            let b = fixture.city_offset + city * 92;
            save[b + 24..b + 28].copy_from_slice(&3u32.to_le_bytes());
            save[b + 82] = 2;
        }
        for civ in 1..8 {
            let b = tot_kit::OFF_CIV_PROPS + 3348 * civ;
            save[b + 18..b + 20].copy_from_slice(&(-1i16).to_le_bytes());
            save[b + 27] = 10;
            save[b + 28] = 0;
            save[b + 29] = 1;
        }
        let mut g = Game::new(
            "test.sav",
            &save,
            3,
            tot_kit::parse_rules(&rules_text),
            None,
        )
        .unwrap();
        let start = g.world.techs;
        for civ in 1..8 {
            g.players[civ] = Some(Box::new(crate::ai::AiV1::new(&g.rules, 3, civ as u8)));
        }
        for _ in 0..50 {
            g.play_turn();
        }
        assert!(g.log().contains(" research "));
        assert!(g.log().contains("= research-set"));
        assert_ne!(g.world.techs, start, "city science caused discoveries");
        let replay = Game::replay(
            "test.sav",
            &save,
            tot_kit::parse_rules(&rules_text),
            None,
            g.log(),
        )
        .unwrap();
        assert_eq!(replay.world.checksum(), g.world.checksum());
        assert_eq!(replay.world.research, g.world.research);
    }
}

#[cfg(test)]
mod tech_group_tests {
    #[test]
    fn original_hut_and_research_group_vectors() {
        let mut count = 0;
        for line in include_str!("../../../docs/evidence/tech-group-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let v: Vec<&str> = line.split_whitespace().collect();
            let rules = tot_kit::parse_rules(&format!("@CIVILIZE\nAdvance,5,1,nil,nil,0,0 ; Adv\n@CIVILIZE2\n{}\n@LEADERS2\n00000000\n00000000\n{}\n", v[0], v[1]));
            let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
            w.ai_state.leader_ids[1] = 2;
            w.techs = [[0; 13]; 8];
            assert_eq!(w.tech_group_flag(&rules, 1, 0) == 0, v[2] == "1", "{line}");
            assert_eq!(
                w.observe_research(&rules, 1).available.contains(&0),
                v[3] == "1",
                "{line}"
            );
            count += 1;
        }
        assert_eq!(count, 128);
    }
}
