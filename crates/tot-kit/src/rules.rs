//! Rules.txt parsing — typed model (T1-1).
//!
//! Parser semantics mirror the proven Python probe (`tools/totctl.py`,
//! issue #44's lesson baked in):
//!
//! - every non-comment line after `@SECTION` is **data** — there are no
//!   header rows (the first data row used to be eaten as a "header",
//!   shifting every terrain name/color by one);
//! - `;` starts a comment anywhere; in `@COSMIC` the comment text is the
//!   key (`3  ; Road movement multiplier` → key "Road movement multiplier",
//!   value "3");
//! - all lines (preamble, comments, blank lines) are retained in order so
//!   text → structs → text round-trips losslessly.
//!
//! Synthetic fixtures only in tests — no game data in the repo. The
//! env-gated test against the real install lives in `tests/rules_install.rs`
//! (skips without `CIVTOT_HOME`).

use std::collections::BTreeMap;

/// One `@SECTION`: parsed rows plus everything needed to re-emit the file
/// byte-identically.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// Section name after `@` (e.g. "TERRAIN").
    pub name: String,
    /// Lines of this section in original order (comments and blanks
    /// included, `@` header line excluded), EOLs stripped.
    pub raw_lines: Vec<String>,
    /// Data rows, one per non-comment non-blank line: comma-split fields,
    /// inline comments stripped. The FIRST data row is included (no
    /// header-eating — issue #44).
    pub rows: Vec<Vec<String>>,
    /// Inline comment text per data row (parallel to `rows`; empty string
    /// when absent). `@COSMIC` uses these as keys.
    pub comments: Vec<String>,
}

/// A parsed Rules.txt.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rules {
    /// Lines before the first `@SECTION` (comments/title), in order.
    pub preamble: Vec<String>,
    /// Section names in original file order (emit follows this).
    pub order: Vec<String>,
    pub sections: BTreeMap<String, Section>,
}

/// One scientific advance (`@CIVILIZE` row, T1-5). Prerequisites are the
/// three-letter abbreviations exactly as written in the file; resolve
/// them via [`Rules::tech_defs`] (abbrev → index).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TechDef {
    pub name: String,
    /// Three-letter abbreviation (row comment, e.g. "Alp").
    pub abbrev: Option<String>,
    /// `nil` means none — each slot is an abbreviation or None.
    pub prereqs: [Option<String>; 2],
    /// AI value (field 1).
    pub ai_value: Option<i32>,
    /// Civilized modifier (field 2).
    pub modifier: Option<i32>,
    /// 0 Ancient, 1 Renaissance, 2 Industrial, 3 Modern (field 5).
    pub epoch: Option<u8>,
    /// 0 Military, 1 Economic, 2 Social, 3 Academic, 4 Applied (field 6).
    pub category: Option<u8>,
}

/// One wonder of the world (T1-5): name, `@IMPROVE` cost/prerequisite,
/// `@ENDWONDER` obsolescence (None = never expires).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WonderDef {
    pub name: String,
    pub cost: Option<u32>,
    pub prereq: Option<String>,
    pub obsolete_by: Option<String>,
}

impl Rules {
    pub fn section(&self, name: &str) -> Option<&Section> {
        self.sections.get(name)
    }

    /// `@TERRAIN` base-terrain names, ids 0..=10 (Desert..Ocean). The
    /// semantic invariant the golden suite enforces on the Python side.
    pub fn base_terrain_names(&self) -> Vec<&str> {
        self.section("TERRAIN")
            .map(|s| s.rows.iter().take(11).map(|r| r[0].as_str()).collect())
            .unwrap_or_default()
    }

    /// True iff terrain id 0 is Desert and id 10 is Ocean (case-insensitive)
    /// — the #44 regression check.
    pub fn terrain_semantic_ok(&self) -> bool {
        let names = self.base_terrain_names();
        names.len() == 11
            && names
                .first()
                .is_some_and(|n| n.eq_ignore_ascii_case("Desert"))
            && names[10].eq_ignore_ascii_case("Ocean")
    }

