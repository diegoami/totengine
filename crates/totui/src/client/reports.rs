//! Own-civilization reports (#201/#202): City status (F1), Defense (F2) and
//! Trade (F5), shared by both adapters. Rows come from the engine's immutable
//! report views; sorting, selection and paging are presentation only, and
//! opening a report never submits a command. An own-city row drills into the
//! city workspace and Esc returns to the same report, sort and selection.
use super::*;
use engine::human::{Report, ReportFamily};

const PAGE_ROWS: usize = 12;
const TABLE_X: f64 = 212.;
const HEADER_Y: f64 = 186.;
const ROW_Y: f64 = 214.;
const ROW_H: f64 = 27.;

/// A sort value: numbers compare numerically, text case-insensitively.
type SortKey = (i64, String);

pub(super) struct TableRow {
    pub cells: Vec<String>,
    pub keys: Vec<SortKey>,
    pub city: Option<Entity>,
    pub warn: bool,
}

pub(super) struct Table {
    pub headers: Vec<&'static str>,
    /// Left edge and width of each column, relative to the table origin.
    pub columns: Vec<(f64, f64)>,
    pub rows: Vec<TableRow>,
    pub footer: Vec<String>,
}

fn layout(widths: &[f64]) -> Vec<(f64, f64)> {
    let mut x = 0.;
    widths
        .iter()
        .map(|w| {
            let c = (x, *w);
            x += w;
            c
        })
        .collect()
}

fn text_key(s: &str) -> SortKey {
    (0, s.to_lowercase())
}

fn num_key(n: i64) -> SortKey {
    (n, String::new())
}

fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.into()
    } else {
        format!("{}…", s.chars().take(max - 1).collect::<String>())
    }
}

/// Stable sort by one column; equal keys keep their ascending city-slot order.
pub(super) fn sort_table(t: &mut Table, sort: usize, descending: bool) {
    let col = sort.min(t.headers.len().saturating_sub(1));
    t.rows.sort_by(|a, b| {
        let o = a.keys[col].cmp(&b.keys[col]);
        if descending {
            o.reverse()
        } else {
            o
        }
    });
}

pub(super) fn family_label(f: ReportFamily) -> &'static str {
    match f {
        ReportFamily::CityStatus => "City status [F1]",
        ReportFamily::Defense => "Defense [F2]",
        ReportFamily::Trade => "Trade [F5]",
    }
}

impl Client {
    pub(super) fn report_table(&self, family: ReportFamily) -> Option<Table> {
        let l = self.loaded.as_ref()?;
        let report = l.session.report(family);
        Some(match report {
            Report::CityStatus(rows) => Table {
                headers: vec![
                    "City", "Sz", "Food", "Shld", "Sci", "Tax", "Lux", "Build", "Prog", "Turns",
                ],
                columns: layout(&[118., 34., 48., 52., 40., 40., 40., 108., 62., 56.]),
                footer: vec![
                    "Orange rows are in disorder. Prog is shields stored / required.".into(),
                ],
                rows: rows
                    .into_iter()
                    .map(|r| {
                        let build = self.item_name(r.item);
                        let required = r.required.map_or("–".into(), |n| n.to_string());
                        let turns = r.turns.map_or("–".into(), |n| n.to_string());
                        TableRow {
                            cells: vec![
                                clip(&r.name, 15),
                                r.size.to_string(),
                                format!("{:+}", r.food_surplus),
                                r.shields.to_string(),
                                r.science.to_string(),
                                r.taxes.to_string(),
                                r.luxuries.to_string(),
                                clip(&build, 13),
                                format!("{}/{required}", r.stored),
                                turns,
                            ],
                            keys: vec![
                                text_key(&r.name),
                                num_key(i64::from(r.size)),
                                num_key(i64::from(r.food_surplus)),
                                num_key(i64::from(r.shields)),
                                num_key(i64::from(r.science)),
                                num_key(i64::from(r.taxes)),
                                num_key(i64::from(r.luxuries)),
                                text_key(&build),
                                num_key(i64::from(r.stored)),
                                num_key(r.turns.map_or(i64::MAX, i64::from)),
                            ],
                            city: Some(r.city),
                            warn: r.disorder,
                        }
                    })
                    .collect(),
            },
            Report::Defense(rows) => Table {
                headers: vec!["Unit", "A", "D", "HP", "FP", "Mv", "Active", "Building"],
                columns: layout(&[170., 36., 36., 44., 44., 44., 84., 100.]),
                footer: vec![
                    "Values are the loaded rules' unit statistics; Building counts own cities building the type."
                        .into(),
                ],
                rows: rows
                    .into_iter()
                    .map(|r| {
                        let name = self.item_name(r.unit_type as i8);
                        TableRow {
                            cells: vec![
                                clip(&name, 20),
                                r.attack.to_string(),
                                r.defense.to_string(),
                                r.hits.to_string(),
                                r.firepower.to_string(),
                                r.moves.to_string(),
                                r.active.to_string(),
                                r.building.to_string(),
                            ],
                            keys: vec![
                                text_key(&name),
                                num_key(i64::from(r.attack)),
                                num_key(i64::from(r.defense)),
                                num_key(i64::from(r.hits)),
                                num_key(i64::from(r.firepower)),
                                num_key(i64::from(r.moves)),
                                num_key(i64::from(r.active)),
                                num_key(i64::from(r.building)),
                            ],
                            city: None,
                            warn: false,
                        }
                    })
                    .collect(),
            },
            Report::Trade { rows, totals } => Table {
                headers: vec!["City", "Tax", "Sci", "Lux", "Upkeep", "Impr"],
                columns: layout(&[150., 70., 70., 70., 90., 80.]),
                footer: vec![
                    format!(
                        "Total: taxes +{} · upkeep −{} · net {:+} per turn · science {} · luxury {}",
                        totals.taxes,
                        totals.upkeep,
                        totals.net(),
                        totals.science,
                        totals.luxuries
                    ),
                    format!(
                        "Treasury {}. Taxes are credited only outside Anarchy and disorder; Impr includes the Palace.",
                        totals.treasury
                    ),
                ],
                rows: rows
                    .into_iter()
                    .map(|r| TableRow {
                        cells: vec![
                            clip(&r.name, 18),
                            r.taxes.to_string(),
                            r.science.to_string(),
                            r.luxuries.to_string(),
                            r.upkeep.to_string(),
                            r.improvements.to_string(),
                        ],
                        keys: vec![
                            text_key(&r.name),
                            num_key(i64::from(r.taxes)),
                            num_key(i64::from(r.science)),
                            num_key(i64::from(r.luxuries)),
                            num_key(i64::from(r.upkeep)),
                            num_key(i64::from(r.improvements)),
                        ],
                        city: Some(r.city),
                        warn: false,
                    })
                    .collect(),
            },
        })
    }

