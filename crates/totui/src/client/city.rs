//! Shared own-city workspace; adapters receive only filtered presentation.
use super::*;
use engine::yields::{RADIUS_DX, RADIUS_DY};

fn spot(k: usize) -> (f64, f64) {
    (
        500. + f64::from(RADIUS_DX[k]) * 45.,
        300. + f64::from(RADIUS_DY[k]) * 25.,
    )
}

impl Client {
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
            "ArrowRight" | "ArrowDown" => {
                self.city_tile = (self.city_tile + 1) % 21;
                Ok(())
            }
            "ArrowLeft" | "ArrowUp" => {
                self.city_tile = (self.city_tile + 20) % 21;
                Ok(())
            }
            "PageDown" => self.action(Action::CityStep(1)),
            "PageUp" => self.action(Action::CityStep(-1)),
            " " => self.action(Action::Worker(
                self.city.ok_or("Select an own city first")?,
                self.city_tile,
            )),
            "Ctrl+s" => self.action(Action::Save),
            "Ctrl+l" => self.action(Action::Load),
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
        add(
            12.,
            48.,
            220.,
            38.,
            "Return to map [Esc]",
            Action::CloseCity,
        );
        add(
            245.,
            48.,
            180.,
            38.,
            "Previous city [PgUp]",
            Action::CityStep(-1),
        );
        add(
            438.,
            48.,
            180.,
            38.,
            "Next city [PgDn]",
            Action::CityStep(1),
        );
        add(632., 48., 120., 38., "Save [Ctrl+S]", Action::Save);
        add(764., 48., 120., 38., "Load [Ctrl+L]", Action::Load);
        add(896., 48., 116., 38., "Research", Action::Research);
        if let Some(city) = self.city {
            add(
                20.,
                480.,
                250.,
                36.,
                "Choose production",
                Action::Production(city),
            );
            add(
                320.,
                440.,
                380.,
                36.,
                "Toggle selected tile [Space]",
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
                for (i, role) in detail.specialists.iter().take(16).enumerate() {
                    let name = match role {
                        1 => "Entertainer",
                        2 => "Tax collector",
                        3 => "Scientist",
                        _ => "Unused",
                    };
                    add(
                        300. + (i % 4) as f64 * 175.,
                        580. + (i / 4) as f64 * 34.,
                        168.,
                        30.,
                        &format!("{}: {name}", i + 1),
                        Action::Specialist(city, i as u8),
                    );
                }
            }
        }
        b
    }

    pub(super) fn city_draw(&self, draws: &mut Vec<Draw>, v: &HumanView) {
        draws.push(rect(0., 0., 1024., 744., "#20251f"));
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
        draws.push(text(
            18.,
            14.,
            &format!("{} — population {}", c.name, c.size),
            24,
            "#fff0b5",
        ));
        for (x, w) in [(12., 266.), (730., 282.)] {
            draws.push(Draw::Tile {
                x,
                y: 102.,
                w,
                h: 620.,
                sprite: "panel".into(),
            });
        }
        if let Some(e) = v
            .observation
            .city_economy
            .iter()
            .find(|e| e.index == city.slot)
        {
            let output = c.output.unwrap_or_default();
            let lines = [
                format!("Food surplus {:+}", e.food_surplus),
                format!(
                    "Food store {} / {}",
                    detail.food_box, detail.growth_threshold
                ),
                format!("Shields {} (support {})", e.net_shields, detail.support),
                format!("Trade {} · corruption {}", output.trade, output.corruption),
                format!("Waste {}", output.waste),
                format!("Science {}", e.finance.science),
                format!("Tax {} · luxury {}", e.finance.taxes, e.finance.luxuries),
                format!("Happy {} · unhappy {}", e.happy, e.unhappy),
                format!("Upkeep {}", e.upkeep),
            ];
            for (i, line) in lines.iter().enumerate() {
                draws.push(text(22., 116. + i as f64 * 24., line, 16, "#eee8d4"));
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
            draws.push(text(22., 330., warning, 15, "#ffcf8c"));
        }
        draws.push(text(22., 358., "Currently producing:", 16, "#fff0b5"));
        let item = c
            .building
            .map(|id| self.item_name(id))
            .unwrap_or_else(|| "Unavailable".into());
        // Keep long scenario names inside the left panel without clipping.
        let mut name: Vec<_> = item.chars().collect();
        if name.len() > 69 {
            name.truncate(68);
            name.push('…');
        }
        for (row, chunk) in name.chunks(23).enumerate() {
            draws.push(text(
                22.,
                382. + row as f64 * 19.,
                &chunk.iter().collect::<String>(),
                16,
                "#ffffff",
            ));
        }
        draws.push(text(
            22.,
            444.,
            &format!("Stored shields: {}", c.shield_box.unwrap_or(0)),
            16,
            "#eee8d4",
        ));
        draws.push(text(
            305.,
            112.,
            "City radius · click a tile to change its worker",
            16,
            "#fff0b5",
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
                        color: if t.river { "#4e9ac4" } else { "#c1a780" }.into(),
                        width: 2.,
                    });
                }
            } else {
                draws.push(rect(x - 35., y - 15., 70., 30., "#090d08"));
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
                draws.push(rect(x - 4., y - 20., 8., 8., "#f4e59a"));
            }
            if !t.available && k != 20 {
                draws.push(text(x - 20., y - 12., "×", 18, "#ff9988"));
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
                        color: "#ffe77c".into(),
                        width: 2.,
                    });
                }
            }
            draws.push(text(
                x - 25.,
                y + 2.,
                &format!("{}/{}/{}", t.yields[0], t.yields[1], t.yields[2]),
                12,
                "#ffffff",
            ));
        }
        let t = &detail.radius[self.city_tile as usize];
        for (i, line) in [
            format!("Tile {}", self.city_tile),
            format!(
                "Position: {}",
                t.position
                    .map_or_else(|| "off map".into(), |(x, y)| format!("{x}, {y}"))
            ),
            format!("Food {} / shields {}", t.yields[0], t.yields[1]),
            format!("Trade {}", t.yields[2]),
            if t.worked {
                "Worked"
            } else if t.available {
                "Available"
            } else {
                "Unavailable"
            }
            .into(),
            "Arrows select a radius tile.".into(),
            "Space changes its worker.".into(),
        ]
        .iter()
        .enumerate()
        {
            draws.push(text(744., 130. + i as f64 * 28., line, 16, "#eee8d4"));
        }
        draws.push(text(744., 370., "Edits commit immediately.", 16, "#ffef9a"));
        let workers = detail.radius.iter().take(20).filter(|t| t.worked).count();
        for i in 0..workers.min(12) {
            draws.push(Draw::Sprite {
                x: 305. + i as f64 * 28.,
                y: 402.,
                w: 27.,
                h: 32.,
                sprite: "citizen-0".into(),
            });
        }
        draws.push(text(
            744.,
            398.,
            "Closing does not undo edits.",
            15,
            "#eee8d4",
        ));
        draws.push(text(
            300.,
            542.,
            &format!(
                "Workers {workers} · specialists {} · click to cycle roles",
                detail.specialists.len()
            ),
            16,
            "#fff0b5",
        ));
        draws.push(text(22., 538., "Remove a worker to", 15, "#eee8d4"));
        draws.push(text(22., 562., "create an entertainer.", 15, "#eee8d4"));
        draws.push(text(22., 602., "Other roles need size 5.", 15, "#eee8d4"));
        draws.push(text(22., 646., "Empty tile, no spare:", 15, "#eee8d4"));
        draws.push(text(22., 670., "automatic reassignment.", 15, "#eee8d4"));
        if v.decision.is_some() {
            draws.push(text(744., 458., "Research/choice required.", 16, "#ffba8a"));
        }
    }
}