    /// `@COSMIC` as key/value pairs — the key is the inline comment text.
    pub fn cosmic(&self) -> Vec<(&str, &str)> {
        match self.section("COSMIC") {
            Some(s) => s
                .comments
                .iter()
                .zip(s.rows.iter())
                .filter(|(_, r)| !r.is_empty() && !r[0].is_empty())
                .map(|(c, r)| {
                    let key = if c.is_empty() {
                        r[0].as_str()
                    } else {
                        c.as_str()
                    };
                    (key, r[0].as_str())
                })
                .collect(),
            None => Vec::new(),
        }
    }

    // ---- typed views over sections (named accessors for fields whose
    // semantics are PROVEN; everything else stays raw) ----

    /// `@UNITS` names in id order (index = Rules @UNITS row).
    pub fn unit_names(&self) -> Vec<&str> {
        self.section("UNITS")
            .map(|s| s.rows.iter().map(|r| r[0].as_str()).collect())
            .unwrap_or_default()
    }

    /// Unit domain (row field 2): 0 = land, 1 = air, 2 = sea. Proven on
    /// FallRome's own table (issue #30: exactly the 32 domain-2 units sit
    /// on ocean). NOTE: scenarios override @UNITS — parse the scenario's
    /// own Rules.txt, not the global one.
    pub fn unit_domain(&self, id: usize) -> Option<u8> {
        self.section("UNITS")
            .and_then(|s| s.rows.get(id))
            .and_then(|r| r.get(2))
            .and_then(|f| f.trim().parse().ok())
    }

    /// Unit movement rate (row field 3, written `N.` — trailing dot
    /// stripped). Column identified 2026-11-02 by cross-checking classic
    /// Civ2 values across ~25 units in the global Rules.txt: Trireme 3,
    /// Caravel 3, Galleon 4, Destroyer 6, Transport 5, Cruise Missile 12,
    /// Nuclear Missile 16, Diplomat 2, Spy 3, Caravan 1 — all match;
    /// Alpine Troops 1 (move 1 + ignore-terrain ability, classic).
    pub fn unit_move(&self, id: usize) -> Option<u32> {
        self.section("UNITS")
            .and_then(|s| s.rows.get(id))
            .and_then(|r| r.get(3))
            .and_then(|f| f.trim().trim_end_matches('.').parse().ok())
    }

    /// Unit shield cost / 10 (row field 9): classic values across the
    /// global table — Trireme 40, Ironclad 60, Battleship 160, Carrier
    /// 160, Settlers 40, Knights 40, Spy 30, Battleship 160.
    pub fn unit_cost(&self, id: usize) -> Option<u32> {
        self.unit_num(id, 9)
    }

    /// Unit transport capacity (`hold`, row field 10): Trireme 2,
    /// Caravel 3, Galleon 4, Transport 8, all others 0 — classic values.
    pub fn unit_hold(&self, id: usize) -> Option<u32> {
        self.unit_num(id, 10)
    }

    /// Unit attack (row field 5, `Na` suffix).
    pub fn unit_attack(&self, id: usize) -> Option<u32> {
        self.unit_num_strip(id, 5, &['a', 'A'])
    }

    /// Unit defense (row field 6, `Nd` suffix).
    pub fn unit_defense(&self, id: usize) -> Option<u32> {
        self.unit_num_strip(id, 6, &['d', 'D'])
    }

    /// Unit hit points (row field 7, `Nh` suffix).
    pub fn unit_hits(&self, id: usize) -> Option<u32> {
        self.unit_num_strip(id, 7, &['h', 'H'])
    }

    /// Unit firepower (row field 8, `Nf` suffix).
    pub fn unit_firepower(&self, id: usize) -> Option<u32> {
        self.unit_num_strip(id, 8, &['f', 'F'])
    }

    /// Unit flags (row field 13): the 15-character 0/1 string read RIGHT
    /// to left, bit 0 = last character. The game's type record holds it as
    /// a u32 at +4 (read live: B. Horsemen `000000000000010` → 2). Bits
    /// used by combat (evidence §T1-6 combat): 4 can attack air, 6
    /// negates city walls, 10 ×1.5 vs horse (pikemen), 12 one-shot.
    pub fn unit_flags(&self, id: usize) -> Option<u32> {
        let f = self
            .section("UNITS")
            .and_then(|s| s.rows.get(id))
            .and_then(|r| r.get(13))?;
        let f = f.trim();
        if f.is_empty() || !f.bytes().all(|b| b == b'0' || b == b'1') {
            return None;
        }
        Some(
            f.bytes()
                .rev()
                .enumerate()
                .fold(0u32, |acc, (i, b)| acc | (((b == b'1') as u32) << i)),
        )
    }

