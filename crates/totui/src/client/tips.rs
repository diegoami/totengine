//! Map tooltips (#284 item 2, opt-in under Settings): a short description of
//! each known tile, city and unit stack, built only from the civilization's
//! filtered view. The clients show a tip's draws when the pointer rests on
//! its area; hovering never reaches the engine.
use super::*;

/// Tips per frame at most; a full map at the smallest zoom stays well below.
const MAX_TIPS: usize = 1200;
const LINE_H: f64 = 17.;
const PAD: f64 = 7.;
/// Stack lines listed before a "+N more" line.
const STACK_LINES: usize = 4;

/// Moves left in thirds as whole moves: 3 → "1", 4 → "1 1/3", 2 → "2/3".
pub(super) fn moves_text(thirds: u32) -> String {
    match (thirds / 3, thirds % 3) {
        (whole, 0) => whole.to_string(),
        (0, part) => format!("{part}/3"),
        (whole, part) => format!("{whole} {part}/3"),
    }
}

impl Client {
    pub(super) fn tip_lines(&self, v: &HumanView, x: u16, y: u16) -> Option<Vec<String>> {
        let o = &v.observation;
        let at = usize::from(y) * (usize::from(o.w2) / 2) + usize::from(x) / 2;
        let tile = o.seen.get(at).copied().flatten()?;
        let rules = &self.loaded.as_ref()?.resources.rules;
        let terrain = rules
            .base_terrain_names()
            .get(tile.terrain as usize)
            .map_or_else(|| "Terrain".to_string(), ToString::to_string);
        let mut lines = vec![format!("{terrain} ({x}, {y})")];
        let features: Vec<&str> = [
            (tile.river, "river"),
            (tile.road && !tile.rail, "road"),
            (tile.rail, "railroad"),
        ]
        .into_iter()
        .filter_map(|(on, name)| on.then_some(name))
        .collect();
        if !features.is_empty() {
            lines.push(features.join(" · "));
        }
        if let Some(c) = o.cities.iter().find(|c| (c.x, c.y) == (x, y)) {
            lines.push(if c.owner == o.civ {
                format!("{} · size {} · yours", c.name, c.size)
            } else {
                format!("{} · size {} · civ {}", c.name, c.size, c.owner)
            });
        }
        let names = rules.unit_names();
        let units: Vec<_> = o.units.iter().filter(|u| (u.x, u.y) == (x, y)).collect();
        for u in units.iter().take(STACK_LINES) {
            let name = names
                .get(u.type_id as usize)
                .map_or_else(|| format!("Unit {}", u.type_id), ToString::to_string);
            let vet = if u.veteran { " (veteran)" } else { "" };
            lines.push(if u.owner == o.civ {
                format!(
                    "{name}{vet} #{} · moves {}",
                    u.index,
                    moves_text(u.moves_left)
                )
            } else {
                format!("{name}{vet} · civ {}", u.owner)
            });
        }
        if units.len() > STACK_LINES {
            lines.push(format!("+{} more units", units.len() - STACK_LINES));
        }
        Some(lines)
    }

    /// The box for `lines` beside the tile at `(px, py)`, kept on the map.
    fn tip_box(lines: &[String], px: f64, py: f64) -> Vec<Draw> {
        let chars = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let w = (chars as f64 * f64::from(style::TYPE_SMALL) * 0.58 + 2. * PAD).ceil();
        let h = lines.len() as f64 * LINE_H + 2. * PAD - 3.;
        let x = (px + 26.).min(748. - w).max(16.);
        let y = if py + 14. + h <= 676. {
            py + 14.
        } else {
            (py - 30. - h).max(104.)
        };
        let mut draws = vec![
            rect(x - 1., y - 1., w + 2., h + 2., style::GOLD),
            rect(x, y, w, h, style::INSET),
        ];
        for (i, line) in lines.iter().enumerate() {
            draws.push(text(
                x + PAD,
                y + PAD - 1. + i as f64 * LINE_H,
                line,
                style::TYPE_SMALL,
                if i == 0 {
                    style::TEXT_BRIGHT
                } else {
                    style::TEXT
                },
            ));
        }
        draws
    }

    /// Tips for the visible map, when the enhancement is on and the map is
    /// the active surface (no dialog, no city window).
    pub(super) fn map_tips(&self, v: &HumanView) -> Vec<Tip> {
        if !self.enhancements.map_tooltips || self.modal.is_some() || self.city_open {
            return Vec::new();
        }
        let o = &v.observation;
        let point = |x: u16, y: u16| {
            (
                self.pan.0 + f64::from(x) * 32. * self.zoom,
                self.pan.1 + f64::from(y) * 16. * self.zoom,
            )
        };
        let on_map = |px: f64, py: f64| (12.0..752.0).contains(&px) && (100.0..680.0).contains(&py);
        let mut sprites = Vec::new();
        let mut tiles = Vec::new();
        // Unit and city sprites stand above their tile; their rectangles win
        // over the diamonds of the tiles they overlap.
        let mut stacks: Vec<(u16, u16)> = o.units.iter().map(|u| (u.x, u.y)).collect();
        stacks.extend(o.cities.iter().map(|c| (c.x, c.y)));
        stacks.sort_unstable();
        stacks.dedup();
        for (x, y) in stacks {
            let (px, py) = point(x, y);
            if !(44.0..720.0).contains(&px) || !(150.0..640.0).contains(&py) {
                continue;
            }
            if let Some(lines) = self.tip_lines(v, x, y) {
                sprites.push(Tip {
                    x: px - 18. * self.zoom,
                    y: py - 44. * self.zoom,
                    hw: 18. * self.zoom,
                    hh: 22. * self.zoom,
                    diamond: false,
                    draws: Self::tip_box(&lines, px, py),
                });
            }
        }
        for (i, tile) in o.seen.iter().enumerate() {
            if tile.is_none() || sprites.len() + tiles.len() >= MAX_TIPS {
                continue;
            }
            let y = (i / (usize::from(o.w2) / 2)) as u16;
            let x = ((i % (usize::from(o.w2) / 2)) * 2) as u16 + (y & 1);
            let (px, py) = point(x, y);
            if !on_map(px, py) {
                continue;
            }
            if let Some(lines) = self.tip_lines(v, x, y) {
                tiles.push(Tip {
                    x: px,
                    y: py,
                    hw: 32. * self.zoom,
                    hh: 16. * self.zoom,
                    diamond: true,
                    draws: Self::tip_box(&lines, px, py),
                });
            }
        }
        sprites.extend(tiles);
        sprites
    }
}
