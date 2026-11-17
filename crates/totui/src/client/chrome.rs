//! Layout B chrome (docs/ui/layout-b.md): the menu bar (#310) and the right
//! column with minimap, status, unit and city boxes (#311).
//!
//! Everything here is presentation. Every menu item and column button runs an
//! `Action` the client already had before layout B, and every key keeps its
//! meaning; menus only make the commands visible in one place. The minimap is
//! drawn from the civilization's filtered view and a click on it only scrolls.
use super::*;

/// Height of the menu bar along the top of the map screen.
pub(super) const MENU_H: f64 = 26.;
/// The map viewport `(x, y, w, h)`.
pub(super) const MAP: (f64, f64, f64, f64) = (8., 34., 768., 682.);
/// The right column `(x, y, w, h)`.
pub(super) const COLUMN: (f64, f64, f64, f64) = (784., 34., 232., 682.);
/// The minimap inside the column's top box.
const MINIMAP: (f64, f64, f64, f64) = (790., 40., 220., 126.);
/// Left edge and width of text and buttons inside the column.
const CX: f64 = 790.;
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

/// Whether `(x, y)` is on the map viewport.
pub(super) fn on_map(x: f64, y: f64) -> bool {
    (MAP.0..MAP.0 + MAP.2).contains(&x) && (MAP.1..MAP.1 + MAP.3).contains(&y)
}

