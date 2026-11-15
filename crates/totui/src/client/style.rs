//! One visual language for every screen (UI quality U1b, #282).
//!
//! Screens reference these tokens instead of literal colors, so panels,
//! buttons, tables and modals look identical across the map, city, reports,
//! Civilopedia and setup screens on both clients. The palette keeps the
//! faithful ToT parchment-on-olive tones and consolidates the near-duplicate
//! golds and text shades that had accumulated per screen. Chrome helpers own
//! the shared frames; screens keep their layout coordinates.
//!
//! Literal colors remain allowed only for **game-data** colors that are not
//! UI chrome: the civilization palette, terrain/rail/road accents and the
//! unit health-bar backing.
use super::Draw;

// Backgrounds and surfaces
/// App background: the map surround and the setup screen.
pub const BG: &str = "#20251f";
/// Editable field background (text inputs).
pub const FIELD: &str = "#121811";
/// Modal and panel fill.
pub const PANEL: &str = "#53564a";
/// Un-emphasized button fill.
pub const BUTTON: &str = "#41483b";
/// Focused buttons and selected table rows.
pub const SELECT: &str = "#6b7057";

// Accents
/// Primary (call-to-action) button fill.
pub const PRIMARY: &str = "#d9b85a";
/// Focused primary button fill.
pub const PRIMARY_FOCUS: &str = "#f4d46c";
/// Text on primary (gold) buttons.
pub const ON_PRIMARY: &str = "#172015";
/// Borders, dividers, markers and accent text; consolidates the gold family.
pub const GOLD: &str = "#e8c85a";

// Text
/// Primary text.
pub const TEXT: &str = "#eee8d4";
/// Secondary, dimmed text.
pub const TEXT_DIM: &str = "#ddd9bd";
/// Emphasized text and titles.
pub const TEXT_BRIGHT: &str = "#fff0b5";
/// Text inside editable fields.
pub const TEXT_FIELD: &str = "#f4eedb";

// States and roles
/// Warnings (disorder rows), errors and destructive hints.
pub const WARN: &str = "#ffba8a";
/// Reference links (Civilopedia entries).
pub const LINK: &str = "#4e9ac4";
/// The bottom status bar.
pub const STATUS_BG: &str = "#3b4034";
/// Near-black inset surfaces (map surround frames, city tile badges).
pub const INSET: &str = "#090d08";
/// Movement-points indicators: full, partial, spent.
pub const MOVE_OK: &str = "#81cd59";
pub const MOVE_SOME: &str = "#edd35a";
pub const MOVE_NONE: &str = "#e57660";
pub const WHITE: &str = "#ffffff";

fn rect(x: f64, y: f64, w: f64, h: f64, color: &str) -> Draw {
    Draw::Rect {
        x,
        y,
        w,
        h,
        color: color.into(),
    }
}

/// The shared modal frame (#282): a gold hairline around the panel fill.
/// Screens pass their own geometry and draw titles and content on top.
pub fn modal(draws: &mut Vec<Draw>, x: f64, y: f64, w: f64, h: f64) {
    draws.push(rect(x, y, w, h, GOLD));
    draws.push(rect(x + 2., y + 2., w - 4., h - 4., PANEL));
}

/// A focused button gets a gold hairline so keyboard focus is always visible
/// alongside the fill change (#282); unfocused buttons keep the flat fill.
pub fn button(draws: &mut Vec<Draw>, x: f64, y: f64, w: f64, h: f64, fill: &str, focused: bool) {
    if focused {
        draws.push(rect(x, y, w, h, GOLD));
        draws.push(rect(x + 1., y + 1., w - 2., h - 2., fill));
    } else {
        draws.push(rect(x, y, w, h, fill));
    }
}
