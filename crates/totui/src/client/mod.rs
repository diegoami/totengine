//! One controller, filtered presentation and persistence for both native adapters.
mod atlas;
pub mod browser;
mod chrome;
mod city;
mod city_panels;
#[cfg(feature = "desktop")]
pub mod desktop;
#[cfg(any(feature = "desktop", test))]
mod editor;
mod isotrident;
mod paths;
mod scenario;
mod science;
mod selection;
pub mod storage;
mod style;
mod tips;
use crate::resources::Resources;
pub use atlas::{Atlas, Sprite};
use engine::human::{
    Command, Decision, Entity, HumanSession, HumanView, Phase, ReportFamily, Request,
};
use engine::newgame::NewGameConfig;
use engine::player::CityName;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use city_panels::CityPanel;

/// Message log rows per page and retained entries (#284).
const MESSAGE_ROWS: usize = 16;
/// Checkpoint names per Load page (#284: more than one page now scrolls).
const LOAD_ROWS: usize = 8;
const MESSAGE_LOG_CAP: usize = 500;
const NOTICE_OVERLAY_LINES: usize = 6;

pub const WIDTH: u32 = 1024;
pub const HEIGHT: u32 = 768;
/// Widest logical stage (21:9 at 768 high); wider windows letterbox (#321).
pub const MAX_WIDTH: u32 = 1792;
/// The logical stage width for a window `w`×`h` pixels: 768 high, as wide as
/// the window's aspect allows, from 1024 (4:3) up to `MAX_WIDTH`.
pub fn stage_width(w: u32, h: u32) -> u32 {
    if h == 0 {
        return WIDTH;
    }
    ((f64::from(w) * f64::from(HEIGHT) / f64::from(h)).floor() as u32).clamp(WIDTH, MAX_WIDTH)
}
// Presentation labels for the authored demo, kept separate from its versioned
// rules so existing demo checkpoints retain their original identity.
const DEMO_TECH_NAMES: [&str; 12] = [
    "Agriculture",
    "Masonry",
    "Metalworking",
    "Writing",
    "Pottery",
    "Navigation",
    "Commerce",
    "Mathematics",
    "Engineering",
    "Medicine",
    "Education",
    "Astronomy",
];

fn random_seed() -> Result<String, String> {
    let mut bytes = [0; 4];
    getrandom::fill(&mut bytes).map_err(|e| format!("Cannot generate a random seed: {e}"))?;
    Ok(u32::from_le_bytes(bytes).to_string())
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Input {
    Click {
        x: f64,
        y: f64,
    },
    DoubleClick {
        x: f64,
        y: f64,
        first_click: u64,
    },
    Key {
        key: String,
    },
    /// One mouse-wheel notch over a logical position: positive scrolls down
    /// (later rows). Presentation-only; it never issues an order (#284).
    Wheel {
        x: f64,
        y: f64,
        delta: i32,
    },
    Edit {
        field: String,
        value: String,
    },
    Focus {
        focused: bool,
    },
    /// The adapter's display (#321): the logical stage width that fits the
    /// window at 768 high, and whether it is full screen. Presentation-only
    /// and accepted at any revision, since a window can resize at any time.
    Display {
        width: u32,
        fullscreen: bool,
    },
    Tick,
}
#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Draw {
    Tile {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        sprite: String,
    },
    Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        color: String,
    },
    Text {
        x: f64,
        y: f64,
        text: String,
        color: String,
        size: u32,
    },
    Sprite {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        sprite: String,
    },
    UnitSprite {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        sprite: String,
        unit: usize,
        own: bool,
        selected: bool,
        moves_left: u32,
        blink: bool,
    },
    Cursor {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        color: String,
        blink: bool,
    },
    Line {
        x: f64,
        y: f64,
        x2: f64,
        y2: f64,
        color: String,
        width: f64,
    },
}
#[derive(Clone, Serialize)]
pub struct Entry {
    pub id: String,
    pub value: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}
#[derive(Serialize)]
pub struct Frame {
    pub revision: u64,
    pub phase: String,
    pub advancing: bool,
    pub city_click: Option<u64>,
    pub draws: Vec<Draw>,
    pub entry: Option<Entry>,
    pub status: String,
    pub assets: String,
    /// Logical width the draws use (#321); the height is always 768.
    pub width: f64,
    /// The player asked for full screen (View menu or Alt+Enter); adapters
    /// apply it and report back through `Input::Display`.
    pub fullscreen: bool,
    /// Map tooltips (#284), when enabled: each client shows a tip's draws
    /// after the pointer rests inside its hit area. Built from the filtered
    /// view only; showing one never sends input.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tips: Vec<Tip>,
    /// Path preview (#284): the route steps tips point into. A tip with a
    /// `route` also shows each step from there back along `parent`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub route: Vec<RouteStep>,
}
/// One step of a previewed route: its marker draws (empty off the map or at
/// the unit itself) and the step before it.
#[derive(Clone, Serialize)]
pub struct RouteStep {
    pub parent: Option<usize>,
    pub draws: Vec<Draw>,
}
/// One hover area: a diamond centred on `(x, y)` with half-extents `(hw, hh)`
/// when `diamond`, else the rectangle `x, y, hw*2, hh*2` from its corner.
#[derive(Clone, Serialize)]
pub struct Tip {
    pub x: f64,
    pub y: f64,
    pub hw: f64,
    pub hh: f64,
    pub diamond: bool,
    pub draws: Vec<Draw>,
    /// Path preview: the last route step before this tip's tile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<usize>,
}
impl Tip {
    pub fn contains(&self, px: f64, py: f64) -> bool {
        if self.diamond {
            (px - self.x).abs() / self.hw + (py - self.y).abs() / self.hh <= 1.
        } else {
            (self.x..self.x + 2. * self.hw).contains(&px)
                && (self.y..self.y + 2. * self.hh).contains(&py)
        }
    }
}
/// The first tip under the pointer; earlier tips (unit and city sprites)
/// take precedence over the tile diamonds behind them.
pub fn tip_at(tips: &[Tip], x: f64, y: f64) -> Option<&Tip> {
    tips.iter().find(|t| t.contains(x, y))
}
/// What a shown tip adds to the frame: its route's step markers, then its
/// own draws on top. Bounded by the route length, so a bad parent cannot loop.
pub fn tip_draws(route: &[RouteStep], tip: &Tip) -> Vec<Draw> {
    let mut draws = Vec::new();
    let mut at = tip.route;
    for _ in 0..route.len() {
        let Some(step) = at.and_then(|i| route.get(i)) else {
            break;
        };
        draws.extend(step.draws.iter().cloned());
        at = step.parent;
    }
    draws.extend(tip.draws.iter().cloned());
    draws
}
/// How long the pointer must rest before a tip shows, in both clients.
pub const TIP_DELAY_MS: u128 = 450;
mod pedia;
mod reports;
mod routes;
/// One opt-in enhancement toggle on the settings screen (#284).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Enhancement {
    NoticeOverlay,
    ConfirmMoves,
    MapTooltips,
    PathPreview,
    GoalEstimates,
    OrderStrip,
    MessageDock,
}
#[derive(Clone)]
enum Modal {
    Name,
    Research,
    /// `review` is the pending `Decision::Rates` id after a government change.
    Rates {
        review: Option<u64>,
        government: u8,
        cap: u8,
        /// `[tax, luxury, science]` draft and the split the dialog opened with.
        draft: [u8; 3],
        opened: [u8; 3],
        row: usize,
        quote: Option<engine::rates::RatesQuote>,
    },
    Revolution,
    Government {
        decision: u64,
    },
    Science,
    /// A plain informational dialog: `title` and its lines, closed with
    /// Enter or Esc (#292: a toolbar button that cannot act explains why).
    Notice {
        title: &'static str,
        lines: Vec<String>,
    },
    /// The keyboard help overlay (#284): original keys plus our additions.
    Help,
    /// The persistent message log (#284), newest first, paged.
    Messages,
    /// Opt-in enhancements (#284): the B-layer settings, all off by default.
    Settings,
    Goal,
    /// Civilopedia reference text; `back` is the dialog (and research choice) to
    /// return to when it was opened from one.
    Pedia {
        section: usize,
        selected: usize,
        reading: bool,
        scroll: usize,
        back: Option<Box<(Modal, Option<i32>)>>,
    },
    /// An own-civilization report; `selected` indexes the sorted rows.
    Report {
        family: ReportFamily,
        sort: usize,
        descending: bool,
        selected: usize,
    },
    Production {
        city: Entity,
    },
    Improvements {
        city: Entity,
    },
    Purchase {
        city: Entity,
        quote: engine::city_production::HurryQuote,
        revision: u64,
    },
    Sale {
        city: Entity,
        preview: engine::city_production::SalePreview,
        revision: u64,
    },
    Units {
        at: (u16, u16),
        units: Vec<Entity>,
    },
    Confirm {
        command: Command,
        revision: u64,
        label: String,
    },
    Save,
    Load,
    NewGame,
    Quit,
    /// Experimental scenario entry (decision 0008, 2026-11-18): the file list
    /// and the preflight shown before starting.
    Scenarios,
    ScenarioStart(Box<scenario::ScenarioChoice>),
}
#[derive(Clone)]
enum Action {
    QuickStart,
    Start,
    /// Opens the experimental scenario list.
    Scenarios,
    ScenarioPick(String),
    ScenarioCiv(usize),
    Civs,
    Human,
    Difficulty,
    Size,
    Mode,
    Work,
    Found,
    /// Opens the enhancements settings.
    Settings,
    /// Flips one enhancement.
    Toggle(Enhancement),
    /// Opens the message log.
    Messages,
    /// Hides the new-messages overlay on the map.
    DismissNotices,
    /// Opens the keyboard help overlay.
    Help,
    Research,
    Rates,
    Revolution,
    Science,
    Pedia,
    PediaSection(usize),
    PediaFor,
    Report(ReportFamily),
    ReportFamily(ReportFamily),
    ReportSort(usize),
    GoalList,
    ClearGoal,
    RateStep(usize, i32),
    RateLock(usize),
    City,
    OpenCity,
    CloseCity,
    CityStep(i32),
    CityPanel(CityPanel),
    CityPanelPage(i32),
    /// Next presentation order of the open city panel's list (#284).
    CityPanelSort,
    InspectSale(Entity, u8),
    Worker(Entity, u8),
    Specialist(Entity, u8),
    Production(Entity),
    Buy(Entity),
    Improvements(Entity),
    Choose(i32),
    Accept,
    Cancel,
    End,
    Wait,
    Skip,
    Center,
    Zoom(f64),
    /// Standard zoom, centred (Shift+Z).
    ZoomReset,
    /// Opens or closes one menu of the menu bar (#310).
    Menu(usize),
    /// Runs one menu entry: `(menu, row)`.
    MenuItem(usize, usize),
    /// Flips the science advisor's available-advances order (#315).
    ScienceSort,
    /// Asks the adapter for full screen, or back to a window (#321).
    FullScreen,
    Pan(f64, f64),
    Save,
    Load,
    NewGame,
    Quit,
    Page(i32),
    Slot(String),
    Unit(Entity),
    Activate,
    Home,
    Seed,
}
#[derive(Clone)]
struct Button {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    label: String,
    action: Action,
}
impl Button {
    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}
struct Loaded {
    resources: Resources,
    atlas: Atlas,
    session: HumanSession,
    identity: String,
    scenario_snapshot: Option<Vec<u8>>,
}

#[derive(Clone, Copy)]
struct CityClick {
    first: u64,
    city: Entity,
    x: f64,
    y: f64,
    session_revision: u64,
}
/// Lines of one event TEXT box in the notice dialog: each source line
/// word-wrapped to the dialog, at most [`EVENT_BOX_LINES`] (Macro.txt allows
/// ten lines of 255 characters; longer boxes end with an ellipsis).
fn event_box_lines(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text
        .lines()
        .flat_map(|l| {
            let w = pedia::wrap(l, 68);
            if w.is_empty() {
                vec![String::new()]
            } else {
                w
            }
        })
        .collect();
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    if lines.len() > EVENT_BOX_LINES {
        lines.truncate(EVENT_BOX_LINES - 1);
        lines.push("…".into());
    }
    lines
}
const EVENT_BOX_LINES: usize = 16;

