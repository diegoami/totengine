//! Science, rates and government routes (#196), shared by both adapters.
//!
//! Design: docs/ui/science-government-milestone.md. This file holds presentation
//! state only (draft shares, locks, which dialog is open). Every committed change
//! goes through an engine `Command`; required decisions (`Government`, rate
//! review, research) cannot be dismissed into the map.
use super::*;
use engine::rates::{max_rate, redistribute, Slider};

pub(super) const GOVERNMENTS: [&str; 7] = [
    "Anarchy",
    "Despotism",
    "Monarchy",
    "Communism",
    "Fundamentalism",
    "Republic",
    "Democracy",
];
const ROWS: [&str; 3] = ["Tax", "Luxury", "Science"];

pub(super) fn government_name(id: u8) -> String {
    GOVERNMENTS
        .get(id as usize)
        .map_or_else(|| format!("Government {id}"), |n| (*n).into())
}

fn slider(row: usize) -> Slider {
    [Slider::Tax, Slider::Luxury, Slider::Science][row]
}

impl Client {
    fn route_view(&self) -> Result<HumanView, String> {
        let v = self.view().ok_or("Start a game first")?;
        if v.phase != Phase::Human {
            return Err("Wait for your turn".into());
        }
        Ok(v)
    }

    /// Shift+T, or the required review after a government change.
    pub(super) fn open_rates(&mut self) -> Result<(), String> {
        let v = self.route_view()?;
        match &v.decision {
            Some(Decision::Rates { id, view, opening }) => {
                let modal = self.rates_modal(Some(*id), view.government, view.cap, *opening)?;
                self.open(modal);
            }
            Some(_) => return Err("Resolve the required choice first".into()),
            None => {
                let r = v.rates.ok_or("No rates for this civilization")?;
                let modal =
                    self.rates_modal(None, r.government, r.cap, [r.tax, r.luxury, r.science])?;
                self.open(modal);
                self.status =
                    "Rates: Left/Right change the selected share, L locks it, Enter applies."
                        .into();
            }
        }
        Ok(())
    }

    fn rates_modal(
        &self,
        review: Option<u64>,
        government: u8,
        cap: u8,
        opened: [u8; 3],
    ) -> Result<Modal, String> {
        let quote = self
            .loaded
            .as_ref()
            .and_then(|l| l.session.preview_rates(opened[2], opened[0]).ok());
        Ok(Modal::Rates {
            review,
            government,
            cap,
            draft: opened,
            opened,
            row: 0,
            quote,
        })
    }

    pub(super) fn rate_step(&mut self, row: usize, delta: i32) -> Result<(), String> {
        let Some(Modal::Rates {
            cap, draft, opened, ..
        }) = &self.modal
        else {
            return Err("No rates dialog".into());
        };
        let (cap, draft, opened) = (*cap, *draft, *opened);
        let value = (i32::from(draft[row]) + delta).clamp(0, i32::from(cap)) as u8;
        if value == draft[row] {
            return Err(format!(
                "{} is already at its {} limit",
                ROWS[row],
                if delta > 0 {
                    format!("{}%", u32::from(cap) * 10)
                } else {
                    "0%".into()
                }
            ));
        }
        let out = redistribute(cap, draft, opened, self.rate_locks, slider(row), value);
        if out == draft {
            return Err(
                "That share cannot move: the other shares are locked or at their limit".into(),
            );
        }
        let quote = self
            .loaded
            .as_ref()
            .and_then(|l| l.session.preview_rates(out[2], out[0]).ok());
        if let Some(Modal::Rates {
            draft,
            row: selected,
            quote: q,
            ..
        }) = &mut self.modal
        {
            *draft = out;
            *selected = row;
            *q = quote;
        }
        self.focus_button = row * 3;
        Ok(())
    }

    pub(super) fn rate_lock(&mut self, row: usize) {
        self.rate_locks[row] = !self.rate_locks[row];
        if let Some(Modal::Rates { row: selected, .. }) = &mut self.modal {
            *selected = row;
        }
    }

    /// Shift+R: the confirmation question; nothing changes until Yes.
    pub(super) fn open_revolution(&mut self) -> Result<(), String> {
        let v = self.route_view()?;
        if v.decision.is_some() {
            return Err("Resolve the required choice first".into());
        }
        if !v.revolution_available {
            return Err(match v.anarchy_choice_turn {
                Some(turn) => {
                    format!("Already in Anarchy; the government chooser opens on turn {turn}")
                }
                None => "A revolution is not available now".into(),
            });
        }
        self.open(Modal::Revolution);
        Ok(())
    }