    fn unit_num(&self, id: usize, field: usize) -> Option<u32> {
        self.section("UNITS")
            .and_then(|s| s.rows.get(id))
            .and_then(|r| r.get(field))
            .and_then(|f| f.trim().parse().ok())
    }

    /// `@COSMIC` row 3 (0-based 2): food each citizen eats per turn.
    /// Verified live: FallRome 3 (Roma 45 F = 15×3 → surplus 0 = the
    /// city screen's Net Food 0), standard 2 (every standard-game city's
    /// box ticks F − 2×size).
    pub fn cosmic_food_per_citizen(&self) -> u32 {
        self.cosmic_row(2).unwrap_or(2)
    }

    /// `@COSMIC` row 4 (0-based 3): food-box rows. The comment
    /// "rows * city_size+1 = box" is ambiguous and does NOT match the
    /// measured capacities (20 at size 1, 27 at size 2 — see engine
    /// `CityRules::growth_cap`); carried because the row exists.
    pub fn cosmic_food_box_rows(&self) -> u32 {
        self.cosmic_row(3).unwrap_or(10)
    }

    /// `@COSMIC` rows 6/7: food eaten by a supported settler under
    /// pre-Monarchy / post-Communism governments.
    pub fn cosmic_settler_food(&self, post_communism: bool) -> u32 {
        self.cosmic_row(if post_communism { 6 } else { 5 })
            .unwrap_or(2)
    }

    /// Cost of improvement or wonder `id` (`@IMPROVE` row `id`, field 1),
    /// in shield-box rows: the original multiplies it by the civ's rows
    /// (RAM table `0x5f6684`, 8-B rows; evidence §T1-4 production).
    pub fn improvement_cost(&self, id: usize) -> Option<u32> {
        self.section("IMPROVE")
            .and_then(|s| s.rows.get(id))
            .and_then(|r| r.get(1))
            .and_then(|f| f.trim().parse().ok())
    }

    /// `@COSMIC` row `idx` (0-based), the value field. The original keeps
    /// these as bytes from RAM `0x5f5eb8` (row 0 = road multiplier, 3 =
    /// food box rows, 4 = shield box rows, 13–15 = free units under
    /// monarchy / communism / fundamentalism, 16 = communism's palace
    /// distance; evidence §T1-4 city totals).
    pub fn cosmic_row(&self, idx: usize) -> Option<u32> {
        self.section("COSMIC")
            .and_then(|s| s.rows.get(idx))
            .and_then(|r| r.first())
            .and_then(|f| f.trim().parse().ok())
    }

    fn unit_num_strip(&self, id: usize, field: usize, suffixes: &[char]) -> Option<u32> {
        self.section("UNITS")
            .and_then(|s| s.rows.get(id))
            .and_then(|r| r.get(field))
            .and_then(|f| f.trim().trim_end_matches(suffixes).parse().ok())
    }

    /// Terrain movement cost (row field 1). Verified against the global
    /// Rules.txt: Desert/Plains/Grassland 1, Forest/Hills/Tundra/Swamp/
    /// Jungle/Glacier 2, Mountains 3, Ocean 1 — the classic Civ2 costs.
    /// (FallRome raises Mountains to 4 — scenario tuning; always read the
    /// scenario's own Rules.txt.)
    pub fn terrain_move_cost(&self, id: usize) -> Option<u32> {
        self.section("TERRAIN")
            .and_then(|s| s.rows.get(id))
            .and_then(|r| r.get(1))
            .and_then(|f| f.trim().parse().ok())
    }

    /// Terrain defense bonus (row field 2; classic relative values:
    /// Desert/Plains/Grassland/Tundra/Glacier/Ocean 2, Forest/Swamp/
    /// Jungle 3, Hills 4, Mountains 6).
    pub fn terrain_defense(&self, id: usize) -> Option<u32> {
        self.section("TERRAIN")
            .and_then(|s| s.rows.get(id))
            .and_then(|r| r.get(2))
            .and_then(|f| f.trim().parse().ok())
    }