    /// The table with the modal's sort applied (ties keep ascending slot order).
    pub(super) fn sorted_report(
        &self,
        family: ReportFamily,
        sort: usize,
        descending: bool,
    ) -> Option<Table> {
        let mut t = self.report_table(family)?;
        sort_table(&mut t, sort, descending);
        Some(t)
    }

    pub(super) fn open_report(&mut self, family: ReportFamily) -> Result<(), String> {
        let v = self.view().ok_or("Start a game first")?;
        if v.phase != Phase::Human {
            return Err("Wait for your turn".into());
        }
        if v.decision.is_some() {
            return Err("Resolve the required choice first".into());
        }
        self.open(Modal::Report {
            family,
            sort: 0,
            descending: false,
            selected: 0,
        });
        self.focus_report_accept();
        self.status = "Report: Up/Down select, S sorts, Enter opens a city, Esc closes.".into();
        Ok(())
    }

    /// Enter opens the selected city unless a tab or header was focused with Tab.
    fn focus_report_accept(&mut self) {
        self.focus_button = self.buttons().len().saturating_sub(2);
    }

    fn report_rows(&self) -> usize {
        match &self.modal {
            Some(Modal::Report {
                family,
                sort,
                descending,
                ..
            }) => self
                .sorted_report(*family, *sort, *descending)
                .map_or(0, |t| t.rows.len()),
            _ => 0,
        }
    }

    pub(super) fn report_select(&mut self, delta: isize) {
        let rows = self.report_rows();
        if let Some(Modal::Report { selected, .. }) = &mut self.modal {
            let max = rows.saturating_sub(1) as isize;
            *selected = (*selected as isize + delta).clamp(0, max) as usize;
        }
    }

    pub(super) fn report_sort(&mut self, column: Option<usize>) {
        let Some(Modal::Report {
            family,
            sort,
            descending,
            selected,
        }) = self.modal.clone()
        else {
            return;
        };
        let columns = self.report_table(family).map_or(1, |t| t.headers.len());
        let (sort, descending) = match column {
            // A header click: the same column flips direction.
            Some(c) if c == sort => (c, !descending),
            Some(c) => (c.min(columns - 1), false),
            // `S`: next column.
            None => ((sort + 1) % columns, false),
        };
        let _ = selected;
        self.modal = Some(Modal::Report {
            family,
            sort,
            descending,
            selected: 0,
        });
    }

    pub(super) fn report_family(&mut self, family: ReportFamily) {
        if matches!(self.modal, Some(Modal::Report { .. })) {
            self.open(Modal::Report {
                family,
                sort: 0,
                descending: false,
                selected: 0,
            });
            self.focus_report_accept();
        }
    }

    pub(super) fn report_click(&mut self, x: f64, y: f64) -> Result<(), String> {
        if !(TABLE_X..TABLE_X + 600.).contains(&x) || y < ROW_Y {
            return Ok(());
        }
        let row = ((y - ROW_Y) / ROW_H) as usize;
        let Some(Modal::Report { selected, .. }) = &self.modal else {
            return Ok(());
        };
        let index = *selected / PAGE_ROWS * PAGE_ROWS + row;
        if row < PAGE_ROWS && index < self.report_rows() {
            if let Some(Modal::Report { selected, .. }) = &mut self.modal {
                *selected = index;
            }
        }
        Ok(())
    }

