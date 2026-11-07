//! City information is presentation state; inspecting it never issues an order.
use super::*;
use engine::{city_info::CityUnitInfo, happiness::Happiness};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CityPanel {
    Info,
    Support,
    Improvements,
    Happy,
}
impl CityPanel {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Support => "Support",
            Self::Improvements => "Improvements",
            Self::Happy => "Happy",
        }
    }
}
const ROWS: usize = 5;
fn short(value: &str, max: usize) -> String {
    let mut chars = value.chars();
    let mut s = chars.by_ref().take(max).collect::<String>();
    if chars.next().is_some() {
        s.push('…');
    }
    s
}

impl Client {
    pub(super) fn city_panel_pages(&self) -> usize {
        let Some(v) = self.view() else { return 1 };
        let Some(d) = self
            .city
            .and_then(|c| v.city_info.iter().find(|d| d.index == c.slot))
        else {
            return 1;
        };
        let count = match self.city_panel {
            CityPanel::Info => d.present.len(),
            CityPanel::Support => d.supported.len(),
            CityPanel::Improvements => d.improvements.len(),
            CityPanel::Happy => 0,
        };
        count
            .div_ceil(if self.city_panel == CityPanel::Info {
                4
            } else {
                ROWS
            })
            .max(1)
    }
    pub(super) fn city_panel_buttons(&self) -> Vec<Button> {
        let mut b = Vec::new();
        for (i, panel) in [
            CityPanel::Info,
            CityPanel::Support,
            CityPanel::Improvements,
            CityPanel::Happy,
        ]
        .into_iter()
        .enumerate()
        {
            b.push(Button {
                x: 740. + (i % 2) as f64 * 134.,
                y: 110. + (i / 2) as f64 * 40.,
                w: 130.,
                h: 34.,
                label: panel.label().into(),
                action: Action::CityPanel(panel),
            });
        }
        let pages = self.city_panel_pages();
        if pages > 1 {
            b.push(Button {
                x: 740.,
                y: 510.,
                w: 124.,
                h: 30.,
                label: "Previous list".into(),
                action: Action::CityPanelPage(-1),
            });
            b.push(Button {
                x: 874.,
                y: 510.,
                w: 124.,
                h: 30.,
                label: "Next list".into(),
                action: Action::CityPanelPage(1),
            });
        }
        if self.city_panel == CityPanel::Improvements {
            if let (Some(city), Some(v)) = (self.city, self.view()) {
                if let Some(d) = v.city_info.iter().find(|d| d.index == city.slot) {
                    let page = self.city_panel_page.min(pages - 1);
                    for (row, item) in d
                        .improvements
                        .iter()
                        .skip(page * ROWS)
                        .take(ROWS)
                        .enumerate()
                    {
                        let saleable = v
                            .city_production
                            .iter()
                            .find(|p| p.index == city.slot)
                            .is_some_and(|p| {
                                p.improvements
                                    .iter()
                                    .any(|(id, q)| *id == item.id && q.is_ok())
                            });
                        if saleable {
                            b.push(Button {
                                x: 930.,
                                y: 228. + row as f64 * 48.,
                                w: 70.,
                                h: 28.,
                                label: "Sell…".into(),
                                action: Action::InspectSale(city, item.id),
                            });
                        }
                    }
                }
            }
        }
        b
    }
    fn panel_units(
        &self,
        draws: &mut Vec<Draw>,
        units: &[CityUnitInfo],
        start: f64,
        page: usize,
        info: bool,
    ) {
        let rows = if info { 4 } else { ROWS };
        for (row, u) in units.iter().skip(page * rows).take(rows).enumerate() {
            let y = start + row as f64 * if info { 25. } else { 40. };
            let name = self
                .loaded
                .as_ref()
                .and_then(|l| {
                    l.resources
                        .rules
                        .unit_names()
                        .get(u.type_id as usize)
                        .map(ToString::to_string)
                })
                .unwrap_or_else(|| format!("Unit {}", u.type_id));
            if !info {
                draws.push(Draw::Sprite {
                    x: 742.,
                    y: y - 4.,
                    w: 32.,
                    h: 24.,
                    sprite: format!("unit-{}", u.type_id),
                });
            }
            draws.push(text(
                if info { 744. } else { 778. },
                y,
                &format!("{} #{}", short(&name, 19), u.id),
                14,
                "#eee8d4",
            ));
            if !info {
                draws.push(text(
                    778.,
                    y + 18.,
                    &format!(
                        "Shields {} · food {}",
                        u.shields.unwrap_or(0),
                        u.food.unwrap_or(0)
                    ),
                    12,
                    "#dfd4b5",
                ));
            }
        }
    }
    pub(super) fn city_panel_draw(&self, draws: &mut Vec<Draw>, v: &HumanView) {
        let Some(city) = self.city else { return };
        let Some(d) = v
            .city_info
            .iter()
            .find(|d| d.index == city.slot && d.city_id == city.id)
        else {
            return;
        };
        let pages = self.city_panel_pages();
        let page = self.city_panel_page.min(pages - 1);
        draws.push(text(744., 193., self.city_panel.label(), 16, "#fff0b5"));
        match self.city_panel {
            CityPanel::Info => {
                let Some(detail) = v.city_citizens.iter().find(|d| d.index == city.slot) else {
                    return;
                };
                let t = &detail.radius[self.city_tile as usize];
                let lines = [
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
                ];
                for (i, line) in lines.iter().enumerate() {
                    draws.push(text(744., 220. + i as f64 * 23., line, 15, "#eee8d4"));
                }
                draws.push(text(
                    744.,
                    342.,
                    "Arrows select; Space edits.",
                    14,
                    "#eee8d4",
                ));
                draws.push(text(744., 365., "Edits commit immediately.", 14, "#ffef9a"));
                draws.push(text(
                    744.,
                    391.,
                    &format!("Own units present: {}", d.present.len()),
                    15,
                    "#fff0b5",
                ));
                self.panel_units(draws, &d.present, 416., page, true);
                if d.present.is_empty() {
                    draws.push(text(744., 416., "No own units here.", 14, "#eee8d4"));
                }
            }
            CityPanel::Support => {
                let shields: i32 = d.supported.iter().filter_map(|u| u.shields).sum();
                let food: i32 = d.supported.iter().filter_map(|u| u.food).sum();
                draws.push(text(
                    744.,
                    221.,
                    &format!("Supported units: {}", d.supported.len()),
                    15,
                    "#eee8d4",
                ));
                draws.push(text(
                    744.,
                    245.,
                    &format!("Shields {shields} · food {food}"),
                    15,
                    "#eee8d4",
                ));
                self.panel_units(draws, &d.supported, 278., page, false);
                if d.supported.is_empty() {
                    draws.push(text(744., 278., "No units supported here.", 14, "#eee8d4"));
                }
                draws.push(text(
                    744.,
                    484.,
                    "Inspection only; no unit orders.",
                    13,
                    "#dfd4b5",
                ));
            }
            CityPanel::Improvements => {
                for (row, item) in d
                    .improvements
                    .iter()
                    .skip(page * ROWS)
                    .take(ROWS)
                    .enumerate()
                {
                    let y = 228. + row as f64 * 48.;
                    draws.push(text(
                        744.,
                        y,
                        &short(&self.item_name(-(item.id as i8)), 21),
                        14,
                        "#eee8d4",
                    ));
                    let status = if item.wonder {
                        if item.active {
                            "Wonder · active".into()
                        } else {
                            "Wonder · expired".into()
                        }
                    } else {
                        format!("Upkeep: {} gold", item.upkeep)
                    };
                    draws.push(text(744., y + 20., &status, 13, "#dfd4b5"));
                }
                if d.improvements.is_empty() {
                    draws.push(text(
                        744.,
                        228.,
                        "No improvements or wonders.",
                        14,
                        "#eee8d4",
                    ));
                }
                draws.push(text(
                    744.,
                    484.,
                    "Sale requires confirmation.",
                    13,
                    "#dfd4b5",
                ));
            }
            CityPanel::Happy => {
                let mut row = 0;
                for (i, (label, stage)) in [
                    "Base citizens",
                    "Luxuries",
                    "Buildings",
                    "Military",
                    "Wonders",
                ]
                .into_iter()
                .zip(d.happiness)
                .enumerate()
                {
                    // The native Happy pane omits a stage whose counts did not change.
                    if i > 0 && stage == d.happiness[i - 1] {
                        continue;
                    }
                    let y = 222. + row as f64 * 51.;
                    row += 1;
                    draws.push(text(744., y, label, 15, "#fff0b5"));
                    draws.push(text(
                        744.,
                        y + 20.,
                        &format!(
                            "Happy {} · unhappy {} · angry {}",
                            stage.happy, stage.unhappy, stage.angry
                        ),
                        13,
                        "#eee8d4",
                    ));
                }
                let Happiness { happy, unhappy, .. } = d.happiness[4];
                draws.push(text(
                    744.,
                    491.,
                    if unhappy > happy {
                        "Unrest: disorder risk"
                    } else {
                        "Final mood stable"
                    },
                    14,
                    "#ffcf8c",
                ));
                draws.push(text(
                    744.,
                    516.,
                    "Unchanged stages are omitted.",
                    13,
                    "#dfd4b5",
                ));
            }
        }
        if pages > 1 {
            draws.push(text(
                744.,
                552.,
                &format!("List {} / {pages}", page + 1),
                13,
                "#dfd4b5",
            ));
        }
    }
}
