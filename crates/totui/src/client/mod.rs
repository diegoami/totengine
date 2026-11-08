//! One controller, filtered presentation and persistence for both native adapters.
mod atlas;
pub mod browser;
mod city;
mod city_panels;
#[cfg(feature = "desktop")]
pub mod desktop;
#[cfg(any(feature = "desktop", test))]
mod editor;
mod selection;
pub mod storage;
use crate::resources::Resources;
pub use atlas::{Atlas, Sprite};
use engine::human::{Command, Decision, Entity, HumanSession, HumanView, Phase, Request};
use engine::newgame::NewGameConfig;
use engine::player::CityName;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use city_panels::CityPanel;

pub const WIDTH: u32 = 1024;
pub const HEIGHT: u32 = 768;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Input {
    Click { x: f64, y: f64 },
    DoubleClick { x: f64, y: f64, first_click: u64 },
    Key { key: String },
    Edit { field: String, value: String },
    Focus { focused: bool },
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
}
#[derive(Clone)]
enum Modal {
    Name,
    Research,
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
}
#[derive(Clone)]
enum Action {
    Start,
    Civs,
    Human,
    Difficulty,
    Size,
    Mode,
    ConfirmMoves,
    Work,
    Found,
    Research,
    City,
    OpenCity,
    CloseCity,
    CityStep(i32),
    CityPanel(CityPanel),
    CityPanelPage(i32),
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
}

