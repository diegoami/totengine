//! Civilopedia descriptions (`Describe.txt`).
//!
//! The file documents its own format: `@NAME_DESCRIPTIONS` opens a section, an
//! optional `@@NAME_INDEX` block maps each *rules entry order* to a description
//! number (`-1` = no description, `-2` terminates the list), and each `@@Title`
//! starts a description whose paragraphs are separated by blank lines. Lines
//! starting with `;` are comments; `@This must be here to terminate search!!!`
//! ends the file. Scenario folders ship their own copy, so descriptions are
//! scenario-aware simply by reading the selected profile's file.
//!
//! This decoder keeps the original text byte-for-byte (after the caller's
//! Latin-1 decoding); it never rewrites, translates or invents entries.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PediaEntry {
    pub title: String,
    pub paragraphs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PediaSection {
    /// Section name without the `_DESCRIPTIONS` suffix, e.g. `ADVANCE`.
    pub name: String,
    /// `index[rules_entry_id]` is the description number, or `-1` for none.
    pub index: Option<Vec<i32>>,
    pub entries: Vec<PediaEntry>,
}

impl PediaSection {
    /// The description for rules entry `id` (advance id, improvement id, ...).
    /// Sections without an index (concepts) are addressed by entry order.
    pub fn for_rules_id(&self, id: usize) -> Option<&PediaEntry> {
        self.entries.get(self.description_number(id)?)
    }

    /// Position in [`entries`](Self::entries) of the description for rules entry
    /// `id`, or `None` when the index says there is none (or names a missing one).
    pub fn description_number(&self, id: usize) -> Option<usize> {
        let n = match &self.index {
            Some(index) => usize::try_from(*index.get(id)?).ok()?,
            None => id,
        };
        (n < self.entries.len()).then_some(n)
    }

    /// Index values that name a description the section does not contain. Real
    /// scenario files have some; they resolve to "no description", never an error.
    pub fn unresolved_index_values(&self) -> usize {
        self.index.as_ref().map_or(0, |index| {
            index
                .iter()
                .filter(|&&v| v >= 0 && v as usize >= self.entries.len())
                .count()
        })
    }