    /// F6: a small read-only advisor with the goal chooser.
    pub(super) fn open_science(&mut self) -> Result<(), String> {
        self.route_view()?;
        self.open(Modal::Science);
        Ok(())
    }

    pub(super) fn open_goal_list(&mut self) -> Result<(), String> {
        let v = self.route_view()?;
        self.open(Modal::Goal);
        self.choice = v.goal.map(|g| g as i32);
        if let Some(g) = v.goal {
            self.page = v.goal_choices.iter().position(|&a| a == g).unwrap_or(0) / 12;
        }
        Ok(())
    }

    pub(super) fn set_goal(&mut self, advance: Option<usize>) -> Result<(), String> {
        self.order(Command::Goal { advance })?;
        self.status = match advance {
            Some(a) => format!("Research goal: {}", self.tech_name(a)),
            None => "Research goal cleared".into(),
        };
        // `order` reopens a pending research prompt, whose suggestion now follows
        // the goal; otherwise return to the advisor.
        if !matches!(self.modal, Some(Modal::Research)) {
            self.open(Modal::Science);
        }
        Ok(())
    }

    /// Keys that only exist inside the new dialogs. `None` leaves the key to the
    /// generic modal handling.
    pub(super) fn route_key(&mut self, key: &str) -> Option<Result<(), String>> {
        let Some(Modal::Rates { row, .. }) = &self.modal else {
            return None;
        };
        let row = *row;
        match key {
            "ArrowUp" | "ArrowDown" => {
                let next = if key == "ArrowDown" {
                    (row + 1).min(2)
                } else {
                    row.saturating_sub(1)
                };
                if let Some(Modal::Rates { row, .. }) = &mut self.modal {
                    *row = next;
                }
                self.focus_button = next * 3;
                Some(Ok(()))
            }
            "ArrowLeft" => Some(self.rate_step(row, -1)),
            "ArrowRight" => Some(self.rate_step(row, 1)),
            "l" => {
                self.rate_lock(row);
                Some(Ok(()))
            }
            _ => None,
        }
    }

    /// Accept for the new dialogs. `Ok(None)` means "not mine"; `Ok(Some(true))`
    /// that the generic close and required-choice reopening should follow;
    /// `Ok(Some(false))` that this function already chose the next dialog.
    pub(super) fn route_accept(&mut self, modal: &Modal) -> Result<Option<bool>, String> {
        match modal {
            Modal::Rates {
                review,
                draft,
                opened,
                ..
            } => {
                let (science, tax) = (draft[2], draft[0]);
                match review {
                    Some(decision) => {
                        let Some(Decision::Rates { id, .. }) = self.view().and_then(|v| v.decision)
                        else {
                            return Err("Rate review changed".into());
                        };
                        if id != *decision {
                            return Err("Rate review changed".into());
                        }
                        self.order(Command::ReviewRates {
                            decision: id,
                            science,
                            tax,
                        })?;
                    }
                    None if draft == opened => {
                        self.status = "Rates unchanged".into();
                    }
                    None => self.order(Command::Rates { science, tax })?,
                }
                Ok(Some(true))
            }
            Modal::Revolution => {
                self.order(Command::Revolution)?;
                self.status = "The old government has fallen. Anarchy has begun.".into();
                Ok(Some(true))
            }
            Modal::Government { decision } => {
                let Some(Decision::Government { id, .. }) = self.view().and_then(|v| v.decision)
                else {
                    return Err("Government decision changed".into());
                };
                if id != *decision {
                    return Err("Government decision changed".into());
                }
                let government = self.choice.ok_or("Select a government explicitly")?;
                self.order(Command::Government {
                    decision: id,
                    government: u8::try_from(government).map_err(|_| "Invalid government")?,
                })?;
                Ok(Some(true))
            }
            Modal::Science => Ok(Some(true)),
            Modal::Pedia { .. } => {
                self.pedia_accept()?;
                Ok(Some(false))
            }
            Modal::Report { .. } => {
                self.report_open_city()?;
                Ok(Some(false))
            }
            Modal::Goal => {
                let goal = self.choice.ok_or("Select a goal explicitly")? as usize;
                self.set_goal(Some(goal))?;
                Ok(Some(false))
            }
            _ => Ok(None),
        }
    }

