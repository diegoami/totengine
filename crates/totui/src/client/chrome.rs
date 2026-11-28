//! Map-screen chrome: the menu bar (#310) and the original's top band
//! (decision 0009, #439) — the status panel, the info panel and the circular
//! overview over a full-width map.
//!
//! Everything here is presentation. Every menu item and band key runs an
//! `Action` the client already had, and every key keeps its meaning; menus
//! only make the commands visible in one place. The overview is drawn from the
//! civilization's filtered view and a click on it only scrolls.
use super::*;

/// Height of the menu bar along the top of the map screen.
pub(super) const MENU_H: f64 = 26.;
/// Height of the top band: the status panel, the info panel and the circular
/// overview over the full-width map (decision 0009, #439).
pub(super) const BAND_H: f64 = 140.;
/// The map viewport `(x, y, w, h)` with every option off; the optional order
/// strip (#312) takes its bottom edge. The map runs full width below the
/// band, as in the original's frame.
pub(super) const MAP: (f64, f64, f64, f64) = (20., 174., 984., 560.);
/// The status panel `(x, y, w, h)` in the band: population, cities/units and
/// the date, gold with rates, and the research icon.
pub(super) const STATUS_PANEL: (f64, f64, f64, f64) = (10., 34., 224., 122.);
/// Left edge and width of the info panel; its right edge meets the overview.
pub(super) const INFO_X: f64 = 242.;
/// Width of the circular-overview cell at the right of the band.
pub(super) const OVERVIEW_W: f64 = 124.;
/// Height the order strip takes from the map (#312).
pub(super) const STRIP_H: f64 = 46.;
/// Width of the side column a wide stage adds at the map's right (#337),
/// and the gap between the two.
pub(super) const SIDE_W: f64 = 232.;
const SIDE_GAP: f64 = 8.;
/// The narrowest stage that has the side column: 4:3 plus the column.
/// 16:9 (1365) has it; 16:10 (1228) and 4:3 keep the frame as it was.
pub(super) const WIDE_MIN: f64 = 1024. + SIDE_W + SIDE_GAP;
/// Width of text and buttons inside the side column and the message dock.
const CW: f64 = 220.;
const ITEM_H: f64 = 26.;
const DROP_W: f64 = 250.;

/// Menu titles, left to right (the original's map-screen menus that the
/// client can fill today).
pub(super) const MENUS: [&str; 7] = [
    "Game",
    "Kingdom",
    "View",
    "Orders",
    "Advisors",
    "World",
    "Civilopedia",
];

/// The civilization palette shared by map and minimap markers.
pub(super) fn civ_color(owner: u8) -> &'static str {
    [
        "#aaa79a", "#e8c55b", "#709de5", "#e77e72", "#77c98a", "#c68add", "#5ac7d0", "#efb775",
    ][usize::from(owner.min(7))]
}

/// Minimap colour of a terrain type: a few broad classes, game data rather
/// than chrome.
fn terrain_color(terrain: u8) -> &'static str {
    match terrain {
        10 => "#2e4a6b",
        3 | 8 | 9 => "#3a5831",
        4 | 5 => "#776848",
        0 => "#ad965c",
        6 | 7 => "#9aa39a",
        _ => "#5d7a40",
    }
}

fn title_w(title: &str) -> f64 {
    (title.chars().count() as f64 * 0.60 * f64::from(style::TYPE_SMALL) + 22.).ceil()
}

/// Left edge of each menu title.
fn title_x(m: usize) -> f64 {
    6. + MENUS[..m].iter().map(|t| title_w(t)).sum::<f64>()
}

/// One menu entry: its label, the key that does the same, and the action, or
/// why it is unavailable right now.
pub(super) struct MenuItem {
    pub label: String,
    pub key: &'static str,
    pub action: Result<Action, &'static str>,
}

fn item(label: &str, key: &'static str, action: Action) -> MenuItem {
    MenuItem {
        label: label.into(),
        key,
        action: Ok(action),
    }
}

impl Client {
    /// The map screen is the active surface: a game is loaded and no dialog
    /// or city window is open.
    pub(super) fn map_surface(&self) -> bool {
        self.loaded.is_some() && self.modal.is_none() && !self.city_open && !self.closed
    }

    /// How far the 1024-wide screens (dialogs, city window, setup) move right
    /// to sit centred on a wider stage (#321); 0 at 4:3.
    pub(super) fn dx(&self) -> f64 {
        ((self.stage_w - f64::from(WIDTH)) / 2.).floor()
    }

    /// Whether the current screen is one of the centred 1024-wide ones.
    pub(super) fn centred(&self) -> bool {
        self.modal.is_some() || self.city_open || self.loaded.is_none()
    }

    /// Where centring puts a tile on the map: the same point as at 4:3, moved
    /// to the middle of a wider map.
    pub(super) fn anchor(&self) -> (f64, f64) {
        (380. + (self.map_rect().2 - MAP.2) / 2., 390.)
    }

    /// Left edge of the side column (#337) when the stage is wide enough
    /// for it: the messages and notices move there from the new-messages
    /// overlay. Owner decision 2026-11-19: on by default from `WIDE_MIN`,
    /// 4:3 unchanged. It sits at the stage's right edge, below the top band
    /// (decision 0009).
    pub(super) fn side_x(&self) -> Option<f64> {
        (self.stage_w >= WIDE_MIN).then_some(self.stage_w - SIDE_W - SIDE_GAP)
    }

