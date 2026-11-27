//! Shared own-city workspace; adapters receive only filtered presentation.
//!
//! Decision 0009 / #439: the original draws floating stone panels over the
//! map (City Resources, Build, radius map, title/panels, Food Storage and the
//! citizen strip). Every string and command the Layout B window had is kept;
//! only the presentation moves.
use super::chrome::group_digits;
use super::style;
use super::*;
use engine::yields::{RADIUS_DX, RADIUS_DY};

/// City Resources panel `(x, y, w, h)` at the top left.
pub(super) const RESOURCES: (f64, f64, f64, f64) = (44., 96., 240., 200.);
/// Build panel at the left, below City Resources.
pub(super) const BUILD: (f64, f64, f64, f64) = (44., 376., 176., 276.);
/// Title panel at the top right: name, population and treasury.
pub(super) const TITLE: (f64, f64, f64, f64) = (608., 88., 388., 78.);
/// Left edge and top of the stacked Info/Support/Improvements/Happy/Close
/// keys.
pub(super) const PANEL_KEY: (f64, f64) = (624., 176.);
/// The info pane at the right.
pub(super) const INFO_PANE: (f64, f64, f64, f64) = (766., 176., 230., 348.);
/// Food Storage panel at the bottom right.
pub(super) const FOOD: (f64, f64, f64, f64) = (770., 584., 226., 122.);
/// Citizen strip along the bottom left.
pub(super) const STRIP: (f64, f64, f64, f64) = (44., 682., 590., 74.);
/// Centre of the city-radius diamonds and the step between tiles.
const RADIUS_CENTRE: (f64, f64) = (500., 466.);
const RADIUS_STEP: (f64, f64) = (60., 33.);
/// Drawn size of one radius tile.
const RADIUS_SIZE: (f64, f64) = (120., 66.);

pub(super) fn spot(k: usize) -> (f64, f64) {
    (
        RADIUS_CENTRE.0 + f64::from(RADIUS_DX[k]) * RADIUS_STEP.0,
        RADIUS_CENTRE.1 + f64::from(RADIUS_DY[k]) * RADIUS_STEP.1,
    )
}

impl Client {
    pub(super) fn close_city_production(&mut self) -> Result<(), String> {
        if let Some(city) = self.view().and_then(|v| v.production_context_city) {
            self.order(Command::CloseProduction { city })?;
        }
        Ok(())
    }
    pub(super) fn city_click(&mut self, x: f64, y: f64) -> Result<(), String> {
        let nearest = (0..21).find(|&k| {
            let (cx, cy) = spot(k);
            (x - cx).abs() / RADIUS_STEP.0 + (y - cy).abs() / RADIUS_STEP.1 <= 1.
        });
        if let Some(k) = nearest {
            self.city_tile = k as u8;
            let city = self.city.ok_or("Select an own city first")?;
            self.action(Action::Worker(city, k as u8))?;
        }
        Ok(())
    }

    pub(super) fn city_key(&mut self, key: &str) -> Result<(), String> {
        match key {
            "ArrowRight" | "ArrowDown" if self.city_panel == CityPanel::Info => {
                self.city_tile = (self.city_tile + 1) % 21;
                Ok(())
            }
            "ArrowLeft" | "ArrowUp" if self.city_panel == CityPanel::Info => {
                self.city_tile = (self.city_tile + 20) % 21;
                Ok(())
            }
            "PageDown" => self.action(Action::CityStep(1)),
            "PageUp" => self.action(Action::CityStep(-1)),
            " " if self.city_panel == CityPanel::Info => self.action(Action::Worker(
                self.city.ok_or("Select an own city first")?,
                self.city_tile,
            )),
            "Ctrl+s" => self.action(Action::Save),
            "Ctrl+l" => self.action(Action::Load),
            "?" => self.action(Action::Help),
            _ => Ok(()),
        }
    }