    /// Cancel/Escape for the new dialogs; `true` when handled here.
    pub(super) fn route_cancel(&mut self) -> bool {
        if matches!(self.modal, Some(Modal::Pedia { .. })) {
            return self.pedia_cancel();
        }
        match &mut self.modal {
            Some(Modal::Rates {
                review: Some(_),
                draft,
                opened,
                ..
            }) => {
                *draft = *opened;
                let (science, tax) = (opened[2], opened[0]);
                let fresh = self
                    .loaded
                    .as_ref()
                    .and_then(|l| l.session.preview_rates(science, tax).ok());
                if let Some(Modal::Rates { quote, .. }) = &mut self.modal {
                    *quote = fresh;
                }
                self.status =
                    "Rates are required after a government change; Enter applies the shown split."
                        .into();
                true
            }
            Some(Modal::Government { .. }) => {
                self.status = "A government must be chosen; Anarchy continues until you do.".into();
                true
            }
            Some(Modal::Rates { review: None, .. }) => {
                self.modal = None;
                self.status = "Rate changes discarded.".into();
                true
            }
            Some(Modal::Revolution) => {
                self.modal = None;
                self.status = "Revolution cancelled; your government is unchanged.".into();
                true
            }
            Some(Modal::Goal) => {
                self.open(Modal::Science);
                true
            }
            _ => false,
        }
    }

    pub(super) fn route_choices(&self, v: &HumanView) -> Option<Vec<(i32, String)>> {
        match &self.modal {
            Some(Modal::Government { .. }) => {
                let Some(Decision::Government { eligible, .. }) = &v.decision else {
                    return Some(Vec::new());
                };
                Some(
                    eligible
                        .iter()
                        .map(|&g| {
                            (
                                i32::from(g),
                                format!(
                                    "{} — each share up to {}%",
                                    government_name(g),
                                    u32::from(max_rate(g)) * 10
                                ),
                            )
                        })
                        .collect(),
                )
            }
            Some(Modal::Goal) => Some(
                v.goal_choices
                    .iter()
                    .map(|&a| {
                        let mark = if v.goal == Some(a) {
                            " (current goal)"
                        } else {
                            ""
                        };
                        (a as i32, format!("{}{mark}", self.tech_name(a)))
                    })
                    .collect(),
            ),
            _ => None,
        }
    }

    pub(super) fn route_buttons(&self, modal: &Modal) -> Vec<Button> {
        let mut b = Vec::new();
        let mut add = |x, y, w, h, label: String, action| {
            b.push(Button {
                x,
                y,
                w,
                h,
                label,
                action,
            })
        };
        match modal {
            Modal::Rates { .. } => {
                for (row, &locked) in self.rate_locks.iter().enumerate() {
                    let y = 178. + row as f64 * 64.;
                    add(676., y, 40., 30., "−".into(), Action::RateStep(row, -1));
                    add(722., y, 40., 30., "+".into(), Action::RateStep(row, 1));
                    add(
                        768.,
                        y,
                        68.,
                        30.,
                        if locked { "Locked" } else { "Lock" }.into(),
                        Action::RateLock(row),
                    );
                }
            }
            Modal::Science => {
                add(212., 170., 220., 34., "Set goal…".into(), Action::GoalList);
                if self.view().is_some_and(|v| v.goal.is_some()) {
                    add(
                        442.,
                        170.,
                        180.,
                        34.,
                        "Clear goal".into(),
                        Action::ClearGoal,
                    );
                }
            }
            _ => (),
        }
        b
    }