    /// Top of the side column and of the map's right edge: below the band.
    pub(super) fn frame_top(&self) -> f64 {
        MENU_H + BAND_H + 4.
    }

    /// The map viewport `(x, y, w, h)`: full width below the band, shorter
    /// while the order strip shows and on a wide stage beside the side
    /// column (#337).
    pub(super) fn map_rect(&self) -> (f64, f64, f64, f64) {
        let (x, y, _, h) = MAP;
        let right = self
            .side_x()
            .map_or(self.stage_w - MAP.0, |sx| sx - SIDE_GAP);
        let w = (right - x).max(64.);
        if self.enhancements.order_strip {
            (x, y, w, h - STRIP_H)
        } else {
            (x, y, w, h)
        }
    }

    /// Whether `(x, y)` is on the map viewport.
    pub(super) fn on_map(&self, x: f64, y: f64) -> bool {
        let (mx, my, mw, mh) = self.map_rect();
        (mx..mx + mw).contains(&x) && (my..my + mh).contains(&y)
    }

    /// Whether a tile centre `(px, py)` is far enough inside the viewport for
    /// its unit or city sprite to be drawn and hit-tested.
    pub(super) fn in_view(&self, px: f64, py: f64) -> bool {
        let (mx, my, mw, mh) = self.map_rect();
        (mx + 32.0..mx + mw - 32.).contains(&px) && (my + 50.0..my + mh - 40.).contains(&py)
    }

