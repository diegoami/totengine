//! Read-only per-turn observer stream (#80). Version 1 is newline-delimited
//! JSON: one boot frame followed by one frame per completed turn. Events
//! are the canonical order/result records and turn checksum since the last
//! frame, not a claim to enumerate every internal simulation effect.
use std::fmt::Write as _;

use crate::player::{CityView, Game, TileView, UnitView};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    /// None = spectator; Some(1..=7) = the player's existing fog rules.
    pub civ: Option<u8>,
    pub tiles: Vec<Option<TileView>>,
    pub units: Vec<UnitView>,
    pub cities: Vec<CityView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub turn: i16,
    pub year: i16,
    pub checksum: u64,
    pub width: usize,
    pub height: usize,
    pub terrain_names: Vec<String>,
    pub unit_names: Vec<String>,
    /// Spectator first, then civs 1..=7. Civ views reuse WorldState::observe.
    pub views: Vec<View>,
    pub events: Vec<String>,
}

/// One recorder per game/replay. Capturing never mutates the world or RNG.
#[derive(Default)]
pub struct Recorder {
    log_offset: Option<usize>,
}

impl Recorder {
    pub fn capture(&mut self, game: &Game) -> Snapshot {
        let w = &game.world;
        let full = View {
            civ: None,
            tiles: (0..w.map.terrain.len())
                .map(|i| {
                    Some(TileView {
                        terrain: w.map.terrain[i],
                        river: w.map.river[i],
                        special: w.map.special[i],
                        road: w.map.road[i],
                        rail: w.map.rail[i],
                    })
                })
                .collect(),
            units: w
                .units
                .iter()
                .enumerate()
                .filter(|(_, u)| u.id != 0)
                .map(|(index, u)| UnitView {
                    index,
                    type_id: u.type_id,
                    owner: u.owner,
                    x: u.x,
                    y: u.y,
                    moves_left: u.moves_left,
                    veteran: u.veteran,
                })
                .collect(),
            cities: w
                .cities
                .iter()
                .enumerate()
                .filter(|(_, c)| c.city_id != 0)
                .map(|(index, c)| CityView {
                    index,
                    name: c.name.clone(),
                    owner: (c.owner & 15) as u8,
                    x: c.x,
                    y: c.y,
                    size: c.size,
                    building: Some(c.building),
                    shield_box: Some(c.shield_box),
                    output: Some(w.city_output(&game.rules, index)),
                })
                .collect(),
        };
        let mut views = vec![full];
        for civ in 1..=7 {
            let obs = w.observe(&game.rules, civ, None);
            views.push(View {
                civ: Some(civ),
                tiles: obs.seen,
                units: obs.units,
                cities: obs.cities,
            });
        }
        let events = self
            .log_offset
            .map(|offset| game.log()[offset..].lines().map(str::to_owned).collect())
            .unwrap_or_default();
        self.log_offset = Some(game.log().len());
        Snapshot {
            turn: w.turn,
            year: w.year,
            checksum: w.checksum(),
            width: w.map.stored_w,
            height: w.map.stored_h,
            terrain_names: game
                .rules
                .sections
                .get("TERRAIN")
                .map(|s| {
                    s.rows
                        .iter()
                        .take(11)
                        .filter_map(|r| r.first().cloned())
                        .collect()
                })
                .unwrap_or_default(),
            unit_names: game
                .rules
                .sections
                .get("UNITS")
                .map(|s| s.rows.iter().filter_map(|r| r.first().cloned()).collect())
                .unwrap_or_default(),
            views,
            events,
        }
    }
}