    pub(super) fn route_draws(&self, modal: &Modal, draws: &mut Vec<Draw>) {
        let Some(v) = self.view() else { return };
        let line = |draws: &mut Vec<Draw>, y: f64, s: &str, size: u32, color: &str| {
            draws.push(text(212., y, s, size, color));
        };
        match modal {
            Modal::Rates {
                review,
                government,
                cap,
                draft,
                row,
                quote,
                ..
            } => {
                line(
                    draws,
                    151.,
                    &format!(
                        "{} · each share at most {}% · the three total 100%",
                        government_name(*government),
                        u32::from(*cap) * 10
                    ),
                    14,
                    "#eee8d4",
                );
                for (i, name) in ROWS.iter().enumerate() {
                    let y = 178. + i as f64 * 64.;
                    if i == *row {
                        draws.push(rect(206., y - 6., 636., 42., "#6b7057"));
                    }
                    line(draws, y + 6., name, 17, "#fff0b5");
                    for cell in 0..10u8 {
                        let color = if cell < draft[i] {
                            "#e8c85a"
                        } else if cell < *cap {
                            "#2c3326"
                        } else {
                            "#171b14"
                        };
                        draws.push(rect(330. + f64::from(cell) * 28., y + 2., 25., 22., color));
                    }
                    draws.push(text(
                        620.,
                        y + 6.,
                        &format!("{}%", u32::from(draft[i]) * 10),
                        17,
                        "#f4eedb",
                    ));
                }
                if let Some(q) = quote {
                    line(
                        draws,
                        384.,
                        &format!(
                            "Treasury {} · taxes +{} · upkeep −{} · net {:+} per turn",
                            q.treasury,
                            q.taxes,
                            q.maintenance,
                            q.net_income()
                        ),
                        16,
                        "#eee8d4",
                    );
                    line(
                        draws,
                        414.,
                        &format!(
                            "Science {} per turn · luxury {}",
                            q.science_output, q.luxuries
                        ),
                        16,
                        "#eee8d4",
                    );
                    if q.disorder_cities > 0 {
                        line(
                            draws,
                            444.,
                            &format!("{} city(ies) would be in disorder", q.disorder_cities),
                            16,
                            "#ffba8a",
                        );
                    }
                }
                let (hint, color) = if review.is_some() {
                    (
                        "Required after the government change · Enter applies · Esc resets the split",
                        "#ffba8a",
                    )
                } else {
                    (
                        "Up/Down row · Left/Right change · L lock · Enter applies · Esc discards",
                        "#eee8d4",
                    )
                };
                line(draws, 500., hint, 13, color);
                if self.rate_locks.iter().any(|l| *l) {
                    line(
                        draws,
                        524.,
                        "Locked shares keep their value while the others move.",
                        13,
                        "#eee8d4",
                    );
                }
            }
            Modal::Revolution => {
                let when = v.revolution_choice_turn.map_or_else(
                    || "when Anarchy ends".to_string(),
                    |t| format!("on turn {t}"),
                );
                for (i, s) in [
                    "Overthrow your government?".to_string(),
                    "Anarchy begins at once: each rate is limited to 60%.".to_string(),
                    format!("You will choose a new government {when}."),
                    "Until then play continues under Anarchy.".to_string(),
                    "Enter confirms · Esc cancels without any change.".to_string(),
                ]
                .iter()
                .enumerate()
                {
                    line(draws, 190. + i as f64 * 40., s, 17, "#eee8d4");
                }
            }
            Modal::Government { .. } => {
                line(
                    draws,
                    151.,
                    "Choose your new government · required · Esc keeps this open",
                    13,
                    "#ffba8a",
                );
            }
            Modal::Science => {
                let o = &v.observation;
                let topic = o
                    .research
                    .target
                    .map_or_else(|| "none yet".to_string(), |t| self.tech_name(t));
                let progress = format!(
                    "{} / {}",
                    o.research.progress,
                    o.research
                        .cost
                        .map_or_else(|| "?".to_string(), |c| c.to_string())
                );
                let science = v
                    .rates
                    .and_then(|r| {
                        self.loaded
                            .as_ref()?
                            .session
                            .preview_rates(r.science, r.tax)
                            .ok()
                    })
                    .map_or_else(|| "?".to_string(), |q| q.science_output.to_string());
                let goal = v
                    .goal
                    .map_or_else(|| "none".to_string(), |g| self.tech_name(g));
                for (i, s) in [
                    format!("Current topic: {topic}"),
                    format!("Progress: {progress} · science {science} per turn"),
                    format!("Long-term goal: {goal}"),
                    "The goal only steers the suggested topic of the next".to_string(),
                    "research choice; it never changes the running project.".to_string(),
                ]
                .iter()
                .enumerate()
                {
                    line(draws, 240. + i as f64 * 34., s, 17, "#eee8d4");
                }
            }
            Modal::Goal => {
                line(
                    draws,
                    151.,
                    "Arrow keys select · Enter sets the goal · Esc returns to the advisor",
                    13,
                    "#eee8d4",
                );
            }
            _ => (),
        }
    }
}
