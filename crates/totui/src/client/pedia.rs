//! Civilopedia reference lookup (#201/#202), shared by both adapters.
//!
//! Text comes from the selected profile's own `Describe.txt` (scenario folders ship
//! their own), decoded by `tot_kit::Pedia`; nothing is rewritten or invented, and no
//! text is stored in the repository. The open demo has no such file, so the lookup
//! says it is unavailable there. Reading never touches game state.
use super::*;

const SECTIONS: [(&str, &str); 7] = [
    ("ADVANCE", "Advances"),
    ("IMPROVEMENT", "Improve"),
    ("WONDER", "Wonders"),
    ("UNIT", "Units"),
    ("GOVERNMENT", "Govts"),
    ("TERRAIN_AND_RESOURCE", "Terrain"),
    ("CONCEPT", "Concepts"),
];
const LIST_ROWS: usize = 12;
const READ_LINES: usize = 17;
const WRAP: usize = 86;
const TEXT_X: f64 = 212.;
const LIST_Y: f64 = 190.;
const ROW_H: f64 = 27.;

/// Greedy word wrap at `width` characters; long words are kept whole.
pub(super) fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn section_name(index: usize) -> &'static str {
    SECTIONS[index].0
}

impl Client {
    fn pedia(&self) -> Option<&tot_kit::Pedia> {
        self.loaded.as_ref()?.resources.pedia.as_ref()
    }

    fn pedia_unavailable() -> String {
        "Civilopedia text is not available: this profile has no Describe.txt".into()
    }

    /// Indices into `SECTIONS` that exist and have entries in this profile.
    fn pedia_present(&self) -> Vec<usize> {
        let Some(p) = self.pedia() else {
            return Vec::new();
        };
        (0..SECTIONS.len())
            .filter(|&i| {
                p.section(section_name(i))
                    .is_some_and(|s| !s.entries.is_empty())
            })
            .collect()
    }

    /// Enter reads/goes back unless a category tab was focused with Tab.
    fn focus_pedia_accept(&mut self) {
        self.focus_button = self.buttons().len().saturating_sub(2);
    }

    pub(super) fn open_pedia(&mut self) -> Result<(), String> {
        let present = self.pedia_present();
        let first = *present.first().ok_or_else(Self::pedia_unavailable)?;
        self.open(Modal::Pedia {
            section: first,
            selected: 0,
            reading: false,
            scroll: 0,
            back: None,
        });
        self.status =
            "Civilopedia: Left/Right choose a category, Up/Down select, Enter reads, Esc closes."
                .into();
        Ok(())
    }

    /// Open straight into the description of rules entry `id`; Esc returns to `back`.
    pub(super) fn open_pedia_for(
        &mut self,
        section: &str,
        id: usize,
        back: Option<(Modal, Option<i32>)>,
    ) -> Result<(), String> {
        let p = self.pedia().ok_or_else(Self::pedia_unavailable)?;
        let s = p
            .section(section)
            .ok_or("This profile's Civilopedia has no such category")?;
        let entry = s
            .description_number(id)
            .ok_or("The Civilopedia has no description for this entry")?;
        let index = SECTIONS
            .iter()
            .position(|(n, _)| *n == section)
            .ok_or("Unknown Civilopedia category")?;
        self.open(Modal::Pedia {
            section: index,
            selected: entry,
            reading: true,
            scroll: 0,
            back: back.map(|(m, c)| Box::new((m, c))),
        });
        self.focus_pedia_accept();
        Ok(())
    }

    /// Research prompt help: describe the highlighted advance; Esc returns to the choice.
    pub(super) fn pedia_for_research_choice(&mut self) -> Result<(), String> {
        let advance = self.choice.ok_or("Select a research topic first")?;
        let back = self.modal.clone().map(|m| (m, self.choice));
        self.open_pedia_for("ADVANCE", advance as usize, back)
    }