    /// Enter on a city row: open the workspace; `Esc` there returns here.
    pub(super) fn report_open_city(&mut self) -> Result<(), String> {
        let Some(
            modal @ Modal::Report {
                family,
                sort,
                descending,
                selected,
            },
        ) = self.modal.clone()
        else {
            return Err("No report".into());
        };
        let table = self
            .sorted_report(family, sort, descending)
            .ok_or("No running game")?;
        let city = table
            .rows
            .get(selected)
            .ok_or("Select a row first")?
            .city
            .ok_or("This report row has no city to open")?;
        self.city = Some(city);
        self.action(Action::OpenCity)?;
        self.report_return = Some(modal);
        self.modal = None;
        self.status = "City from report. Esc returns to the report.".into();
        Ok(())
    }

    /// Esc/close in the city workspace: reopen the report we came from.
    pub(super) fn report_return_after_city(&mut self) {
        if let Some(modal) = self.report_return.take() {
            self.open(modal);
            self.focus_report_accept();
        }
    }

    pub(super) fn report_buttons(&self) -> Vec<Button> {
        let Some(Modal::Report {
            family,
            sort,
            descending,
            ..
        }) = &self.modal
        else {
            return Vec::new();
        };
        let mut b = Vec::new();
        for (i, f) in [
            ReportFamily::CityStatus,
            ReportFamily::Defense,
            ReportFamily::Trade,
        ]
        .into_iter()
        .enumerate()
        {
            b.push(Button {
                x: TABLE_X + i as f64 * 150.,
                y: 150.,
                w: 144.,
                h: 28.,
                label: family_label(f).into(),
                action: Action::ReportFamily(f),
            });
        }
        if let Some(t) = self.report_table(*family) {
            for (c, (h, (x, w))) in t.headers.iter().zip(&t.columns).enumerate() {
                let mark = if c == (*sort).min(t.headers.len() - 1) {
                    if *descending {
                        " v"
                    } else {
                        " ^"
                    }
                } else {
                    ""
                };
                b.push(Button {
                    x: TABLE_X + x,
                    y: HEADER_Y,
                    w: w - 2.,
                    h: 24.,
                    label: format!("{h}{mark}"),
                    action: Action::ReportSort(c),
                });
            }
        }
        b
    }

    pub(super) fn report_draws(&self, draws: &mut Vec<Draw>) {
        let Some(Modal::Report {
            family,
            sort,
            descending,
            selected,
        }) = &self.modal
        else {
            return;
        };
        let Some(t) = self.sorted_report(*family, *sort, *descending) else {
            return;
        };
        let page = selected / PAGE_ROWS;
        let pages = t.rows.len().div_ceil(PAGE_ROWS).max(1);
        if t.rows.is_empty() {
            draws.push(text(
                TABLE_X,
                ROW_Y + 8.,
                "Nothing to report yet.",
                16,
                "#eee8d4",
            ));
        }
        for (i, row) in t
            .rows
            .iter()
            .enumerate()
            .skip(page * PAGE_ROWS)
            .take(PAGE_ROWS)
        {
            let y = ROW_Y + (i - page * PAGE_ROWS) as f64 * ROW_H;
            if i == *selected {
                draws.push(rect(TABLE_X - 4., y - 2., 608., ROW_H - 1., "#6b7057"));
            }
            let color = if row.warn { "#ffba8a" } else { "#eee8d4" };
            for (cell, (x, _)) in row.cells.iter().zip(&t.columns) {
                draws.push(text(TABLE_X + x + 4., y + 3., cell, 13, color));
            }
        }
        let foot_y = ROW_Y + PAGE_ROWS as f64 * ROW_H + 8.;
        for (i, line) in t.footer.iter().enumerate() {
            draws.push(text(TABLE_X, foot_y + i as f64 * 18., line, 13, "#ddd9bd"));
        }
        draws.push(text(
            TABLE_X + 470.,
            158.,
            &format!(
                "Row {}/{} · page {}/{}",
                (*selected + 1).min(t.rows.len()),
                t.rows.len(),
                page + 1,
                pages
            ),
            13,
            "#eee8d4",
        ));
    }

    /// Keys that exist only inside a report; `None` leaves the key to the generic code.
    pub(super) fn report_key(&mut self, key: &str) -> Option<Result<(), String>> {
        if !matches!(self.modal, Some(Modal::Report { .. })) {
            return None;
        }
        match key {
            "ArrowDown" => self.report_select(1),
            "ArrowUp" => self.report_select(-1),
            "PageDown" => self.report_select(PAGE_ROWS as isize),
            "PageUp" => self.report_select(-(PAGE_ROWS as isize)),
            "s" => self.report_sort(None),
            "F1" => self.report_family(ReportFamily::CityStatus),
            "F2" => self.report_family(ReportFamily::Defense),
            "F5" => self.report_family(ReportFamily::Trade),
            _ => return None,
        }
        Some(Ok(()))
    }
}