    /// Food/shields/trade (`kind` 0/1/2) of @TERRAIN row `row` (fields
    /// 3/4/5). Rows 0–10 are the base terrains, 11–21 their first special
    /// resource and 22–32 their second: the game indexes the table as
    /// `special * 11 + terrain` (`0x4c22a7`, evidence §T1-4 tile yields).
    pub fn terrain_yield(&self, row: usize, kind: usize) -> Option<i32> {
        self.section("TERRAIN")
            .and_then(|s| s.rows.get(row))
            .and_then(|r| r.get(3 + kind.min(2)))
            .and_then(|f| f.trim().parse().ok())
    }

    /// Irrigation food bonus of base terrain `id` (field 7). Read as written:
    /// the replay matches the city records without zeroing it for "no"
    /// terrains (evidence §T1-4 tile yields).
    pub fn irrigation_bonus(&self, id: usize) -> Option<i32> {
        self.section("TERRAIN")
            .and_then(|s| s.rows.get(id))
            .and_then(|r| r.get(7))
            .and_then(|f| f.trim().parse().ok())
    }

    /// Mining shield bonus of base terrain `id` (field 11).
    pub fn mine_bonus(&self, id: usize) -> Option<i32> {
        self.section("TERRAIN")
            .and_then(|s| s.rows.get(id))
            .and_then(|r| r.get(11))
            .and_then(|f| f.trim().parse().ok())
    }

    /// `@COSMIC` road movement multiplier (comment-keyed row; the global
    /// value is 3 — three road tiles per movement point).
    pub fn road_multiplier(&self) -> Option<u32> {
        self.cosmic()
            .into_iter()
            .find(|(k, _)| k.starts_with("Road movement multiplier"))
            .and_then(|(_, v)| v.trim().parse().ok())
    }

    /// Unit AI role (row field 11; 0=attack, 1=defend, 2=naval, 3=air,
    /// 4=transport, 5=settle — roles ≥5 change abilities). Documented in
    /// the @UNITS comment block in Rules.txt itself
    /// (`until, domain, move, rng, att, def, hit, firepwr, cost, hold,
    /// role, preq, flags`).
    pub fn unit_role(&self, id: usize) -> Option<u8> {
        self.unit_num(id, 11).map(|v| v as u8)
    }

    /// Unit air range/fuel (row field 4) — 0 for non-air units.
    pub fn unit_range(&self, id: usize) -> Option<u32> {
        self.unit_num(id, 4)
    }

    /// Unit prerequisite field (row field 12) as written: `nil` (none),
    /// `no` (never buildable) or an advance abbreviation.
    pub fn unit_prereq_raw(&self, id: usize) -> Option<&str> {
        self.section("UNITS")
            .and_then(|s| s.rows.get(id))
            .and_then(|r| r.get(12))
            .map(|f| f.trim())
    }

    /// Unit "until" field (row field 1) as written: the advance that makes
    /// the unit obsolete, or `nil`.
    pub fn unit_until_raw(&self, id: usize) -> Option<&str> {
        self.section("UNITS")
            .and_then(|s| s.rows.get(id))
            .and_then(|r| r.get(1))
            .map(|f| f.trim())
    }

    /// Improvement prerequisite field (`@IMPROVE` row `id`, field 3) as
    /// written: `nil`, `no` or an advance abbreviation.
    pub fn improvement_prereq_raw(&self, id: usize) -> Option<&str> {
        self.section("IMPROVE")
            .and_then(|s| s.rows.get(id))
            .and_then(|r| r.get(3))
            .map(|f| f.trim())
    }

    /// Unit prerequisite advance abbreviation (row field 12), if any.
    pub fn unit_prereq_abbrev(&self, id: usize) -> Option<&str> {
        self.section("UNITS")
            .and_then(|s| s.rows.get(id))
            .and_then(|r| r.get(12))
            .map(|f| f.trim())
            .filter(|f| !f.is_empty() && *f != "nil" && *f != "no")
    }