    fn pedia_entries(&self, section: usize) -> usize {
        self.pedia()
            .and_then(|p| p.section(section_name(section)))
            .map_or(0, |s| s.entries.len())
    }

    fn pedia_lines(&self, section: usize, entry: usize) -> Vec<String> {
        let Some(e) = self
            .pedia()
            .and_then(|p| p.section(section_name(section)))
            .and_then(|s| s.entries.get(entry))
        else {
            return Vec::new();
        };
        let mut lines = Vec::new();
        for (i, paragraph) in e.paragraphs.iter().enumerate() {
            if i > 0 {
                lines.push(String::new());
            }
            lines.extend(wrap(paragraph, WRAP));
        }
        lines
    }

    pub(super) fn pedia_section(&mut self, section: usize) {
        if let Some(Modal::Pedia { back, .. }) = &self.modal {
            let back = back.clone();
            if self.pedia_entries(section) > 0 {
                self.modal = Some(Modal::Pedia {
                    section,
                    selected: 0,
                    reading: false,
                    scroll: 0,
                    back,
                });
                self.focus_pedia_accept();
            }
        }
    }

    pub(super) fn pedia_click(&mut self, x: f64, y: f64) {
        if !(TEXT_X..TEXT_X + 600.).contains(&x) || y < LIST_Y {
            return;
        }
        let Some(Modal::Pedia {
            section,
            selected,
            reading: false,
            ..
        }) = &self.modal
        else {
            return;
        };
        let index = *selected / LIST_ROWS * LIST_ROWS + ((y - LIST_Y) / ROW_H) as usize;
        if (y - LIST_Y) / ROW_H < LIST_ROWS as f64 && index < self.pedia_entries(*section) {
            if let Some(Modal::Pedia { selected, .. }) = &mut self.modal {
                *selected = index;
            }
        }
    }

    pub(super) fn pedia_key(&mut self, key: &str) -> Option<Result<(), String>> {
        let Some(Modal::Pedia {
            section,
            selected,
            reading,
            scroll,
            ..
        }) = self.modal.clone()
        else {
            return None;
        };
        let present = self.pedia_present();
        let total = self.pedia_entries(section);
        let lines = if reading {
            self.pedia_lines(section, selected).len()
        } else {
            0
        };
        let step = |n: usize, delta: isize, max: usize| {
            (n as isize + delta).clamp(0, max.saturating_sub(1) as isize) as usize
        };
        match key {
            "ArrowUp" | "ArrowDown" | "PageUp" | "PageDown" => {
                let delta: isize = match key {
                    "ArrowUp" => -1,
                    "ArrowDown" => 1,
                    "PageUp" => -(if reading { READ_LINES } else { LIST_ROWS } as isize),
                    _ => (if reading { READ_LINES } else { LIST_ROWS }) as isize,
                };
                if let Some(Modal::Pedia {
                    selected, scroll, ..
                }) = &mut self.modal
                {
                    if reading {
                        *scroll = step(*scroll, delta, lines.saturating_sub(READ_LINES - 1).max(1));
                    } else {
                        *selected = step(*selected, delta, total);
                    }
                }
            }
            "ArrowLeft" | "ArrowRight" if !reading => {
                let at = present.iter().position(|&i| i == section).unwrap_or(0);
                let next = if key == "ArrowRight" {
                    (at + 1) % present.len().max(1)
                } else {
                    (at + present.len().max(1) - 1) % present.len().max(1)
                };
                if let Some(&target) = present.get(next) {
                    self.pedia_section(target);
                }
            }
            _ => return None,
        }
        let _ = scroll;
        Some(Ok(()))
    }

    /// Enter / Accept: read the selected entry, or go back to the list.
    pub(super) fn pedia_accept(&mut self) -> Result<(), String> {
        if let Some(Modal::Pedia {
            reading, scroll, ..
        }) = &mut self.modal
        {
            *reading = !*reading;
            *scroll = 0;
        }
        Ok(())
    }