/// Whether a tile centre `(px, py)` is far enough inside the viewport for
/// its unit or city sprite to be drawn and hit-tested.
pub(super) fn in_view(px: f64, py: f64) -> bool {
    (MAP.0 + 32.0..MAP.0 + MAP.2 - 32.).contains(&px)
        && (MAP.1 + 50.0..MAP.1 + MAP.3 - 40.).contains(&py)
}

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
    (title.chars().count() as f64 * 0.62 * f64::from(style::TYPE_BODY) + 22.).ceil()
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
            let x = title_x(m).min(1024. - DROP_W - 4.);
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

    /// The menu bar, drawn under the column and map: its background and the
    /// window title at the right end.
    pub(super) fn menu_bar_draw(&self, draws: &mut Vec<Draw>, title: &str) {
        draws.push(rect(0., 0., 1024., MENU_H, style::MENU_BAR));
        draws.push(rect(0., MENU_H, 1024., 1., style::MENU_EDGE));
        // The menu bar already says which application this is.
        let title = title.strip_prefix("ToTEngine — ").unwrap_or(title);
        let w = title.chars().count() as f64 * 0.58 * f64::from(style::TYPE_SMALL);
        draws.push(text(
            // The browser's licence link sits at the top-right corner.
            (826. - w).max(title_x(MENUS.len()) + 8.),
            7.,
            title,
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
            let x = title_x(m);
            if on {
                draws.push(rect(x, 1., title_w(title), MENU_H - 2., style::MENU_SELECT));
            }
            draws.push(text(
                x + 11.,
                5.,
                title,
                style::TYPE_BODY,
                if on {
                    style::MENU_SELECT_TEXT
                } else {
                    style::MENU_TEXT
                },
            ));
        }
        let Some(m) = self.menu else { return };
        let items = self.menu_items(m);
        let n = items.len() as f64;
        let x = title_x(m).min(1024. - DROP_W - 4.);
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

    /// Buttons in the right column: the stack under the cursor, the selected
    /// city's two entries and the end-of-turn slot.
    pub(super) fn column_buttons(&self, v: &HumanView) -> Vec<Button> {
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
        for (i, e) in v
            .units
            .iter()
            .filter(|e| {
                v.observation
                    .units
                    .iter()
                    .any(|u| u.index == e.slot && self.inspect == Some((u.x, u.y)))
            })
            .take(3)
            .enumerate()
        {
            add(
                CX,
                400. + i as f64 * 28.,
                CW,
                26.,
                format!("Activate unit {}", e.slot),
                Action::Unit(*e),
            );
        }
        if let Some(city) = self.city {
            add(CX, 564., CW, 28., "Manage city".into(), Action::OpenCity);
            add(
                CX,
                596.,
                CW,
                28.,
                "Choose production".into(),
                Action::Production(city),
            );
        }
        if v.phase == Phase::Opponents {
            add(
                CX,
                672.,
                CW,
                38.,
                if self.advancing {
                    "Pause opponents"
                } else {
                    "Continue opponents"
                }
                .into(),
                Action::Work,
            );
        } else {
            add(CX, 672., CW, 38., "End turn [Ctrl+N]".into(), Action::End);
        }
        b
    }

    /// The column's frames, minimap and text (its buttons draw with the rest).
    pub(super) fn column_draw(&self, draws: &mut Vec<Draw>, v: &HumanView) {
        let (x, y, w, h) = COLUMN;
        draws.push(Draw::Tile {
            x,
            y,
            w,
            h,
            sprite: "panel".into(),
        });
        for (top, bottom) in [
            (34., 172.),
            (176., 300.),
            (304., 480.),
            (484., 630.),
            (634., 716.),
        ] {
            draws.push(rect(x, bottom, w, 1., style::INSET));
            if top > 34. {
                draws.push(rect(x, top - 1., w, 1., style::GOLD_DIM));
            }
        }
        self.minimap_draw(draws, v);
        let o = &v.observation;
        let mut line = |y: f64, s: &str, size: u32, color: &str| {
            draws.push(text(CX, y, s, size, color));
        };
        // Status: turn, government, treasury, rates and research.
        line(
            182.,
            &format!(
                "Turn {} · {}",
                o.turn,
                routes::government_name(o.government)
            ),
            style::TYPE_BODY,
            style::TEXT_BRIGHT,
        );
        line(
            204.,
            &format!("Treasury: {} gold", o.treasury),
            style::TYPE_BODY,
            style::TEXT,
        );
        if let Some(r) = &v.rates {
            line(
                226.,
                &format!(
                    "Tax {}% · Lux {}% · Sci {}%",
                    u32::from(r.tax) * 10,
                    u32::from(r.luxury) * 10,
                    u32::from(r.science) * 10
                ),
                style::TYPE_SMALL,
                style::TEXT_DIM,
            );
        }
        line(
            246.,
            &o.research.target.map_or_else(
                || "Topic: none chosen".into(),
                |t| format!("Topic: {}", self.tech_name(t)),
            ),
            style::TYPE_SMALL,
            style::TEXT_BRIGHT,
        );
        let cost = o.research.cost.unwrap_or(0);
        draws.push(rect(CX, 266., CW, 8., style::INSET));
        if cost > 0 {
            let done = (f64::from(o.research.progress) / f64::from(cost)).clamp(0., 1.);
            draws.push(rect(CX, 266., CW * done, 8., style::LINK));
        }
        draws.push(text(
            CX,
            280.,
            &format!(
                "Research: {} / {}",
                o.research.progress,
                o.research
                    .cost
                    .map_or_else(|| "unavailable".into(), |n| n.to_string())
            ),
            style::TYPE_SMALL,
            style::TEXT_DIM,
        ));
        // The active unit.
        let rules = self.loaded.as_ref().map(|l| &l.resources.rules);
        if let (Some(u), Some(rules)) = (
            self.active
                .and_then(|e| o.units.iter().find(|u| u.index == e.slot)),
            rules,
        ) {
            let name = rules
                .unit_names()
                .get(u.type_id as usize)
                .map_or_else(|| "Unit".to_string(), ToString::to_string);
            let vet = if u.veteran { " (veteran)" } else { "" };
            draws.push(text(
                CX,
                310.,
                &format!("{name} #{}{vet}", u.index),
                style::TYPE_BODY,
                style::GOLD,
            ));
            let health = v
                .unit_health
                .iter()
                .find(|(slot, _, _)| *slot == u.index)
                .map_or_else(String::new, |(_, hp, max)| format!(" · health {hp}/{max}"));
            draws.push(text(
                CX,
                332.,
                &format!("Moves {}{health}", tips::moves_text(u.moves_left)),
                style::TYPE_SMALL,
                style::TEXT,
            ));
            let at = paths::stored(o, u.x, u.y);
            if let Some(tile) = o.seen.get(at).copied().flatten() {
                let terrain = rules
                    .base_terrain_names()
                    .get(tile.terrain as usize)
                    .map_or_else(|| "Terrain".to_string(), ToString::to_string);
                let extra = if tile.rail {
                    " · railroad"
                } else if tile.road {
                    " · road"
                } else {
                    ""
                };
                draws.push(text(
                    CX,
                    350.,
                    &format!("{terrain} ({}, {}){extra}", u.x, u.y),
                    style::TYPE_SMALL,
                    style::TEXT_DIM,
                ));
            }
            let others = o
                .units
                .iter()
                .filter(|w| (w.x, w.y) == (u.x, u.y) && w.index != u.index)
                .count();
            if others > 0 {
                draws.push(text(
                    CX,
                    368.,
                    &format!(
                        "{others} more unit{} here",
                        if others == 1 { "" } else { "s" }
                    ),
                    style::TYPE_SMALL,
                    style::TEXT_DIM,
                ));
            }
        } else {
            draws.push(text(
                CX,
                310.,
                "No unit selected",
                style::TYPE_SMALL,
                style::TEXT_DIM,
            ));
        }
        // The selected own city.
        if let Some(c) = self
            .city
            .and_then(|e| o.cities.iter().find(|c| c.index == e.slot))
        {
            draws.push(text(
                CX,
                490.,
                &format!("{} · size {}", c.name, c.size),
                style::TYPE_BODY,
                style::GOLD,
            ));
            draws.push(text(
                CX,
                512.,
                &format!(
                    "{} · shields {}",
                    self.item_name(c.building.unwrap_or(0)),
                    c.shield_box.unwrap_or(0)
                ),
                style::TYPE_SMALL,
                style::TEXT,
            ));
            if let Some(e) = o.city_economy.iter().find(|e| e.index == c.index) {
                draws.push(text(
                    CX,
                    528.,
                    &format!(
                        "Food {:+} · shields {} · sci {}",
                        e.food_surplus, e.net_shields, e.finance.science
                    ),
                    style::TYPE_SMALL,
                    style::TEXT_DIM,
                ));
                draws.push(text(
                    CX,
                    544.,
                    &format!(
                        "Tax {} · happy {} · unhappy {}",
                        e.finance.taxes, e.happy, e.unhappy
                    ),
                    style::TYPE_SMALL,
                    style::TEXT_DIM,
                ));
            }
        }
        // The turn: what the player is doing, and the end-of-turn prompt.
        let idle = !o.units.iter().any(|u| u.owner == o.civ && u.moves_left > 0);
        let phase = match v.phase {
            Phase::Human if v.decision.is_some() => "Choice required",
            Phase::Human if self.moving => "Moving units",
            Phase::Human if idle => "End of turn",
            Phase::Human => "Viewing pieces",
            Phase::Opponents => "Opponents playing",
            Phase::Eliminated => "Your civilization is eliminated",
            Phase::TurnLimit => "Turn limit reached",
        };
        draws.push(text(
            CX,
            640.,
            phase,
            style::TYPE_BODY,
            if phase == "End of turn" {
                style::GOLD
            } else {
                style::TEXT
            },
        ));
        if phase == "End of turn" {
            draws.push(text(
                CX,
                656.,
                "(Press Enter or Ctrl+N)",
                style::TYPE_SMALL,
                style::TEXT_DIM,
            ));
        }
    }

    /// Minimap cell size `(w, h)` for the observed map.
    fn minimap_cell(o: &engine::player::Observation) -> (f64, f64) {
        let cols = f64::from(o.w2 / 2).max(1.);
        let rows = f64::from(o.h).max(1.);
        (MINIMAP.2 / (cols + 0.5), MINIMAP.3 / rows)
    }

    fn minimap_draw(&self, draws: &mut Vec<Draw>, v: &HumanView) {
        let (mx, my, mw, mh) = MINIMAP;
        draws.push(rect(mx, my, mw, mh, style::FIELD));
        let o = &v.observation;
        let (cw, ch) = Self::minimap_cell(o);
        let cols = usize::from(o.w2 / 2);
        // One rectangle per run of equal colour in a row keeps frames small.
        for row in 0..usize::from(o.h) {
            let shift = if row % 2 == 1 { cw / 2. } else { 0. };
            let mut run: Option<(usize, &str)> = None;
            for col in 0..=cols {
                let here = (col < cols)
                    .then(|| o.seen.get(row * cols + col).copied().flatten())
                    .flatten()
                    .map(|t| terrain_color(t.terrain));
                if run.map(|(_, c)| Some(c)) != Some(here) {
                    if let Some((start, c)) = run {
                        draws.push(rect(
                            mx + shift + start as f64 * cw,
                            my + row as f64 * ch,
                            (col - start) as f64 * cw,
                            ch,
                            c,
                        ));
                    }
                    run = here.map(|c| (col, c));
                }
            }
        }
        let dot = |x: u16, y: u16| (mx + f64::from(x) * cw / 2., my + f64::from(y) * ch);
        for u in &o.units {
            let (px, py) = dot(u.x, u.y);
            draws.push(rect(px, py, 2., 2., civ_color(u.owner)));
        }
        for c in &o.cities {
            let (px, py) = dot(c.x, c.y);
            draws.push(rect(px - 1., py - 1., 4., 4., civ_color(c.owner)));
        }
        // The map view's outline, clipped to the minimap.
        let z = self.zoom;
        let lx = |sx: f64| ((sx - self.pan.0) / (32. * z)).clamp(0., f64::from(o.w2));
        let ly = |sy: f64| ((sy - self.pan.1) / (16. * z)).clamp(0., f64::from(o.h));
        let (x0, y0) = (mx + lx(MAP.0) * cw / 2., my + ly(MAP.1) * ch);
        let (x1, y1) = (
            (mx + lx(MAP.0 + MAP.2) * cw / 2.).min(mx + mw - 1.),
            (my + ly(MAP.1 + MAP.3) * ch).min(my + mh - 1.),
        );
        for (x, y, w, h) in [
            (x0, y0, x1 - x0, 1.),
            (x0, y1, x1 - x0 + 1., 1.),
            (x0, y0, 1., y1 - y0),
            (x1, y0, 1., y1 - y0),
        ] {
            draws.push(rect(x, y, w.max(1.), h.max(1.), style::TEXT_BRIGHT));
        }
    }

    /// A click on the minimap centres the map there; it never selects or
    /// orders anything. `false` when the point is not on the minimap.
    pub(super) fn minimap_click(&mut self, x: f64, y: f64) -> bool {
        let (mx, my, mw, mh) = MINIMAP;
        if !self.map_surface() || !(mx..mx + mw).contains(&x) || !(my..my + mh).contains(&y) {
            return false;
        }
        let Some(v) = self.view() else { return false };
        let (cw, ch) = Self::minimap_cell(&v.observation);
        let lx = (x - mx) / (cw / 2.);
        let ly = (y - my) / ch;
        self.pan = (380. - lx * 32. * self.zoom, 390. - ly * 16. * self.zoom);
        self.status = "Map centred from the minimap.".into();
        true
    }
}