/// JSON string quoting, including controls and markup characters so a frame
/// is safe both as NDJSON and when embedded by downstream viewers.
fn quoted(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c < ' ' || matches!(c, '<' | '>' | '&') => {
                write!(out, "\\u{:04x}", c as u32).unwrap();
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

impl Snapshot {
    /// Compact, deterministic schema v1. Checksums are hex strings to avoid
    /// JavaScript's loss of precision for 64-bit integers. Tile arrays are
    /// [terrain, river, road, rail, special-suppressed]; absent tiles are null. Indices follow
    /// y * width + x/2 (logical x parity equals y parity).
    pub fn to_json(&self) -> String {
        let mut out = format!("{{\"schema\":1,\"turn\":{},\"year\":{},\"checksum\":\"{:016x}\",\"width\":{},\"height\":{},\"views\":[", self.turn, self.year, self.checksum, self.width, self.height);
        for (i, v) in self.views.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            write!(
                out,
                "{{\"civ\":{},\"tiles\":[",
                v.civ
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "null".into())
            )
            .unwrap();
            for (n, t) in v.tiles.iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                match t {
                    None => out.push_str("null"),
                    Some(t) => write!(
                        out,
                        "[{},{},{},{},{}]",
                        t.terrain, t.river, t.road, t.rail, t.special
                    )
                    .unwrap(),
                }
            }
            out.push_str("],\"units\":[");
            for (n, u) in v.units.iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                write!(
                    out,
                    "{{\"id\":{},\"type\":{},\"owner\":{},\"x\":{},\"y\":{},\"veteran\":{}}}",
                    u.index, u.type_id, u.owner, u.x, u.y, u.veteran
                )
                .unwrap();
            }
            out.push_str("],\"cities\":[");
            for (n, c) in v.cities.iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                write!(
                    out,
                    "{{\"id\":{},\"name\":{},\"owner\":{},\"x\":{},\"y\":{},\"size\":{}}}",
                    c.index,
                    quoted(&c.name),
                    c.owner,
                    c.x,
                    c.y,
                    c.size
                )
                .unwrap();
            }
            out.push_str("]}");
        }
        out.push_str("],\"terrain_names\":[");
        out.push_str(
            &self
                .terrain_names
                .iter()
                .map(|s| quoted(s))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push_str("],\"unit_names\":[");
        out.push_str(
            &self
                .unit_names
                .iter()
                .map(|s| quoted(s))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push_str("],\"events\":[");
        for (i, event) in self.events.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&quoted(event));
        }
        out.push_str("]}");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ai::AiV0, tests_support::synthetic_save_bytes};

    #[test]
    fn hundred_turn_stream_replays_identically_and_preserves_fog() {
        let save = synthetic_save_bytes();
        let rules = || {
            tot_kit::parse_rules("@UNITS\nSettlers, nil, 0, 1., 0, 0a, 1d, 1h, 1f, 3, 0, 5, nil, 000000000000000\n@TERRAIN\nDesert, 1, 2, 0, 1, 0\nPlains, 1, 2, 1, 1, 0\nGrassland, 1, 2, 2, 0, 0\n")
        };
        let mut game = Game::new("synthetic.sav", &save, 3, rules(), None).unwrap();
        for civ in 1..8 {
            game.players[civ] = Some(Box::new(AiV0::new(&game.rules, 3, civ as u8)));
        }
        let mut recorder = Recorder::default();
        let before = game.world.checksum();
        let boot = recorder.capture(&game);
        assert_eq!(before, game.world.checksum());
        assert!(boot.events.is_empty());
        assert!(boot.views[0].tiles.iter().all(Option::is_some));
        for civ in 1..8 {
            let fog = game.world.observe(&game.rules, civ as u8, None);
            assert_eq!(boot.views[civ].tiles, fog.seen);
            assert_eq!(boot.views[civ].units, fog.units);
            assert_eq!(boot.views[civ].cities, fog.cities);
        }
        let mut frames = vec![boot];
        for _ in 0..100 {
            game.play_turn();
            frames.push(recorder.capture(&game));
        }
        assert!(frames[1..]
            .iter()
            .all(|f| f.events.last().unwrap().starts_with("R ")));
        let mut replay_recorder = Recorder::default();
        let mut replay_frames = Vec::new();
        Game::replay_observed("synthetic.sav", &save, rules(), None, game.log(), |g| {
            replay_frames.push(replay_recorder.capture(g))
        })
        .unwrap();
        assert_eq!(frames, replay_frames);
        assert_eq!(
            frames.iter().map(Snapshot::to_json).collect::<Vec<_>>(),
            replay_frames
                .iter()
                .map(Snapshot::to_json)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            quoted("a\"\\\n</script>"),
            "\"a\\\"\\\\\\u000a\\u003c/script\\u003e\""
        );
    }
}