    pub(super) fn city_buttons(&self) -> Vec<Button> {
        let mut b = Vec::new();
        let mut add = |x, y, w, h, label: &str, action| {
            b.push(Button {
                x,
                y,
                w,
                h,
                label: label.into(),
                action,
            })
        };
        // Enter on a freshly opened window returns to the map, so Close is
        // the first focus target; it draws at the bottom of the stack.
        add(
            PANEL_KEY.0,
            PANEL_KEY.1 + 4. * 38.,
            136.,
            34.,
            "Close [Esc]",
            Action::CloseCity,
        );
        // The original's stacked panel keys above it.
        for (i, panel) in [
            CityPanel::Info,
            CityPanel::Support,
            CityPanel::Improvements,
            CityPanel::Happy,
        ]
        .into_iter()
        .enumerate()
        {
            add(
                PANEL_KEY.0,
                PANEL_KEY.1 + i as f64 * 38.,
                136.,
                34.,
                panel.label(),
                Action::CityPanel(panel),
            );
        }
        // City cycling arrows in the title panel (PgUp/PgDn keep working).
        add(
            TITLE.0 + TITLE.2 - 92.,
            TITLE.1 + 8.,
            40.,
            20.,
            "PgUp",
            Action::CityStep(-1),
        );
        add(
            TITLE.0 + TITLE.2 - 48.,
            TITLE.1 + 8.,
            40.,
            20.,
            "PgDn",
            Action::CityStep(1),
        );
        if let Some(city) = self.city {
            let (bx, by, _, _) = BUILD;
            add(
                bx + 8.,
                by + 178.,
                160.,
                26.,
                "Choose production",
                Action::Production(city),
            );
            add(
                bx + 8.,
                by + 208.,
                104.,
                26.,
                "Buy production",
                Action::Buy(city),
            );
            add(
                bx + 8.,
                by + 238.,
                160.,
                26.,
                "Improvements / sell",
                Action::Improvements(city),
            );
            add(
                300.,
                626.,
                190.,
                26.,
                if self.city_panel == CityPanel::Info {
                    "Toggle selected tile [Space]"
                } else {
                    "Toggle selected tile"
                },
                Action::Worker(city, self.city_tile),
            );
            add(
                500.,
                626.,
                180.,
                26.,
                "Auto workers (centre tile)",
                Action::Worker(city, 20),
            );
            if let Some(detail) = self
                .view()
                .and_then(|v| v.city_citizens.into_iter().find(|c| c.index == city.slot))
            {
                // The citizens strip: specialists as role keys, six a row.
                for (i, role) in detail.specialists.iter().take(12).enumerate() {
                    let name = match role {
                        1 => "Entertainer",
                        2 => "Tax collector",
                        3 => "Scientist",
                        _ => "Unused",
                    };
                    add(
                        90. + (i % 6) as f64 * 84.,
                        708. + (i / 6) as f64 * 22.,
                        80.,
                        20.,
                        &format!("{}: {name}", i + 1),
                        Action::Specialist(city, i as u8),
                    );
                }
            }
        }
        b.extend(self.city_panel_buttons());
        b
    }

    /// One floating stone panel, framed like the band's inset panels.
    fn city_panel(&self, draws: &mut Vec<Draw>, rect: (f64, f64, f64, f64)) {
        chrome::panel_frame(draws, rect.0, rect.1, rect.2, rect.3, false);
    }

    /// A gold serif section heading, as the original's city panels use.
    fn heading(&self, draws: &mut Vec<Draw>, x: f64, y: f64, label: &str) {
        draws.push(text(x, y, label, style::TYPE_BODY, style::GOLD));
        draws.push(rect(x, y + 20., 40., 1., style::GOLD_DIM));
    }

    /// A small trade/food/shield glyph for the resources rows: the install
    /// exposes no resource icons here (the classic-art branch adds them), so
    /// the marker is drawn.
    fn resource_icon(&self, draws: &mut Vec<Draw>, x: f64, y: f64, kind: usize) {
        match kind {
            // Food: a green sheaf.
            0 => {
                draws.push(rect(x + 3., y + 2., 2., 9., style::MOVE_OK));
                draws.push(rect(x + 7., y + 2., 2., 9., style::MOVE_OK));
                draws.push(rect(x + 2., y + 8., 8., 2., style::MOVE_OK));
            }
            // Taxes: a gold coin.
            1 => {
                draws.push(rect(x + 2., y + 2., 8., 8., style::GOLD));
                draws.push(rect(x + 4., y + 4., 4., 4., style::INSET));
            }
            // Luxury: a purple gem.
            2 => {
                draws.push(rect(x + 3., y + 1., 6., 6., "#c68add"));
                draws.push(rect(x + 4., y + 7., 4., 3., "#8f5aa8"));
            }
            // Science: a blue flask.
            3 => {
                draws.push(rect(x + 5., y, 2., 5., style::TEXT));
                draws.push(rect(x + 3., y + 5., 6., 6., style::LINK));
            }
            // Production: a grey shield.
            _ => {
                draws.push(rect(x + 3., y + 1., 6., 6., "#b9b7ab"));
                draws.push(rect(x + 4., y + 7., 4., 3., "#8f9188"));
            }
        }
    }

