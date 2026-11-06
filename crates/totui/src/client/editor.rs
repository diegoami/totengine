//! Native single-line UTF-8 editing. Field tokens bind edits to their dialog.
use super::Entry;
use std::ops::Range;
#[derive(Default)]
pub struct Editor {
    pub field: String,
    pub value: String,
    pub cursor: usize,
    anchor: Option<usize>,
}
pub enum Key {
    Left,
    Right,
    Home,
    End,
    Backspace,
    Delete,
    All,
}
impl Editor {
    pub fn sync(&mut self, entry: &Entry) {
        if self.field != entry.id || self.value != entry.value {
            self.field = entry.id.clone();
            self.value = entry.value.clone();
            self.cursor = self.value.len();
            self.anchor = None;
        }
    }
    pub fn range(&self) -> Range<usize> {
        let a = self.anchor.unwrap_or(self.cursor);
        a.min(self.cursor)..a.max(self.cursor)
    }
    pub fn selected(&self) -> String {
        self.value[self.range()].into()
    }
    pub fn insert(&mut self, text: &str) {
        let r = self.range();
        self.cursor = r.start + text.len();
        self.value.replace_range(r, text);
        self.anchor = None;
    }
    pub fn key(&mut self, key: Key, shift: bool) -> bool {
        let old = self.value.clone();
        let prev = self.value[..self.cursor]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i);
        let next = self.cursor
            + self.value[self.cursor..]
                .chars()
                .next()
                .map_or(0, char::len_utf8);
        match key {
            Key::All => {
                self.anchor = Some(0);
                self.cursor = self.value.len();
            }
            Key::Backspace | Key::Delete => {
                let mut r = self.range();
                if r.is_empty() {
                    r = if matches!(key, Key::Backspace) {
                        prev..self.cursor
                    } else {
                        self.cursor..next
                    };
                }
                self.cursor = r.start;
                self.value.replace_range(r, "");
                self.anchor = None;
            }
            _ => {
                let r = self.range();
                if shift {
                    self.anchor.get_or_insert(self.cursor);
                } else {
                    self.anchor = None;
                }
                self.cursor = match key {
                    Key::Home => 0,
                    Key::End => self.value.len(),
                    Key::Left => {
                        if !shift && !r.is_empty() {
                            r.start
                        } else {
                            prev
                        }
                    }
                    Key::Right => {
                        if !shift && !r.is_empty() {
                            r.end
                        } else {
                            next
                        }
                    }
                    _ => unreachable!(),
                };
            }
        }
        old != self.value
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utf8_selection_paste_and_deletion_preserve_boundaries() {
        let mut e = Editor::default();
        let entry = Entry {
            id: "name-1".into(),
            value: "Aé中".into(),
            x: 0.,
            y: 0.,
            w: 10.,
            h: 10.,
        };
        e.sync(&entry);
        e.key(Key::Left, false);
        e.key(Key::Backspace, false);
        assert_eq!(e.value, "A中");
        e.key(Key::Home, false);
        e.key(Key::Right, true);
        assert_eq!(e.selected(), "A");
        e.insert("Ω");
        assert_eq!(e.value, "Ω中");
        e.key(Key::All, false);
        e.insert("new");
        assert_eq!(e.value, "new");
        e.key(Key::Home, false);
        e.key(Key::Delete, false);
        assert_eq!(e.value, "ew");
        e.key(Key::End, false);
        e.insert("Ω");
        assert_eq!(e.value, "ewΩ");
        e.sync(&Entry {
            id: "name-2".into(),
            value: String::new(),
            ..entry
        });
        assert!(e.value.is_empty());
        assert_eq!(e.cursor, 0);
    }
}