pub struct Client {
    loaded: Option<Loaded>,
    /// Presentation-only message log (#284): each turn's notices, oldest
    /// first. Not part of checkpoints; a new game or a load starts it empty.
    notice_log: Vec<(i16, String)>,
    /// The turn whose notices were last appended, so each turn logs once.
    notice_turn: Option<i16>,
    /// New notices arrived since the log or overlay was last dismissed.
    notice_unread: bool,
    /// Scenario event TEXT boxes (#326) waiting to open, oldest first, and
    /// the session's `event_serial` already taken into the queue.
    event_boxes: Vec<String>,
    event_seen: u64,
    pub closed: bool,
    advancing: bool,
    saves: PathBuf,
    home: String,
    seed: String,
    civs: u8,
    human: u8,
    difficulty: u8,
    size: usize,
    revision: u64,
    receipts: BTreeMap<u64, (u64, Input)>,
    status: String,
    modal: Option<Modal>,
    entry: String,
    modal_id: u64,
    editing: String,
    focused: bool,
    focus_button: usize,
    page: usize,
    choice: Option<i32>,
    active: Option<Entity>,
    city: Option<Entity>,
    city_open: bool,
    city_tile: u8,
    pending_city_click: Option<CityClick>,
    city_panel: CityPanel,
    city_panel_page: usize,
    city_panel_sort: city_panels::CitySort,
    /// Slider locks persist for the client session only (the original keeps
    /// them in globals); they are never engine state or checkpointed.
    rate_locks: [bool; 3],
    /// The report a city workspace was opened from; Esc returns to it.
    report_return: Option<Modal>,
    inspect: Option<(u16, u16)>,
    moving: bool,
    /// Opt-in enhancements (#284), persisted beside the saves; all off by default.
    enhancements: storage::Enhancements,
    pan: (f64, f64),
    zoom: f64,
    /// The open menu of the menu bar (#310) and its highlighted row;
    /// presentation only.
    menu: Option<usize>,
    menu_row: Option<usize>,
    /// Logical stage width reported by the adapter (#321).
    stage_w: f64,
    /// Full screen as requested by the player (#321).
    fullscreen: bool,
    /// The science advisor lists available advances by name (#315).
    science_by_name: bool,
    /// Files listed by the experimental scenario entry.
    scenarios: Vec<String>,
    assets: String,
    fallback_pack: Option<PathBuf>,
}
impl Client {
    pub fn new(home: String, saves: PathBuf) -> Self {
        let pack = std::env::var_os("TOTENGINE_ISOTRIDENT")
            .map(PathBuf::from)
            .or_else(|| {
                let path = std::env::current_exe().ok()?.parent()?.join("isotrident");
                path.is_dir().then_some(path)
            });
        Self::with_fallback(home, saves, pack)
    }

    /// Explicit configuration also lets tests avoid process-global environment.
    pub fn with_fallback(home: String, saves: PathBuf, fallback_pack: Option<PathBuf>) -> Self {
        let enhancements = storage::load_enhancements(&saves);
        let memory = storage::load_setup_memory(&saves);
        let (seed, status) = match random_seed() {
            Ok(seed) => (
                seed,
                "Choose a ToT install, or leave blank to use the Isotrident open demo.".into(),
            ),
            Err(why) => (String::new(), format!("{why}. Enter a seed manually.")),
        };
        Self {
            loaded: None,
            closed: false,
            advancing: false,
            notice_log: Vec::new(),
            notice_turn: None,
            notice_unread: false,
            event_boxes: Vec::new(),
            event_seen: 0,
            saves,
            // An explicit home (e.g. CIVTOT_HOME) wins; otherwise the last
            // successfully started ToT directory is remembered (#266).
            home: if home.is_empty() {
                memory.as_ref().map(|m| m.home.clone()).unwrap_or_default()
            } else {
                home
            },
            seed,
            civs: memory.as_ref().map_or(5, |m| m.civs),
            human: memory.as_ref().map_or(1, |m| m.human),
            difficulty: memory.as_ref().map_or(2, |m| m.difficulty),
            size: memory.as_ref().map_or(0, |m| m.size),
            revision: 0,
            receipts: BTreeMap::new(),
            status,
            modal: None,
            entry: String::new(),
            modal_id: 0,
            editing: "home".into(),
            focused: true,
            focus_button: 0,
            page: 0,
            choice: None,
            active: None,
            city: None,
            city_open: false,
            city_tile: 20,
            pending_city_click: None,
            city_panel: CityPanel::Info,
            city_panel_page: 0,
            city_panel_sort: city_panels::CitySort::Listed,
            rate_locks: [false; 3],
            report_return: None,
            inspect: None,
            moving: false,
            enhancements,
            pan: (0., 0.),
            zoom: 1.,
            menu: None,
            menu_row: None,
            stage_w: f64::from(WIDTH),
            fullscreen: false,
            science_by_name: false,
            scenarios: Vec::new(),
            assets: "none".into(),
            fallback_pack,
        }
    }
    fn load_resources(&self) -> Result<(Resources, Atlas), String> {
        if !self.home.trim().is_empty() {
            match Resources::load(Path::new(&self.home), "Original")
                .and_then(|r| Atlas::load(&r).map(|a| (r, a)))
            {
                Ok(loaded) => return Ok(loaded),
                Err(original_error) => {
                    // An explicitly entered install must supply its own rules.
                    // Never replace a typo/corrupt install with different balance.
                    let root = Path::new(&self.home)
                        .canonicalize()
                        .map_err(|_| original_error.clone())?;
                    let path = if root.is_file() {
                        root.clone()
                    } else {
                        crate::resources::client_rules_path(&root)?.ok_or(original_error.clone())?
                    };
                    let pack = self.fallback_pack.as_ref().ok_or(original_error)?;
                    let r = Resources::load_isotrident(pack, Some(&path))?;
                    let a = Atlas::load(&r)?;
                    return Ok((r, a));
                }
            }
        }
        let pack = self.fallback_pack.as_ref().ok_or("Isotrident artwork pack missing; install it beside the client or set TOTENGINE_ISOTRIDENT")?;
        let r = Resources::load_isotrident(pack, None)?;
        let a = Atlas::load(&r)?;
        Ok((r, a))
    }
    pub fn atlas(&self) -> Option<&Atlas> {
        self.loaded.as_ref().map(|l| &l.atlas)
    }
    pub fn checkpoint(&self) -> Option<String> {
        self.loaded
            .as_ref()
            .map(|l| l.session.checkpoint(&l.identity))
    }
    pub fn view(&self) -> Option<HumanView> {
        self.loaded.as_ref().map(|l| l.session.view())
    }
    fn open(&mut self, modal: Modal) {
        self.modal = Some(modal);
        self.modal_id += 1;
        self.page = 0;
        self.choice = None;
        self.entry.clear();
        self.focus_button = if matches!(self.modal, Some(Modal::Name)) {
            1
        } else {
            0
        };
    }
    fn required(&mut self) {
        match self.view().and_then(|v| v.decision) {
            Some(Decision::Founding { .. }) => self.open(Modal::Name),
            Some(Decision::Research {
                available, default, ..
            }) => {
                self.open(Modal::Research);
                // A goal-steered suggestion is pre-selected, never accepted for you.
                if let Some(d) = default {
                    self.choice = Some(d as i32);
                    self.page = available.iter().position(|&a| a == d).unwrap_or(0) / 12;
                }
            }
            Some(Decision::Government { id, .. }) => self.open(Modal::Government { decision: id }),
            Some(Decision::Rates { .. }) => {
                if let Err(why) = self.open_rates() {
                    self.status = why;
                }
            }
            _ => (),
        }
    }
    fn center(&mut self) {
        if let Some(v) = self.view() {
            let spot = self
                .active
                .and_then(|a| {
                    v.observation
                        .units
                        .iter()
                        .find(|u| u.index == a.slot)
                        .map(|u| (u.x, u.y))
                })
                .or_else(|| {
                    v.observation
                        .cities
                        .iter()
                        .find(|c| c.owner == v.observation.civ)
                        .map(|c| (c.x, c.y))
                });
            if let Some((x, y)) = spot {
                let (ax, ay) = self.anchor();
                self.pan = (
                    ax - f64::from(x) * 32. * self.zoom,
                    ay - f64::from(y) * 16. * self.zoom,
                );
            }
        }
    }
    /// The new-messages overlay (#284) shows on the map until dismissed.
    fn notice_overlay(&self) -> bool {
        self.enhancements.notice_overlay
            && self.side_x().is_none()
            && self.notice_unread
            && self.modal.is_none()
            && !self.city_open
            && self.loaded.is_some()
    }
    /// Flips one opt-in enhancement and persists it (best effort).
    fn toggle_enhancement(&mut self, e: Enhancement) {
        let (flag, name) = match e {
            Enhancement::NoticeOverlay => (
                &mut self.enhancements.notice_overlay,
                "New-messages overlay",
            ),
            Enhancement::ConfirmMoves => {
                (&mut self.enhancements.confirm_moves, "Confirm every move")
            }
            Enhancement::MapTooltips => (&mut self.enhancements.map_tooltips, "Map tooltips"),
            Enhancement::PathPreview => (&mut self.enhancements.path_preview, "Path preview"),
            Enhancement::GoalEstimates => (&mut self.enhancements.goal_estimates, "Goal estimates"),
            Enhancement::OrderStrip => (&mut self.enhancements.order_strip, "Order strip"),
            Enhancement::MessageDock => (&mut self.enhancements.message_dock, "Message dock"),
        };
        *flag = !*flag;
        let state = if *flag { "on" } else { "off" };
        self.status = match storage::save_enhancements(&self.saves, &self.enhancements) {
            Ok(()) => format!("{name}: {state}."),
            Err(why) => format!("{name}: {state} for this session (not saved: {why})."),
        };
    }
    /// The overlay's latest-turn lines and its frame `(x, y, w, h)`.
    fn notice_overlay_rect(&self) -> (Vec<&str>, (f64, f64, f64, f64)) {
        let turn = self.notice_turn;
        let lines: Vec<&str> = self
            .notice_log
            .iter()
            .filter(|(t, _)| Some(*t) == turn)
            .map(|(_, s)| s.as_str())
            .take(NOTICE_OVERLAY_LINES)
            .collect();
        let h = 44. + lines.len() as f64 * 20. + 40.;
        (lines, (24., 112., 400., h))
    }
    fn reset_notices(&mut self) {
        self.notice_log.clear();
        self.notice_turn = None;
        self.notice_unread = false;
        self.event_boxes.clear();
        self.event_seen = 0;
    }
    /// Queues new scenario event TEXT boxes (#326) and opens the oldest as a
    /// pop-up once the human's turn has no other dialog or city window open
    /// (required choices come first). Each box is also kept in the log.
    fn sync_event_boxes(&mut self, v: &HumanView) {
        if v.event_serial > self.event_seen {
            let new = usize::try_from(v.event_serial - self.event_seen)
                .unwrap_or(usize::MAX)
                .min(v.event_texts.len());
            let turn = v.observation.turn;
            for t in &v.event_texts[v.event_texts.len() - new..] {
                let first = t.lines().map(str::trim).find(|l| !l.is_empty());
                self.notice_log.push((
                    turn,
                    format!("Event: {}", first.unwrap_or("(empty message)")),
                ));
                self.event_boxes.push(t.clone());
            }
            let excess = self.notice_log.len().saturating_sub(MESSAGE_LOG_CAP);
            self.notice_log.drain(..excess);
            self.event_seen = v.event_serial;
        }
        if self.event_boxes.is_empty()
            || self.modal.is_some()
            || self.city_open
            || !matches!(v.phase, Phase::Human | Phase::GameOver)
        {
            return;
        }
        let text = self.event_boxes.remove(0);
        self.open(Modal::Notice {
            title: "Scenario event",
            lines: event_box_lines(&text),
        });
    }
    /// Appends the current turn's notices to the log once (#284).
    fn sync_notices(&mut self) {
        let Some(v) = self.view() else {
            self.reset_notices();
            return;
        };
        self.sync_event_boxes(&v);
        let turn = v.observation.turn;
        if v.notices.is_empty() || self.notice_turn == Some(turn) {
            return;
        }
        self.notice_turn = Some(turn);
        for n in &v.notices {
            let line = self.notice_text(&v, n);
            self.notice_log.push((turn, line));
        }
        let excess = self.notice_log.len().saturating_sub(MESSAGE_LOG_CAP);
        self.notice_log.drain(..excess);
        self.notice_unread = true;
    }
    fn notice_text(&self, v: &HumanView, n: &engine::human::Notice) -> String {
        use engine::human::Notice;
        let city_name = |id: usize| {
            v.observation
                .cities
                .iter()
                .find(|c| c.index == id)
                .map_or_else(|| format!("City {id}"), |c| c.name.clone())
        };
        match n {
            Notice::Grew(id) => format!("{} grew", city_name(*id)),
            Notice::Starved(id) => format!("{} starved", city_name(*id)),
            Notice::CityDestroyed(id) => format!("City {id} was lost"),
            Notice::UnitDisbanded(id) => format!("Unit {id} disbanded for support"),
            Notice::Production(id, built) => {
                use engine::production::Built;
                let item = match built {
                    Built::Unit(unit) => v
                        .observation
                        .units
                        .iter()
                        .find(|u| u.index == *unit)
                        .map(|u| self.item_name(u.type_id as i8)),
                    Built::Improvement(i) => Some(self.item_name(-(*i as i8))),
                    Built::Wonder(w) => Some(self.item_name(-((*w + 40) as i8))),
                };
                match item {
                    Some(item) => format!("{} completed {item}", city_name(*id)),
                    None => format!("{} completed production", city_name(*id)),
                }
            }
            Notice::Discovery(id) => format!("Discovered {}", self.tech_name(*id)),
            Notice::Revolution => {
                "Your government fell into anarchy; save to preserve this unsupported choice".into()
            }
        }
    }
    fn refresh(&mut self) {
        if let Some(v) = self.view() {
            if self.active.is_none_or(|a| !v.units.contains(&a)) {
                self.active = v.units.iter().copied().find(|e| {
                    v.observation
                        .units
                        .iter()
                        .any(|u| u.index == e.slot && u.moves_left > 0)
                });
            }
            if self.city.is_none_or(|c| !v.cities.contains(&c)) {
                self.city = v.cities.first().copied();
            }
            if self.city.is_none() {
                self.city_open = false;
            }
        }
    }
    fn order(&mut self, command: Command) -> Result<(), String> {
        let loaded = self.loaded.as_mut().ok_or("Start a game first")?;
        let v = loaded.session.view();
        let result = loaded.session.submit(Request {
            civ: v.observation.civ,
            revision: v.revision,
            id: loaded.session.next_request_id()?,
            command,
        })?;
        self.status = result
            .result
            .map_or_else(|| "Decision ready".into(), |r| r.to_string());
        if command == Command::EndTurn {
            self.advancing = true;
        }
        self.refresh();
        match command {
            Command::Move { unit, .. } | Command::Occupy { unit, .. } | Command::Skip { unit } => {
                self.schedule_unit(Some(unit), false);
            }
            Command::Research { .. } | Command::NameCity { .. } | Command::CancelFound { .. } => {
                self.schedule_unit(None, true);
            }
            Command::EndTurn => self.moving = false,
            _ => (),
        }
        self.required();
        Ok(())
    }
    pub fn input(&mut self, revision: u64, id: u64, input: Input) -> Result<(), String> {
        if let Some((old, input_old)) = self.receipts.get(&id) {
            return if *old == revision && *input_old == input {
                Ok(())
            } else {
                Err("Request ID already used".into())
            };
        }
        if self.closed {
            return Err("Client is closed".into());
        }
        if self.receipts.len() >= 100_000 {
            return Err("Client input limit reached; save and restart".into());
        }
        if let Input::Display { width, fullscreen } = input {
            // A window can resize at any time; never stale (#321).
            self.stage_w = f64::from(width.clamp(WIDTH, MAX_WIDTH));
            self.fullscreen = fullscreen;
            self.revision += 1;
            self.receipts.insert(id, (revision, input));
            return Ok(());
        }
        if revision != self.revision {
            return Err("Stale input: inspect the refreshed selection before acting".into());
        }
        let was_overlay = self.modal.is_some() || self.city_open;
        if !matches!(input, Input::DoubleClick { .. }) {
            self.pending_city_click = None;
        }
        let result = self.handle(input.clone(), id);
        if was_overlay && self.modal.is_none() && !self.city_open {
            self.focus_end_if_idle();
        }
        // UI errors are shown and acknowledged; retrying one cannot change state.
        if let Err(error) = &result {
            self.status = error.clone();
        }
        self.sync_notices();
        self.revision += 1;
        self.receipts.insert(id, (revision, input));
        if let Some(path) = std::env::var_os("TOTENGINE_CLIENT_TRACE") {
            use std::io::Write;
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(path)
            {
                if let Ok(mut bytes) = serde_json::to_vec(
                    &serde_json::json!({"input":self.receipts.get(&id).map(|(_,input)|input),"frame":self.frame(),"checkpoint":self.checkpoint()}),
                ) {
                    bytes.push(b'\n');
                    let _ = file.write_all(&bytes);
                }
            }
        }
        Ok(())
    }
    fn opponent_work_running(&self) -> bool {
        self.advancing && !matches!(self.modal, Some(Modal::Scenarios | Modal::ScenarioStart(_)))
    }

