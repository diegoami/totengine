//! Experimental scenario entry in both clients (decision 0008, 2026-11-18).
//!
//! **Open scenario (experimental)** lists the installation's scenario and
//! classic save files, shows a preflight with every gap the engine knows
//! about, and only then starts. The preflight is the point: nothing here is
//! presented as faithful scenario play (`docs/scenario-entry-contract.md`
//! remains the faithful target).
use super::*;
use crate::resources::original::OriginalCapture;
use engine::campaign::AiPolicy;
use engine::scenario::Preflight;

/// Scenario list rows per page.
pub(super) const SCENARIO_ROWS: usize = 11;
const ROW_Y: f64 = 172.;
const ROW_STEP: f64 = 34.;
/// Gap lines shown in the preflight.
const GAP_ROWS: usize = 10;

/// What the preflight needs to start the game it describes.
#[derive(Clone)]
pub(super) struct ScenarioChoice {
    pub path: String,
    identity: String,
    lua: bool,
    pub preflight: Preflight,
    /// Index into `preflight.civs`.
    pub civ: usize,
    pub policy: AiPolicy,
    /// The starting date in the scenario's own labels (#330).
    pub date: Option<String>,
}

fn restore_loaded(
    resources: Resources,
    files: crate::resources::ScenarioFiles,
    identity: String,
    text: &str,
) -> Result<Loaded, String> {
    let session = HumanSession::restore_with(
        resources.rules.clone(),
        &identity,
        text,
        Some((
            &files.bytes,
            files.events.map(engine::events::EventEngine::new),
        )),
    )?;
    let atlas = Atlas::load(&resources)?;
    Ok(Loaded {
        resources,
        atlas,
        identity,
        session,
        scenario_snapshot: Some(files.bytes),
    })
}

impl Client {
    fn scenario_home(&self) -> Result<&Path, String> {
        if self.home.trim().is_empty() {
            return Err(
                "Enter the installation directory first; scenarios come from your ToT install"
                    .into(),
            );
        }
        Ok(Path::new(&self.home))
    }

    /// Opens the scenario list.
    pub(super) fn open_scenarios(&mut self) -> Result<(), String> {
        self.open_scenario_list(Modal::Scenarios)
    }

    pub(super) fn open_original_saves(&mut self) -> Result<(), String> {
        self.open_scenario_list(Modal::OriginalSaves)
    }

    fn open_scenario_list(&mut self, modal: Modal) -> Result<(), String> {
        let home = self.scenario_home()?.to_path_buf();
        self.scenarios = crate::resources::list_scenarios(&home);
        if self.scenarios.is_empty() {
            return Err(
                "No scenario (.scn) or saved game (.sav) found in the installation's folders"
                    .into(),
            );
        }
        self.open(modal);
        Ok(())
    }

    pub(super) fn inspect_original_save(&mut self, path: &str) -> Result<(), String> {
        let home = self.scenario_home()?;
        let report =
            OriginalCapture::capture(home, path).and_then(|capture| capture.inspect_save());
        let lines = match report {
            Ok(report) => {
                let phase = &report["saved_phase"];
                let view = &report["saved_view"];
                let pending = &report["saved_pending_choice_check"];
                let event = &report["saved_event_initialization"];
                vec![
                    "Matched the reference Original dependencies.".into(),
                    format!(
                        "Turn {} · saved human civilization {}",
                        phase["turn"], phase["human_civ"]
                    ),
                    format!(
                        "Saved human view · {} explored tiles, {} units, {} cities",
                        view["explored_tiles"], view["units"], view["cities"]
                    ),
                    if pending["verified"] == true {
                        "The retained native resume reaches input with no pending choice.".into()
                    } else {
                        format!(
                            "Pending choices are unverified ({}).",
                            pending["reason"]
                                .as_str()
                                .unwrap_or("no matching native evidence")
                        )
                    },
                    if event["verified"] == true {
                        "The stock victory records can initialize a clean event state.".into()
                    } else {
                        format!(
                            "Event initialization is unverified ({}).",
                            event["reason"]
                                .as_str()
                                .unwrap_or("no matching native evidence")
                        )
                    },
                    "Faithful entry still requires verified saved-round continuation.".into(),
                ]
            }
            Err(reason) => vec![
                reason,
                "Select a classic single-player SAV and its stock inputs in Original.".into(),
            ],
        };
        self.open_original_inspection(Modal::OriginalSaveReport {
            path: path.into(),
            lines,
        });
        Ok(())
    }

