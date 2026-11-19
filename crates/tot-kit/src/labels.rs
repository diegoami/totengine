//! `Labels.txt` — the game's interface strings.
//!
//! The `@LABELS` section starts with a count line, then one label per line;
//! the first label is index 0. The original's date formatter (`0x46dca5`,
//! evidence README §Scenario dates) reads index 0 (before-era suffix, "B.C."),
//! 1 (era prefix, "A.D.") and 420..=431 (month names). Scenario folders ship
//! their own copy with renamed eras and months (Fantasy, Sci-fi, Constantine).

/// Index of the first month label (`0x1a4` in the original's formatter).
pub const MONTH_LABEL: usize = 420;

/// The `@LABELS` strings of one `Labels.txt`, in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Labels {
    pub labels: Vec<String>,
}

impl Labels {
    /// Parse the `@LABELS` section. Lines keep their text; the line break
    /// and trailing blanks are dropped (some files pad labels with a space).
    /// `None` without a `@LABELS` section or count line.
    pub fn parse(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        lines.find(|l| l.trim_end().eq_ignore_ascii_case("@LABELS"))?;
        let count: usize = lines.next()?.split(';').next()?.trim().parse().ok()?;
        let labels = lines
            .take(count)
            .take_while(|l| !l.starts_with('@'))
            .map(|l| l.trim_end().to_string())
            .collect();
        Some(Self { labels })
    }

    /// Label `index`, if the file has it.
    pub fn get(&self, index: usize) -> Option<&str> {
        self.labels.get(index).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_count_then_labels() {
        let l =
            Labels::parse("@POPUPS\nOK\n\n@LABELS\n3\nB.C.\nSecond Age \r\nX\nextra\n").unwrap();
        assert_eq!(l.labels, ["B.C.", "Second Age", "X"]);
        assert_eq!(l.get(1), Some("Second Age"));
        assert_eq!(l.get(3), None);
        assert!(Labels::parse("@POPUPS\nOK\n").is_none());
    }

    #[test]
    fn stops_at_the_next_section() {
        let l = Labels::parse("@LABELS\n5\nA\nB\n@OTHER\nC\n").unwrap();
        assert_eq!(l.labels, ["A", "B"]);
    }
}