    /// Esc: leave reading, then return to the originating dialog or close.
    pub(super) fn pedia_cancel(&mut self) -> bool {
        let Some(Modal::Pedia { reading, back, .. }) = self.modal.clone() else {
            return false;
        };
        if let Some(origin) = back {
            let (modal, choice) = *origin;
            self.open(modal);
            if let Some(c) = choice {
                self.choice = Some(c);
                self.page = self
                    .current_choices()
                    .iter()
                    .position(|(id, _)| *id == c)
                    .unwrap_or(0)
                    / 12;
            }
            return true;
        }
        if reading {
            if let Some(Modal::Pedia {
                reading, scroll, ..
            }) = &mut self.modal
            {
                *reading = false;
                *scroll = 0;
            }
            return true;
        }
        false
    }

    pub(super) fn pedia_buttons(&self) -> Vec<Button> {
        let Some(Modal::Pedia { .. }) = &self.modal else {
            return Vec::new();
        };
        let mut b = Vec::new();
        for (slot, i) in self.pedia_present().into_iter().enumerate() {
            b.push(Button {
                x: TEXT_X + slot as f64 * 86.,
                y: 150.,
                w: 84.,
                h: 28.,
                label: SECTIONS[i].1.into(),
                action: Action::PediaSection(i),
            });
        }
        b
    }

    pub(super) fn pedia_draws(&self, draws: &mut Vec<Draw>) {
        let Some(Modal::Pedia {
            section,
            selected,
            reading,
            scroll,
            ..
        }) = &self.modal
        else {
            return;
        };
        let Some(s) = self.pedia().and_then(|p| p.section(section_name(*section))) else {
            return;
        };
        // The open category is marked with an underline under its tab.
        if let Some(slot) = self.pedia_present().iter().position(|i| i == section) {
            draws.push(rect(TEXT_X + slot as f64 * 86., 179., 84., 3., "#e8c85a"));
        }
        if *reading {
            let Some(entry) = s.entries.get(*selected) else {
                return;
            };
            draws.push(text(TEXT_X, 188., &entry.title, 19, "#fff0b5"));
            let lines = self.pedia_lines(*section, *selected);
            for (i, line) in lines.iter().skip(*scroll).take(READ_LINES).enumerate() {
                draws.push(text(TEXT_X, 218. + i as f64 * 19., line, 13, "#eee8d4"));
            }
            draws.push(text(
                TEXT_X,
                548.,
                &format!(
                    "Lines {}-{} of {} · Up/Down scroll · Enter or Esc goes back",
                    (*scroll + 1).min(lines.len()),
                    (*scroll + READ_LINES).min(lines.len()),
                    lines.len()
                ),
                13,
                "#ddd9bd",
            ));
        } else {
            let page = selected / LIST_ROWS;
            for (i, title) in s
                .titles()
                .enumerate()
                .skip(page * LIST_ROWS)
                .take(LIST_ROWS)
            {
                let y = LIST_Y + (i - page * LIST_ROWS) as f64 * ROW_H;
                if i == *selected {
                    draws.push(rect(TEXT_X - 4., y - 2., 608., ROW_H - 1., "#6b7057"));
                }
                draws.push(text(TEXT_X + 4., y + 3., title, 15, "#eee8d4"));
            }
            draws.push(text(
                TEXT_X,
                548.,
                &format!(
                    "{} entries · page {}/{} · Left/Right category · Enter reads",
                    s.entries.len(),
                    page + 1,
                    s.entries.len().div_ceil(LIST_ROWS).max(1)
                ),
                13,
                "#ddd9bd",
            ));
        }
        let source = self
            .loaded
            .as_ref()
            .map_or("", |l| l.resources.profile.as_str());
        draws.push(text(
            TEXT_X,
            566.,
            &format!("Text from this profile's Describe.txt ({source})."),
            12,
            "#c9c4a8",
        ));
    }
}