    /// Issue a fresh modal ID while retaining the file list page for Back.
    pub(super) fn open_original_inspection(&mut self, modal: Modal) {
        let page = self.page;
        self.open(modal);
        self.page = page;
    }

    /// Reads the chosen file and opens its preflight; nothing starts yet.
    pub(super) fn pick_scenario(&mut self, path: &str) -> Result<(), String> {
        let home = self.scenario_home()?.to_path_buf();
        let (resources, files) = crate::resources::Resources::load_scenario(&home, path)?;
        let mut preflight = engine::scenario::preflight(
            &files.bytes,
            &resources.rules,
            files.events.as_ref(),
            files.events_unreadable,
            files.lua,
        )?;
        // The built-in opponent needs the Rules' research tables. Renamed
        // advances resolve by the original's fixed slot identifiers (#329);
        // Rules it still cannot read fall back to AI v1, and say so.
        let policy = match engine::ai::BuiltInAi::new(&resources.rules, 0, 1) {
            Ok(_) => AiPolicy::BuiltIn1,
            Err(e) => {
                preflight.gaps.insert(
                    1,
                    format!("Opponents use AI v1: the built-in AI cannot read these Rules ({e})"),
                );
                AiPolicy::AiV1
            }
        };
        let civ = preflight
            .civs
            .iter()
            .position(|c| c.human_in_file)
            .unwrap_or(0);
        let date = preflight
            .date
            .map(|(n, style)| engine::calendar::date_text(n, style, resources.labels.as_ref()));
        self.open(Modal::ScenarioStart(Box::new(ScenarioChoice {
            date,
            path: files.path,
            identity: resources.scenario_identity(),
            lua: files.lua,
            preflight,
            civ,
            policy,
        })));
        Ok(())
    }

    /// Starts the game the preflight showed.
    pub(super) fn start_scenario(&mut self, choice: &ScenarioChoice) -> Result<(), String> {
        let home = self.scenario_home()?.to_path_buf();
        let (resources, files) = crate::resources::Resources::load_scenario(&home, &choice.path)?;
        let identity = resources.scenario_identity();
        if identity != choice.identity || files.lua != choice.lua {
            return Err("Scenario or resources changed. Go back and select the file again.".into());
        }
        let atlas = Atlas::load(&resources)?;
        let civ = choice
            .preflight
            .civs
            .get(choice.civ)
            .ok_or("Choose a civilization")?;
        let seed: u64 = random_seed()?.parse().map_err(|_| "seed")?;
        let session = HumanSession::scenario_initialized_current(
            engine::human::ScenarioInput {
                path: &files.path,
                snapshot: &files.bytes,
                rules: resources.rules.clone(),
                events: files.events.map(engine::events::EventEngine::new),
            },
            civ.civ,
            choice.policy,
            seed,
            &identity,
        )?;
        let assets = storage::nonce()?;
        self.report_return = None;
        self.reset_notices();
        self.loaded = Some(Loaded {
            resources,
            atlas,
            session,
            identity,
            scenario_snapshot: Some(files.bytes),
        });
        self.assets = assets;
        self.advancing = self.view().is_some_and(|v| v.phase == Phase::Opponents);
        self.city_panel = CityPanel::Info;
        self.city_panel_page = 0;
        self.editing.clear();
        self.active = None;
        self.city = None;
        self.refresh();
        self.schedule_unit(None, true);
        self.center();
        self.status = format!(
            "Experimental scenario: {} as the {}. Not faithful scenario play; see the gaps shown before starting.",
            choice.path, civ.name
        );
        Ok(())
    }