    /// Scientific advances: one [`TechDef`] per `@CIVILIZE` row. Column
    /// layout is documented in the file itself above `@CIVILIZE`
    /// (`Ai Value, Civilized Modifier, Preq1, Preq2, Epoch, Category`);
    /// the trailing comment is the three-letter abbreviation used by
    /// prerequisite references (`Adp` rows, `@ENDWONDER`, unit `preq`).
    /// Epoch: 0 Ancient, 1 Renaissance, 2 Industrial, 3 Modern.
    /// Category: 0 Military, 1 Economic, 2 Social, 3 Academic, 4 Applied.
    pub fn tech_defs(&self) -> Vec<TechDef> {
        let Some(s) = self.section("CIVILIZE") else {
            return Vec::new();
        };
        s.rows
            .iter()
            .enumerate()
            .filter(|(_, r)| !r.is_empty() && !r[0].trim().is_empty())
            .map(|(i, r)| TechDef {
                name: r[0].trim().to_string(),
                abbrev: s
                    .comments
                    .get(i)
                    .map(|c| c.trim().to_string())
                    .filter(|c| !c.is_empty()),
                ai_value: r.get(1).and_then(|f| f.trim().parse().ok()),
                modifier: r.get(2).and_then(|f| f.trim().parse().ok()),
                prereqs: [
                    r.get(3)
                        .map(|f| f.trim().to_string())
                        .filter(|f| f != "nil" && f != "no"),
                    r.get(4)
                        .map(|f| f.trim().to_string())
                        .filter(|f| f != "nil" && f != "no"),
                ],
                epoch: r.get(5).and_then(|f| f.trim().parse().ok()),
                category: r.get(6).and_then(|f| f.trim().parse().ok()),
            })
            .collect()
    }

    /// Wonders of the world: names come from the `@ENDWONDER` comments
    /// (Pyramids … Cure for Cancer — 28 in the base rules; the global
    /// files carry a duplicated "Women's Suffrage" row, deduplicated
    /// here). Cost/prerequisite are read from the wonder's `@IMPROVE`
    /// row by name; obsolescence from its `@ENDWONDER` row (field 0,
    /// `nil` = never expires).
    pub fn wonder_defs(&self) -> Vec<WonderDef> {
        let Some(end) = self.section("ENDWONDER") else {
            return Vec::new();
        };
        let improve = self.section("IMPROVE");
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for (i, row) in end.rows.iter().enumerate() {
            let Some(name) = end.comments.get(i).map(|c| c.trim().to_string()) else {
                continue;
            };
            if name.is_empty() || !seen.insert(name.clone()) {
                continue;
            }
            let obsolete_by = row
                .first()
                .map(|f| f.trim().to_string())
                .filter(|f| f != "nil" && f != "no");
            // @ENDWONDER labels can be shorter than the @IMPROVE names
            // ("Apollo" vs "Apollo Program", "SETI" vs "SETI Program"):
            // match exact first, then a UNIQUE prefix; the canonical name
            // is the @IMPROVE one (the 28 @IMPROVE wonder rows are what
            // the save's 28 slots align with).
            let rows = improve.map(|s| s.rows.as_slice()).unwrap_or_default();
            let imp = rows
                .iter()
                .find(|r| r.first().map(|f| f.trim()) == Some(name.as_str()))
                .or_else(|| {
                    let hits: Vec<&Vec<String>> = rows
                        .iter()
                        .filter(|r| {
                            r.first()
                                .map(|f| f.trim().starts_with(name.as_str()))
                                .unwrap_or(false)
                        })
                        .collect();
                    (hits.len() == 1).then(|| hits[0])
                });
            let canonical = imp
                .and_then(|r| r.first())
                .map(|f| f.trim().to_string())
                .unwrap_or(name);
            out.push(WonderDef {
                name: canonical,
                cost: imp
                    .and_then(|r| r.get(1))
                    .and_then(|f| f.trim().parse().ok()),
                prereq: imp
                    .and_then(|r| r.get(3))
                    .map(|f| f.trim().to_string())
                    .filter(|f| f != "nil" && f != "no"),
                obsolete_by,
            });
        }
        out
    }