    /// Entry titles in file order, for a browsable list.
    pub fn titles(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|e| e.title.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Pedia {
    pub sections: Vec<PediaSection>,
}

impl Pedia {
    pub fn section(&self, name: &str) -> Option<&PediaSection> {
        self.sections.iter().find(|s| s.name == name)
    }

    /// Parse a decoded `Describe.txt`. Structural problems are errors with a line
    /// number: text outside an entry, an index before a section, or a malformed
    /// or unterminated index. An index value that names a missing description is
    /// tolerated (see [`PediaSection::unresolved_index_values`]).
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut sections: Vec<PediaSection> = Vec::new();
        let mut entry: Option<PediaEntry> = None;
        let mut open_paragraph = false;
        let mut in_index = false;
        let flush = |sections: &mut Vec<PediaSection>, entry: &mut Option<PediaEntry>| {
            if let (Some(e), Some(s)) = (entry.take(), sections.last_mut()) {
                s.entries.push(e);
            }
        };
        for (n, raw) in text.lines().enumerate() {
            let line = raw.trim_end_matches('\r');
            let at = n + 1;
            if line.starts_with(';') {
                continue;
            }
            if in_index {
                let value = line.split(';').next().unwrap_or("").trim();
                if value.is_empty() {
                    if line.starts_with('@') {
                        return Err(format!("line {at}: index not terminated by -2"));
                    }
                    continue;
                }
                // The file says every number ends with a comma; shipped files omit
                // it on some lines, so a trailing comma is optional here.
                let number = value
                    .strip_suffix(',')
                    .unwrap_or(value)
                    .trim()
                    .parse::<i32>()
                    .map_err(|_| format!("line {at}: bad index number"))?;
                let index = sections
                    .last_mut()
                    .and_then(|s| s.index.as_mut())
                    .ok_or_else(|| format!("line {at}: index outside a section"))?;
                if number == -2 {
                    in_index = false;
                } else if number < -1 {
                    return Err(format!("line {at}: bad index number"));
                } else {
                    index.push(number);
                }
                continue;
            }
            if let Some(title) = line.strip_prefix("@@") {
                flush(&mut sections, &mut entry);
                if title.ends_with("_INDEX") {
                    let section = sections
                        .last_mut()
                        .ok_or_else(|| format!("line {at}: index before any section"))?;
                    if section.index.is_some() || !section.entries.is_empty() {
                        return Err(format!("line {at}: second or late index"));
                    }
                    section.index = Some(Vec::new());
                    in_index = true;
                } else {
                    if sections.is_empty() {
                        return Err(format!("line {at}: entry before any section"));
                    }
                    entry = Some(PediaEntry {
                        title: title.trim().to_string(),
                        paragraphs: Vec::new(),
                    });
                    open_paragraph = false;
                }
                continue;
            }
            if let Some(name) = line.strip_prefix('@') {
                flush(&mut sections, &mut entry);
                if name.starts_with("This must be here") {
                    break;
                }
                sections.push(PediaSection {
                    name: name
                        .trim()
                        .strip_suffix("_DESCRIPTIONS")
                        .unwrap_or(name.trim())
                        .to_string(),
                    index: None,
                    entries: Vec::new(),
                });
                continue;
            }
            if line.trim().is_empty() {
                open_paragraph = false;
                continue;
            }
            let Some(e) = entry.as_mut() else {
                return Err(format!("line {at}: text outside an entry"));
            };
            if open_paragraph {
                let last = e.paragraphs.last_mut().expect("open paragraph");
                last.push(' ');
                last.push_str(line.trim());
            } else {
                e.paragraphs.push(line.trim().to_string());
                open_paragraph = true;
            }
        }
        if in_index {
            return Err("index not terminated by -2".into());
        }
        flush(&mut sections, &mut entry);
        Ok(Pedia { sections })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "; comment\r\n\r\n@THING_DESCRIPTIONS\r\n;note\r\n@@THING_INDEX\r\n1,\t; Second\r\n0,\t; First\r\n-1,\t; None\r\n-2,\r\n\r\n@@Alpha\r\nOne line.\r\nStill one paragraph.\r\n\r\nSecond paragraph.\r\n\r\n@@Beta\r\nBeta text.\r\n\r\n@CONCEPT_DESCRIPTIONS\r\n@@Idea\r\nText.\r\n@This must be here to terminate search!!!\r\nignored\r\n";

    #[test]
    fn sections_index_entries_and_paragraphs() {
        let p = Pedia::parse(SAMPLE).unwrap();
        assert_eq!(p.sections.len(), 2);
        let thing = p.section("THING").unwrap();
        assert_eq!(thing.index.as_deref(), Some(&[1, 0, -1][..]));
        assert_eq!(thing.titles().collect::<Vec<_>>(), ["Alpha", "Beta"]);
        // rules entry 0 maps to description 1; -1 and out-of-range have none
        assert_eq!(thing.for_rules_id(0).unwrap().title, "Beta");
        assert_eq!(thing.for_rules_id(1).unwrap().title, "Alpha");
        assert!(thing.for_rules_id(2).is_none());
        assert!(thing.for_rules_id(9).is_none());
        assert_eq!(
            thing.entries[0].paragraphs,
            ["One line. Still one paragraph.", "Second paragraph."]
        );
        let idea = p.section("CONCEPT").unwrap();
        assert!(idea.index.is_none());
        assert_eq!(idea.for_rules_id(0).unwrap().title, "Idea");
        assert_eq!(Pedia::parse(&SAMPLE.replace('\r', "")).unwrap(), p);
    }

    #[test]
    fn index_values_past_the_entries_resolve_to_nothing() {
        let p = Pedia::parse("@A_DESCRIPTIONS\n@@A_INDEX\n0,\n5,\n-1,\n-2,\n@@X\nt\n").unwrap();
        let a = p.section("A").unwrap();
        assert_eq!(a.unresolved_index_values(), 1);
        assert_eq!(a.for_rules_id(0).unwrap().title, "X");
        assert!(a.for_rules_id(1).is_none() && a.for_rules_id(2).is_none());
    }

    #[test]
    fn index_commas_are_optional_and_indentation_is_ignored() {
        let p = Pedia::parse(
            "@A_DESCRIPTIONS\n@@A_INDEX\n 0,\t;x\n1\t\t;no comma\n-2,\n@@X\nt\n@@Y\nu\n",
        )
        .unwrap();
        assert_eq!(p.section("A").unwrap().index.as_deref(), Some(&[0, 1][..]));
    }

    #[test]
    fn structural_errors_are_reported_with_lines() {
        for (text, want) in [
            ("@@Orphan\ntext\n", "before any section"),
            ("@A_DESCRIPTIONS\nloose text\n", "outside an entry"),
            ("@A_DESCRIPTIONS\n@@A_INDEX\nabc,\n", "bad index"),
            ("@A_DESCRIPTIONS\n@@A_INDEX\n0,\n", "not terminated"),
            ("@A_DESCRIPTIONS\n@@A_INDEX\n-3,\n", "bad index"),
        ] {
            let err = Pedia::parse(text).unwrap_err();
            assert!(err.contains(want), "{text:?}: {err}");
        }
    }
}
