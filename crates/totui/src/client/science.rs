//! The F6 science advisor window (layout B, #315).
//!
//! Read-only: it shows the running project, the goal and the advances from
//! the civilization's own view. Its only commands are the existing goal
//! chooser and Clear goal; research rules are unchanged (native list rules,
//! #195). With the opt-in goal-estimates setting it also shows the path to
//! the goal with a turn estimate, and lets the available list sort by name.
use super::*;
use engine::research::TechTree;

const X: f64 = 326.;
const LIST_Y: f64 = 340.;
const LIST_ROWS: usize = 10;

impl Client {
    fn known_advances(o: &engine::player::Observation, count: usize) -> Vec<bool> {
        (0..count)
            .map(|t| o.techs.get(t / 8).is_some_and(|b| b & (1 << (t % 8)) != 0))
            .collect()
    }

    /// The unknown advances the goal still needs, in an order they can be
    /// researched (prerequisites first, then by advance number).
    fn goal_path(&self, v: &HumanView) -> Option<Vec<usize>> {
        let goal = v.goal?;
        let rules = &self.loaded.as_ref()?.resources.rules;
        let tree = TechTree::from_rules(rules);
        let n = tree.techs.len();
        let mut known = Self::known_advances(&v.observation, n);
        let mut need = vec![false; n];
        let mut stack = vec![goal];
        while let Some(t) = stack.pop() {
            if t >= n || known[t] || std::mem::replace(&mut need[t], true) {
                continue;
            }
            stack.extend(tree.techs[t].prereqs.iter().flatten().copied());
        }
        let mut path = Vec::new();
        while let Some(next) = (0..n).find(|&t| need[t] && tree.can_research(t, &known)) {
            known[next] = true;
            need[next] = false;
            path.push(next);
        }
        Some(path)
    }

    /// Science per turn at the current rates, when the engine can quote it.
    fn science_per_turn(&self, v: &HumanView) -> Option<i32> {
        let r = v.rates?;
        let q = self
            .loaded
            .as_ref()?
            .session
            .preview_rates(r.science, r.tax)
            .ok()?;
        Some(q.science_output)
    }

    pub(super) fn science_buttons(&self) -> Vec<Button> {
        let mut b = vec![Button {
            x: X,
            y: 252.,
            w: 170.,
            h: 30.,
            label: "Set goal…".into(),
            action: Action::GoalList,
        }];
        if self.view().is_some_and(|v| v.goal.is_some()) {
            b.push(Button {
                x: X + 178.,
                y: 252.,
                w: 140.,
                h: 30.,
                label: "Clear goal".into(),
                action: Action::ClearGoal,
            });
        }
        if self.enhancements.goal_estimates {
            b.push(Button {
                x: 412.,
                y: LIST_Y - 3.,
                w: 110.,
                h: 24.,
                label: if self.science_by_name {
                    "Sort: name"
                } else {
                    "Sort: listed"
                }
                .into(),
                action: Action::ScienceSort,
            });
        }
        b
    }

