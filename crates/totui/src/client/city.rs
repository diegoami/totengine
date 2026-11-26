//! Shared own-city workspace; adapters receive only filtered presentation.
use super::style;
use super::*;
use engine::yields::{RADIUS_DX, RADIUS_DY};

/// Top of the city window's footer row (layout B, #314).
const FOOTER_Y: f64 = 700.;
/// Top of the window body under the citizens strip.
const BODY_Y: f64 = 182.;

pub(super) fn spot(k: usize) -> (f64, f64) {
    (
        500. + f64::from(RADIUS_DX[k]) * 45.,
        300. + f64::from(RADIUS_DY[k]) * 25.,
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
            (x - cx).abs() / 45. + (y - cy).abs() / 25. <= 1.
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
        // Layout B (#314): the classic city window. Return to map stays the
        // first button, so Enter on a freshly opened window still returns.
        add(
            792.,
            FOOTER_Y,
            220.,
            34.,
            "Return to map [Esc]",
            Action::CloseCity,
        );
        add(
            12.,
            36.,
            200.,
            26.,
            "Previous city [PgUp]",
            Action::CityStep(-1),
        );
        add(
            812.,
            36.,
            200.,
            26.,
            "Next city [PgDn]",
            Action::CityStep(1),
        );
        add(16., FOOTER_Y, 140., 34., "Save [Ctrl+S]", Action::Save);
        add(164., FOOTER_Y, 140., 34., "Load [Ctrl+L]", Action::Load);
        add(312., FOOTER_Y, 140., 34., "Research", Action::Research);
        if let Some(city) = self.city {
            add(
                24.,
                598.,
                244.,
                28.,
                "Choose production",
                Action::Production(city),
            );
            add(24., 630., 244., 28., "Buy production", Action::Buy(city));
            add(
                24.,
                662.,
                244.,
                28.,
                "Improvements / sell",
                Action::Improvements(city),
            );
            add(
                320.,
                440.,
                380.,
                36.,
                if self.city_panel == CityPanel::Info {
                    "Toggle selected tile [Space]"
                } else {
                    "Toggle selected tile"
                },
                Action::Worker(city, self.city_tile),
            );
            add(
                320.,
                488.,
                380.,
                36.,
                "Auto workers (centre tile)",
                Action::Worker(city, 20),
            );
            if let Some(detail) = self
                .view()
                .and_then(|v| v.city_citizens.into_iter().find(|c| c.index == city.slot))
            {
                // The citizens strip: specialists, six to a row under the workers.
                for (i, role) in detail.specialists.iter().take(12).enumerate() {
                    let name = match role {
                        1 => "Entertainer",
                        2 => "Tax collector",
                        3 => "Scientist",
                        _ => "Unused",
                    };
                    add(
                        16. + (i % 6) as f64 * 166.,
                        108. + (i / 6) as f64 * 36.,
                        160.,
                        32.,
                        &format!("{}: {name}", i + 1),
                        Action::Specialist(city, i as u8),
                    );
                }
            }
        }
        b.extend(self.city_panel_buttons());
        b
    }

    pub(super) fn city_draw(&self, draws: &mut Vec<Draw>, v: &HumanView) {
        draws.push(rect(0., 0., 1024., 744., style::BG));
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
        // Layout B (#314): one framed window under the (inactive) menu bar,
        // with a title bar, the citizens strip, three columns and a footer.
        draws.push(rect(4., 30., 1016., 712., style::GOLD));
        draws.push(rect(6., 32., 1012., 708., style::BG));
        draws.push(rect(6., 32., 1012., 34., style::MENU_SELECT));
        let title = format!("{} — population {}", c.name, c.size);
        let w = title.chars().count() as f64 * 0.56 * f64::from(style::TYPE_TITLE);
        draws.push(text(
            (512. - w / 2.).max(220.),
            38.,
            &title,
            style::TYPE_TITLE,
            style::MENU_SELECT_TEXT,
        ));
        draws.push(rect(6., 66., 1012., 1., style::GOLD_DIM));
        draws.push(rect(6., BODY_Y - 6., 1012., 1., style::GOLD_DIM));
        draws.push(rect(6., FOOTER_Y - 6., 1012., 1., style::GOLD_DIM));
        // Citizens strip: the workers, then the specialists (buttons).
        let workers = detail.radius.iter().take(20).filter(|t| t.worked).count();
        for i in 0..workers.min(16) {
            draws.push(Draw::Sprite {
                x: 16. + i as f64 * 28.,
                y: 70.,
                w: 27.,
                h: 32.,
                sprite: "citizen-0".into(),
            });
        }
        draws.push(text(
            24. + workers.min(16) as f64 * 28.,
            78.,
            &format!(
                "Workers {workers} · specialists {} · click a specialist to cycle its role",
                detail.specialists.len()
            ),
            style::TYPE_SMALL,
            style::TEXT_BRIGHT,
        ));
        for (x, y, w, h) in [(12., BODY_Y, 264., 508.), (730., BODY_Y, 282., 508.)] {
            draws.push(Draw::Tile {
                x,
                y,
                w,
                h,
                sprite: "panel".into(),
            });
        }
        draws.push(text(
            22.,
            BODY_Y + 6.,
            "City resources",
            style::TYPE_BODY,
            style::TEXT_BRIGHT,
        ));
        if let Some(e) = v
            .observation
            .city_economy
            .iter()
            .find(|e| e.index == city.slot)
        {
            let output = c.output.unwrap_or_default();
            let lines = [
                (
                    Some("classic-food"),
                    format!("Food surplus {:+}", e.food_surplus),
                ),
                (
                    Some("classic-shields"),
                    format!("Shields {} (support {})", e.net_shields, detail.support),
                ),
                (
                    None,
                    format!("Trade {} · corruption {}", output.trade, output.corruption),
                ),
                (None, format!("Waste {}", output.waste)),
                (
                    Some("classic-science"),
                    format!("Science {}", e.finance.science),
                ),
                (
                    Some("classic-tax"),
                    format!("Tax {} · luxury {}", e.finance.taxes, e.finance.luxuries),
                ),
                (None, format!("Happy {} · unhappy {}", e.happy, e.unhappy)),
                (None, format!("Upkeep {}", e.upkeep)),
            ];
            for (i, (icon, line)) in lines.iter().enumerate() {
                let y = BODY_Y + 30. + i as f64 * 20.;
                if *icon == Some("classic-tax") && self.classic_icons_active() {
                    for (x, icon, label) in [
                        (22., "classic-tax", format!("Tax {}", e.finance.taxes)),
                        (
                            130.,
                            "classic-luxury",
                            format!("Luxury {}", e.finance.luxuries),
                        ),
                    ] {
                        draws.extend(self.classic_icon(icon, x, y));
                        draws.push(text(x + 20., y, &label, style::TYPE_SMALL, style::TEXT));
                    }
                } else {
                    let sprite = icon.and_then(|name| self.classic_icon(name, 22., y));
                    let x = if sprite.is_some() { 42. } else { 22. };
                    draws.extend(sprite);
                    draws.push(text(x, y, line, style::TYPE_SMALL, style::TEXT));
                }
            }
            let warning = if detail.food_box < 0 {
                "Food debt: famine pending"
            } else if e.food_surplus < 0 {
                "Food deficit: starvation risk"
            } else if e.growth_block.is_some() {
                "Growth requires improvement"
            } else if e.unhappy > e.happy {
                "Unrest: disorder risk"
            } else {
                "Growth and mood stable"
            };
            draws.push(text(22., 372., warning, style::TYPE_SMALL, style::WARN));
        }
        // Food storage.
        draws.push(text(
            22.,
            400.,
            "Food storage",
            style::TYPE_BODY,
            style::TEXT_BRIGHT,
        ));
        let full = f64::from(detail.growth_threshold.max(1));
        let stored = f64::from(detail.food_box.max(0)).min(full);
        draws.push(rect(24., 424., 240., 10., style::INSET));
        draws.push(rect(24., 424., 240. * stored / full, 10., style::MOVE_OK));
        draws.push(text(
            22.,
            440.,
            &format!(
                "Food store {} / {}",
                detail.food_box, detail.growth_threshold
            ),
            style::TYPE_SMALL,
            style::TEXT,
        ));
        // Production.
        draws.push(text(
            22.,
            466.,
            "Currently producing:",
            style::TYPE_BODY,
            style::TEXT_BRIGHT,
        ));
        let item = c
            .building
            .map(|id| self.item_name(id))
            .unwrap_or_else(|| "Unavailable".into());
        // Keep long scenario names inside the left column without clipping.
        let mut name: Vec<_> = item.chars().collect();
        if name.len() > 50 {
            name.truncate(49);
            name.push('…');
        }
        for (row, chunk) in name.chunks(25).enumerate() {
            draws.push(text(
                22.,
                488. + row as f64 * 18.,
                &chunk.iter().collect::<String>(),
                style::TYPE_BODY,
                style::WHITE,
            ));
        }
        if let Some(p) = v.city_production.iter().find(|p| p.index == city.slot) {
            if let Some(cost) = p.required.filter(|c| *c > 0) {
                let done = (f64::from(p.stored) / f64::from(cost)).clamp(0., 1.);
                draws.push(rect(24., 528., 240., 10., style::INSET));
                draws.push(rect(24., 528., 240. * done, 10., style::GOLD));
            }
            let shields = p.required.map_or_else(
                || format!("Shields: {}", p.stored),
                |cost| format!("Shields: {} / {cost}", p.stored),
            );
            draws.push(text(22., 544., &shields, style::TYPE_SMALL, style::TEXT));
            let estimate = match (p.remaining, p.turns) {
                (Some(0), _) => "Funded — next production phase".into(),
                (Some(n), Some(turns)) => format!("Remaining: {n} (~{turns} turns)"),
                (Some(n), None) => format!("Remaining: {n}; no current output"),
                _ => "No finite shield target".into(),
            };
            draws.push(text(22., 562., &estimate, style::TYPE_SMALL, style::TEXT));
        }
        draws.push(text(
            305.,
            BODY_Y,
            "City radius · click a tile to change its worker",
            style::TYPE_BODY,
            style::TEXT_BRIGHT,
        ));
        // Back-to-front diamonds, with yields and authored work/selection markers.
        let mut tiles = (0..21).collect::<Vec<_>>();
        tiles.sort_by_key(|&k| RADIUS_DY[k]);
        for k in tiles {
            let t = &detail.radius[k];
            let (x, y) = spot(k);
            if let Some(t) = &t.terrain {
                draws.push(Draw::Sprite {
                    x: x - 45.,
                    y: y - 25.,
                    w: 90.,
                    h: 50.,
                    sprite: format!("terrain-{}-0", t.terrain),
                });
                if let Some(name) = match t.terrain {
                    3 => Some("forest"),
                    4 => Some("hills"),
                    5 => Some("mountains"),
                    _ => None,
                } {
                    draws.push(Draw::Sprite {
                        x: x - 45.,
                        y: y - 25.,
                        w: 90.,
                        h: 50.,
                        sprite: format!("{name}-0"),
                    });
                }
                if t.river || t.road || t.rail {
                    draws.push(Draw::Line {
                        x: x - 12.,
                        y: y + 6.,
                        x2: x + 12.,
                        y2: y + 6.,
                        color: if t.river { style::LINK } else { "#c1a780" }.into(),
                        width: 2.,
                    });
                }
            } else {
                draws.push(rect(x - 35., y - 15., 70., 30., style::INSET));
            }
            if k == 20 {
                draws.push(Draw::Sprite {
                    x: x - 32.,
                    y: y - 38.,
                    w: 64.,
                    h: 48.,
                    sprite: format!("city-{}", usize::from(c.size >= 8)),
                });
            }
            if t.worked {
                draws.push(rect(x - 4., y - 20., 8., 8., style::GOLD));
            }
            if !t.available && k != 20 {
                draws.push(text(x - 20., y - 12., "×", style::TYPE_TITLE, style::WARN));
            }
            if self.city_tile as usize == k {
                for (a, b, cc, dd) in [
                    (x - 44., y, x, y - 24.),
                    (x, y - 24., x + 44., y),
                    (x + 44., y, x, y + 24.),
                    (x, y + 24., x - 44., y),
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
            draws.push(text(
                x - 25.,
                y + 2.,
                &format!("{}/{}/{}", t.yields[0], t.yields[1], t.yields[2]),
                style::TYPE_SMALL,
                style::WHITE,
            ));
        }
        self.city_panel_draw(draws, v);
        if v.decision.is_some() {
            draws.push(text(
                320.,
                536.,
                "Research or another choice is required.",
                style::TYPE_SMALL,
                style::WARN,
            ));
        }
    }
}