    /// `@GOVERNMENTS` names (row field 0; Anarchy first — the #44 lesson).
    pub fn government_names(&self) -> Vec<&str> {
        self.section("GOVERNMENTS")
            .map(|s| s.rows.iter().map(|r| r[0].as_str()).collect())
            .unwrap_or_default()
    }

    /// `@DIFFICULTY` levels, easiest first (Chieftain first).
    pub fn difficulties(&self) -> Vec<&str> {
        self.section("DIFFICULTY")
            .map(|s| s.rows.iter().map(|r| r[0].as_str()).collect())
            .unwrap_or_default()
    }

    /// `@ATTITUDES` (diplomacy stance names, best first).
    pub fn attitudes(&self) -> Vec<&str> {
        self.section("ATTITUDES")
            .map(|s| s.rows.iter().map(|r| r[0].as_str()).collect())
            .unwrap_or_default()
    }

    /// `@CARAVAN` commodity names (field 0).
    pub fn caravan_commodities(&self) -> Vec<&str> {
        self.section("CARAVAN")
            .map(|s| s.rows.iter().map(|r| r[0].as_str()).collect())
            .unwrap_or_default()
    }

    /// `@MAP_TRANSPORT_RELATIONSHIPS` pairs (map A, map B) — ToT
    /// multi-map transport links.
    pub fn transport_relationships(&self) -> Vec<(u8, u8)> {
        self.section("MAP_TRANSPORT_RELATIONSHIPS")
            .map(|s| {
                s.rows
                    .iter()
                    .filter_map(|r| {
                        Some((
                            r.first()?.trim().parse().ok()?,
                            r.get(1)?.trim().parse().ok()?,
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Section names in original file order.
    pub fn section_order(&self) -> &[String] {
        &self.order
    }

    /// Row count per section name, in file order.
    pub fn row_counts(&self) -> Vec<(&str, usize)> {
        self.order
            .iter()
            .map(|n| (n.as_str(), self.sections[n].rows.len()))
            .collect()
    }
}

/// Parse Rules.txt text (CRLF or LF tolerant).
pub fn parse_rules(text: &str) -> Rules {
    let mut rules = Rules::default();
    let mut current: Option<String> = None;
    for raw in text.lines() {
        let line = raw.trim_end_matches('\r');
        let trimmed = line.trim();
        if trimmed.starts_with('@') && trimmed.len() > 1 {
            let name = trimmed[1..].trim().to_string();
            rules.sections.entry(name.clone()).or_insert(Section {
                name: name.clone(),
                raw_lines: Vec::new(),
                rows: Vec::new(),
                comments: Vec::new(),
            });
            if !rules.order.contains(&name) {
                rules.order.push(name.clone());
            }
            current = Some(name);
            continue;
        }
        match &current {
            Some(sec) => rules
                .sections
                .get_mut(sec)
                .expect("section exists")
                .raw_lines
                .push(line.to_string()),
            None => rules.preamble.push(line.to_string()),
        }
        if trimmed.is_empty() || trimmed.starts_with(';') {
            continue;
        }
        let Some(sec_name) = current.clone() else {
            continue;
        };
        let (data, comment) = match line.split_once(';') {
            Some((d, c)) => (d, c.trim().to_string()),
            None => (line, String::new()),
        };
        if data.trim().is_empty() {
            continue;
        }
        let sec = rules.sections.get_mut(&sec_name).expect("section exists");
        sec.rows.push(
            data.split(',')
                .map(|f| f.trim().to_string())
                .collect::<Vec<_>>(),
        );
        sec.comments.push(comment);
    }
    rules
}

/// Re-emit the file from the parsed model — lossless by construction
/// (all lines retained in order); proves the parser consumed everything.
pub fn emit_rules(rules: &Rules) -> String {
    let mut out = String::new();
    for line in &rules.preamble {
        out.push_str(line);
        out.push('\n');
    }
    for name in &rules.order {
        let sec = &rules.sections[name];
        out.push_str(&format!("@{}\n", sec.name));
        for line in &sec.raw_lines {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic fixture mirroring the real structure (no game data).
    const FIXTURE: &str = "; Rules.txt fixture\r\n; second preamble line\r\n@COSMIC\r\n3       ; Road movement multiplier\r\n2       ; food each citizen eats\r\n@TERRAIN\r\nDesert,  0, 1, a\r\nPlains,  1, 1, b\r\nOcean, 10, 1, c\r\n; an inter-row comment\r\n@UNITS\r\nSettlers,     Exp, 0,  1.,0\r\nEngineers,    nil, 0,  2.,0 ; build faster\r\nTrireme,      nil, 2,  1.,0\r\n@GOVERNMENTS\r\nAnarchy,        Mr.,         Ms.\r\n@MAP_TRANSPORT_RELATIONSHIPS\r\n1,2\r\n2,3\r\n";

    #[test]
    fn unit_flags_read_right_to_left() {
        // Rows shaped like FallRome's B. Horsemen / Scan. Warship.
        let r = parse_rules(
            "@UNITS\r\nB. Horsemen, nil, 0, 3.,0, 5a,3d, 1h,1f, 6,0, 0, AFl, 000000000000010\r\nWarship, nil, 2, 7.,0, 4a,3d, 2h,1f, 7,4, 4, Amp, 100000000000001\r\nBroken, nil, 0, 1.,0, 1a,1d, 1h,1f, 1,0, 0, no, 01x\r\n",
        );
        assert_eq!(r.unit_flags(0), Some(0x2)); // live type record +4 = 2
        assert_eq!(r.unit_flags(1), Some(0x4001));
        assert_eq!(r.unit_flags(2), None);
        assert_eq!(r.unit_attack(0), Some(5));
        assert_eq!(r.unit_defense(0), Some(3));
    }

    #[test]
    fn first_data_row_is_not_eaten() {
        let r = parse_rules(FIXTURE);
        let terr = r.section("TERRAIN").unwrap();
        assert_eq!(terr.rows.len(), 3);
        assert_eq!(terr.rows[0][0], "Desert", "first data row is data (#44)");
        let units = r.section("UNITS").unwrap();
        assert_eq!(units.rows[0][0], "Settlers");
        assert_eq!(units.rows[1][0], "Engineers");
        assert_eq!(units.comments[1], "build faster");
        assert_eq!(r.preamble.len(), 2);
    }

    #[test]
    fn typed_views_read_proven_fields() {
        let r = parse_rules(FIXTURE);
        assert_eq!(r.unit_names(), vec!["Settlers", "Engineers", "Trireme"]);
        assert_eq!(r.unit_domain(0), Some(0), "Settlers land");
        assert_eq!(r.unit_domain(2), Some(2), "Trireme sea");
        assert_eq!(r.government_names(), vec!["Anarchy"]);
        assert_eq!(r.transport_relationships(), vec![(1u8, 2u8), (2u8, 3u8)]);
        assert_eq!(
            r.section_order(),
            vec![
                "COSMIC",
                "TERRAIN",
                "UNITS",
                "GOVERNMENTS",
                "MAP_TRANSPORT_RELATIONSHIPS"
            ]
        );
        assert_eq!(
            r.row_counts(),
            vec![
                ("COSMIC", 2),
                ("TERRAIN", 3),
                ("UNITS", 3),
                ("GOVERNMENTS", 1),
                ("MAP_TRANSPORT_RELATIONSHIPS", 2)
            ]
        );
    }

    #[test]
    fn cosmic_keys_come_from_comments() {
        let r = parse_rules(FIXTURE);
        assert_eq!(
            r.cosmic(),
            vec![
                ("Road movement multiplier", "3"),
                ("food each citizen eats", "2"),
            ]
        );
    }

    #[test]
    fn roundtrip_is_lossless() {
        let r = parse_rules(FIXTURE);
        let emitted = emit_rules(&r);
        let normalized: String = FIXTURE.replace("\r\n", "\n");
        assert_eq!(
            emitted, normalized,
            "text -> structs -> text must not lose bytes"
        );
    }

    #[test]
    fn semantic_check_on_fixture_shape() {
        // the fixture's TERRAIN has only 3 rows — the check must be false,
        // not panic (the real 11-row check is env-gated in tests/)
        let r = parse_rules(FIXTURE);
        assert!(!r.terrain_semantic_ok());
    }
}