    pub(super) fn science_draws(&self, draws: &mut Vec<Draw>, v: &HumanView) {
        let o = &v.observation;
        let estimates = self.enhancements.goal_estimates;
        // Portrait slot: the installed advisor art is not loaded yet, so a
        // plain frame keeps the classic arrangement.
        draws.push(rect(212., 160., 96., 116., style::GOLD_DIM));
        draws.push(rect(213., 161., 94., 114., style::INSET));
        let topic = o
            .research
            .target
            .map_or_else(|| "none yet".to_string(), |t| self.tech_name(t));
        draws.push(text(
            X,
            160.,
            &format!("Researching: {topic}"),
            style::TYPE_BODY,
            style::TEXT_BRIGHT,
        ));
        let cost = o.research.cost.unwrap_or(0);
        let progress = i32::from(o.research.progress);
        draws.push(rect(X, 186., 486., 12., style::INSET));
        if cost > 0 {
            let done = (f64::from(progress) / f64::from(cost)).clamp(0., 1.);
            draws.push(rect(X, 186., 486. * done, 12., style::LINK));
        }
        let science = self.science_per_turn(v);
        let mut bulbs = format!(
            "{} / {} bulbs · {} per turn",
            progress,
            o.research
                .cost
                .map_or_else(|| "?".to_string(), |c| c.to_string()),
            science.map_or_else(|| "?".to_string(), |s| s.to_string())
        );
        let turns = |bulbs: i32| {
            science
                .filter(|s| *s > 0)
                .map(|s| (bulbs.max(0) + s - 1) / s)
        };
        if estimates && o.research.target.is_some() && cost > 0 {
            if let Some(t) = turns(cost - progress) {
                bulbs.push_str(&format!(" · about {t} turns"));
            }
        }
        draws.push(text(X, 206., &bulbs, style::TYPE_SMALL, style::TEXT));
        let goal = v
            .goal
            .map_or_else(|| "none".to_string(), |g| self.tech_name(g));
        draws.push(text(
            X,
            228.,
            &format!("Goal: {goal}"),
            style::TYPE_BODY,
            style::TEXT,
        ));
        let path = if estimates { self.goal_path(v) } else { None };
        if let Some(path) = &path {
            let names: Vec<String> = path.iter().map(|&t| self.tech_name(t)).collect();
            let mut line = format!("Path: {}", names.join(" → "));
            if line.chars().count() > 78 {
                line = line.chars().take(77).collect::<String>() + "…";
            }
            draws.push(text(X, 292., &line, style::TYPE_SMALL, style::TEXT_BRIGHT));
            // Each advance costs at least the current one, so this is a lower
            // bound; the running project finishes first either way.
            let current_on_path = o.research.target.is_some_and(|t| path.contains(&t));
            let after = path.len() - usize::from(current_on_path);
            let bulbs_left = (cost - progress).max(0) + after as i32 * cost;
            let estimate = match turns(bulbs_left) {
                Some(t) if cost > 0 => format!(
                    "{} advance{} · at least about {t} turns at today's science rate (estimate)",
                    path.len(),
                    if path.len() == 1 { "" } else { "s" }
                ),
                _ => format!(
                    "{} advances · no estimate without science output",
                    path.len()
                ),
            };
            draws.push(text(X, 310., &estimate, style::TYPE_SMALL, style::GOLD));
        }
        // Available next.
        let mut available: Vec<(usize, String)> = o
            .research
            .available
            .iter()
            .map(|&t| (t, self.tech_name(t)))
            .collect();
        if estimates && self.science_by_name {
            available.sort_by_key(|(_, n)| n.to_lowercase());
        }
        draws.push(text(
            212.,
            LIST_Y,
            &format!("Available next ({})", available.len()),
            style::TYPE_BODY,
            style::TEXT_BRIGHT,
        ));
        for (row, (t, name)) in available.iter().take(LIST_ROWS).enumerate() {
            let toward = path.as_ref().is_some_and(|p| p.contains(t));
            let label = if toward {
                format!("{name} · toward goal")
            } else {
                name.clone()
            };
            draws.push(text(
                214.,
                LIST_Y + 26. + row as f64 * 20.,
                &label,
                style::TYPE_SMALL,
                if toward { style::GOLD } else { style::TEXT },
            ));
        }
        if available.len() > LIST_ROWS {
            draws.push(text(
                214.,
                LIST_Y + 26. + LIST_ROWS as f64 * 20.,
                &format!(
                    "+{} more in the research choice",
                    available.len() - LIST_ROWS
                ),
                style::TYPE_SMALL,
                style::TEXT_DIM,
            ));
        }
        // Known advances, alphabetical, two columns.
        let count = self
            .loaded
            .as_ref()
            .map_or(0, |l| TechTree::from_rules(&l.resources.rules).techs.len());
        let mut known: Vec<String> = Self::known_advances(o, count)
            .into_iter()
            .enumerate()
            .filter(|(_, k)| *k)
            .map(|(t, _)| self.tech_name(t))
            .collect();
        known.sort_by_key(|n| n.to_lowercase());
        draws.push(text(
            540.,
            LIST_Y,
            &format!("Known advances ({})", known.len()),
            style::TYPE_BODY,
            style::TEXT_BRIGHT,
        ));
        let shown = 2 * LIST_ROWS;
        for (i, name) in known.iter().take(shown).enumerate() {
            let mut name = name.clone();
            if name.chars().count() > 17 {
                name = name.chars().take(16).collect::<String>() + "…";
            }
            draws.push(text(
                542. + (i / LIST_ROWS) as f64 * 138.,
                LIST_Y + 26. + (i % LIST_ROWS) as f64 * 20.,
                &name,
                style::TYPE_SMALL,
                style::TEXT,
            ));
        }
        if known.len() > shown {
            draws.push(text(
                542.,
                LIST_Y + 26. + LIST_ROWS as f64 * 20.,
                &format!("+{} more (Civilopedia lists them all)", known.len() - shown),
                style::TYPE_SMALL,
                style::TEXT_DIM,
            ));
        }
        draws.push(text(
            212.,
            574.,
            "The goal only steers the next research suggestion; the running project never changes.",
            style::TYPE_FINE,
            style::TEXT_DIM,
        ));
    }
}
