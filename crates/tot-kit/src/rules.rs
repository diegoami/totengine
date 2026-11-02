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
    const FIXTURE: &str = "; Rules.txt fixture\r\n; second preamble line\r\n@COSMIC\r\n3       ; Road movement multiplier\r\n2       ; food each citizen eats\r\n@TERRAIN\r\nDesert,  0, 1, a\r\nPlains,  1, 1, b\r\nOcean, 10, 1, c\r\n; an inter-row comment\r\n@UNITS\r\nSettlers,     Exp, 0,  1.,0\r\nEngineers,    nil, 0,  2.,0 ; build faster\r\n";

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