    /// One labelled resource row: icon, label and value, as the original's
    /// City Resources panel.
    fn resource_row(
        &self,
        draws: &mut Vec<Draw>,
        x: f64,
        y: f64,
        kind: usize,
        label: &str,
        value: String,
    ) {
        self.resource_icon(draws, x, y + 1., kind);
        draws.push(text(x + 16., y, label, style::TYPE_SMALL, style::TEXT));
        draws.push(text(
            x + 16. + label.chars().count() as f64 * 7. + 6.,
            y,
            &value,
            style::TYPE_SMALL,
            style::TEXT_BRIGHT,
        ));
    }

    pub(super) fn city_draw(&self, draws: &mut Vec<Draw>, v: &HumanView) {
        let Some(city) = self.city else { return };
        let Some(c) = v
            .observation
            .cities
            .iter()
            .find(|c| c.index == city.slot && c.owner == v.observation.civ)
        else {
            return;
        };
        let Some(detail) = v.city_citizens.iter().find(|d| d.index == city.slot) else {
            return;
        };
        // The title panel: name, population and treasury (the original).
        self.city_panel(draws, TITLE);
        draws.push(text(
            TITLE.0 + 14.,
            TITLE.1 + 8.,
            &c.name,
            style::TYPE_TITLE,
            style::GOLD,
        ));
        let population = u64::from(c.size) * 10_000;
        draws.push(text(
            TITLE.0 + 14.,
            TITLE.1 + 38.,
            &format!("Population {}", group_digits(population)),
            style::TYPE_SMALL,
            style::TEXT_BRIGHT,
        ));
        draws.push(text(
            TITLE.0 + 14.,
            TITLE.1 + 56.,
            &format!("Treasury: {} Gold", v.observation.treasury),
            style::TYPE_SMALL,
            style::TEXT,
        ));
        // City Resources: the original's five net rows.
        self.city_panel(draws, RESOURCES);
        self.heading(draws, RESOURCES.0 + 10., RESOURCES.1 + 8., "City Resources");
        let (rx, ry) = (RESOURCES.0 + 10., RESOURCES.1 + 40.);
        if let Some(e) = v
            .observation
            .city_economy
            .iter()
            .find(|e| e.index == city.slot)
        {
            self.resource_row(
                draws,
                rx,
                ry,
                0,
                "Net Food",
                format!("{:+}", e.food_surplus),
            );
            self.resource_row(
                draws,
                rx,
                ry + 30.,
                1,
                "Net Taxes",
                e.finance.taxes.to_string(),
            );
            self.resource_row(
                draws,
                rx,
                ry + 60.,
                2,
                "Net Luxury",
                e.finance.luxuries.to_string(),
            );
            self.resource_row(
                draws,
                rx,
                ry + 90.,
                3,
                "Net Science",
                e.finance.science.to_string(),
            );
            self.resource_row(
                draws,
                rx,
                ry + 120.,
                4,
                "Net Production",
                e.net_shields.to_string(),
            );
            // The Layout B detail stays available below the net rows.
            draws.push(text(
                rx,
                ry + 150.,
                &format!("Trade {}", c.output.unwrap_or_default().trade),
                style::TYPE_SMALL,
                style::TEXT_DIM,
            ));
            draws.push(text(
                rx,
                ry + 164.,
                &format!("Happy {} · unhappy {}", e.happy, e.unhappy),
                style::TYPE_SMALL,
                style::TEXT_DIM,
            ));
            draws.push(text(
                rx,
                ry + 178.,
                &format!("Upkeep {}", e.upkeep),
                style::TYPE_SMALL,
                style::TEXT_DIM,
            ));
        }
        // Build: item, progress, picture, turns and the buy price.
        self.city_panel(draws, BUILD);
        self.heading(draws, BUILD.0 + 10., BUILD.1 + 8., "Build");
        draws.push(text(
            BUILD.0 + 10.,
            BUILD.1 + 34.,
            "Currently producing:",
            style::TYPE_SMALL,
            style::TEXT_DIM,
        ));
        let item_id = c.building;
        let item = item_id
            .map(|id| self.item_name(id))
            .unwrap_or_else(|| "Unavailable".into());
        let mut name: Vec<_> = item.chars().collect();
        if name.len() > 22 {
            name.truncate(21);
            name.push('…');
        }
        draws.push(text(
            BUILD.0 + 10.,
            BUILD.1 + 50.,
            &name.iter().collect::<String>(),
            style::TYPE_BODY,
            style::WHITE,
        ));
        if let Some(p) = v.city_production.iter().find(|p| p.index == city.slot) {
            if let Some(cost) = p.required.filter(|c| *c > 0) {
                let done = (f64::from(p.stored) / f64::from(cost)).clamp(0., 1.);
                draws.push(rect(BUILD.0 + 10., BUILD.1 + 72., 156., 10., style::INSET));
                draws.push(rect(
                    BUILD.0 + 10.,
                    BUILD.1 + 72.,
                    156. * done,
                    10.,
                    style::GOLD,
                ));
            }
            let shields = p.required.map_or_else(
                || format!("Shields: {}", p.stored),
                |cost| format!("Shields: {} / {cost}", p.stored),
            );
            draws.push(text(
                BUILD.0 + 10.,
                BUILD.1 + 88.,
                &shields,
                style::TYPE_SMALL,
                style::TEXT,
            ));
            let estimate = match (p.remaining, p.turns) {
                (Some(0), _) => "Funded — next production phase".into(),
                (Some(n), Some(turns)) => format!("Remaining: {n} (~{turns} turns)"),
                (Some(n), None) => format!("Remaining: {n}; no current output"),
                _ => "No finite shield target".into(),
            };
            draws.push(text(
                BUILD.0 + 10.,
                BUILD.1 + 104.,
                &estimate,
                style::TYPE_SMALL,
                style::TEXT,
            ));
            // The item's picture, as the original's Build panel shows.
            if let Some(id) = item_id.filter(|id| *id > 0) {
                draws.push(Draw::Sprite {
                    x: BUILD.0 + 10.,
                    y: BUILD.1 + 122.,
                    w: 48.,
                    h: 48.,
                    sprite: format!("unit-{id}"),
                });
            }
            if let Ok(q) = &p.hurry {
                draws.push(text(
                    BUILD.0 + 118.,
                    BUILD.1 + 214.,
                    &format!("{} gold", q.price),
                    style::TYPE_SMALL,
                    style::TEXT_BRIGHT,
                ));
            }
        }
        // Food Storage, bottom right.
        self.city_panel(draws, FOOD);
        self.heading(draws, FOOD.0 + 10., FOOD.1 + 8., "Food Storage");
        let full = f64::from(detail.growth_threshold.max(1));
        let stored = f64::from(detail.food_box.max(0)).min(full);
        draws.push(rect(
            FOOD.0 + 12.,
            FOOD.1 + 36.,
            FOOD.2 - 24.,
            10.,
            style::INSET,
        ));
        draws.push(rect(
            FOOD.0 + 12.,
            FOOD.1 + 36.,
            (FOOD.2 - 24.) * stored / full,
            10.,
            style::MOVE_OK,
        ));
        draws.push(text(
            FOOD.0 + 10.,
            FOOD.1 + 52.,
            &format!(
                "Food store {} / {}",
                detail.food_box, detail.growth_threshold
            ),
            style::TYPE_SMALL,
            style::TEXT,
        ));
        let surplus = v
            .observation
            .city_economy
            .iter()
            .find(|e| e.index == city.slot)
            .map_or(0, |e| e.food_surplus);
        let stored_food = i32::from(detail.food_box);
        let next = if surplus > 0 && stored_food < detail.growth_threshold {
            let left = detail.growth_threshold - stored_food;
            let turns = (left + surplus - 1) / surplus;
            format!("Next Citizen: {turns} turns")
        } else {
            "Next Citizen: —".to_string()
        };
        draws.push(text(
            FOOD.0 + 10.,
            FOOD.1 + 92.,
            &next,
            style::TYPE_SMALL,
            style::GOLD,
        ));
        // The citizen strip: portrait, worker count and the specialist keys.
        self.city_panel(draws, STRIP);
        draws.push(Draw::Sprite {
            x: STRIP.0 + 8.,
            y: STRIP.1 + 8.,
            w: 27.,
            h: 32.,
            sprite: "citizen-0".into(),
        });
        let workers = detail.radius.iter().take(20).filter(|t| t.worked).count();
        draws.push(text(
            STRIP.0 + 46.,
            STRIP.1 + 6.,
            &format!(
                "Workers {workers} · specialists {} · click a specialist to cycle its role",
                detail.specialists.len()
            ),
            style::TYPE_SMALL,
            style::TEXT_BRIGHT,
        ));
        // The radius map: bigger, as the original's city view, with yields.
        let mut tiles = (0..21).collect::<Vec<_>>();
        tiles.sort_by_key(|&k| RADIUS_DY[k]);
        for k in tiles {
            let t = &detail.radius[k];
            let (x, y) = spot(k);
            let (tw, th) = RADIUS_SIZE;
            if let Some(t) = &t.terrain {
                draws.push(Draw::Sprite {
                    x: x - tw / 2.,
                    y: y - th / 2.,
                    w: tw,
                    h: th,
                    sprite: format!("terrain-{}-0", t.terrain),
                });
                if let Some(name) = match t.terrain {
                    3 => Some("forest"),
                    4 => Some("hills"),
                    5 => Some("mountains"),
                    _ => None,
                } {
                    draws.push(Draw::Sprite {
                        x: x - tw / 2.,
                        y: y - th / 2.,
                        w: tw,
                        h: th,
                        sprite: format!("{name}-0"),
                    });
                }
                if t.river || t.road || t.rail {
                    draws.push(Draw::Line {
                        x: x - 16.,
                        y: y + 8.,
                        x2: x + 16.,
                        y2: y + 8.,
                        color: if t.river { style::LINK } else { "#c1a780" }.into(),
                        width: 2.,
                    });
                }
            } else {
                draws.push(rect(x - 48., y - 20., 96., 40., style::INSET));
            }
            if k == 20 {
                draws.push(Draw::Sprite {
                    x: x - 32.,
                    y: y - 44.,
                    w: 64.,
                    h: 48.,
                    sprite: format!("city-{}", usize::from(c.size >= 8)),
                });
            }
            if t.worked {
                draws.push(rect(x - 6., y - 28., 12., 10., style::GOLD));
            }
            if !t.available && k != 20 {
                draws.push(text(x - 22., y - 18., "×", style::TYPE_TITLE, style::WARN));
            }
            if self.city_tile as usize == k {
                for (a, b, cc, dd) in [
                    (x - 58., y, x, y - 32.),
                    (x, y - 32., x + 58., y),
                    (x + 58., y, x, y + 32.),
                    (x, y + 32., x - 58., y),
                ] {
                    draws.push(Draw::Line {
                        x: a,
                        y: b,
                        x2: cc,
                        y2: dd,
                        color: style::GOLD.into(),
                        width: 2.,
                    });
                }
            }
            // Yields: the original's three icons; the install exposes no
            // resource sheet here, so the numeric triple is kept.
            draws.push(text(
                x - 30.,
                y + 6.,
                &format!("{}/{}/{}", t.yields[0], t.yields[1], t.yields[2]),
                style::TYPE_SMALL,
                style::WHITE,
            ));
        }
        // The information pane and its content.
        self.city_panel(draws, INFO_PANE);
        self.city_panel_draw(draws, v);
        if v.decision.is_some() {
            draws.push(text(
                300.,
                612.,
                "Research or another choice is required.",
                style::TYPE_SMALL,
                style::WARN,
            ));
        }
    }
}