    /// Restore the replacement before committing assets or selection. The
    /// optional focus comes from a checked viewport or a replayed order.
    pub(super) fn restore_scenario(
        &mut self,
        origin: &engine::human::ScenarioOrigin,
        text: &str,
    ) -> Result<Option<Entity>, String> {
        let home = self.scenario_home()?.to_path_buf();
        let mut snapshot = storage::scenario_snapshot(&self.saves, text)?;
        let reference_recipe = text.starts_with("totengine-human 3\n")
            && text.lines().nth(5).is_some_and(|line| {
                use engine::human::ScenarioInitializer;
                let mut fields = line.split_whitespace();
                matches!(
                    (fields.next(), fields.next(), fields.next(), fields.next()),
                    (
                        Some("initializer"),
                        Some("7"),
                        Some("profile"),
                        Some(ScenarioInitializer::SAVED_OPPONENT_COMBAT_PROFILE)
                    ) | (
                        Some("initializer"),
                        Some("8"),
                        Some("profile"),
                        Some(ScenarioInitializer::SAVED_HUMAN_PRODUCTION_PROFILE)
                    ) | (
                        Some("initializer"),
                        Some("9"),
                        Some("profile"),
                        Some(ScenarioInitializer::SAVED_OPPONENT_CONTACT_PROFILE)
                    ) | (
                        Some("initializer"),
                        Some("10"),
                        Some("profile"),
                        Some(ScenarioInitializer::SAVED_OPPONENT_CITY_COMBAT_PROFILE)
                    ) | (
                        Some("initializer"),
                        Some("11"),
                        Some("profile"),
                        Some(ScenarioInitializer::SAVED_OPPONENT_HUT_PROFILE)
                    ) | (
                        Some("initializer"),
                        Some("12"),
                        Some("profile"),
                        Some(ScenarioInitializer::SAVED_OPPONENT_ARMED_STACK_PROFILE)
                    ) | (
                        Some("initializer"),
                        Some("13"),
                        Some("profile"),
                        Some(ScenarioInitializer::SAVED_OPPONENT_ARMED_STACK_V2_PROFILE)
                    ) | (
                        Some("initializer"),
                        Some("14"),
                        Some("profile"),
                        Some(ScenarioInitializer::SAVED_OPPONENT_ZONE_PROFILE)
                    )
                )
            });
        let legacy_snapshot = if reference_recipe {
            snapshot.clone()
        } else {
            snapshot.take()
        };
        let legacy = Resources::load_scenario_inputs(&home, &origin.path, legacy_snapshot)
            .map_err(|e| format!("This checkpoint needs the scenario {}: {e}", origin.path))
            .and_then(|(resources, files)| {
                let identity = if text.starts_with("totengine-human 3\n") {
                    resources.scenario_identity()
                } else {
                    resources.identity()
                };
                restore_loaded(resources, files, identity, text)
            });
        let (loaded, focus) = match legacy {
            Ok(loaded) => {
                let focus = loaded.session.saved_order_focus();
                (loaded, focus)
            }
            Err(_) if reference_recipe => {
                let capture = OriginalCapture::capture_checkpoint(&home, &origin.path, snapshot)?;
                let viewport = capture.checkpoint_viewport()?;
                let (resources, files, identity) = capture.into_checkpoint_inputs();
                let loaded = restore_loaded(resources, files, identity, text)?;
                let focus = if loaded.session.view().revision == 0 {
                    viewport.and_then(|anchor| loaded.session.saved_initial_focus(anchor))
                } else {
                    loaded.session.saved_order_focus()
                };
                (loaded, focus)
            }
            Err(error) => return Err(error),
        };
        let assets = storage::nonce()?;
        self.reset_notices();
        self.loaded = Some(loaded);
        self.assets = assets;
        Ok(focus)
    }

    pub(super) fn scenario_buttons(&self, modal: &Modal) -> Vec<Button> {
        let mut b = Vec::new();
        let mut add = |x: f64, y: f64, w: f64, h: f64, label: String, action: Action| {
            b.push(Button {
                x,
                y,
                w,
                h,
                label,
                action,
            });
        };
        match modal {
            Modal::Scenarios | Modal::OriginalSaves => {
                let pages = self.scenarios.len().div_ceil(SCENARIO_ROWS);
                for (row, path) in self
                    .scenarios
                    .iter()
                    .skip(self.page * SCENARIO_ROWS)
                    .take(SCENARIO_ROWS)
                    .enumerate()
                {
                    add(
                        212.,
                        ROW_Y + row as f64 * ROW_STEP,
                        600.,
                        30.,
                        path.clone(),
                        Action::ScenarioPick(path.clone()),
                    );
                }
                if pages > 1 {
                    add(212., 552., 120., 28., "Previous".into(), Action::Page(-1));
                    add(352., 552., 120., 28., "Next page".into(), Action::Page(1));
                }
            }
            Modal::ScenarioStart(choice) => {
                for (i, c) in choice.preflight.civs.iter().enumerate() {
                    let (col, row) = (i % 4, i / 4);
                    add(
                        212. + col as f64 * 152.,
                        236. + row as f64 * 36.,
                        146.,
                        30.,
                        format!("{} {}", if i == choice.civ { "●" } else { "○" }, c.name),
                        Action::ScenarioCiv(i),
                    );
                }
            }
            _ => {}
        }
        b
    }