    pub(super) fn menu_items(&self, m: usize) -> Vec<MenuItem> {
        let v = self.view();
        let human = v.as_ref().is_some_and(|v| v.phase == Phase::Human);
        let order = |label: &str, key: &'static str, action: Action, unit: bool| MenuItem {
            label: label.into(),
            key,
            action: if !human {
                Err("Orders wait until your turn.")
            } else if unit && self.active.is_none() {
                Err("Select an own unit first.")
            } else {
                Ok(action)
            },
        };
        match m {
            0 => vec![
                item("Save game…", "Ctrl+S", Action::Save),
                item("Load game…", "Ctrl+L", Action::Load),
                item("New game…", "", Action::NewGame),
                item("Open scenario…", "", Action::Scenarios),
                item(
                    "Inspect ToT save / scenario…",
                    "",
                    Action::InspectOriginalSaves,
                ),
                item("Settings…", "", Action::Settings),
                item("Keys…", "?", Action::Help),
                item("Close client", "", Action::Quit),
            ],
            1 => vec![
                item("Tax rates…", "Shift+T", Action::Rates),
                item("Revolution…", "Shift+R", Action::Revolution),
                item("Cities", "", Action::City),
            ],
            2 => vec![
                item(
                    if self.moving {
                        "Switch to viewing"
                    } else {
                        "Switch to moving"
                    },
                    "V",
                    Action::Mode,
                ),
                item("Center view", "C", Action::Center),
                item("Zoom in", "Z", Action::Zoom(1.25)),
                item("Zoom out", "X", Action::Zoom(0.8)),
                item("Standard zoom", "Shift+Z", Action::ZoomReset),
                item(
                    if self.fullscreen {
                        "Window"
                    } else {
                        "Full screen"
                    },
                    "Alt+Enter",
                    Action::FullScreen,
                ),
            ],
            3 => vec![
                order("Found city", "B", Action::Found, true),
                order("Wait", "W", Action::Wait, true),
                order("Skip turn", "Space", Action::Skip, true),
                order("Activate unit", "A", Action::Activate, false),
                order("End player turn", "Ctrl+N", Action::End, false),
            ],
            4 => vec![
                item(
                    "City status",
                    "F1",
                    Action::Report(ReportFamily::CityStatus),
                ),
                item("Defense", "F2", Action::Report(ReportFamily::Defense)),
                item("Known contacts", "F3", Action::Contacts),
                item("Attitude", "F4", Action::Report(ReportFamily::Attitude)),
                item("Trade", "F5", Action::Report(ReportFamily::Trade)),
                item("Science", "F6", Action::Science),
                item("Research…", "", Action::Research),
            ],
            5 => vec![
                item(
                    "Wonders of the World",
                    "F7",
                    Action::Report(ReportFamily::Wonders),
                ),
                item(
                    "Top cities",
                    "F8",
                    Action::Report(ReportFamily::RankedCities),
                ),
                item(
                    "Civilization score",
                    "F9",
                    Action::Report(ReportFamily::Score),
                ),
                item(
                    "Demographics",
                    "F11",
                    Action::Report(ReportFamily::Demographics),
                ),
                MenuItem {
                    label: if self.notice_log.is_empty() {
                        "Messages".into()
                    } else {
                        format!("Messages ({})", self.notice_log.len())
                    },
                    key: "",
                    action: Ok(Action::Messages),
                },
            ],
            _ => vec![item("Civilopedia…", "", Action::Pedia)],
        }
    }

    /// Menu titles, then the open menu's items.
    pub(super) fn menu_buttons(&self) -> Vec<Button> {
        let mut b: Vec<Button> = MENUS
            .iter()
            .enumerate()
            .map(|(m, title)| Button {
                x: title_x(m),
                y: 0.,
                w: title_w(title),
                h: MENU_H,
                label: (*title).into(),
                action: Action::Menu(m),
            })
            .collect();
        if let Some(m) = self.menu {
            let x = title_x(m).min(self.stage_w - DROP_W - 4.);
            for (row, it) in self.menu_items(m).into_iter().enumerate() {
                b.push(Button {
                    x,
                    y: MENU_H + 3. + row as f64 * ITEM_H,
                    w: DROP_W,
                    h: ITEM_H,
                    label: it.label,
                    action: Action::MenuItem(m, row),
                });
            }
        }
        b
    }

    /// Runs one menu entry, or says why it is unavailable.
    pub(super) fn menu_item(&mut self, m: usize, row: usize) -> Result<(), String> {
        self.menu = None;
        self.menu_row = None;
        let item = self
            .menu_items(m)
            .into_iter()
            .nth(row)
            .ok_or("That menu changed; open it again.")?;
        match item.action {
            Ok(a) => self.action(a),
            Err(why) => Err(why.into()),
        }
    }

    /// Keys while a menu is open; `None` lets the key fall through to the map
    /// (the menu closes first, so a shortcut still does what it always did).
    pub(super) fn menu_key(&mut self, key: &str) -> Option<Result<(), String>> {
        let m = self.menu?;
        let rows = self.menu_items(m).len();
        match key {
            "ArrowLeft" | "ArrowRight" => {
                let step = if key == "ArrowLeft" {
                    MENUS.len() - 1
                } else {
                    1
                };
                self.menu = Some((m + step) % MENUS.len());
                self.menu_row = Some(0);
            }
            "ArrowDown" => self.menu_row = Some(self.menu_row.map_or(0, |r| (r + 1) % rows)),
            "ArrowUp" => {
                self.menu_row = Some(self.menu_row.map_or(rows - 1, |r| (r + rows - 1) % rows))
            }
            "Enter" => {
                return Some(match self.menu_row {
                    Some(row) => self.menu_item(m, row),
                    None => {
                        self.menu_row = Some(0);
                        Ok(())
                    }
                })
            }
            "Escape" | "F10" => {
                self.menu = None;
                self.menu_row = None;
            }
            _ => {
                self.menu = None;
                self.menu_row = None;
                return None;
            }
        }
        Some(Ok(()))
    }

    /// The menu bar, drawn under the band and map: its background and the
    /// window title at the right end.
    pub(super) fn menu_bar_draw(&self, draws: &mut Vec<Draw>, title: &str) {
        draws.push(rect(0., 0., self.stage_w, MENU_H, style::MENU_BAR));
        draws.push(rect(0., MENU_H, self.stage_w, 1., style::MENU_EDGE));
        // The menu bar already says which application this is.
        let title = title.strip_prefix("ToT Engine — ").unwrap_or(title);
        let width = |t: &str| t.chars().count() as f64 * 0.58 * f64::from(style::TYPE_SMALL);
        // The browser's licence link sits at the top-right corner; a long
        // title (a scenario name) first drops the turn, then the date (#330),
        // both of which the band also shows, then ends in an ellipsis.
        let room = self.stage_w - 198. - (title_x(MENUS.len()) + 8.);
        let mut title = title.to_string();
        if width(&title) > room {
            if let Some((head, _)) = title.rsplit_once(" · Turn ") {
                title = head.to_string();
                if width(&title) > room && self.date_label_shown() {
                    if let Some((head, _)) = title.rsplit_once(" · ") {
                        title = head.to_string();
                    }
                }
            }
        }
        if width(&title) > room {
            let keep = (room / width("x")).floor().max(1.) as usize - 1;
            title = title.chars().take(keep).collect::<String>() + "…";
        }
        let w = width(&title);
        draws.push(text(
            (self.stage_w - 198. - w).max(title_x(MENUS.len()) + 8.),
            8.,
            &title,
            style::TYPE_SMALL,
            style::MENU_TEXT,
        ));
    }

    /// Menu titles and the open dropdown, drawn on top of everything else on
    /// the map screen. Titles show under dialogs too; only the map surface
    /// opens them.
    pub(super) fn menu_draw(&self, draws: &mut Vec<Draw>, buttons: &[Button], focus: usize) {
        let focused = buttons.get(focus).map(|b| &b.action);
        for (m, title) in MENUS.iter().enumerate() {
            let on = self.menu == Some(m) || matches!(focused, Some(Action::Menu(f)) if *f == m);
            // Over a dialog or the city window the titles are inert, and look it.
            let idle = if self.map_surface() {
                style::MENU_TEXT
            } else {
                style::MENU_DISABLED
            };
            let x = title_x(m);
            if on {
                draws.push(rect(x, 1., title_w(title), MENU_H - 2., style::MENU_SELECT));
            }
            draws.push(text(
                x + 11.,
                6.,
                title,
                style::TYPE_SMALL,
                if on { style::MENU_SELECT_TEXT } else { idle },
            ));
        }
        let Some(m) = self.menu else { return };
        let items = self.menu_items(m);
        let n = items.len() as f64;
        let x = title_x(m).min(self.stage_w - DROP_W - 4.);
        draws.push(rect(
            x + 3.,
            MENU_H + 3.,
            DROP_W,
            n * ITEM_H + 6.,
            style::SHADOW,
        ));
        draws.push(rect(
            x - 1.,
            MENU_H,
            DROP_W + 2.,
            n * ITEM_H + 6.,
            style::MENU_EDGE,
        ));
        draws.push(rect(
            x,
            MENU_H + 1.,
            DROP_W,
            n * ITEM_H + 4.,
            style::MENU_DROP,
        ));
        for (i, b) in buttons.iter().enumerate() {
            let Action::MenuItem(_, row) = b.action else {
                continue;
            };
            let Some(it) = items.get(row) else { continue };
            let selected = self.menu_row == Some(row) || i == focus;
            let color = if selected {
                draws.push(rect(b.x, b.y, b.w, b.h, style::MENU_SELECT));
                style::MENU_SELECT_TEXT
            } else if it.action.is_ok() {
                style::MENU_TEXT
            } else {
                style::MENU_DISABLED
            };
            draws.push(text(b.x + 12., b.y + 5., &b.label, style::TYPE_BODY, color));
            if !it.key.is_empty() {
                let kw = it.key.chars().count() as f64 * 0.56 * f64::from(style::TYPE_SMALL);
                draws.push(text(
                    b.x + b.w - 10. - kw,
                    b.y + 7.,
                    it.key,
                    style::TYPE_SMALL,
                    color,
                ));
            }
        }
    }

    /// Own units on the inspected tile, in view order.
    fn stack_at_cursor(&self, v: &HumanView) -> Vec<Entity> {
        v.units
            .iter()
            .filter(|e| {
                v.observation
                    .units
                    .iter()
                    .any(|u| u.index == e.slot && self.inspect == Some((u.x, u.y)))
            })
            .copied()
            .collect()
    }

    /// The order strip under the map (#312): the active unit's orders that
    /// the client supports, each sending exactly what its key sends.
    fn strip_buttons(&self, v: &HumanView) -> Vec<Button> {
        if !self.enhancements.order_strip || v.phase != Phase::Human {
            return Vec::new();
        }
        let (_, my, _, mh) = self.map_rect();
        let y = my + mh + 6.;
        let mut x = 84.;
        [
            (
                if self.moving { "View [V]" } else { "Move [V]" },
                100.,
                Action::Mode,
            ),
            ("Wait [W]", 100., Action::Wait),
            ("Skip [Space]", 130., Action::Skip),
            ("Found city [B]", 140., Action::Found),
            ("Activate [A]", 120., Action::Activate),
        ]
        .into_iter()
        .map(|(label, w, action)| {
            let b = Button {
                x,
                y,
                w,
                h: 34.,
                label: label.into(),
                action,
            };
            x += w + 6.;
            b
        })
        .collect()
    }

    /// The order strip's frame and caption.
    pub(super) fn strip_draw(&self, draws: &mut Vec<Draw>, v: &HumanView) {
        if !self.enhancements.order_strip {
            return;
        }
        let (mx, my, mw, mh) = self.map_rect();
        draws.push(rect(mx, my + mh, mw, STRIP_H, style::STATUS_BG));
        draws.push(rect(mx, my + mh, mw, 1., style::GOLD_DIM));
        draws.push(text(
            mx + 8.,
            my + mh + 16.,
            if v.phase == Phase::Human {
                "Orders:"
            } else {
                "Orders wait for your turn."
            },
            style::TYPE_SMALL,
            style::TEXT_BRIGHT,
        ));
    }

    /// The status panel `(x, y, w, h)` in the band.
    fn status_rect() -> (f64, f64, f64, f64) {
        STATUS_PANEL
    }

    /// The info panel `(x, y, w, h)`: it stretches from the status panel to
    /// the circular overview.
    fn info_rect(&self) -> (f64, f64, f64, f64) {
        let (ox, _, _, _) = self.overview_rect();
        (INFO_X, STATUS_PANEL.1, ox - INFO_X - 8., STATUS_PANEL.3)
    }

    /// The overview cell `(x, y, w, h)` at the right of the band.
    fn overview_rect(&self) -> (f64, f64, f64, f64) {
        (
            self.stage_w - OVERVIEW_W - 4.,
            MENU_H + 4.,
            OVERVIEW_W - 4.,
            BAND_H - 8.,
        )
    }

    /// Centre and radius of the circular overview.
    fn overview_circle(&self) -> (f64, f64, f64) {
        let (x, y, w, h) = self.overview_rect();
        (x + w / 2., y + h / 2., (w.min(h) / 2. - 12.).max(8.))
    }

    /// Buttons of the top band (#439): the end-of-turn slot (or the pacer),
    /// the stack under the inspected tile, the overview's scroll arrows and
    /// the optional dock's message link.
    pub(super) fn band_buttons(&self, v: &HumanView) -> Vec<Button> {
        let mut b = self.strip_buttons(v);
        let (ix, iy, iw, ih) = self.info_rect();
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
        for (i, e) in self.stack_at_cursor(v).iter().take(3).enumerate() {
            add(
                ix + iw - 190.,
                iy + 34. + i as f64 * 26.,
                182.,
                24.,
                format!("Activate unit {}", e.slot),
                Action::Unit(*e),
            );
        }
        if v.phase == Phase::Opponents {
            add(
                ix + 6.,
                iy + ih - 48.,
                150.,
                40.,
                if self.advancing {
                    "Pause opponents"
                } else {
                    "Continue opponents"
                }
                .into(),
                Action::Work,
            );
        } else {
            add(
                ix + 6.,
                iy + ih - 48.,
                150.,
                40.,
                "End turn [Ctrl+N]".into(),
                Action::End,
            );
        }
        add(
            ix + iw - 58.,
            iy + ih - 28.,
            24.,
            20.,
            "Scroll map left".into(),
            Action::Pan(iw * 0.6, 0.),
        );
        add(
            ix + iw - 30.,
            iy + ih - 28.,
            24.,
            20.,
            "Scroll map right".into(),
            Action::Pan(-iw * 0.6, 0.),
        );
        if let Some((x, y, w, h)) = self.dock_rect() {
            add(
                x + 6.,
                y + h - 32.,
                w - 12.,
                26.,
                format!("All messages ({})", self.notice_log.len()),
                Action::Messages,
            );
        }
        if let Some(sx) = self.side_x() {
            add(
                sx + 6.,
                768. - 8. - 30.,
                CW,
                26.,
                format!("All messages ({})", self.notice_log.len()),
                Action::Messages,
            );
        }
        b
    }

    /// The date as the original writes it, with the game's `Labels.txt`
    /// (#330); `None` while the scenario calendar is not located.
    pub(super) fn date_label(&self, o: &engine::player::Observation) -> Option<String> {
        let (n, style) = o.date?;
        let labels = self
            .loaded
            .as_ref()
            .and_then(|l| l.resources.labels.as_ref());
        Some(engine::calendar::date_text(n, style, labels))
    }

    /// A date is part of the title (the game's calendar is located).
    fn date_label_shown(&self) -> bool {
        self.view()
            .is_some_and(|v| self.date_label(&v.observation).is_some())
    }

    /// The side column (#337): this turn's messages in full (wrapped), then
    /// earlier ones newest first, as many as fit. Presentation only, like
    /// the dock it replaces on a wide stage.
    pub(super) fn side_draw(&self, draws: &mut Vec<Draw>, v: &HumanView) {
        let Some(x) = self.side_x() else {
            return;
        };
        let top = self.frame_top();
        let w = SIDE_W;
        let h = 768. - 8. - top;
        style::panel_frame(draws, x, top, w, h, false);
        let cx = x + 6.;
        let bottom = top + h - 44.;
        let mut y = top + 8.;
        let turn = self.notice_turn;
        let header = match turn {
            // "New" only on the turn they arrived and until read.
            Some(t) if self.notice_unread && t == v.observation.turn => {
                format!("Turn {t} — new messages")
            }
            Some(t) => format!("Turn {t} messages"),
            None => "Messages".into(),
        };
        draws.push(text(cx, y, &header, style::TYPE_SMALL, style::TEXT_BRIGHT));
        y += 22.;
        if self.notice_log.is_empty() {
            draws.push(text(
                cx,
                y,
                "No messages yet.",
                style::TYPE_SMALL,
                style::TEXT_DIM,
            ));
        }
        let (now, earlier): (Vec<_>, Vec<_>) = self
            .notice_log
            .iter()
            .rev()
            .partition(|(t, _)| Some(*t) == turn);
        'now: for (_, line) in now.iter().rev() {
            for part in wrap(line, SIDE_CHARS) {
                if y + 16. > bottom {
                    break 'now;
                }
                draws.push(text(cx, y, &part, style::TYPE_SMALL, style::TEXT));
                y += 16.;
            }
            y += 4.;
        }
        if !earlier.is_empty() && y + 42. <= bottom {
            y += 6.;
            draws.push(rect(x, y - 4., w, 1., style::INSET));
            draws.push(rect(x, y - 1., w, 1., style::GOLD_DIM));
            draws.push(text(
                cx,
                y + 4.,
                "Earlier",
                style::TYPE_SMALL,
                style::TEXT_BRIGHT,
            ));
            y += 26.;
            for (t, line) in earlier {
                if y + 16. > bottom {
                    break;
                }
                let mut s = format!("T{t} {line}");
                if s.chars().count() > SIDE_CHARS {
                    s = s.chars().take(SIDE_CHARS - 1).collect::<String>() + "…";
                }
                draws.push(text(cx, y, &s, style::TYPE_SMALL, style::TEXT_DIM));
                y += 16.;
            }
        }
    }

    /// The band's frame, the status panel, the info panel's map report and
    /// the circular overview (decision 0009, #439).
    pub(super) fn band_draw(&self, draws: &mut Vec<Draw>, v: &HumanView) {
        draws.push(Draw::Tile {
            x: 0.,
            y: MENU_H,
            w: self.stage_w,
            h: BAND_H,
            sprite: "band".into(),
        });
        draws.push(rect(0., MENU_H, self.stage_w, 1., style::INSET));
        draws.push(rect(0., MENU_H + 1., self.stage_w, 1., style::BAND_LIGHT));
        draws.push(rect(
            0.,
            MENU_H + BAND_H - 1.,
            self.stage_w,
            1.,
            style::INSET,
        ));
        let (sx, sy, sw, sh) = Self::status_rect();
        style::panel_frame(draws, sx, sy, sw, sh, true);
        self.status_panel_draw(draws, v, (sx, sy, sw, sh));
        let (ix, iy, iw, ih) = self.info_rect();
        style::panel_frame(draws, ix, iy, iw, ih, false);
        self.info_panel_draw(draws, v, (ix, iy, iw, ih));
        self.overview_draw(draws, v);
    }

    /// The status panel: population, cities/units with the date, gold with
    /// the tax/luxury/science digits and the research icon.
    fn status_panel_draw(
        &self,
        draws: &mut Vec<Draw>,
        v: &HumanView,
        (x, y, w, _h): (f64, f64, f64, f64),
    ) {
        let o = &v.observation;
        let own = |owner: u8| owner == o.civ;
        let cities = o.cities.iter().filter(|c| own(c.owner)).count();
        let units = o.units.iter().filter(|u| own(u.owner)).count();
        let population: u64 = o
            .cities
            .iter()
            .filter(|c| own(c.owner))
            .map(|c| u64::from(c.size) * 10_000)
            .sum();
        draws.push(text(
            x + 10.,
            y + 22.,
            &format!("{} People", group_digits(population)),
            style::TYPE_BODY,
            style::TEXT_BRIGHT,
        ));
        draws.push(text(
            x + 10.,
            y + 44.,
            &format!("{cities}C {units}U"),
            style::TYPE_SMALL,
            style::TEXT,
        ));
        if let Some(date) = self.date_label(o) {
            draws.push(text(
                x + w - 10. - estimate(&date, style::TYPE_SMALL),
                y + 44.,
                &date,
                style::TYPE_SMALL,
                style::TEXT,
            ));
        }
        let rates = v.rates.map_or_else(
            || "0.0.0".to_string(),
            |r| format!("{}.{}.{}", r.tax, r.luxury, r.science),
        );
        draws.push(text(
            x + 10.,
            y + 64.,
            &format!("{} Gold {rates}", o.treasury),
            style::TYPE_SMALL,
            style::TEXT_BRIGHT,
        ));
        // The research icon: a small flask over the panel stone. The install
        // exposes no font or icon sheet here, so the glyph is drawn.
        let fx = x + 10.;
        let fy = y + 76.;
        draws.push(rect(fx + 9., fy, 5., 7., style::TEXT));
        draws.push(rect(fx + 6., fy + 7., 11., 10., style::TEXT));
        draws.push(rect(fx + 7., fy + 11., 9., 5., style::GOLD));
        let topic = o
            .research
            .target
            .map_or_else(|| "No research".to_string(), |t| self.tech_name(t));
        let cost = o.research.cost.unwrap_or(0);
        draws.push(text(
            x + 30.,
            y + 86.,
            &topic,
            style::TYPE_SMALL,
            style::TEXT_BRIGHT,
        ));
        draws.push(text(
            x + 30.,
            y + 102.,
            &format!(
                "{} / {}",
                o.research.progress,
                if cost > 0 {
                    cost.to_string()
                } else {
                    "—".to_string()
                }
            ),
            style::TYPE_SMALL,
            style::TEXT_DIM,
        ));
    }

    /// The info panel: the mode across the top, the location and terrain of
    /// the inspected tile, the active unit, the end-of-turn prompt and the
    /// overview arrows.
    fn info_panel_draw(
        &self,
        draws: &mut Vec<Draw>,
        v: &HumanView,
        (x, y, w, h): (f64, f64, f64, f64),
    ) {
        let o = &v.observation;
        let idle = !o.units.iter().any(|u| u.owner == o.civ && u.moves_left > 0);
        let mode = match v.phase {
            Phase::Human if v.decision.is_some() => "Choice Required",
            Phase::Human if self.moving => "Moving Units",
            Phase::Human => "Viewing Pieces",
            Phase::Opponents => "Opponents Playing",
            Phase::Eliminated => "Civilization Eliminated",
            Phase::TurnLimit => "Turn Limit Reached",
            Phase::GameOver => "Game Over",
        };
        draws.push(text(
            x + (w - estimate(mode, style::TYPE_BODY)) / 2.,
            y + 20.,
            mode,
            style::TYPE_BODY,
            style::TEXT_BRIGHT,
        ));
        // The inspected tile: the viewing cursor, else the selected unit.
        let active = self
            .active
            .and_then(|e| o.units.iter().find(|u| u.index == e.slot));
        let at = self
            .inspect
            .or_else(|| active.map(|u| (u.x, u.y)))
            .or_else(|| {
                o.units
                    .iter()
                    .find(|u| u.owner == o.civ)
                    .map(|u| (u.x, u.y))
            });
        if let Some((tx, ty)) = at {
            draws.push(text(
                x + 10.,
                y + 42.,
                &format!("Loc: ({tx}, {ty})"),
                style::TYPE_SMALL,
                style::TEXT_BRIGHT,
            ));
            let rules = self.loaded.as_ref().map(|l| &l.resources.rules);
            if let (Some(tile), Some(rules)) = (
                o.seen.get(paths::stored(o, tx, ty)).copied().flatten(),
                rules,
            ) {
                let terrain = rules
                    .base_terrain_names()
                    .get(tile.terrain as usize)
                    .map_or_else(|| "Terrain".to_string(), ToString::to_string);
                let mut parts = vec![terrain];
                if tile.river {
                    parts.push("River".into());
                }
                if tile.rail {
                    parts.push("Railroad".into());
                } else if tile.road {
                    parts.push("Road".into());
                }
                draws.push(text(
                    x + 10.,
                    y + 58.,
                    &format!("({})", parts.join(", ")),
                    style::TYPE_SMALL,
                    style::TEXT,
                ));
            }
        }
        let rules = self.loaded.as_ref().map(|l| &l.resources.rules);
        if let (Some(u), Some(rules)) = (active, rules) {
            let name = rules
                .unit_names()
                .get(u.type_id as usize)
                .map_or_else(|| "Unit".to_string(), ToString::to_string);
            let vet = if u.veteran { " (veteran)" } else { "" };
            draws.push(text(
                x + 200.,
                y + 42.,
                &format!("{name} #{}{vet}", u.index),
                style::TYPE_SMALL,
                style::GOLD,
            ));
            let health = v
                .unit_health
                .iter()
                .find(|(slot, _, _)| *slot == u.index)
                .map_or_else(String::new, |(_, hp, max)| format!(" · {hp}/{max} hp"));
            draws.push(text(
                x + 200.,
                y + 58.,
                &format!("Moves {}{health}", tips::moves_text(u.moves_left)),
                style::TYPE_SMALL,
                style::TEXT,
            ));
        } else {
            draws.push(text(
                x + 200.,
                y + 42.,
                "No unit selected",
                style::TYPE_SMALL,
                style::TEXT_DIM,
            ));
        }
        // Units on the inspected tile, as small stone keys (the original
        // shows a stack chooser when units share a tile).
        for (i, e) in self.stack_at_cursor(v).iter().take(3).enumerate() {
            let kx = x + w - 190.;
            let ky = y + 34. + i as f64 * 26.;
            draws.push(rect(kx, ky, 182., 24., style::INSET));
            draws.push(rect(kx + 1., ky + 1., 180., 22., style::BAND_KEY));
            draws.push(rect(kx + 1., ky + 1., 180., 1., style::BAND_LIGHT));
            let label = format!("Activate unit {}", e.slot);
            draws.push(text(
                kx + 8.,
                ky + 5.,
                &label,
                style::TYPE_SMALL,
                style::MENU_TEXT,
            ));
        }
        // The end-of-turn prompt, or what the opponents are doing.
        let (line, sub) = match v.phase {
            Phase::Human if v.decision.is_some() => {
                ("Resolve the required choice", Some("Advisors reopen it"))
            }
            Phase::Human if idle => ("End of Turn", Some("(Press ENTER)")),
            Phase::Human => (
                if self.moving {
                    "Moving Units"
                } else {
                    "Viewing Pieces"
                },
                None,
            ),
            Phase::Opponents => (
                if self.advancing {
                    "Opponents are playing"
                } else {
                    "Opponents paused"
                },
                None,
            ),
            Phase::Eliminated => ("No cities or units remain", None),
            Phase::TurnLimit => ("Turn limit reached", Some("Save a checkpoint")),
            Phase::GameOver => ("Game over: scenario event", None),
        };
        draws.push(text(
            x + 10.,
            y + h - 42.,
            line,
            style::TYPE_SMALL,
            if line == "End of Turn" {
                style::GOLD
            } else {
                style::TEXT_BRIGHT
            },
        ));
        if let Some(sub) = sub {
            draws.push(text(
                x + 10.,
                y + h - 26.,
                sub,
                style::TYPE_SMALL,
                style::TEXT_DIM,
            ));
        }
        // The overview arrows: raised stone keys that scroll the map.
        for (ax, glyph) in [(x + w - 58., "<"), (x + w - 30., ">")] {
            let ay = y + h - 28.;
            draws.push(rect(ax, ay, 24., 20., style::INSET));
            draws.push(rect(ax + 1., ay + 1., 22., 18., style::BAND_KEY));
            draws.push(rect(ax + 1., ay + 1., 22., 1., style::BAND_LIGHT));
            draws.push(text(
                ax + 9.,
                ay + 3.,
                glyph,
                style::TYPE_SMALL,
                style::MENU_TEXT,
            ));
        }
    }

    /// The circular overview: the known world clipped to a disc, the view
    /// outline and the scroll arrows' ring.
    fn overview_draw(&self, draws: &mut Vec<Draw>, v: &HumanView) {
        let (x, y, w, h) = self.overview_rect();
        style::panel_frame(draws, x, y, w, h, false);
        let (cx, cy, r) = self.overview_circle();
        // A dark disc, row by row, before the known-world cells: the original
        // overview is an opaque circle rather than a square minimap.
        let mut yy = -r;
        while yy < r {
            let half = (r * r - yy * yy).sqrt();
            draws.push(rect(cx - half, cy + yy, 2. * half, 2., style::FIELD));
            yy += 2.;
        }
        let m = (cx - r, cy - r, 2. * r, 2. * r);
        let o = &v.observation;
        let (cw, ch) = Self::minimap_cell(o, m);
        let cols = usize::from(o.w2 / 2);
        for row in 0..usize::from(o.h) {
            let row_y = m.1 + row as f64 * ch + ch / 2.;
            let dy = row_y - cy;
            if dy.abs() >= r {
                continue;
            }
            let half = (r * r - dy * dy).sqrt();
            let (left, right) = (cx - half, cx + half);
            let shift = if row % 2 == 1 { cw / 2. } else { 0. };
            let mut run: Option<(usize, &str)> = None;
            for col in 0..=cols {
                let here = (col < cols)
                    .then(|| o.seen.get(row * cols + col).copied().flatten())
                    .flatten()
                    .map(|t| terrain_color(t.terrain));
                if run.map(|(_, c)| Some(c)) != Some(here) {
                    if let Some((start, c)) = run {
                        let x0 = (m.0 + shift + start as f64 * cw).max(left);
                        let x1 = (m.0 + shift + col as f64 * cw).min(right);
                        if x1 > x0 {
                            draws.push(rect(x0, m.1 + row as f64 * ch, x1 - x0, ch, c));
                        }
                    }
                    run = here.map(|c| (col, c));
                }
            }
        }
        let dot = |x: u16, y: u16| (m.0 + f64::from(x) * cw / 2., m.1 + f64::from(y) * ch);
        let inside = |px: f64, py: f64| (px - cx).powi(2) + (py - cy).powi(2) <= (r - 1.).powi(2);
        for u in &o.units {
            let (px, py) = dot(u.x, u.y);
            if inside(px, py) {
                draws.push(rect(px, py, 2., 2., civ_color(u.owner)));
            }
        }
        for c in &o.cities {
            let (px, py) = dot(c.x, c.y);
            if inside(px, py) {
                draws.push(rect(px - 1., py - 1., 4., 4., civ_color(c.owner)));
            }
        }
        // The map view's outline, clipped to the disc.
        let z = self.zoom;
        let lx = |sx: f64| ((sx - self.pan.0) / (32. * z)).clamp(0., f64::from(o.w2));
        let ly = |sy: f64| ((sy - self.pan.1) / (16. * z)).clamp(0., f64::from(o.h));
        let map = self.map_rect();
        let (x0, y0) = (m.0 + lx(map.0) * cw / 2., m.1 + ly(map.1) * ch);
        let (x1, y1) = (
            (m.0 + lx(map.0 + map.2) * cw / 2.).min(m.0 + m.2 - 1.),
            (m.1 + ly(map.1 + map.3) * ch).min(m.1 + m.3 - 1.),
        );
        for (px, py, pw, ph) in [
            (x0, y0, x1 - x0, 1.),
            (x0, y1, x1 - x0 + 1., 1.),
            (x0, y0, 1., y1 - y0),
            (x1, y0, 1., y1 - y0),
        ] {
            if inside(px, py) {
                draws.push(rect(px, py, pw.max(1.), ph.max(1.), style::TEXT_BRIGHT));
            }
        }
        // A thin dark rim around the disc, drawn in short segments.
        for i in 0..48 {
            let a = std::f64::consts::TAU * i as f64 / 48.;
            let b = std::f64::consts::TAU * (i + 1) as f64 / 48.;
            draws.push(Draw::Line {
                x: cx + a.cos() * r,
                y: cy + a.sin() * r,
                x2: cx + b.cos() * r,
                y2: cy + b.sin() * r,
                color: style::INSET.into(),
                width: 2.,
            });
        }
    }

    /// The optional message dock (#313): a floating panel over the map's
    /// right side, newest first, as many rows as fit.
    pub(super) fn dock_rect(&self) -> Option<(f64, f64, f64, f64)> {
        if !self.enhancements.message_dock || self.side_x().is_some() {
            return None;
        }
        let (mx, my, mw, mh) = self.map_rect();
        let h = (mh - 48.).min(300.);
        Some((mx + mw - 232., my + 24., 224., h))
    }

    pub(super) fn dock_draw(&self, draws: &mut Vec<Draw>) {
        let Some((x, y, w, h)) = self.dock_rect() else {
            return;
        };
        style::panel_frame(draws, x, y, w, h, false);
        draws.push(text(
            x + 8.,
            y + 18.,
            "Messages",
            style::TYPE_SMALL,
            style::TEXT_BRIGHT,
        ));
        let rows = ((h - 40.) / 16.).floor().max(0.) as usize;
        if self.notice_log.is_empty() {
            draws.push(text(
                x + 8.,
                y + 38.,
                "No messages yet.",
                style::TYPE_SMALL,
                style::TEXT_DIM,
            ));
            return;
        }
        for (i, (turn, line)) in self.notice_log.iter().rev().take(rows).enumerate() {
            let mut s = format!("T{turn} {line}");
            if s.chars().count() > 31 {
                s = s.chars().take(30).collect::<String>() + "…";
            }
            draws.push(text(
                x + 8.,
                y + 38. + i as f64 * 16.,
                &s,
                style::TYPE_SMALL,
                style::TEXT,
            ));
        }
    }

    /// Minimap cell size `(w, h)` for the observed map.
    fn minimap_cell(o: &engine::player::Observation, m: (f64, f64, f64, f64)) -> (f64, f64) {
        let cols = f64::from(o.w2 / 2).max(1.);
        let rows = f64::from(o.h).max(1.);
        (m.2 / (cols + 0.5), m.3 / rows)
    }

    /// A click on the minimap centres the map there; it never selects or
    /// orders anything. `false` when the point is not on the minimap.
    pub(super) fn minimap_click(&mut self, x: f64, y: f64) -> bool {
        let Some(v) = self.view() else { return false };
        let (cx, cy, r) = self.overview_circle();
        if !self.map_surface() || (x - cx).powi(2) + (y - cy).powi(2) > r * r {
            return false;
        }
        let m = (cx - r, cy - r, 2. * r, 2. * r);
        let (cw, ch) = Self::minimap_cell(&v.observation, m);
        let lx = (x - m.0) / (cw / 2.);
        let ly = (y - m.1) / ch;
        let (ax, ay) = self.anchor();
        self.pan = (ax - lx * 32. * self.zoom, ay - ly * 16. * self.zoom);
        self.status = "Map centred from the overview.".into();
        true
    }
}

/// Rough width of a string at one type step, for right-aligned band text.
fn estimate(s: &str, size: u32) -> f64 {
    s.chars().count() as f64 * 0.52 * f64::from(size)
}

/// Population grouped as the original writes it (`10,000`).
fn group_digits(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Characters per side-column line (the dock's 220 px measure).
const SIDE_CHARS: usize = 31;

/// Greedy word wrap to `width` characters; longer words are cut.
fn wrap(line: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in line.split_whitespace() {
        let mut word: String = word.into();
        while word.chars().count() > width {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            out.push(word.chars().take(width).collect());
            word = word.chars().skip(width).collect();
        }
        let n = cur.chars().count();
        if n > 0 && n + 1 + word.chars().count() > width {
            out.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(&word);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}
