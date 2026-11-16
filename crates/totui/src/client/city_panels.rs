//! City information is presentation state; inspecting it never issues an order.
use super::style;
use super::*;
use engine::{
    city_info::{CityImprovementInfo, CityInfo, CityUnitInfo},
    happiness::Happiness,
};

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
/// Presentation order of a city panel list (#284). `Listed` is the engine's
/// order; numeric orders put the largest first; ties keep the listed order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CitySort {
    Listed,
    Name,
    Shields,
    Food,
    Upkeep,
}
impl CitySort {
    fn label(self) -> &'static str {
        match self {
            Self::Listed => "Sort: listed",
            Self::Name => "Sort: name",
            Self::Shields => "Sort: shields",
            Self::Food => "Sort: food",
            Self::Upkeep => "Sort: upkeep",
        }
    }
    /// The orders a panel offers; panels without a list offer none.
    pub(super) fn cycle(panel: CityPanel) -> &'static [CitySort] {
        match panel {
            CityPanel::Info => &[Self::Listed, Self::Name],
            CityPanel::Support => &[Self::Listed, Self::Name, Self::Shields, Self::Food],
            CityPanel::Improvements => &[Self::Listed, Self::Name, Self::Upkeep],
            CityPanel::Happy => &[],
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
    fn unit_name(&self, type_id: u8) -> String {
        self.loaded
            .as_ref()
            .and_then(|l| {
                l.resources
                    .rules
                    .unit_names()
                    .get(type_id as usize)
                    .map(ToString::to_string)
            })
            .unwrap_or_else(|| format!("Unit {type_id}"))
    }
    /// A unit list in the panel's current order.
    pub(super) fn sorted_units<'a>(&self, units: &'a [CityUnitInfo]) -> Vec<&'a CityUnitInfo> {
        let mut out: Vec<&CityUnitInfo> = units.iter().collect();
        match self.city_panel_sort {
            CitySort::Name => out.sort_by_cached_key(|u| self.unit_name(u.type_id).to_lowercase()),
            CitySort::Shields => out.sort_by_key(|u| std::cmp::Reverse(u.shields.unwrap_or(0))),
            CitySort::Food => out.sort_by_key(|u| std::cmp::Reverse(u.food.unwrap_or(0))),
            CitySort::Listed | CitySort::Upkeep => (),
        }
        out
    }
    /// Improvements and wonders in the panel's current order.
    pub(super) fn sorted_improvements<'a>(&self, d: &'a CityInfo) -> Vec<&'a CityImprovementInfo> {
        let mut out: Vec<&CityImprovementInfo> = d.improvements.iter().collect();
        match self.city_panel_sort {
            CitySort::Name => {
                out.sort_by_cached_key(|i| self.item_name(-(i.id as i8)).to_lowercase())
            }
            CitySort::Upkeep => out.sort_by_key(|i| std::cmp::Reverse(i.upkeep)),
            _ => (),
        }
        out
    }
    pub(super) fn city_panel_sort_next(&mut self) {
        let orders = CitySort::cycle(self.city_panel);
        if let Some(at) = orders.iter().position(|o| *o == self.city_panel_sort) {
            self.city_panel_sort = orders[(at + 1) % orders.len()];
        } else if let Some(first) = orders.first() {
            self.city_panel_sort = *first;
        }
        self.city_panel_page = 0;
    }
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
        if !CitySort::cycle(self.city_panel).is_empty() {
            b.push(Button {
                x: 880.,
                y: 188.,
                w: 124.,
                h: 26.,
                label: self.city_panel_sort.label().into(),
                action: Action::CityPanelSort,
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
                    for (row, item) in self
                        .sorted_improvements(d)
                        .into_iter()
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
        for (row, u) in self
            .sorted_units(units)
            .into_iter()
            .skip(page * rows)
            .take(rows)
            .enumerate()
        {
            let y = start + row as f64 * if info { 25. } else { 40. };
            let name = self.unit_name(u.type_id);
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
                &format!("{} #{}", short(&name, 19), u.slot),
                style::TYPE_SMALL,
                style::TEXT,
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
                    style::TYPE_SMALL,
                    style::TEXT_DIM,
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
        draws.push(text(
            744.,
            193.,
            self.city_panel.label(),
            style::TYPE_BODY,
            style::TEXT_BRIGHT,
        ));
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
                    draws.push(text(
                        744.,
                        220. + i as f64 * 23.,
                        line,
                        style::TYPE_BODY,
                        style::TEXT,
                    ));
                }
                draws.push(text(
                    744.,
                    342.,
                    "Arrows select; Space edits.",
                    style::TYPE_SMALL,
                    style::TEXT,
                ));
                draws.push(text(
                    744.,
                    365.,
                    "Edits commit immediately.",
                    style::TYPE_SMALL,
                    style::GOLD,
                ));
                draws.push(text(
                    744.,
                    391.,
                    &format!("Own units present: {}", d.present.len()),
                    style::TYPE_BODY,
                    style::TEXT_BRIGHT,
                ));
                self.panel_units(draws, &d.present, 416., page, true);
                if d.present.is_empty() {
                    draws.push(text(
                        744.,
                        416.,
                        "No own units here.",
                        style::TYPE_SMALL,
                        style::TEXT,
                    ));
                }
            }
            CityPanel::Support => {
                let shields: i32 = d.supported.iter().filter_map(|u| u.shields).sum();
                let food: i32 = d.supported.iter().filter_map(|u| u.food).sum();
                draws.push(text(
                    744.,
                    221.,
                    &format!("Supported units: {}", d.supported.len()),
                    style::TYPE_BODY,
                    style::TEXT,
                ));
                draws.push(text(
                    744.,
                    245.,
                    &format!("Shields {shields} · food {food}"),
                    style::TYPE_BODY,
                    style::TEXT,
                ));
                self.panel_units(draws, &d.supported, 278., page, false);
                if d.supported.is_empty() {
                    draws.push(text(
                        744.,
                        278.,
                        "No units supported here.",
                        style::TYPE_SMALL,
                        style::TEXT,
                    ));
                }
                draws.push(text(
                    744.,
                    484.,
                    "Inspection only; no unit orders.",
                    style::TYPE_SMALL,
                    style::TEXT_DIM,
                ));
            }
            CityPanel::Improvements => {
                for (row, item) in self
                    .sorted_improvements(d)
                    .into_iter()
                    .skip(page * ROWS)
                    .take(ROWS)
                    .enumerate()
                {
                    let y = 228. + row as f64 * 48.;
                    draws.push(text(
                        744.,
                        y,
                        &short(&self.item_name(-(item.id as i8)), 21),
                        style::TYPE_SMALL,
                        style::TEXT,
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
                    draws.push(text(
                        744.,
                        y + 20.,
                        &status,
                        style::TYPE_SMALL,
                        style::TEXT_DIM,
                    ));
                }
                if d.improvements.is_empty() {
                    draws.push(text(
                        744.,
                        228.,
                        "No improvements or wonders.",
                        style::TYPE_SMALL,
                        style::TEXT,
                    ));
                }
                draws.push(text(
                    744.,
                    484.,
                    "Sale requires confirmation.",
                    style::TYPE_SMALL,
                    style::TEXT_DIM,
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
                    draws.push(text(744., y, label, style::TYPE_BODY, style::TEXT_BRIGHT));
                    draws.push(text(
                        744.,
                        y + 20.,
                        &format!(
                            "Happy {} · unhappy {} · angry {}",
                            stage.happy, stage.unhappy, stage.angry
                        ),
                        style::TYPE_SMALL,
                        style::TEXT,
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
                    style::TYPE_SMALL,
                    style::WARN,
                ));
                draws.push(text(
                    744.,
                    516.,
                    "Unchanged stages are omitted.",
                    style::TYPE_SMALL,
                    style::TEXT_DIM,
                ));
            }
        }
        if pages > 1 {
            draws.push(text(
                744.,
                552.,
                &format!("List {} / {pages}", page + 1),
                style::TYPE_SMALL,
                style::TEXT_DIM,
            ));
        }
    }
}