    fn handle(&mut self, input: Input, id: u64) -> Result<(), String> {
        match input {
            Input::Focus { focused } => {
                self.focused = focused;
                Ok(())
            }
            Input::Tick => {
                if !self.opponent_work_running() {
                    return Ok(());
                }
                if let Some(l) = self.loaded.as_mut() {
                    if l.session.phase() == Phase::Opponents {
                        l.session.advance(16);
                        self.refresh();
                        if self.view().is_some_and(|v| v.phase != Phase::Opponents) {
                            self.advancing = false;
                            self.schedule_unit(None, true);
                            self.status=match self.view().unwrap().phase {Phase::Human=>"Your turn. Choose actions or resolve the required choice.",Phase::Eliminated=>"Your civilization has no units or cities. Load a checkpoint or start a new game.",Phase::TurnLimit=>"Turn limit reached. Save a checkpoint to preserve this game.",Phase::GameOver=>"Game over: scenario event. You can save a checkpoint, load a game or start a new game.",Phase::Opponents=>"Opponents playing"}.into();
                            self.required();
                        }
                    }
                }
                Ok(())
            }
            _ if !self.focused => Err("Window is unfocused; input ignored".into()),
            Input::Edit { field, value } => {
                if self.entry_meta().is_none_or(|e| e.id != field) {
                    return Err("Text field changed; input ignored".into());
                }
                if value.len() > 1024 || value.chars().any(char::is_control) {
                    return Err("Text is too long or contains control characters".into());
                }
                if self.loaded.is_none() && self.modal.is_none() {
                    if self.editing == "home" {
                        self.home = value
                    } else {
                        self.seed = value
                    }
                } else {
                    self.entry = value;
                }
                Ok(())
            }
            Input::DoubleClick { x, y, first_click } => {
                if self.can_open_double_click(first_click, x, y) {
                    // Keep the consumed gesture until a new intent arrives, so a
                    // third mouse-down cannot become a city worker edit.
                    let gesture = self.pending_city_click.unwrap();
                    self.modal = None;
                    self.city = Some(gesture.city);
                    self.action(Action::OpenCity)?;
                } else if !self.city_open {
                    // A failed second click cannot be repaired by a third event.
                    self.pending_city_click = None;
                }
                Ok(())
            }
            Input::Click { x, y } => {
                if !x.is_finite() || !y.is_finite() {
                    return Err("Invalid pointer position".into());
                }
                let buttons = self.buttons();
                // An open menu takes the click: an entry runs, a title
                // switches menus, anywhere else only closes it (#310).
                if self.menu.is_some() {
                    if let Some(b) = buttons.into_iter().find(|b| {
                        matches!(b.action, Action::Menu(_) | Action::MenuItem(..))
                            && b.contains(x, y)
                    }) {
                        return self.action(b.action);
                    }
                    self.menu = None;
                    self.menu_row = None;
                    return Ok(());
                }
                if let Some(b) = buttons.into_iter().find(|b| b.contains(x, y)) {
                    return self.action(b.action);
                }
                if self.minimap_click(x, y) {
                    return Ok(());
                }
                // Dialogs and the city window are centred on a wide stage.
                let cx = x - self.dx();
                if matches!(self.modal, Some(Modal::Report { .. })) {
                    return self.report_click(cx, y);
                }
                if matches!(self.modal, Some(Modal::Pedia { .. })) {
                    self.pedia_click(cx, y);
                    return Ok(());
                }
                if self.modal.is_none() && self.city_open {
                    return self.city_click(cx, y);
                }
                if self.notice_overlay() {
                    let (_, (ox, oy, ow, oh)) = self.notice_overlay_rect();
                    if (ox..ox + ow).contains(&x) && (oy..oy + oh).contains(&y) {
                        // The overlay sits over the map: a click on it
                        // dismisses it and never becomes a map order.
                        self.notice_unread = false;
                        return Ok(());
                    }
                }
                if self.modal.is_none() && self.loaded.is_some() && self.on_map(x, y) {
                    self.map_click(x, y, id)
                } else if self.view().is_some_and(|v| v.decision.is_some()) {
                    // A click that lands on nothing while a required choice is
                    // pending gets the same visible reason the keys give
                    // (#283/#284: no silent dead zones during acceptance).
                    self.status = "Resolve the required choice first; Esc returns to inspection, Advisors > Research reopens it.".into();
                    Ok(())
                } else {
                    Ok(())
                }
            }
            Input::Key { key } => self.key(&key),
            Input::Display { .. } => Ok(()),
            Input::Wheel { x, y, delta } => {
                if !x.is_finite() || !y.is_finite() {
                    return Err("Invalid pointer position".into());
                }
                self.wheel(x, delta.signum());
                Ok(())
            }
        }
    }
    /// Scroll the list under the pointer one page per notch. Only lists
    /// scroll: over the map or a dialog without a list the wheel does
    /// nothing, so it can never move a unit or change a game choice.
    fn wheel(&mut self, x: f64, delta: i32) {
        if delta == 0 {
            return;
        }
        let key = if delta > 0 { "PageDown" } else { "PageUp" };
        let _ = match &self.modal {
            Some(Modal::Report { .. }) => self.report_key(key).unwrap_or(Ok(())),
            Some(Modal::Pedia { .. }) => self.pedia_key(key).unwrap_or(Ok(())),
            Some(Modal::Messages | Modal::Load | Modal::Scenarios) => {
                self.action(Action::Page(delta))
            }
            Some(_) if !self.current_choices().is_empty() => self.action(Action::Page(delta)),
            Some(_) => Ok(()),
            None if self.city_open && x - self.dx() >= 736. => {
                self.action(Action::CityPanelPage(delta))
            }
            None => Ok(()),
        };
    }
    fn key(&mut self, key: &str) -> Result<(), String> {
        if let Some(result) = self.menu_key(key) {
            return result;
        }
        if key == "Alt+Enter" {
            return self.action(Action::FullScreen);
        }
        if key == "F10" && self.map_surface() {
            self.menu = Some(0);
            self.menu_row = Some(0);
            return Ok(());
        }
        if key == "Tab" || key == "ShiftTab" {
            let count = self.buttons().len();
            if count > 0 {
                self.focus_button =
                    (self.focus_button + if key == "Tab" { 1 } else { count - 1 }) % count;
            }
            return Ok(());
        }
        if key == "Enter" {
            if self.modal.is_some() {
                if let Some(b) = self.buttons().get(self.focus_button) {
                    if matches!(
                        b.action,
                        Action::Save
                            | Action::Load
                            | Action::ReportSort(_)
                            | Action::ReportFamily(_)
                            | Action::PediaSection(_)
                    ) {
                        return self.action(b.action.clone());
                    }
                }
            }
            if self.modal.is_some()
                && self
                    .buttons()
                    .get(self.focus_button)
                    .is_some_and(|b| matches!(b.action, Action::Cancel))
            {
                return self.action(Action::Cancel);
            }
            return if self.modal.is_some() {
                self.action(Action::Accept)
            } else {
                let buttons = self.buttons();
                if let Some(b) = buttons.get(self.focus_button) {
                    self.action(b.action.clone())
                } else {
                    Ok(())
                }
            };
        }
        if key == "Escape" {
            if self.modal.is_none() && self.city_open {
                return self.action(Action::CloseCity);
            }
            return self.action(Action::Cancel);
        }
        if self.modal.is_some() {
            if let Some(result) = self.route_key(key) {
                return result;
            }
            if let Some(result) = self.report_key(key) {
                return result;
            }
            if let Some(result) = self.pedia_key(key) {
                return result;
            }
            match key {
                "ArrowDown" | "ArrowUp" => {
                    let choices = self.current_choices();
                    if choices.is_empty() {
                        return Ok(());
                    }
                    let at = choices.iter().position(|(id, _)| Some(*id) == self.choice);
                    let next = if key == "ArrowDown" {
                        at.map_or(0, |i| (i + 1).min(choices.len() - 1))
                    } else {
                        at.map_or(choices.len() - 1, |i| i.saturating_sub(1))
                    };
                    self.page = next / 12;
                    self.focus_button = next % 12;
                    self.action(Action::Choose(choices[next].0))
                }
                "PageDown" => self.action(Action::Page(1)),
                "PageUp" => self.action(Action::Page(-1)),
                // A key the open modal does not handle: during a REQUIRED
                // choice, say so instead of swallowing it silently — live,
                // the route keys looked "not connected" (#283/#284).
                _ => {
                    if self.view().is_some_and(|v| v.decision.is_some())
                        && (key.starts_with('F') || key.len() == 1)
                    {
                        self.status = "Resolve the required choice first; Esc returns to inspection, Advisors > Research reopens it.".into();
                    }
                    Ok(())
                }
            }
        } else if self.city_open {
            self.city_key(key)
        } else {
            match key {
                "b" => self.action(Action::Found),
                "?" => self.action(Action::Help),
                "Shift+t" => self.action(Action::Rates),
                "Shift+r" => self.action(Action::Revolution),
                "F6" => self.action(Action::Science),
                "F1" => self.action(Action::Report(ReportFamily::CityStatus)),
                "F2" => self.action(Action::Report(ReportFamily::Defense)),
                "F4" => self.action(Action::Report(ReportFamily::Attitude)),
                "F5" => self.action(Action::Report(ReportFamily::Trade)),
                "F7" => self.action(Action::Report(ReportFamily::Wonders)),
                "F8" => self.action(Action::Report(ReportFamily::RankedCities)),
                "F9" => self.action(Action::Report(ReportFamily::Score)),
                "F11" => self.action(Action::Report(ReportFamily::Demographics)),
                "v" => self.action(Action::Mode),
                "a" => self.action(Action::Activate),
                "c" => self.action(Action::Center),
                "Ctrl+n" => self.action(Action::End),
                "w" => self.action(Action::Wait),
                " " => self.action(Action::Skip),
                "Ctrl+s" => self.action(Action::Save),
                "Ctrl+l" => self.action(Action::Load),
                "z" | "+" | "=" => self.action(Action::Zoom(1.25)),
                "x" | "-" => self.action(Action::Zoom(0.8)),
                "Shift+z" => self.action(Action::ZoomReset),
                "ArrowLeft" if self.moving => self.step(-2, 0),
                "ArrowRight" if self.moving => self.step(2, 0),
                "ArrowUp" if self.moving => self.step(0, -2),
                "ArrowDown" if self.moving => self.step(0, 2),
                "ArrowLeft" => self.action(Action::Pan(64., 0.)),
                "ArrowRight" => self.action(Action::Pan(-64., 0.)),
                "ArrowUp" => self.action(Action::Pan(0., 32.)),
                "ArrowDown" => self.action(Action::Pan(0., -32.)),
                "1" => self.step(-1, 1),
                "2" => self.step(0, 2),
                "3" => self.step(1, 1),
                "4" => self.step(-2, 0),
                "6" => self.step(2, 0),
                "7" => self.step(-1, -1),
                "8" => self.step(0, -2),
                "9" => self.step(1, -1),
                "f" | "r" | "m" | "o" | "e" | "s" | "l" => {
                    Err("That original unit order is not implemented in this client yet.".into())
                }
                _ => Ok(()),
            }
        }
    }
    fn action(&mut self, a: Action) -> Result<(), String> {
        match a {
            Action::Home => self.editing = "home".into(),
            Action::Seed => self.editing = "seed".into(),
            Action::Civs => {
                self.civs = if self.civs == 7 { 2 } else { self.civs + 1 };
                self.human = self.human.min(self.civs)
            }
            Action::Human => {
                self.human = if self.human == self.civs {
                    1
                } else {
                    self.human + 1
                }
            }
            Action::Difficulty => self.difficulty = (self.difficulty + 1) % 6,
            Action::Size => self.size = (self.size + 1) % 3,
            Action::QuickStart => {
                self.seed = random_seed()?;
                self.action(Action::Start)?;
            }
            Action::Start => {
                self.report_return = None;
                let seed = self
                    .seed
                    .parse::<u32>()
                    .map_err(|_| "Seed must be a whole number from 0 to 4294967295")?;
                let (resources, atlas) = self.load_resources()?;
                let identity = resources.identity();
                let session = HumanSession::generated(
                    NewGameConfig {
                        world: engine::worldgen::WorldConfig {
                            width: [80, 40, 32][self.size],
                            height: [50, 30, 24][self.size],
                            seed,
                            ..Default::default()
                        },
                        gameplay_seed: seed,
                        civilizations: self.civs,
                        difficulty: self.difficulty,
                    },
                    resources.rules.clone(),
                    self.human,
                )?;
                self.reset_notices();
                self.loaded = Some(Loaded {
                    resources,
                    atlas,
                    session,
                    identity,
                    scenario_snapshot: None,
                });
                self.assets = storage::nonce()?;
                self.advancing = self.view().is_some_and(|v| v.phase == Phase::Opponents);
                // Remember this setup for the next launch (#266); best-effort.
                let _ = storage::save_setup_memory(
                    &self.saves,
                    &storage::SetupMemory {
                        format: "totengine-setup 1".into(),
                        home: self.home.clone(),
                        civs: self.civs,
                        human: self.human,
                        difficulty: self.difficulty,
                        size: self.size,
                    },
                );
                self.city_panel = CityPanel::Info;
                self.city_panel_page = 0;
                self.editing.clear();
                self.refresh();
                self.schedule_unit(None, true);
                self.center();
                self.required();
                self.status = format!(
                    "Generated game ready. Seed: {}. Required choices are never selected for you.",
                    self.seed
                );
            }
            Action::Work => {
                if self.view().is_none_or(|v| v.phase != Phase::Opponents) {
                    return Err("Opponents are not waiting".into());
                }
                self.advancing = !self.advancing;
            }
            Action::Mode => {
                if self.moving {
                    self.moving = false;
                    self.inspect = self.active_position();
                    self.status =
                        "Viewing: inspect terrain or click an own unit to activate it.".into();
                } else {
                    self.schedule_unit(None, false);
                    if !self.moving {
                        return Err(
                            "No ready unit; resolve any required choice or end your turn.".into(),
                        );
                    }
                    self.status = "Moving: keypad, cursor arrows or adjacent empty terrain. Click an own unit to select it.".into();
                }
            }
            Action::Toggle(e) => self.toggle_enhancement(e),
            Action::Settings => self.open(Modal::Settings),
            Action::Scenarios => self.open_scenarios()?,
            Action::ScenarioPick(path) => self.pick_scenario(&path)?,
            Action::ScenarioCiv(i) => {
                if let Some(Modal::ScenarioStart(choice)) = &mut self.modal {
                    if i < choice.preflight.civs.len() {
                        choice.civ = i;
                    }
                }
            }
            Action::Found => {
                let unit = self.active.ok_or("Select an own settler first")?;
                self.order(Command::BeginFound { unit })?;
            }
            // A pending required choice reopens; with none, the toolbar
            // button shows the Science advisor (current topic and goal)
            // instead of silently doing nothing (#292).
            Action::Research => match self.view().and_then(|v| v.decision) {
                Some(_) => self.required(),
                None => self.open_science()?,
            },
            Action::Rates => self.open_rates()?,
            Action::Help => self.open(Modal::Help),
            Action::Messages => {
                self.notice_unread = false;
                self.open(Modal::Messages);
            }
            Action::DismissNotices => self.notice_unread = false,
            Action::Report(family) => self.open_report(family)?,
            Action::Pedia => self.open_pedia()?,
            Action::PediaSection(i) => self.pedia_section(i),
            Action::PediaFor => self.pedia_for_research_choice()?,
            Action::ReportFamily(family) => self.report_family(family),
            Action::ReportSort(column) => self.report_sort(Some(column)),
            Action::Revolution => self.open_revolution()?,
            Action::Science => self.open_science()?,
            Action::GoalList => self.open_goal_list()?,
            Action::ClearGoal => self.set_goal(None)?,
            Action::RateStep(row, delta) => self.rate_step(row, delta)?,
            Action::RateLock(row) => self.rate_lock(row),
            // Cities opens the next own city's window directly; each further
            // click cycles on. Without cities a notice says how to found one
            // (#292: select-and-scroll alone read as a dead button).
            Action::City => {
                let Some(v) = self.view() else {
                    return Ok(());
                };
                if v.cities.is_empty() {
                    self.open(Modal::Notice {
                        title: "Cities",
                        lines: vec![
                            "You have no cities yet.".into(),
                            "Select a settler and press B (or Found) to found one;".into(),
                            "Cities then opens each city's window in turn.".into(),
                        ],
                    });
                    return Ok(());
                }
                let next = v
                    .cities
                    .iter()
                    .position(|e| Some(*e) == self.city)
                    .map_or(0, |i| (i + 1) % v.cities.len().max(1));
                self.city = v.cities.get(next).copied();
                if let Some(c) = self
                    .city
                    .and_then(|e| v.observation.cities.iter().find(|c| c.index == e.slot))
                {
                    let (ax, ay) = self.anchor();
                    self.pan = (
                        ax - f64::from(c.x) * 32. * self.zoom,
                        ay - f64::from(c.y) * 16. * self.zoom,
                    );
                }
                self.action(Action::OpenCity)?;
            }
            Action::OpenCity => {
                self.refresh();
                let city = self.city.ok_or("No own city to manage")?;
                if !self
                    .view()
                    .is_some_and(|v| v.city_citizens.iter().any(|d| d.index == city.slot))
                {
                    return Err("This city is outside the supported surface context".into());
                }
                self.city_open = true;
                self.city_tile = 20;
                self.focus_button = 0;
                self.status = "City inspection. Citizen edits commit immediately.".into();
            }
            Action::CloseCity => {
                self.close_city_production()?;
                self.city_open = false;
                self.focus_button = 0;
                self.status = "Returned to map; accepted city edits remain committed.".into();
                self.report_return_after_city();
            }
            Action::CityPanel(panel) => {
                self.city_panel = panel;
                self.city_panel_page = 0;
                self.city_panel_sort = city_panels::CitySort::Listed;
                self.focus_button = 0;
            }
            Action::CityPanelSort => self.city_panel_sort_next(),
            Action::CityPanelPage(delta) => {
                self.city_panel_page = self
                    .city_panel_page
                    .min(self.city_panel_pages().saturating_sub(1))
                    .saturating_add_signed(delta as isize)
                    .min(self.city_panel_pages().saturating_sub(1));
            }
            Action::InspectSale(city, improvement) => {
                let loaded = self.loaded.as_ref().ok_or("No running game")?;
                let preview = loaded.session.preview_sale(city, improvement)?;
                let revision = loaded.session.view().revision;
                self.open(Modal::Sale {
                    city,
                    preview,
                    revision,
                });
            }
            Action::CityStep(delta) => {
                self.close_city_production()?;
                let v = self.view().ok_or("No running game")?;
                let at = v
                    .cities
                    .iter()
                    .position(|c| Some(*c) == self.city)
                    .unwrap_or(0) as i32;
                if !v.cities.is_empty() {
                    self.city =
                        Some(v.cities[(at + delta).rem_euclid(v.cities.len() as i32) as usize]);
                }
                self.city_tile = 20;
                self.city_panel_page = 0;
                self.focus_button = 0;
            }
            Action::Worker(city, tile) => {
                self.order(Command::Worker { city, tile })?;
            }
            Action::Specialist(city, slot) => {
                self.order(Command::Specialist { city, slot })?;
            }
            Action::Production(city) => self.open(Modal::Production { city }),
            Action::Improvements(city) => self.open(Modal::Improvements { city }),
            Action::Buy(city) => {
                let v = self.view().ok_or("No running game")?;
                if !v.cities.contains(&city) {
                    return Err("City selection changed".into());
                }
                let quote = v
                    .city_production
                    .iter()
                    .find(|p| p.index == city.slot)
                    .ok_or("Unsupported city")?
                    .hurry?;
                self.open(Modal::Purchase {
                    city,
                    quote,
                    revision: v.revision,
                });
            }
            Action::Choose(id) => self.choice = Some(id),
            Action::Slot(name) => self.entry = name,
            Action::Page(delta) => {
                let last = if matches!(self.modal, Some(Modal::Messages)) {
                    self.notice_log.len().saturating_sub(1) / MESSAGE_ROWS
                } else if matches!(self.modal, Some(Modal::Load)) {
                    storage::list(&self.saves).len().saturating_sub(1) / LOAD_ROWS
                } else if matches!(self.modal, Some(Modal::Scenarios)) {
                    self.scenarios.len().saturating_sub(1) / scenario::SCENARIO_ROWS
                } else {
                    self.current_choices().len().saturating_sub(1) / 12
                };
                self.page = self.page.saturating_add_signed(delta as isize).min(last)
            }
            Action::Accept => {
                let modal = self.modal.clone().ok_or("No dialog to accept")?;
                match self.route_accept(&modal)? {
                    Some(true) => {
                        self.modal = None;
                        self.required();
                        return Ok(());
                    }
                    Some(false) => return Ok(()),
                    None => (),
                }
                match modal {
                    Modal::Improvements { city } => {
                        let id = self.choice.ok_or("Select an improvement explicitly")?;
                        let improvement = u8::try_from(id).map_err(|_| "Invalid improvement")?;
                        let l = self.loaded.as_ref().ok_or("No running game")?;
                        let preview = l.session.preview_sale(city, improvement)?;
                        let revision = l.session.view().revision;
                        self.open(Modal::Sale {
                            city,
                            preview,
                            revision,
                        });
                        return Ok(());
                    }
                    Modal::Purchase {
                        city,
                        quote,
                        revision,
                    } => {
                        if self.view().is_none_or(|v| v.revision != revision) {
                            return Err("The game changed; request a new quote".into());
                        }
                        self.order(Command::Buy { city, quote })?;
                    }
                    Modal::Sale {
                        city,
                        preview,
                        revision,
                    } => {
                        if self.view().is_none_or(|v| v.revision != revision) {
                            return Err("The game changed; request a new quote".into());
                        }
                        self.order(Command::Sell {
                            city,
                            quote: preview.quote,
                        })?;
                    }
                    Modal::Rates { .. }
                    | Modal::Revolution
                    | Modal::Government { .. }
                    | Modal::Science
                    | Modal::Notice { .. }
                    | Modal::Help
                    | Modal::Messages
                    | Modal::Settings
                    | Modal::Scenarios
                    | Modal::ScenarioStart(_)
                    | Modal::Report { .. }
                    | Modal::Pedia { .. }
                    | Modal::Goal => unreachable!("handled by route_accept"),
                    Modal::Quit => {
                        self.closed = true;
                        self.modal = None;
                        self.status =
                            "Client closed. Your saved checkpoints remain available.".into();
                        return Ok(());
                    }
                    Modal::NewGame => {
                        self.report_return = None;
                        let seed = random_seed()?;
                        self.loaded = None;
                        self.seed = seed;
                        self.city_open = false;
                        self.modal = None;
                        self.active = None;
                        self.city = None;
                        self.inspect = None;
                        self.assets = "none".into();
                        self.editing = "home".into();
                        self.status = "Choose setup for a new game.".into();
                        return Ok(());
                    }
                    Modal::Name => {
                        let Some(Decision::Founding { id, .. }) =
                            self.view().and_then(|v| v.decision)
                        else {
                            return Err("Founding decision changed".into());
                        };
                        let name = CityName::new(&self.entry)?;
                        self.order(Command::NameCity { decision: id, name })?;
                    }
                    Modal::Research => {
                        let Some(Decision::Research { id, .. }) =
                            self.view().and_then(|v| v.decision)
                        else {
                            return Err("Research decision changed".into());
                        };
                        self.order(Command::Research {
                            decision: id,
                            advance: self.choice.ok_or("Select a research topic explicitly")?
                                as usize,
                        })?;
                    }
                    Modal::Units { at, units } => {
                        let slot = self.choice.ok_or("Choose a unit explicitly")? as usize;
                        let v = self.view().ok_or("No running game")?;
                        let entity = units
                            .iter()
                            .find(|e| e.slot == slot)
                            .copied()
                            .filter(|e| v.units.contains(e))
                            .filter(|_| {
                                v.observation
                                    .units
                                    .iter()
                                    .any(|u| u.index == slot && (u.x, u.y) == at)
                            })
                            .ok_or("Unit selection changed")?;
                        self.action(Action::Unit(entity))?;
                    }
                    Modal::Production { city } => {
                        let item = self.choice.ok_or("Select a production item explicitly")? as i8;
                        let quote = self
                            .loaded
                            .as_ref()
                            .ok_or("No running game")?
                            .session
                            .preview_production(city, item)?;
                        self.order(Command::QuotedProduction { city, quote })?;
                        // The map's quick picker has no continuing city workspace.
                        // Its captured budget ends with this accepted decision.
                        if !self.city_open {
                            let status = self.status.clone();
                            self.close_city_production()?;
                            self.status = status;
                        }
                    }
                    Modal::Confirm {
                        command, revision, ..
                    } => {
                        if self.view().is_none_or(|v| v.revision != revision) {
                            return Err("The game changed; inspect the action again".into());
                        }
                        self.order(command)?;
                    }
                    Modal::Save => {
                        let l = self.loaded.as_ref().ok_or("Start a game first")?;
                        let checkpoint = l.session.checkpoint(&l.identity);
                        if let Some(snapshot) = &l.scenario_snapshot {
                            storage::save_scenario(
                                &self.saves,
                                &self.entry,
                                &checkpoint,
                                snapshot,
                            )?;
                        } else {
                            storage::save(&self.saves, &self.entry, &checkpoint)?;
                        }
                        self.status = format!("Engine checkpoint saved: {}", self.entry);
                    }
                    Modal::Load => {
                        self.report_return = None;
                        let text = storage::load(&self.saves, &self.entry)?;
                        let scenario = HumanSession::checkpoint_scenario(&text);
                        if let Some(origin) = &scenario {
                            self.restore_scenario(origin, &text)?;
                        } else if let Some(l) = self
                            .loaded
                            .as_mut()
                            .filter(|l| l.session.scenario_origin().is_none())
                        {
                            let restored = HumanSession::restore(
                                l.resources.rules.clone(),
                                &l.identity,
                                &text,
                            )?;
                            l.session = restored;
                        } else {
                            let (resources, atlas) = self.load_resources()?;
                            let identity = resources.identity();
                            let session =
                                HumanSession::restore(resources.rules.clone(), &identity, &text)?;
                            let assets = storage::nonce()?;
                            self.reset_notices();
                            self.loaded = Some(Loaded {
                                resources,
                                atlas,
                                identity,
                                session,
                                scenario_snapshot: None,
                            });
                            self.assets = assets;
                        }
                        if !self.city_open {
                            self.city_panel = CityPanel::Info;
                            self.city_panel_page = 0;
                        }
                        self.advancing = false;
                        self.active = None;
                        self.city = None;
                        self.refresh();
                        if let Some(city) = self.view().and_then(|v| v.production_context_city) {
                            self.city = Some(city);
                            self.city_open = true;
                        }
                        self.schedule_unit(None, true);
                        self.center();
                        self.status = match scenario {
                            Some(o) => format!("Experimental scenario checkpoint restored ({}).", o.path),
                            None => "Checkpoint restored, including pending decisions. Original SAV import is not supported.".into(),
                        };
                    }
                }
                self.modal = None;
                self.required();
            }
            Action::Cancel => {
                if self.route_cancel() {
                    return Ok(());
                }
                let restore_required = matches!(self.modal, Some(Modal::Save | Modal::Load));
                let production_cancel = matches!(self.modal, Some(Modal::Production { .. }));
                if matches!(self.modal, Some(Modal::Name)) {
                    if let Some(Decision::Founding { id, .. }) =
                        self.view().and_then(|v| v.decision)
                    {
                        self.order(Command::CancelFound { decision: id })?;
                    }
                }
                self.modal = None;
                if restore_required {
                    self.required();
                }
                self.status = match self.view().and_then(|v| v.decision) {
                    Some(Decision::Research { .. }) => {
                        "Research is still required; reopen with Research."
                    }
                    Some(Decision::Founding { .. }) => "City naming remains pending.",
                    Some(Decision::Government { .. }) => "A government choice remains pending.",
                    Some(Decision::Rates { .. }) => "A rate review remains pending.",
                    None if production_cancel => {
                        "Production selection cancelled; existing item unchanged."
                    }
                    None => "Cancelled.",
                }
                .into();
            }
            Action::End => {
                let v = self.view().ok_or("Start a game first")?;
                if v.phase != Phase::Human {
                    return Err("The game is not waiting for human orders".into());
                }
                if v.observation
                    .units
                    .iter()
                    .any(|u| u.owner == v.observation.civ && u.moves_left > 0)
                {
                    self.open(Modal::Confirm {
                        command: Command::EndTurn,
                        revision: v.revision,
                        label: "End your turn with movable units?".into(),
                    })
                } else {
                    self.order(Command::EndTurn)?;
                }
            }
            Action::Wait => {
                let after = self.active;
                self.active = None;
                self.schedule_unit(after, false);
            }
            Action::Skip => {
                self.order(Command::Skip {
                    unit: self.active.ok_or("Select an own unit first")?,
                })?;
            }
            Action::Center => self.center(),
            Action::ZoomReset => {
                self.zoom = 1.;
                self.center();
            }
            Action::Menu(m) => {
                if self.menu == Some(m) {
                    self.menu = None;
                } else if self.map_surface() {
                    self.menu = Some(m);
                }
                self.menu_row = None;
            }
            Action::MenuItem(m, row) => self.menu_item(m, row)?,
            Action::ScienceSort => self.science_by_name = !self.science_by_name,
            Action::FullScreen => {
                self.fullscreen = !self.fullscreen;
                self.status = if self.fullscreen {
                    "Full screen: Alt+Enter or View > Window returns to a window."
                } else {
                    "Windowed."
                }
                .into();
            }
            Action::Zoom(f) => {
                self.zoom = (self.zoom * f).clamp(0.5, 2.);
                self.center();
            }
            Action::Pan(x, y) => {
                self.pan.0 += x;
                self.pan.1 += y;
            }
            Action::Save => {
                if self.loaded.is_none() {
                    return Err("Start a game first".into());
                }
                self.open(Modal::Save);
            }
            Action::Load => self.open(Modal::Load),
            Action::NewGame => self.open(Modal::NewGame),
            Action::Quit => self.open(Modal::Quit),
            Action::Activate => {
                let v = self.view().ok_or("Start a game first")?;
                let at = if self.moving {
                    self.active.and_then(|e| {
                        v.observation
                            .units
                            .iter()
                            .find(|u| u.index == e.slot)
                            .map(|u| (u.x, u.y))
                    })
                } else {
                    self.inspect
                }
                .ok_or("Inspect an own unit first")?;
                self.select_at(at)?;
            }
            Action::Unit(unit) => {
                let v = self.view().ok_or("Start a game first")?;
                if !v.units.contains(&unit) {
                    return Err("Unit selection changed".into());
                }
                let u = v
                    .observation
                    .units
                    .iter()
                    .find(|u| u.index == unit.slot)
                    .ok_or("Unit disappeared")?;
                self.active = Some(unit);
                self.inspect = Some((u.x, u.y));
                self.moving = u.moves_left > 0 && v.phase == Phase::Human && v.decision.is_none();
                self.center();
                self.status = if self.moving {
                    "Unit activated. Moving: keypad, cursor arrows or adjacent terrain click."
                } else {
                    if u.moves_left == 0 {
                        "Unit selected; no moves remaining this turn. Viewing only."
                    } else {
                        "Unit selected for inspection; orders are currently unavailable."
                    }
                }
                .into();
            }
        }
        Ok(())
    }
    fn step(&mut self, dx: i32, dy: i32) -> Result<(), String> {
        let v = self.view().ok_or("Start a game first")?;
        if !self.moving {
            let origin = self
                .inspect
                .or_else(|| {
                    self.active.and_then(|e| {
                        v.observation
                            .units
                            .iter()
                            .find(|u| u.index == e.slot)
                            .map(|u| (u.x, u.y))
                    })
                })
                .ok_or("Inspect a tile first")?;
            let (x, y) = (i32::from(origin.0) + dx, i32::from(origin.1) + dy);
            if x < 0 || y < 0 || x >= i32::from(v.observation.w2) || y >= i32::from(v.observation.h)
            {
                return Err("Inspection cursor reached the map edge".into());
            }
            self.inspect = Some((x as u16, y as u16));
            // Move the inspection cursor across the map; scroll only when its
            // diamond would leave the visible workspace.
            let px = self.pan.0 + f64::from(x) * 32. * self.zoom;
            let py = self.pan.1 + f64::from(y) * 16. * self.zoom;
            let (mx, my, mw, mh) = self.map_rect();
            self.pan.0 +=
                px.clamp(mx + 32. + 32. * self.zoom, mx + mw - 32. - 32. * self.zoom) - px;
            self.pan.1 +=
                py.clamp(my + 50. + 16. * self.zoom, my + mh - 41. - 16. * self.zoom) - py;
            self.status = format!("Viewing ({x}, {y}); A activates an own unit, V changes mode.");
            return Ok(());
        }
        let active = self.active.ok_or("Select an own unit")?;
        let u = v
            .observation
            .units
            .iter()
            .find(|u| u.index == active.slot)
            .ok_or("Unit disappeared")?;
        let (x, y) = (i32::from(u.x) + dx, i32::from(u.y) + dy);
        if x < 0 || y < 0 || x >= i32::from(v.observation.w2) || y >= i32::from(v.observation.h) {
            return Err("Map edge movement is unsupported here".into());
        }
        self.move_to((x as u16, y as u16))
    }
    fn move_to(&mut self, to: (u16, u16)) -> Result<(), String> {
        let v = self.view().ok_or("Start a game first")?;
        let unit = self.active.ok_or("Select an own unit")?;
        let u = v
            .observation
            .units
            .iter()
            .find(|u| u.index == unit.slot)
            .ok_or("Selected unit disappeared")?;
        if self
            .loaded
            .as_ref()
            .unwrap()
            .resources
            .rules
            .unit_domain(u.type_id as usize)
            != Some(0)
        {
            return Err("Sea, air and cargo orders are outside this first playable slice".into());
        }
        let city = v
            .observation
            .cities
            .iter()
            .find(|c| c.x == to.0 && c.y == to.1 && c.owner != v.observation.civ);
        let enemy = v
            .observation
            .units
            .iter()
            .any(|u| u.x == to.0 && u.y == to.1 && u.owner != v.observation.civ);
        if enemy
            && v.observation
                .units
                .iter()
                .filter(|u| u.x == to.0 && u.y == to.1 && u.owner != v.observation.civ)
                .any(|u| {
                    self.loaded
                        .as_ref()
                        .unwrap()
                        .resources
                        .rules
                        .unit_domain(u.type_id as usize)
                        != Some(0)
                })
        {
            return Err(
                "Naval, air and cargo battles are outside this first playable slice".into(),
            );
        }
        let capture = city
            .filter(|_| !enemy)
            .and_then(|c| v.known_cities.iter().find(|e| e.slot == c.index));
        let command = capture.map_or(Command::Move { unit, to }, |city| Command::Occupy {
            unit,
            city: *city,
        });
        if enemy || capture.is_some() || self.enhancements.confirm_moves {
            self.open(Modal::Confirm {
                command,
                revision: v.revision,
                label: if enemy {
                    "Confirm supported ground attack?"
                } else if capture.is_some() {
                    "Confirm capture of the undefended city?"
                } else {
                    "Confirm move?"
                }
                .into(),
            });
            Ok(())
        } else {
            self.order(command)
        }
    }
    fn can_open_double_click(&self, first: u64, x: f64, y: f64) -> bool {
        let Some(g) = self.pending_city_click else {
            return false;
        };
        if !self.focused
            || self.city_open
            || !x.is_finite()
            || !y.is_finite()
            || g.first != first
            || (g.x - x).abs() > 6.
            || (g.y - y).abs() > 6.
        {
            return false;
        }
        let Some(v) = self.view() else { return false };
        if v.revision != g.session_revision
            || !v.cities.contains(&g.city)
            || !v.city_citizens.iter().any(|d| d.index == g.city.slot)
        {
            return false;
        }
        match &self.modal {
            None => true,
            Some(Modal::Units { at, .. }) => v
                .observation
                .cities
                .iter()
                .any(|c| c.index == g.city.slot && (c.x, c.y) == *at),
            _ => false,
        }
    }
    fn map_click(&mut self, x: f64, y: f64, id: u64) -> Result<(), String> {
        let v = self.view().ok_or("Start a game first")?;
        let pointer = (x, y);
        let logical = (
            (x - self.pan.0) / (32. * self.zoom),
            (y - self.pan.1) / (16. * self.zoom),
        );
        let mut best = None;
        for row in (logical.1.round() as i32 - 1)..=(logical.1.round() as i32 + 1) {
            if row < 0 || row >= i32::from(v.observation.h) {
                continue;
            }
            let col = ((logical.0 - f64::from(row & 1)) / 2.).round() as i32 * 2 + (row & 1);
            if col < 0 || col >= i32::from(v.observation.w2) {
                continue;
            }
            let distance = (logical.0 - f64::from(col)).abs() + (logical.1 - f64::from(row)).abs();
            if distance <= 1.0 && best.is_none_or(|(_, _, d)| distance < d) {
                best = Some((col as u16, row as u16, distance));
            }
        }
        if let Some(u) = self.hit_unit(&v, x, y) {
            best = Some((u.x, u.y, 0.));
        } else if let Some(c) = self.hit_own_city(&v, x, y) {
            best = Some((c.x, c.y, 0.));
        }
        if let Some((x, y, _)) = best {
            self.inspect = Some((x, y));
            let at = usize::from(y) * (usize::from(v.observation.w2) / 2) + usize::from(x) / 2;
            self.status = v.observation.seen.get(at).and_then(|t| *t).map_or_else(
                || format!("Unknown tile ({x}, {y})"),
                |t| {
                    let name = self
                        .loaded
                        .as_ref()
                        .unwrap()
                        .resources
                        .rules
                        .base_terrain_names()
                        .get(t.terrain as usize)
                        .unwrap_or(&"Terrain")
                        .to_string();
                    format!(
                        "{name} ({x}, {y}){}{}{}",
                        if t.river { " · river" } else { "" },
                        if t.road { " · road" } else { "" },
                        if t.rail { " · railroad" } else { "" }
                    )
                },
            );
            let own_city = v
                .observation
                .cities
                .iter()
                .find(|c| c.owner == v.observation.civ && (c.x, c.y) == (x, y))
                .and_then(|c| v.cities.iter().find(|e| e.slot == c.index))
                .copied();
            if v.observation
                .units
                .iter()
                .any(|u| u.owner == v.observation.civ && (u.x, u.y) == (x, y))
            {
                self.select_at((x, y))?;
            } else if self.moving && own_city.is_none() {
                self.move_to((x, y))?;
            }
            if let Some(city) = own_city {
                self.city = Some(city);
                self.pending_city_click = Some(CityClick {
                    first: id,
                    city,
                    x: pointer.0,
                    y: pointer.1,
                    session_revision: v.revision,
                });
                self.status = "Own city selected; double-click to manage.".into();
            }
            if let Some(c) = v
                .observation
                .cities
                .iter()
                .find(|c| c.owner == v.observation.civ && c.x == x && c.y == y)
            {
                self.city = v.cities.iter().copied().find(|e| e.slot == c.index);
            }
        }
        Ok(())
    }
    fn current_choices(&self) -> Vec<(i32, String)> {
        let Some(l) = &self.loaded else {
            return Vec::new();
        };
        let v = l.session.view();
        if let Some(choices) = self.route_choices(&v) {
            return choices;
        }
        match &self.modal {
            Some(Modal::Research) => {
                if let Some(Decision::Research {
                    available, default, ..
                }) = v.decision
                {
                    available
                        .into_iter()
                        .map(|id| {
                            let mark = if default == Some(id) {
                                " (suggested)"
                            } else {
                                ""
                            };
                            (id as i32, format!("{}{mark}", self.tech_name(id)))
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            }
            Some(Modal::Units { at, units }) => v
                .observation
                .units
                .iter()
                .filter(|u| {
                    (u.x, u.y) == *at
                        && units
                            .iter()
                            .any(|e| e.slot == u.index && v.units.contains(e))
                })
                .map(|u| {
                    (
                        u.index as i32,
                        format!(
                            "{} #{} · moves {}",
                            self.item_name(u.type_id as i8),
                            u.index,
                            u.moves_left
                        ),
                    )
                })
                .collect(),
            Some(Modal::Production { city }) => v
                .production_choices
                .iter()
                .find(|(c, _)| c == city)
                .map(|(_, items)| {
                    items
                        .iter()
                        .filter(|&&id| {
                            id < 0 || l.resources.rules.unit_domain(id as usize) == Some(0)
                        })
                        .map(|&id| (i32::from(id), self.item_name(id)))
                        .collect()
                })
                .unwrap_or_default(),
            Some(Modal::Improvements { city }) => v
                .city_production
                .iter()
                .find(|p| p.index == city.slot && v.cities.contains(city))
                .map(|p| {
                    p.improvements
                        .iter()
                        .map(|(id, quote)| {
                            (
                                i32::from(*id),
                                match quote {
                                    Ok(q) => format!(
                                        "{} · refund {} gold",
                                        self.item_name(-(*id as i8)),
                                        q.income
                                    ),
                                    Err(why) => format!("{} · {why}", self.item_name(-(*id as i8))),
                                },
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }
    fn item_name(&self, item: i8) -> String {
        let Some(l) = &self.loaded else {
            return String::new();
        };
        if item >= 0 {
            l.resources
                .rules
                .unit_names()
                .get(item as usize)
                .unwrap_or(&"Unknown unit")
                .to_string()
        } else {
            l.resources
                .rules
                .section("IMPROVE")
                .and_then(|s| s.rows.get((-i32::from(item)) as usize))
                .and_then(|r| r.first())
                .cloned()
                .unwrap_or_else(|| format!("Improvement {}", -i32::from(item)))
        }
    }
    fn tech_name(&self, id: usize) -> String {
        if let Some(l) = &self.loaded {
            if l.resources.profile == "Open demo + Isotrident" {
                if let Some(name) = DEMO_TECH_NAMES.get(id) {
                    return (*name).into();
                }
            }
            if let Some(tech) = l.resources.rules.tech_defs().get(id) {
                return tech.name.clone();
            }
        }
        format!("Advance {id}")
    }

    fn entry_meta(&self) -> Option<Entry> {
        if self.loaded.is_none() && self.modal.is_none() {
            return Some(Entry {
                id: self.editing.clone(),
                value: if self.editing == "home" {
                    self.home.clone()
                } else {
                    self.seed.clone()
                },
                x: 100. + self.dx(),
                y: if self.editing == "home" { 220. } else { 310. },
                w: 824.,
                h: 42.,
            });
        }
        match self.modal {
            Some(Modal::Name | Modal::Save | Modal::Load) => Some(Entry {
                id: format!("dialog-{}", self.modal_id),
                value: self.entry.clone(),
                x: 212. + self.dx(),
                y: 230.,
                w: 600.,
                h: 42.,
            }),
            _ => None,
        }
    }
    /// Every button of the current screen. Dialogs, the city window and setup
    /// keep their 1024-wide layout and are centred on a wider stage (#321).
    fn buttons(&self) -> Vec<Button> {
        let mut b = self.screen_buttons();
        if self.centred() {
            let dx = self.dx();
            for button in &mut b {
                button.x += dx;
            }
        }
        b
    }
    fn screen_buttons(&self) -> Vec<Button> {
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
        if let Some(modal) = &self.modal {
            match modal {
                Modal::Research
                | Modal::Goal
                | Modal::Government { .. }
                | Modal::Production { .. }
                | Modal::Units { .. }
                | Modal::Improvements { .. } => {
                    let choices = self.current_choices();
                    // Page buttons only when the list is longer than a page (#284).
                    let paged = choices.len() > 12;
                    for (row, (id, label)) in choices
                        .into_iter()
                        .skip(self.page * 12)
                        .take(12)
                        .enumerate()
                    {
                        add(
                            212.,
                            170. + row as f64 * 29.,
                            600.,
                            27.,
                            format!(
                                "{} {label}",
                                if self.choice == Some(id) {
                                    "●"
                                } else {
                                    "○"
                                }
                            ),
                            Action::Choose(id),
                        );
                    }
                    if paged {
                        add(212., 529., 120., 28., "Previous".into(), Action::Page(-1));
                        add(352., 529., 120., 28., "Next page".into(), Action::Page(1));
                    }
                }
                Modal::Load => {
                    let saves = storage::list(&self.saves);
                    let paged = saves.len() > LOAD_ROWS;
                    for (row, name) in saves
                        .into_iter()
                        .skip(self.page * LOAD_ROWS)
                        .take(LOAD_ROWS)
                        .enumerate()
                    {
                        add(
                            212.,
                            300. + row as f64 * 28.,
                            600.,
                            26.,
                            name.clone(),
                            Action::Slot(name),
                        );
                    }
                    if paged {
                        add(212., 529., 120., 28., "Previous".into(), Action::Page(-1));
                        add(352., 529., 120., 28., "Next page".into(), Action::Page(1));
                    }
                }
                _ => (),
            }
            for rb in self
                .route_buttons(modal)
                .into_iter()
                .chain(self.report_buttons())
                .chain(self.pedia_buttons())
                .chain(self.scenario_buttons(modal))
            {
                add(rb.x, rb.y, rb.w, rb.h, rb.label, rb.action);
            }
            if matches!(
                modal,
                Modal::Name | Modal::Research | Modal::Government { .. }
            ) || matches!(
                modal,
                Modal::Rates {
                    review: Some(_),
                    ..
                }
            ) {
                add(
                    622.,
                    120.,
                    190.,
                    30.,
                    "Save pending choice".into(),
                    Action::Save,
                );
                // Required government/rate decisions cannot be dismissed, so a
                // saved checkpoint must stay loadable from them.
                if matches!(modal, Modal::Research) {
                    add(
                        480.,
                        120.,
                        134.,
                        30.,
                        "Civilopedia".into(),
                        Action::PediaFor,
                    );
                }
                if !matches!(modal, Modal::Name | Modal::Research) {
                    add(
                        480.,
                        120.,
                        134.,
                        30.,
                        "Load checkpoint".into(),
                        Action::Load,
                    );
                }
            }
            if !matches!(modal,Modal::Purchase {quote,..} if quote.price > quote.treasury)
                && !matches!(modal, Modal::Scenarios)
            {
                add(
                    212.,
                    595.,
                    240.,
                    40.,
                    match modal {
                        Modal::Rates { review: None, .. } => "Apply [Enter]",
                        Modal::Rates { .. } => "Apply rates [Enter]",
                        Modal::Revolution => "Yes, revolt [Enter]",
                        Modal::Science
                        | Modal::Notice { .. }
                        | Modal::Help
                        | Modal::Messages
                        | Modal::Settings => "Close [Enter]",
                        Modal::Report { .. } => "Open city [Enter]",
                        Modal::Pedia { reading: false, .. } => "Read [Enter]",
                        Modal::Pedia { .. } => "Back to list [Enter]",
                        Modal::ScenarioStart(_) => "Start experimental game [Enter]",
                        _ => "Accept [Enter]",
                    }
                    .into(),
                    Action::Accept,
                );
            }
            // A required government choice has no cancel button (owner decision).
            if !matches!(modal, Modal::Government { .. }) {
                add(
                    572.,
                    595.,
                    240.,
                    40.,
                    match modal {
                        Modal::Rates { review: None, .. } => "Discard [Esc]",
                        Modal::Rates { .. } => "Reset split [Esc]",
                        Modal::Revolution => "Cancel [Esc]",
                        Modal::Goal | Modal::ScenarioStart(_) => "Back [Esc]",
                        Modal::Scenarios => "Close [Esc]",
                        Modal::Science
                        | Modal::Notice { .. }
                        | Modal::Help
                        | Modal::Messages
                        | Modal::Settings
                        | Modal::Report { .. }
                        | Modal::Pedia { .. } => "Close [Esc]",
                        _ => "Cancel / inspect [Esc]",
                    }
                    .into(),
                    Action::Cancel,
                );
            }
            return b;
        }
        if self.city_open {
            return self.city_buttons();
        }
        if self.loaded.is_none() {
            add(
                100.,
                100.,
                824.,
                56.,
                "Quick Start — random map".into(),
                Action::QuickStart,
            );
            add(
                100.,
                180.,
                824.,
                34.,
                "Installation directory".into(),
                Action::Home,
            );
            add(100., 275., 824., 30., "Generated seed".into(), Action::Seed);
            add(
                100.,
                390.,
                240.,
                42.,
                format!("Civilizations: {}", self.civs),
                Action::Civs,
            );
            add(
                360.,
                390.,
                240.,
                42.,
                format!("Human: {}", self.human),
                Action::Human,
            );
            add(
                620.,
                390.,
                304.,
                42.,
                format!("Difficulty: {}", self.difficulty),
                Action::Difficulty,
            );
            add(
                100.,
                438.,
                824.,
                32.,
                format!(
                    "Map size: {} × {}",
                    [80, 40, 32][self.size],
                    [50, 30, 24][self.size]
                ),
                Action::Size,
            );
            add(
                100.,
                480.,
                824.,
                50.,
                if self.home.trim().is_empty() {
                    "Start open demo"
                } else {
                    "Start game (Original rules)"
                }
                .into(),
                Action::Start,
            );
            add(
                100.,
                545.,
                404.,
                42.,
                "Load engine checkpoint".into(),
                Action::Load,
            );
            add(
                520.,
                545.,
                404.,
                42.,
                "Open scenario (experimental)".into(),
                Action::Scenarios,
            );
            add(
                100.,
                632.,
                300.,
                36.,
                "Enhancements…".into(),
                Action::Settings,
            );
            return b;
        }
        // Layout B (#310/#311): every map-screen command lives in the menu
        // bar; the right column keeps the stack under the cursor, the
        // selected city's two entries and the end-of-turn slot.
        if let Some(v) = self.view() {
            for cb in self.column_buttons(&v) {
                add(cb.x, cb.y, cb.w, cb.h, cb.label, cb.action);
            }
        }
        if self.notice_overlay() {
            let (_, (x, y, w, h)) = self.notice_overlay_rect();
            add(
                x + 16.,
                y + h - 38.,
                170.,
                28.,
                "Open messages".into(),
                Action::Messages,
            );
            add(
                x + w - 136.,
                y + h - 38.,
                120.,
                28.,
                "Dismiss".into(),
                Action::DismissNotices,
            );
        }
        for mb in self.menu_buttons() {
            add(mb.x, mb.y, mb.w, mb.h, mb.label, mb.action);
        }
        b
    }
    pub fn frame(&self) -> Frame {
        if self.closed {
            return Frame {
                revision: self.revision,
                phase: "Closed".into(),
                advancing: false,
                city_click: None,
                draws: vec![
                    rect(0., 0., self.stage_w, 768., style::BG),
                    text(
                        100. + self.dx(),
                        200.,
                        "Client closed. Launch again to resume a saved checkpoint.",
                        style::TYPE_TITLE,
                        style::TEXT_BRIGHT,
                    ),
                ],
                entry: None,
                status: self.status.clone(),
                assets: "none".into(),
                width: self.stage_w,
                fullscreen: self.fullscreen,
                tips: Vec::new(),
                route: Vec::new(),
            };
        }
        let mut draws = vec![rect(0., 0., self.stage_w, 768., style::BG)];
        let title: String = self.loaded.as_ref().map_or_else(
            || "ToT Engine".to_string(),
            |l| {
                let scenario = l
                    .resources
                    .profile
                    .strip_prefix("Scenario (experimental): ")
                    .map(|p| {
                        let file = p.rsplit('/').next().unwrap_or(p);
                        let stem = file.rsplit_once('.').map_or(file, |(s, _)| s);
                        format!("ToT Engine — {stem} · experimental scenario")
                    });
                let name = if let Some(name) = &scenario {
                    name.as_str()
                } else if l.resources.profile.starts_with("Open demo") {
                    "ToT Engine — Open demo / Isotrident"
                } else if l.resources.isotrident.is_some() {
                    "ToT Engine — ToT rules / Isotrident"
                } else {
                    "ToT Engine — Original rules"
                };
                // The current round in the title keeps the turn visible
                // through opponent phases (#283).
                // Scenario dates read as the original writes them (#330).
                match self
                    .view()
                    .map(|v| (v.observation.turn, self.date_label(&v.observation)))
                {
                    Some((turn, Some(date))) => format!("{name} · {date} · Turn {turn}"),
                    Some((turn, None)) if turn > 0 => format!("{name} · Turn {turn}"),
                    _ => name.to_string(),
                }
            },
        );
        // Layout B (#310/#314): with a game loaded the title sits at the right
        // end of the menu bar, over the map and the city window alike; setup
        // keeps the large title.
        let map_screen = self.loaded.is_some();
        let setup_start = draws.len();
        if !map_screen {
            draws.push(text(
                16.,
                16.,
                &title,
                style::TYPE_DISPLAY,
                style::TEXT_BRIGHT,
            ));
        }
        let view = self.view();
        if let Some(v) = &view {
            if self.city_open {
                let start = draws.len();
                self.city_draw(&mut draws, v);
                shift(&mut draws[start..], self.dx());
                self.menu_bar_draw(&mut draws, &title);
            } else {
                let (mx, my, mw, mh) = self.map_rect();
                draws.push(rect(mx, my, mw, mh, style::INSET));
                self.map_draw(&mut draws, v);
                self.column_draw(&mut draws, v);
                self.side_draw(&mut draws, v);
                self.strip_draw(&mut draws, v);
                self.menu_bar_draw(&mut draws, &title);
                let notices = v
                    .notices
                    .iter()
                    .map(|n| self.notice_text(v, n))
                    .collect::<Vec<_>>()
                    .join(" · ");
                draws.push(text(
                    16.,
                    722.,
                    &notices.chars().take(100).collect::<String>(),
                    style::TYPE_SMALL,
                    style::TEXT_DIM,
                ));
            }
        } else {
            draws.push(text(
                100.,
                62.,
                "ToT directory (optional). Leave blank for the open demo.",
                style::TYPE_BODY,
                style::TEXT,
            ));
            draws.push(text(100., 160., "Isotrident / Freeciv artwork: GPL-2.0-or-later. Credits and editable source in the artwork pack.", style::TYPE_FINE, style::TEXT_DIM));
            if self.editing != "seed" {
                draws.push(text(
                    110.,
                    320.,
                    &self.seed,
                    style::TYPE_TITLE,
                    style::TEXT_FIELD,
                ));
            }
            draws.push(text(
                100.,
                604.,
                "Generated games: land play, cities, research, government, reports and checkpoints.",
                style::TYPE_SMALL,
                style::TEXT_DIM,
            ));
        }
        if self.loaded.is_none() {
            shift(&mut draws[setup_start..], self.dx());
        }
        let modal_start = draws.len();
        if let Some(modal) = &self.modal {
            style::modal(&mut draws, 180., 105., 664., 550.);
            // Advisors and reports carry the date, as the original's do.
            let dated = |t: &str| match self.view().and_then(|v| self.date_label(&v.observation)) {
                Some(date) => format!("{t} · {date}"),
                None => t.to_string(),
            };
            let title: String = match modal {
                Modal::Science => dated("Science advisor"),
                Modal::Report { .. } => dated("Reports"),
                _ => match modal {
                    Modal::Name => "Name your city",
                    Modal::Research => "Choose research",
                    Modal::Rates { review: None, .. } => "Tax, luxury and science rates",
                    Modal::Rates { .. } => "Review your rates",
                    Modal::Revolution => "Revolution",
                    Modal::Government { .. } => "Choose a government",
                    Modal::Science => "Science advisor",
                    Modal::Notice { title, .. } => title,
                    Modal::Help => "Keys",
                    Modal::Messages => "Messages",
                    Modal::Settings => "Enhancements (optional, off by default)",
                    Modal::Report { .. } => "Reports",
                    Modal::Pedia { .. } => "Civilopedia",
                    Modal::Goal => "Choose a research goal",
                    Modal::Production { .. } => "Choose production",
                    Modal::Improvements { .. } => "Improvements — select a building to sell",
                    Modal::Purchase { .. } => "Confirm purchase",
                    Modal::Sale { .. } => "Confirm improvement sale",
                    Modal::Units { .. } => "Activate a unit",
                    Modal::Confirm { label, .. } => label,
                    Modal::Save => "Save engine checkpoint (new name)",
                    Modal::Load => "Load engine checkpoint (same resources)",
                    Modal::Scenarios => "Open scenario (experimental)",
                    Modal::ScenarioStart(_) => "Scenario preflight (experimental)",
                    Modal::NewGame => "Start a new game? Unsaved progress will be lost.",
                    Modal::Quit => "Close client? Save first if you want to resume.",
                }
                .to_string(),
            };
            draws.push(text(
                212.,
                125.,
                &title,
                style::TYPE_TITLE,
                style::TEXT_BRIGHT,
            ));
            self.route_draws(modal, &mut draws);
            self.report_draws(&mut draws);
            self.pedia_draws(&mut draws);
            if let (Modal::Production { city }, Some(item), Some(l)) =
                (modal, self.choice, self.loaded.as_ref())
            {
                match l.session.preview_production(*city, item as i8) {
                    Ok(q) => {
                        let cost = q.target_cost.map_or_else(
                            || "no finite shield target".into(),
                            |n| format!("cost {n} shields"),
                        );
                        draws.push(text(
                            212.,
                            562.,
                            &format!("Draft: {} · {cost}", self.item_name(q.item)),
                            style::TYPE_SMALL,
                            style::TEXT_BRIGHT,
                        ));
                        let difference = i32::from(q.retained) - i32::from(q.current_shields);
                        let effect = if difference < 0 {
                            format!("{} lost", -difference)
                        } else if difference > 0 {
                            format!("{difference} restored from city budget")
                        } else {
                            "unchanged".into()
                        };
                        draws.push(text(
                            212.,
                            580.,
                            &format!("Shields: {} → {} · {effect}", q.current_shields, q.retained),
                            style::TYPE_SMALL,
                            style::TEXT,
                        ));
                    }
                    Err(why) => draws.push(text(212., 570., why, style::TYPE_SMALL, style::WARN)),
                }
            }
            if let Modal::Purchase { quote, .. } = modal {
                let lines = [
                    format!("Buy {}", self.item_name(quote.item)),
                    format!("Cost: {} gold · treasury: {}", quote.price, quote.treasury),
                    if quote.treasury >= quote.price {
                        format!("Treasury after purchase: {}", quote.treasury - quote.price)
                    } else {
                        format!(
                            "Need {} more gold",
                            i64::from(quote.price) - i64::from(quote.treasury)
                        )
                    },
                    format!("Shields: {} → {}", quote.stored, quote.required),
                    "The item completes during the next production phase.".into(),
                    if quote.price > quote.treasury {
                        "Insufficient treasury. Purchase is unavailable."
                    } else {
                        "Confirm to pay and fill the shield box."
                    }
                    .into(),
                ];
                for (i, line) in lines.iter().enumerate() {
                    draws.push(text(
                        212.,
                        190. + i as f64 * 40.,
                        line,
                        style::TYPE_BODY,
                        style::TEXT,
                    ));
                }
            }
            if let Modal::Sale { preview, .. } = modal {
                let q = preview.quote;
                let a = preview.before;
                let b = preview.after;
                let lines = [
                    format!(
                        "Sell {} · refund {} gold",
                        self.item_name(-(q.improvement as i8)),
                        q.income
                    ),
                    format!("Treasury: {} → {}", q.treasury, q.treasury + q.income),
                    "Removes this city's building and its effects.".into(),
                    format!(
                        "Food surplus: {:+} → {:+} · shields: {} → {}",
                        a.food_surplus, b.food_surplus, a.net_shields, b.net_shields
                    ),
                    format!(
                        "Tax: {} → {} · science: {} → {}",
                        a.taxes, b.taxes, a.science, b.science
                    ),
                    format!(
                        "Luxury: {} → {} · upkeep: {} → {}",
                        a.luxuries, b.luxuries, a.upkeep, b.upkeep
                    ),
                    format!(
                        "Happy: {} → {} · unhappy: {} → {}",
                        a.happy, b.happy, a.unhappy, b.unhappy
                    ),
                    "Single-city sale. One improvement sale per turn.".into(),
                ];
                for (i, line) in lines.iter().enumerate() {
                    draws.push(text(
                        212.,
                        180. + i as f64 * 40.,
                        line,
                        style::TYPE_BODY,
                        style::TEXT,
                    ));
                }
            }
            if matches!(
                modal,
                Modal::Confirm {
                    command: Command::Occupy { .. },
                    ..
                }
            ) {
                draws.push(text(
                    212.,
                    220.,
                    "Capture covers ownership, treasury, population and buildings.",
                    style::TYPE_BODY,
                    style::TEXT,
                ));
                draws.push(text(
                    212.,
                    250.,
                    "Capital relocation, civil war, technology plunder and respawn",
                    style::TYPE_BODY,
                    style::TEXT,
                ));
                draws.push(text(
                    212.,
                    280.,
                    "are not implemented in this slice.",
                    style::TYPE_BODY,
                    style::TEXT,
                ));
            }
            if matches!(
                modal,
                Modal::Research
                    | Modal::Production { .. }
                    | Modal::Units { .. }
                    | Modal::Improvements { .. }
            ) {
                draws.push(text(
                    212.,
                    151.,
                    "Arrow keys select · Enter accepts · Esc returns to inspection",
                    style::TYPE_SMALL,
                    style::TEXT,
                ));
            }
        }
        if self.modal.is_some() {
            shift(&mut draws[modal_start..], self.dx());
        }
        if self.notice_overlay() {
            let (lines, (x, y, w, h)) = self.notice_overlay_rect();
            style::modal(&mut draws, x, y, w, h);
            let more = self
                .notice_log
                .iter()
                .filter(|(t, _)| Some(*t) == self.notice_turn)
                .count()
                .saturating_sub(lines.len());
            draws.push(text(
                x + 16.,
                y + 12.,
                &format!("Turn {} — new messages", self.notice_turn.unwrap_or(0)),
                style::TYPE_BODY,
                style::TEXT_BRIGHT,
            ));
            for (i, line) in lines.iter().enumerate() {
                draws.push(text(
                    x + 16.,
                    y + 42. + i as f64 * 20.,
                    line,
                    style::TYPE_SMALL,
                    style::TEXT,
                ));
            }
            if more > 0 {
                draws.push(text(
                    x + 200.,
                    y + 12.,
                    &format!("+{more} more in the log"),
                    style::TYPE_SMALL,
                    style::TEXT_DIM,
                ));
            }
        }
        let buttons = self.buttons();
        // One label size per button row (#283): if any label in a row would
        // clip at body size, the whole row uses the small step, so a toolbar
        // never mixes two sizes.
        let row_size = |y: f64| {
            buttons
                .iter()
                .filter(|o| (o.y - y).abs() < 1. && !matches!(o.action, Action::QuickStart))
                .map(|o| style::button_label_size(&o.label, o.w))
                .min()
                .unwrap_or(style::TYPE_BODY)
        };
        for (i, b) in buttons.iter().enumerate() {
            if matches!(b.action, Action::Menu(_) | Action::MenuItem(..)) {
                continue;
            }
            let primary = matches!(b.action, Action::QuickStart);
            let focused = i == self.focus_button;
            let fill = if primary {
                if focused {
                    style::PRIMARY_FOCUS
                } else {
                    style::PRIMARY
                }
            } else if focused {
                style::SELECT
            } else {
                style::BUTTON
            };
            style::button(&mut draws, b.x, b.y, b.w, b.h, fill, focused);
            if self.loaded.is_some() {
                draws.push(Draw::Tile {
                    x: b.x + 2.,
                    y: b.y + 2.,
                    w: b.w - 4.,
                    h: b.h - 4.,
                    sprite: "chrome".into(),
                });
            }
            let size = if primary {
                style::TYPE_DISPLAY
            } else if matches!(b.action, Action::ReportSort(_)) {
                // Column headers match the table cells beneath them.
                style::TYPE_SMALL
            } else {
                row_size(b.y)
            };
            draws.push(text(
                b.x + 7.,
                // A smaller label keeps the same visual centre.
                b.y + 7. + f64::from(style::TYPE_BODY.saturating_sub(size)) / 2.,
                &b.label,
                size,
                if primary {
                    style::ON_PRIMARY
                } else {
                    style::TEXT_FIELD
                },
            ));
            if let Action::Specialist(city, slot) = &b.action {
                let role = view
                    .as_ref()
                    .and_then(|v| v.city_citizens.iter().find(|c| c.index == city.slot))
                    .and_then(|c| c.specialists.get(*slot as usize))
                    .copied()
                    .unwrap_or(0);
                draws.push(Draw::Sprite {
                    x: b.x + b.w - 28.,
                    y: b.y + 1.,
                    w: 24.,
                    h: 28.,
                    sprite: format!("citizen-{role}"),
                });
            }
        }
        if map_screen {
            self.menu_draw(&mut draws, &buttons, self.focus_button);
        }
        if let Some(e) = self.entry_meta() {
            draws.push(rect(e.x - 1., e.y - 1., e.w + 2., e.h + 2., style::GOLD));
            draws.push(rect(e.x, e.y, e.w, e.h, style::FIELD));
            draws.push(text(
                e.x + 7.,
                e.y + 10.,
                &e.value,
                style::TYPE_TITLE,
                style::TEXT_FIELD,
            ));
        }
        // Clear turn-transition feedback (#283): while the opponents run, a
        // framed strip over the map says so; the title bar carries the round.
        let advancing = self.opponent_work_running();
        if advancing {
            let start = draws.len();
            style::modal(&mut draws, 212., 360., 460., 52.);
            draws.push(text(
                228.,
                368.,
                "Opponents are moving — your turn continues when they finish.",
                style::TYPE_BODY,
                style::TEXT_BRIGHT,
            ));
            draws.push(text(
                228.,
                388.,
                "You can reconnect or wait; nothing is lost.",
                style::TYPE_SMALL,
                style::TEXT_DIM,
            ));
            shift(&mut draws[start..], self.dx());
        }
        draws.push(rect(0., 744., self.stage_w, 24., style::STATUS_BG));
        draws.push(text(
            10.,
            749.,
            &self.feedback().chars().take(145).collect::<String>(),
            style::TYPE_SMALL,
            style::GOLD,
        ));
        let (tips, route) = view
            .as_ref()
            .map_or_else(Default::default, |v| self.map_tips(v));
        Frame {
            city_click: self.pending_city_click.map(|g| g.first),
            revision: self.revision,
            advancing,
            phase: view.map_or_else(|| "Setup".into(), |v| format!("{:?}", v.phase)),
            draws,
            entry: self.entry_meta(),
            status: self.status.clone(),
            assets: self.assets.clone(),
            width: self.stage_w,
            fullscreen: self.fullscreen,
            tips,
            route,
        }
    }
    fn feedback(&self) -> String {
        match self.status.as_str() {
            "moved" => "Unit moved".into(),
            "citizens-set" => "Citizen assignment updated immediately.".into(),
            "prod-set" => "Production changed".into(),
            "production-closed" => "Production workspace closed.".into(),
            "production-bought" => {
                "Purchase paid; item completes during the next production phase.".into()
            }
            s if s.starts_with("improvement-sold ") => {
                "Improvement sold; city totals refreshed.".into()
            }
            "research-set" => "Research selected".into(),
            "rates-set" => "Rates updated; city finance refreshed.".into(),
            "government-set" => "Government changed.".into(),
            "revolution-started" => "The old government has fallen. Anarchy has begun.".into(),
            "ended" => "Your turn ended; opponents are playing".into(),
            "skipped" => "Unit skipped for this turn".into(),
            s if s.starts_with("founded ") => "City founded".into(),
            s if s.starts_with("attack won") => "You won the battle".into(),
            s if s.starts_with("attack lost") => "You lost the battle".into(),
            s if s.starts_with("occupied ") => "City occupation completed".into(),
            s => s.into(),
        }
    }
    fn map_draw(&self, draws: &mut Vec<Draw>, v: &HumanView) {
        let o = &v.observation;
        let color = chrome::civ_color;
        let (mx, my, mw, mh) = self.map_rect();
        let point = |x: u16, y: u16| {
            (
                self.pan.0 + f64::from(x) * 32. * self.zoom,
                self.pan.1 + f64::from(y) * 16. * self.zoom,
            )
        };
        for (i, tile) in o.seen.iter().enumerate() {
            let Some(tile) = tile else { continue };
            let y = (i / (usize::from(o.w2) / 2)) as u16;
            let x = ((i % (usize::from(o.w2) / 2)) * 2) as u16 + (y & 1);
            let (px, py) = point(x, y);
            if !(mx + 32. * self.zoom..mx + mw - 32. * self.zoom).contains(&px)
                || !(my + 48. * self.zoom..my + mh - 16. * self.zoom).contains(&py)
            {
                continue;
            }
            let variant = u32::from((x / 2 + y) % 3);
            draws.push(Draw::Sprite {
                x: px - 32. * self.zoom,
                y: py - 16. * self.zoom,
                w: 64. * self.zoom,
                h: 32. * self.zoom,
                sprite: format!("terrain-{}-{variant}", tile.terrain),
            });
            if let Some(name) = match tile.terrain {
                3 => Some("forest"),
                4 => Some("hills"),
                5 => Some("mountains"),
                _ => None,
            } {
                draws.push(Draw::Sprite {
                    x: px - 32. * self.zoom,
                    y: py - 16. * self.zoom,
                    w: 64. * self.zoom,
                    h: 32. * self.zoom,
                    sprite: format!("{name}-{}", (x / 2 + y) % 8),
                });
            }
            if tile.river {
                draws.push(Draw::Line {
                    x: px - 12. * self.zoom,
                    y: py,
                    x2: px + 12. * self.zoom,
                    y2: py,
                    color: style::LINK.into(),
                    width: 2. * self.zoom,
                });
            }
            if tile.road || tile.rail {
                draws.push(Draw::Line {
                    x: px - 8. * self.zoom,
                    y: py + 3. * self.zoom,
                    x2: px + 8. * self.zoom,
                    y2: py + 3. * self.zoom,
                    color: if tile.rail { "#deded0" } else { "#ba9b6e" }.into(),
                    width: if tile.rail { 3. } else { 2. } * self.zoom,
                });
            }
        }
        for c in &o.cities {
            let (px, py) = point(c.x, c.y);
            if !self.in_view(px, py) {
                continue;
            }
            draws.push(Draw::Sprite {
                x: px - 32. * self.zoom,
                y: py - 34. * self.zoom,
                w: 64. * self.zoom,
                h: 48. * self.zoom,
                sprite: format!("city-{}", u8::from(c.size > 4)),
            });
            draws.push(rect(px - 5., py - 30., 10., 8., color(c.owner)));
            draws.push(text(
                px - 30.,
                py + 20.,
                &c.name,
                style::TYPE_SMALL,
                color(c.owner),
            ));
        }
        for u in self.display_units(v) {
            let (px, py) = point(u.x, u.y);
            if !self.in_view(px, py) {
                continue;
            }
            let selected = self.active.is_some_and(|e| e.slot == u.index);
            draws.push(Draw::UnitSprite {
                x: px - 32. * self.zoom,
                y: py - 48. * self.zoom,
                w: 64. * self.zoom,
                h: 64. * self.zoom,
                sprite: format!("unit-{}", u.type_id),
                unit: u.index,
                own: u.owner == o.civ,
                selected,
                moves_left: u.moves_left,
                blink: selected
                    && self.moving
                    && u.moves_left > 0
                    && self.focused
                    && self.modal.is_none()
                    && v.phase == Phase::Human
                    && v.decision.is_none(),
            });
            draws.push(rect(px - 4., py - 45., 9., 10., color(u.owner)));
            if let Some((_, hp, max)) = v.unit_health.iter().find(|(slot, _, _)| *slot == u.index) {
                let ratio = f64::from(*hp) / f64::from((*max).max(1));
                draws.push(rect(px - 18., py - 52., 36., 4., "#322a25"));
                draws.push(rect(
                    px - 18.,
                    py - 52.,
                    36. * ratio,
                    4.,
                    if ratio > 0.6 {
                        style::MOVE_OK
                    } else if ratio > 0.3 {
                        style::MOVE_SOME
                    } else {
                        style::MOVE_NONE
                    },
                ));
            }
        }
        let cursor = if self.moving { None } else { self.inspect };
        if let Some((x, y)) = cursor {
            let (px, py) = point(x, y);
            if self.in_view(px, py) {
                draws.push(Draw::Cursor {
                    x: px,
                    y: py,
                    w: 64. * self.zoom,
                    h: 32. * self.zoom,
                    color: if self.moving {
                        style::GOLD
                    } else {
                        style::TEXT
                    }
                    .into(),
                    blink: self.focused && self.modal.is_none() && v.phase == Phase::Human,
                });
            }
        }
    }
}
/// Moves draws right by `dx` (#321: centring 1024-wide screens).
fn shift(draws: &mut [Draw], dx: f64) {
    if dx == 0. {
        return;
    }
    for d in draws {
        match d {
            Draw::Tile { x, .. }
            | Draw::Rect { x, .. }
            | Draw::Text { x, .. }
            | Draw::Sprite { x, .. }
            | Draw::UnitSprite { x, .. }
            | Draw::Cursor { x, .. } => *x += dx,
            Draw::Line { x, x2, .. } => {
                *x += dx;
                *x2 += dx;
            }
        }
    }
}
fn rect(x: f64, y: f64, w: f64, h: f64, color: &str) -> Draw {
    Draw::Rect {
        x,
        y,
        w,
        h,
        color: color.into(),
    }
}
fn text(x: f64, y: f64, s: &str, size: u32, color: &str) -> Draw {
    Draw::Text {
        x,
        y,
        text: s.into(),
        color: color.into(),
        size,
    }
}

#[cfg(test)]
mod tests;