#[derive(Clone, Copy)]
struct CityClick {
    first: u64,
    city: Entity,
    x: f64,
    y: f64,
    session_revision: u64,
}
pub struct Client {
    loaded: Option<Loaded>,
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
    inspect: Option<(u16, u16)>,
    moving: bool,
    confirm_moves: bool,
    pan: (f64, f64),
    zoom: f64,
    assets: String,
}
impl Client {
    pub fn new(home: String, saves: PathBuf) -> Self {
        Self {
            loaded: None,
            closed: false,
            advancing: false,
            saves,
            home,
            seed: "7".into(),
            civs: 5,
            human: 1,
            difficulty: 2,
            size: 0,
            revision: 0,
            receipts: BTreeMap::new(),
            status:
                "Choose your owned ToT installation; generated Original-profile surface games only."
                    .into(),
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
            inspect: None,
            moving: false,
            confirm_moves: false,
            pan: (0., 0.),
            zoom: 1.,
            assets: "none".into(),
        }
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
            Some(Decision::Research { .. }) => self.open(Modal::Research),
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
                self.pan = (
                    380. - f64::from(x) * 32. * self.zoom,
                    390. - f64::from(y) * 16. * self.zoom,
                );
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
    fn handle(&mut self, input: Input, id: u64) -> Result<(), String> {
        match input {
            Input::Focus { focused } => {
                self.focused = focused;
                Ok(())
            }
            Input::Tick => {
                if let Some(l) = self.loaded.as_mut() {
                    if self.advancing && l.session.phase() == Phase::Opponents {
                        l.session.advance(16);
                        self.refresh();
                        if self.view().is_some_and(|v| v.phase != Phase::Opponents) {
                            self.advancing = false;
                            self.schedule_unit(None, true);
                            self.status=match self.view().unwrap().phase {Phase::Human=>"Your turn. Choose actions or resolve the required research choice.",Phase::Eliminated=>"Your civilization has no units or cities. Load a checkpoint or start a new game.",Phase::UnsupportedGovernment=>"Government choice is unsupported. Save a checkpoint to preserve this game.",Phase::TurnLimit=>"Turn limit reached. Save a checkpoint to preserve this game.",Phase::Opponents=>"Opponents playing"}.into();
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
                if let Some(b) = buttons.into_iter().find(|b| b.contains(x, y)) {
                    return self.action(b.action);
                }
                if self.modal.is_none() && self.city_open {
                    return self.city_click(x, y);
                }
                if self.modal.is_none()
                    && self.loaded.is_some()
                    && (12.0..752.0).contains(&x)
                    && (100.0..680.0).contains(&y)
                {
                    self.map_click(x, y, id)
                } else {
                    Ok(())
                }
            }
            Input::Key { key } => self.key(&key),
        }
    }
    fn key(&mut self, key: &str) -> Result<(), String> {
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
                    if matches!(b.action, Action::Save | Action::Load) {
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
                _ => Ok(()),
            }
        } else if self.city_open {
            self.city_key(key)
        } else {
            match key {
                "b" => self.action(Action::Found),
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
                "Shift+z" => {
                    self.zoom = 1.;
                    self.center();
                    Ok(())
                }
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
            Action::Start => {
                let seed = self
                    .seed
                    .parse::<u32>()
                    .map_err(|_| "Seed must be a whole number from 0 to 4294967295")?;
                let resources = Resources::load(Path::new(&self.home), "Original")?;
                let atlas = Atlas::load(&resources)?;
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
                self.loaded = Some(Loaded {
                    resources,
                    atlas,
                    session,
                    identity,
                });
                self.assets = storage::nonce()?;
                self.advancing = self.view().is_some_and(|v| v.phase == Phase::Opponents);
                self.city_panel = CityPanel::Info;
                self.city_panel_page = 0;
                self.editing.clear();
                self.refresh();
                self.schedule_unit(None, true);
                self.center();
                self.required();
                self.status =
                    "Generated game ready. Required choices are never selected for you.".into();
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
            Action::ConfirmMoves => self.confirm_moves = !self.confirm_moves,
            Action::Found => {
                let unit = self.active.ok_or("Select an own settler first")?;
                self.order(Command::BeginFound { unit })?;
            }
            Action::Research => self.required(),
            Action::City => {
                if let Some(v) = self.view() {
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
                        self.pan = (
                            380. - f64::from(c.x) * 32. * self.zoom,
                            390. - f64::from(c.y) * 16. * self.zoom,
                        );
                    }
                }
                self.status = "Own city selected; Production opens its explicit picker.".into();
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
            }
            Action::CityPanel(panel) => {
                self.city_panel = panel;
                self.city_panel_page = 0;
                self.focus_button = 0;
            }
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
                self.page = self
                    .page
                    .saturating_add_signed(delta as isize)
                    .min(self.current_choices().len().saturating_sub(1) / 12)
            }
            Action::Accept => {
                let modal = self.modal.clone().ok_or("No dialog to accept")?;
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
                    Modal::Quit => {
                        self.closed = true;
                        self.modal = None;
                        self.status =
                            "Client closed. Your saved checkpoints remain available.".into();
                        return Ok(());
                    }
                    Modal::NewGame => {
                        self.loaded = None;
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
                        storage::save(
                            &self.saves,
                            &self.entry,
                            &l.session.checkpoint(&l.identity),
                        )?;
                        self.status = format!("Engine checkpoint saved: {}", self.entry);
                    }
                    Modal::Load => {
                        let text = storage::load(&self.saves, &self.entry)?;
                        if let Some(l) = self.loaded.as_mut() {
                            let restored = HumanSession::restore(
                                l.resources.rules.clone(),
                                &l.identity,
                                &text,
                            )?;
                            l.session = restored;
                        } else {
                            let resources = Resources::load(Path::new(&self.home), "Original")?;
                            let atlas = Atlas::load(&resources)?;
                            let identity = resources.identity();
                            let session =
                                HumanSession::restore(resources.rules.clone(), &identity, &text)?;
                            let assets = storage::nonce()?;
                            self.loaded = Some(Loaded {
                                resources,
                                atlas,
                                identity,
                                session,
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
                        self.status="Checkpoint restored, including pending decisions. Original SAV import is not supported.".into();
                    }
                }
                self.modal = None;
                self.required();
            }
            Action::Cancel => {
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
                    None if production_cancel => {
                        "Production selection cancelled; existing item unchanged."
                    }
                    None => "Cancelled.",
                }
                .into();
            }
            Action::End => {
                let v = self.view().ok_or("Start a game first")?;
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
            self.pan.0 += px.clamp(44. + 32. * self.zoom, 720. - 32. * self.zoom) - px;
            self.pan.1 += py.clamp(150. + 16. * self.zoom, 639. - 16. * self.zoom) - py;
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
        if enemy || capture.is_some() || self.confirm_moves {
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
        match &self.modal {
            Some(Modal::Research) => {
                if let Some(Decision::Research { available, .. }) = v.decision {
                    let techs = l.resources.rules.tech_defs();
                    available
                        .into_iter()
                        .map(|id| {
                            (
                                id as i32,
                                techs
                                    .get(id)
                                    .map_or_else(|| format!("Advance {id}"), |t| t.name.clone()),
                            )
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
    fn entry_meta(&self) -> Option<Entry> {
        if self.loaded.is_none() && self.modal.is_none() {
            return Some(Entry {
                id: self.editing.clone(),
                value: if self.editing == "home" {
                    self.home.clone()
                } else {
                    self.seed.clone()
                },
                x: 100.,
                y: if self.editing == "home" { 220. } else { 310. },
                w: 824.,
                h: 42.,
            });
        }
        match self.modal {
            Some(Modal::Name | Modal::Save | Modal::Load) => Some(Entry {
                id: format!("dialog-{}", self.modal_id),
                value: self.entry.clone(),
                x: 212.,
                y: 230.,
                w: 600.,
                h: 42.,
            }),
            _ => None,
        }
    }
    fn buttons(&self) -> Vec<Button> {
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
                | Modal::Production { .. }
                | Modal::Units { .. }
                | Modal::Improvements { .. } => {
                    for (row, (id, label)) in self
                        .current_choices()
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
                    add(212., 529., 120., 28., "Previous".into(), Action::Page(-1));
                    add(352., 529., 120., 28., "Next page".into(), Action::Page(1));
                }
                Modal::Load => {
                    for (row, name) in storage::list(&self.saves)
                        .into_iter()
                        .skip(self.page * 8)
                        .take(8)
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
                }
                _ => (),
            }
            if matches!(modal, Modal::Name | Modal::Research) {
                add(
                    650.,
                    120.,
                    160.,
                    30.,
                    "Save pending choice".into(),
                    Action::Save,
                );
            }
            if !matches!(modal,Modal::Purchase {quote,..} if quote.price > quote.treasury) {
                add(
                    212.,
                    595.,
                    240.,
                    40.,
                    "Accept [Enter]".into(),
                    Action::Accept,
                );
            }
            add(
                572.,
                595.,
                240.,
                40.,
                "Cancel / inspect [Esc]".into(),
                Action::Cancel,
            );
            return b;
        }
        if self.city_open {
            return self.city_buttons();
        }
        if self.loaded.is_none() {
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
                "Start Original surface game".into(),
                Action::Start,
            );
            add(
                100.,
                545.,
                824.,
                42.,
                "Load engine checkpoint".into(),
                Action::Load,
            );
            return b;
        }
        for (i, (label, action)) in [
            (
                if self.moving {
                    "Moving [V]"
                } else {
                    "Viewing [V]"
                },
                Action::Mode,
            ),
            ("Found [B]", Action::Found),
            ("Research", Action::Research),
            ("Cities", Action::City),
            ("Wait [W]", Action::Wait),
            ("Skip [Space]", Action::Skip),
            ("Save [Ctrl+S]", Action::Save),
            ("Load [Ctrl+L]", Action::Load),
        ]
        .into_iter()
        .enumerate()
        {
            add(12. + i as f64 * 126., 48., 120., 38., label.into(), action);
        }
        add(770., 622., 240., 50., "End [Ctrl+N]".into(), Action::End);
        add(770., 574., 115., 36., "Center".into(), Action::Center);
        add(895., 574., 52., 36., "+".into(), Action::Zoom(1.25));
        add(955., 574., 55., 36., "−".into(), Action::Zoom(0.8));
        add(
            12.,
            685.,
            300.,
            30.,
            format!("Confirm all moves: {}", self.confirm_moves),
            Action::ConfirmMoves,
        );
        add(325., 685., 160., 30., "New game".into(), Action::NewGame);
        add(500., 685., 160., 30., "Close client".into(), Action::Quit);
        if self.city.is_some() {
            add(
                674.,
                685.,
                336.,
                30.,
                "Manage city".into(),
                Action::OpenCity,
            );
        }
        if self.view().is_some_and(|v| v.phase == Phase::Opponents) {
            add(
                770.,
                530.,
                240.,
                36.,
                if self.advancing {
                    "Pause opponents"
                } else {
                    "Continue opponents"
                }
                .into(),
                Action::Work,
            );
        }
        if let Some(city) = self.city {
            add(
                770.,
                350.,
                240.,
                38.,
                "Choose production".into(),
                Action::Production(city),
            );
        }
        if let Some(v) = self.view() {
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
                    770.,
                    475. + i as f64 * 30.,
                    240.,
                    28.,
                    format!("Activate unit {}", e.slot),
                    Action::Unit(*e),
                );
            }
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
                    rect(0., 0., 1024., 768., "#20251f"),
                    text(
                        100.,
                        200.,
                        "Client closed. Launch again to resume a saved checkpoint.",
                        22,
                        "#fff0b5",
                    ),
                ],
                entry: None,
                status: self.status.clone(),
                assets: "none".into(),
            };
        }
        let mut draws = vec![rect(0., 0., 1024., 768., "#20251f")];
        draws.push(text(
            16.,
            16.,
            "ToTEngine — Original surface game",
            24,
            "#fff0b5",
        ));
        let view = self.view();
        if let Some(v) = &view {
            if self.city_open {
                self.city_draw(&mut draws, v);
            } else {
                draws.push(Draw::Tile {
                    x: 764.,
                    y: 100.,
                    w: 250.,
                    h: 580.,
                    sprite: "panel".into(),
                });
                draws.push(rect(12., 100., 740., 580., "#050906"));
                self.map_draw(&mut draws, v);
                let o = &v.observation;
                for (i, line) in [
                    format!("Turn {} · Civ {}", o.turn, o.civ),
                    format!("Treasury: {}", o.treasury),
                    format!(
                        "Research: {} / {}",
                        o.research.progress,
                        o.research
                            .cost
                            .map_or_else(|| "unavailable".into(), |n| n.to_string())
                    ),
                    match v.phase {
                        Phase::Human if v.decision.is_some() => "Choice required",
                        Phase::Human if self.moving => "Moving units",
                        Phase::Human
                            if !v
                                .observation
                                .units
                                .iter()
                                .any(|u| u.owner == o.civ && u.moves_left > 0) =>
                        {
                            "End of turn"
                        }
                        Phase::Human => "Viewing pieces",
                        Phase::Opponents => "Opponents playing",
                        Phase::Eliminated => "Your civilization is eliminated",
                        Phase::UnsupportedGovernment => "Government choice required",
                        Phase::TurnLimit => "Turn limit reached",
                    }
                    .into(),
                ]
                .into_iter()
                .enumerate()
                {
                    draws.push(text(770., 110. + i as f64 * 30., &line, 17, "#eee8d4"));
                }
                if let Some(u) = self
                    .active
                    .and_then(|e| o.units.iter().find(|u| u.index == e.slot))
                {
                    let name = self
                        .loaded
                        .as_ref()
                        .unwrap()
                        .resources
                        .rules
                        .unit_names()
                        .get(u.type_id as usize)
                        .unwrap_or(&"Unit")
                        .to_string();
                    draws.push(text(
                        770.,
                        240.,
                        &format!("{name} #{}", u.index),
                        17,
                        "#ffef9a",
                    ));
                    draws.push(text(
                        770.,
                        268.,
                        &format!("({}, {}) · moves {}/3", u.x, u.y, u.moves_left),
                        17,
                        "#eee8d4",
                    ));
                }
                if let Some((_, hp, max)) = self
                    .active
                    .and_then(|e| v.unit_health.iter().find(|(slot, _, _)| *slot == e.slot))
                {
                    draws.push(text(
                        770.,
                        285.,
                        &format!("Health: {hp}/{max}"),
                        13,
                        "#ddd9bd",
                    ));
                }
                if let Some(c) = self
                    .city
                    .and_then(|e| o.cities.iter().find(|c| c.index == e.slot))
                {
                    draws.push(text(
                        770.,
                        300.,
                        &format!("{} · size {}", c.name, c.size),
                        17,
                        "#ffef9a",
                    ));
                    draws.push(text(
                        770.,
                        325.,
                        &format!(
                            "{} · shields {}",
                            self.item_name(c.building.unwrap_or(0)),
                            c.shield_box.unwrap_or(0)
                        ),
                        15,
                        "#eee8d4",
                    ));
                }
                if let Some(e) = self
                    .city
                    .and_then(|city| o.city_economy.iter().find(|e| e.index == city.slot))
                {
                    for (i, line) in [
                        format!("Food {:+} · shields {}", e.food_surplus, e.net_shields),
                        format!("Science {} · tax {}", e.finance.science, e.finance.taxes),
                        format!("Happy {} · unhappy {}", e.happy, e.unhappy),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        draws.push(text(770., 400. + i as f64 * 23., &line, 15, "#ddd9bd"));
                    }
                }
                if v.phase == Phase::UnsupportedGovernment {
                    draws.push(text(
                    25.,
                    140.,
                    "Government choice is unsupported. Save a checkpoint to preserve this game.",
                    17,
                    "#ffba8a",
                ));
                }
                let notices=v.notices.iter().map(|n| {
                use engine::human::Notice;
                let city_name=|id:usize|o.cities.iter().find(|c|c.index==id).map_or_else(||format!("City {id}"),|c|c.name.clone());
                match n {
                    Notice::Grew(id)=>format!("{} grew",city_name(*id)),
                    Notice::Starved(id)=>format!("{} starved",city_name(*id)),
                    Notice::CityDestroyed(id)=>format!("City {id} was lost"),
                    Notice::UnitDisbanded(id)=>format!("Unit {id} disbanded for support"),
                    Notice::Production(id,_)=>format!("{} completed production",city_name(*id)),
                    Notice::Discovery(id)=>{let name=self.loaded.as_ref().unwrap().resources.rules.tech_defs().get(*id).map_or_else(||format!("advance {id}"),|t|t.name.clone());format!("Discovered {name}")},
                    Notice::Revolution=>"Your government fell into anarchy; save to preserve this unsupported choice".into(),
                }
            }).collect::<Vec<_>>().join(" · ");
                draws.push(text(
                    16.,
                    724.,
                    &notices.chars().take(110).collect::<String>(),
                    14,
                    "#ddd9bd",
                ));
            }
        } else {
            draws.push(text(
                100.,
                120.,
                "Use your owned ToT install. No original assets are bundled.",
                20,
                "#eee8d4",
            ));
            draws.push(text(
                100.,
                570.,
                "First slice: generated land play, production, research and engine checkpoints.",
                18,
                "#ddd9bd",
            ));
        }
        if let Some(modal) = &self.modal {
            draws.push(rect(180., 105., 664., 550., "#53564a"));
            let title = match modal {
                Modal::Name => "Name your city",
                Modal::Research => "Choose research explicitly",
                Modal::Production { .. } => "Choose production",
                Modal::Improvements { .. } => "Improvements — select a building to sell",
                Modal::Purchase { .. } => "Confirm purchase",
                Modal::Sale { .. } => "Confirm improvement sale",
                Modal::Units { .. } => "Activate a unit",
                Modal::Confirm { label, .. } => label,
                Modal::Save => "Save engine checkpoint (new name)",
                Modal::Load => "Load engine checkpoint (same resources)",
                Modal::NewGame => "Start a new game? Unsaved progress will be lost.",
                Modal::Quit => "Close client? Save first if you want to resume.",
            };
            draws.push(text(212., 125., title, 22, "#fff0b5"));
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
                            14,
                            "#fff0b5",
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
                            12,
                            "#eee8d4",
                        ));
                    }
                    Err(why) => draws.push(text(212., 570., why, 14, "#ffcf8c")),
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
                    draws.push(text(212., 190. + i as f64 * 40., line, 17, "#eee8d4"));
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
                    draws.push(text(212., 180. + i as f64 * 40., line, 17, "#eee8d4"));
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
                    17,
                    "#eee8d4",
                ));
                draws.push(text(
                    212.,
                    250.,
                    "Capital relocation, civil war, technology plunder and respawn",
                    17,
                    "#eee8d4",
                ));
                draws.push(text(
                    212.,
                    280.,
                    "are not implemented in this slice.",
                    17,
                    "#eee8d4",
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
                    13,
                    "#eee8d4",
                ));
            }
        }
        for (i, b) in self.buttons().iter().enumerate() {
            draws.push(rect(
                b.x,
                b.y,
                b.w,
                b.h,
                if i == self.focus_button {
                    "#6b7057"
                } else {
                    "#41483b"
                },
            ));
            if self.loaded.is_some() {
                draws.push(Draw::Tile {
                    x: b.x + 2.,
                    y: b.y + 2.,
                    w: b.w - 4.,
                    h: b.h - 4.,
                    sprite: "chrome".into(),
                });
            }
            draws.push(text(b.x + 7., b.y + 7., &b.label, 16, "#f4eedb"));
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
        if let Some(e) = self.entry_meta() {
            draws.push(rect(e.x - 1., e.y - 1., e.w + 2., e.h + 2., "#d6c885"));
            draws.push(rect(e.x, e.y, e.w, e.h, "#121811"));
            draws.push(text(e.x + 7., e.y + 10., &e.value, 18, "#f4eedb"));
        }
        draws.push(rect(0., 744., 1024., 24., "#3b4034"));
        draws.push(text(
            10.,
            749.,
            &self.feedback().chars().take(145).collect::<String>(),
            13,
            "#fff0c6",
        ));
        Frame {
            city_click: self.pending_city_click.map(|g| g.first),
            revision: self.revision,
            advancing: self.advancing,
            phase: view.map_or_else(|| "Setup".into(), |v| format!("{:?}", v.phase)),
            draws,
            entry: self.entry_meta(),
            status: self.status.clone(),
            assets: self.assets.clone(),
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
        let color = |owner: u8| {
            [
                "#aaa79a", "#e8c55b", "#709de5", "#e77e72", "#77c98a", "#c68add", "#5ac7d0",
                "#efb775",
            ][usize::from(owner.min(7))]
        };
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
            if !(12. + 32. * self.zoom..752. - 32. * self.zoom).contains(&px)
                || !(100. + 48. * self.zoom..680. - 16. * self.zoom).contains(&py)
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
                    color: "#4e9ac4".into(),
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
            if !(44.0..720.0).contains(&px) || !(150.0..640.0).contains(&py) {
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
            draws.push(text(px - 30., py + 20., &c.name, 14, color(c.owner)));
        }
        for u in self.display_units(v) {
            let (px, py) = point(u.x, u.y);
            if !(44.0..720.0).contains(&px) || !(150.0..640.0).contains(&py) {
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
                        "#81cd59"
                    } else if ratio > 0.3 {
                        "#edd35a"
                    } else {
                        "#e57660"
                    },
                ));
            }
        }
        let cursor = if self.moving { None } else { self.inspect };
        if let Some((x, y)) = cursor {
            let (px, py) = point(x, y);
            if (44.0..720.0).contains(&px) && (150.0..640.0).contains(&py) {
                draws.push(Draw::Cursor {
                    x: px,
                    y: py,
                    w: 64. * self.zoom,
                    h: 32. * self.zoom,
                    color: if self.moving { "#ffff8c" } else { "#f0eee0" }.into(),
                    blink: self.focused && self.modal.is_none() && v.phase == Phase::Human,
                });
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