    pub(super) fn scenario_draws(&self, modal: &Modal, draws: &mut Vec<Draw>) {
        match modal {
            Modal::Scenarios | Modal::OriginalSaves => {
                let pages = self.scenarios.len().div_ceil(SCENARIO_ROWS).max(1);
                let mode = if matches!(modal, Modal::OriginalSaves) {
                    "read-only compatibility checks"
                } else {
                    "experimental: gaps are listed before starting"
                };
                draws.push(text(
                    212.,
                    150.,
                    &format!(
                        "{} files in the installation · page {} of {pages} · {mode}",
                        self.scenarios.len(),
                        self.page + 1
                    ),
                    style::TYPE_SMALL,
                    style::TEXT_DIM,
                ));
            }
            Modal::OriginalSaveReport { path, lines } => {
                draws.push(text(212., 150., path, style::TYPE_BODY, style::TEXT_BRIGHT));
                let mut y = 198.;
                for paragraph in lines {
                    for line in pedia::wrap(paragraph, 70) {
                        draws.push(text(212., y, &line, style::TYPE_SMALL, style::TEXT));
                        y += 21.;
                    }
                    y += 12.;
                }
            }
            Modal::ScenarioStart(choice) => {
                let p = &choice.preflight;
                draws.push(text(
                    212.,
                    150.,
                    &choice.path,
                    style::TYPE_BODY,
                    style::TEXT_BRIGHT,
                ));
                draws.push(text(
                    212.,
                    176.,
                    &format!(
                        "{}Turn {} · map {} × {} · {} civilizations · opponents: {}",
                        choice
                            .date
                            .as_ref()
                            .map_or_else(String::new, |d| format!("{d} · ")),
                        p.turn,
                        p.width * 2,
                        p.height,
                        p.civs.len(),
                        choice.policy.id()
                    ),
                    style::TYPE_SMALL,
                    style::TEXT,
                ));
                draws.push(text(
                    212.,
                    214.,
                    "Play as:",
                    style::TYPE_BODY,
                    style::TEXT_BRIGHT,
                ));
                let rows = p.civs.len().div_ceil(4) as f64;
                let civ = &p.civs[choice.civ.min(p.civs.len() - 1)];
                let y = 236. + rows * 36. + 6.;
                draws.push(text(
                    212.,
                    y,
                    &format!(
                        "{}: {} units, {} cities{}",
                        civ.name,
                        civ.units,
                        civ.cities,
                        if civ.human_in_file {
                            " · human in the file"
                        } else {
                            ""
                        }
                    ),
                    style::TYPE_SMALL,
                    style::TEXT,
                ));
                let y = if self.loaded.is_some() {
                    draws.push(text(
                        212.,
                        y + 20.,
                        "Starting replaces the running game; save it first to keep it.",
                        style::TYPE_SMALL,
                        style::WARN,
                    ));
                    y + 16.
                } else {
                    y
                };
                draws.push(text(
                    212.,
                    y + 28.,
                    "Not supported yet (experimental entry):",
                    style::TYPE_BODY,
                    style::WARN,
                ));
                let mut lines: Vec<String> = Vec::new();
                for gap in &p.gaps {
                    lines.extend(wrap(&format!("• {gap}"), 80));
                }
                let shown = lines.len().min(GAP_ROWS);
                for (i, line) in lines.iter().take(shown).enumerate() {
                    draws.push(text(
                        214.,
                        y + 52. + i as f64 * 19.,
                        line,
                        style::TYPE_SMALL,
                        style::TEXT,
                    ));
                }
                if lines.len() > shown {
                    draws.push(text(
                        214.,
                        y + 52. + shown as f64 * 19.,
                        &format!("+{} more lines", lines.len() - shown),
                        style::TYPE_SMALL,
                        style::TEXT_DIM,
                    ));
                }
            }
            _ => {}
        }
    }
}

/// Word-wraps `line` at `width` characters; continuation lines are indented.
fn wrap(line: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for word in line.split(' ') {
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > width {
            out.push(std::mem::take(&mut current));
            current.push_str("   ");
        }
        if !current.is_empty() && !current.ends_with("   ") {
            current.push(' ');
        }
        current.push_str(word);
    }
    out.push(current);
    out
}
