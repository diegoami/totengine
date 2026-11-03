//! `engine` — world state model + deterministic turn kernel skeleton (T1-2).
//!
//! [`boot`] turns a parsed scenario/save ([`tot_kit::SaveFile`]) into a
//! [`WorldState`]; [`WorldState::checksum`] is the reproducibility anchor
//! (same input ⇒ same checksum, independent of process, time, or hash-map
//! ordering); [`WorldState::advance_turn`] is the kernel skeleton — turn
//! sequencing plus a seeded [`GameRng`], with per-phase work landing in
//! T1-3..T1-7.

pub mod cities;
pub mod combat;
pub mod events;
pub mod movement;
pub mod research;
pub mod rng;

use std::fmt::Write as _;

use rng::{GameRng, Lcg64};
use tot_kit::{CityRecord, SaveFile, UnitRecord};

/// A civilization slot: the 7 name blocks are players `1..7` in unit
/// `owner` encoding; owner 0 is the barbarians (no name block).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Civ {
    /// Owner id: 1..=7.
    pub id: u8,
    /// Printable strings of the civ-name block, file order (slot
    /// semantics unproven; ROMFALL reads leader, plural, adjective…).
    pub names: Vec<String>,
    /// Plural name from the block's fixed slot @24 ("West Romans") — the
    /// name macro events match civs by.
    pub plural: String,
    /// Unit count at boot (measured, not yet a maintained roster).
    pub unit_count: usize,
}

/// Engine-side unit: the proven 40-B record fields, keyed by stable data
/// (coordinates + type + owner) rather than the per-save re-assigned id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    pub type_id: u8,
    pub owner: u8,
    pub x: u16,
    pub y: u16,
    pub is_leader: bool,
    /// Movement left this turn, in thirds of a point (T1-3). 0 until the
    /// first `start_turn` (movement rates come from Rules).
    pub moves_left: u32,
    /// Veteran (record attr bit 0x2000), damage taken (HP lost, HP = Rules
    /// `h`·10) and orders (0xff none, 2 fortified) — evidence §T1-6 combat.
    pub veteran: bool,
    pub damage: u8,
    pub orders: u8,
    /// Position in its tile's stack list (from the save's +0x1a/+0x1c
    /// links; 0 = head). Defender ties go to the highest position. Units
    /// the engine moves keep their position — the original's insertion
    /// point on entering a tile is not decoded.
    pub stack_pos: u16,
}

/// Engine-side city: identity, coordinates and the proven economics
/// fields (§T1-4). `size` is the record's u8@11.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct City {
    pub name: String,
    pub x: u16,
    pub y: u16,
    pub size: u16,
    pub owner: u16,
    pub city_id: u16,
    /// Gross food/shields/trade (the City Status report's columns).
    pub gross_food: u8,
    pub gross_shields: u8,
    pub gross_trade: u16,
    /// Net science/taxes after the rate split and improvements.
    pub net_science: u16,
    pub net_taxes: u16,
    /// Food/shield boxes (u16@86/@88 — AI-city evolution proven).
    pub food_box: u16,
    pub shield_box: u16,
}

/// Terrain grid state: the decoded surface map block in stored order,
/// with logical→stored access via the proven diamond mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapState {
    pub stored_w: usize,
    pub stored_h: usize,
    /// One entry per stored tile: terrain id (& 0x3F).
    pub terrain: Vec<u8>,
    pub special: Vec<bool>,
    pub river: Vec<bool>,
    pub road: Vec<bool>,
    pub rail: Vec<bool>,
}

impl MapState {
    /// Logical (x, y) → stored index (`sx = (x - (y&1))/2 - 1, sy = y`;
    /// outside-the-grid logical tiles map to None — the 2 western border
    /// columns are not stored).
    pub fn stored_index(&self, x: u16, y: u16) -> Option<usize> {
        let (w, h) = (self.stored_w as isize, self.stored_h as isize);
        let sx = (x as isize - (y as isize & 1)) / 2 - 1;
        if sx < 0 || y as isize >= h || sx >= w {
            return None;
        }
        Some(y as usize * self.stored_w + sx as usize)
    }
    pub fn terrain_at(&self, x: u16, y: u16) -> Option<u8> {
        self.stored_index(x, y).map(|i| self.terrain[i])
    }
}

/// The turn kernel's state. Boot is a pure function of the parsed file;
/// every later transition is a pure function of (state, orders, rng).
#[derive(Debug, Clone)]
pub struct WorldState {
    pub turn: i16,
    pub year: i16,
    pub difficulty: u8,
    pub rivals: u8,
    pub seed_word: u16,
    /// Game-flags word (save @652); bit 0x10 = damage persists (T1-6).
    pub game_flags: u16,
    /// Human-players bitmask (save @687; bit n = civ n).
    pub human_mask: u8,
    pub wonders: [i16; 28],
    pub civs: Vec<Civ>,
    pub units: Vec<Unit>,
    pub cities: Vec<City>,
    pub map: MapState,
    pub rng: Lcg64,
    /// Combat's generator: the original's MSVC CRT `rand()`, which it seeds
    /// from the wall clock at startup (`srand(time())`, VA 0x598423) — so a
    /// save does not determine combat, and the engine seeds it from
    /// `rng_seed`.
    pub combat_rng: combat::CrtRand,
    rng_seed: u64,
}

/// Boot a world from a parsed scenario/save. `rng_seed` seeds the kernel's
/// generator — deliberately independent of the file's replay-seed word
/// (that word is the original game's combat/event seed, whose generator is
/// unidentified; T1-6).
pub fn boot(sav: &SaveFile, rng_seed: u64) -> WorldState {
    let surface = sav.map_header.surface();
    let m = sav
        .maps
        .first()
        .expect("decodable saves carry at least the surface map block");
    let mut terrain = Vec::with_capacity(surface);
    let mut special = Vec::with_capacity(surface);
    let mut river = Vec::with_capacity(surface);
    let mut road = Vec::with_capacity(surface);
    let mut rail = Vec::with_capacity(surface);
    for t in 0..surface {
        terrain.push(m.terrain(t));
        special.push(m.is_special(t));
        river.push(m.is_river(t));
        road.push(m.has_road(t));
        rail.push(m.has_rail(t));
    }
    let civs = sav
        .civ_names
        .iter()
        .enumerate()
        .map(|(i, names)| Civ {
            id: i as u8 + 1,
            names: names.strings.clone(),
            plural: names.plural.clone(),
            unit_count: sav
                .units
                .iter()
                .filter(|u| u.owner() & 0x0F == i as u8 + 1)
                .count(),
        })
        .collect();
    WorldState {
        turn: sav.turn,
        year: sav.year,
        difficulty: sav.difficulty,
        rivals: sav.rivals,
        seed_word: sav.seed_word,
        game_flags: sav.game_flags,
        human_mask: sav.human_mask,
        wonders: sav.wonders,
        civs,
        units: units_from(&sav.units),
        cities: sav.cities.iter().map(city_from).collect(),
        map: MapState {
            stored_w: sav.map_header.stored_w(),
            stored_h: sav.map_header.stored_h(),
            terrain,
            special,
            river,
            road,
            rail,
        },
        rng: Lcg64::new(rng_seed as u32),
        combat_rng: combat::CrtRand::new(rng_seed as u32),
        rng_seed,
    }
}

/// Map records to engine units and number each tile stack by walking the
/// save's links from every head (prev = −1). Link values are unit
/// indices (= record indices since the 2026-11-03 framing fix).
fn units_from(recs: &[UnitRecord]) -> Vec<Unit> {
    let mut units: Vec<Unit> = recs.iter().map(unit_from).collect();
    for (i, r) in recs.iter().enumerate() {
        if r.stack_prev() >= 0 {
            continue;
        }
        let (mut cur, mut pos) = (i, 0u16);
        loop {
            units[cur].stack_pos = pos;
            let next = recs[cur].stack_next();
            if next < 0 || next as usize >= recs.len() || pos == u16::MAX {
                break;
            }
            cur = next as usize;
            pos += 1;
        }
    }
    units
}

fn unit_from(r: &UnitRecord) -> Unit {
    Unit {
        type_id: r.type_id(),
        owner: r.owner() & 0x0F,
        x: r.x(),
        y: r.y(),
        is_leader: r.is_leader(),
        moves_left: 0,
        veteran: r.is_veteran(),
        damage: r.damage(),
        orders: r.orders(),
        stack_pos: 0,
    }
}

fn city_from(r: &CityRecord) -> City {
    City {
        name: r.name(),
        x: r.x(),
        y: r.y(),
        size: r.size(),
        owner: r.owner(),
        city_id: r.city_id(),
        gross_food: r.gross_food(),
        gross_shields: r.gross_shields(),
        gross_trade: r.gross_trade(),
        net_science: r.net_science(),
        net_taxes: r.net_taxes(),
        food_box: r.food_box(),
        shield_box: r.shield_box(),
    }
}

/// What one kernel tick did (skeleton: turn sequencing only — per-phase
/// simulation reports grow in T1-3..T1-7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnReport {
    pub turn: i16,
    pub year: i16,
    pub rng_draws: u64,
}

impl WorldState {
    /// The name events use for `owner`: the civ's plural; owner 0 is
    /// "Barbarians" (an engine label — no name block exists for it, and
    /// FallRome's events only ever match barbarians via ANYBODY).
    pub fn civ_name(&self, owner: u8) -> String {
        if owner == 0 {
            return "Barbarians".into();
        }
        self.civs
            .iter()
            .find(|c| c.id == owner)
            .map(|c| c.plural.clone())
            .unwrap_or_default()
    }

    /// Advance one turn. The year-advance schedule is deliberately NOT
    /// implemented: no oracle-verified table exists yet (the -4000…
    /// progression needs a turnloop experiment — folklore is banned by
    /// the evidence rule). Year stays put until that lands (T1-3).
    pub fn advance_turn(&mut self) -> TurnReport {
        self.turn += 1;
        TurnReport {
            turn: self.turn,
            year: self.year,
            rng_draws: self.rng.draws(),
        }
    }

    /// Reproducibility anchor: FNV-1a over the canonical field order —
    /// settings, wonders, civs, map bytes, units, cities, turn/year and
    /// the RNG seed (NOT the RNG state: the checksum pins *game* state;
    /// draw-sequence reproducibility is tested separately).
    pub fn checksum(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mix = |h: &mut u64, bytes: &[u8]| {
            for &b in bytes {
                *h ^= b as u64;
                *h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        let mut buf = String::new();
        let _ = write!(
            buf,
            "v1|turn|{}|year|{}|diff|{}|riv|{}|seed|{}|rngseed|{}",
            self.turn, self.year, self.difficulty, self.rivals, self.seed_word, self.rng_seed
        );
        mix(&mut h, buf.as_bytes());
        buf.clear();
        for w in self.wonders {
            let _ = write!(buf, "{w},");
        }
        mix(&mut h, buf.as_bytes());
        for c in &self.civs {
            buf.clear();
            let _ = write!(buf, "civ|{}|{}|", c.id, c.names.join("\x1f"));
            mix(&mut h, buf.as_bytes());
            mix(&mut h, &c.unit_count.to_le_bytes());
        }
        mix(&mut h, &self.map.stored_w.to_le_bytes());
        mix(&mut h, &self.map.stored_h.to_le_bytes());
        for t in 0..self.map.terrain.len() {
            let flags = (self.map.special[t] as u8)
                | ((self.map.river[t] as u8) << 1)
                | ((self.map.road[t] as u8) << 2)
                | ((self.map.rail[t] as u8) << 3);
            mix(&mut h, &[self.map.terrain[t], flags]);
        }
        for u in &self.units {
            buf.clear();
            let _ = write!(
                buf,
                "u|{}|{}|{}|{}|{}|",
                u.type_id, u.owner, u.x, u.y, u.is_leader as u8
            );
            mix(&mut h, buf.as_bytes());
        }
        for c in &self.cities {
            buf.clear();
            let _ = write!(buf, "city|{}|{}|{}|", c.name, c.x, c.y);
            mix(&mut h, buf.as_bytes());
        }
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_sav() -> tot_kit::SaveFile {
        // reuse tot-kit's own synthetic fixture logic minimally: build a
        // small decodable file via the crate's test helpers is not public,
        // so boot from a hand-built SaveFile via parse of the same bytes
        // is overkill here — instead construct via parse on fixture bytes
        // produced by the same recipe (kept in sync by the boot test in
        // totctl). Here: a tiny hand-built state suffices for kernel laws.
        crate::tests_support::synthetic_save()
    }

    #[test]
    fn boot_is_pure_and_checksum_reproducible() {
        let sav = synthetic_sav();
        let a = boot(&sav, 1);
        let b = boot(&sav, 1);
        assert_eq!(a.checksum(), b.checksum());
        // different rng seed changes the checksum (it is part of state)
        let c = boot(&sav, 2);
        assert_ne!(a.checksum(), c.checksum());
    }

    #[test]
    fn turn_kernel_is_deterministic_given_seed() {
        let sav = synthetic_sav();
        let mut a = boot(&sav, 7);
        let mut b = boot(&sav, 7);
        for _ in 0..10 {
            assert_eq!(a.advance_turn(), b.advance_turn());
        }
        assert_eq!(a.checksum(), b.checksum());
        assert_eq!(a.turn, sav.turn + 10);
    }

    #[test]
    fn civ_unit_counts_match_owner_bytes() {
        let sav = synthetic_sav();
        let w = boot(&sav, 1);
        let total: usize = w.civs.iter().map(|c| c.unit_count).sum();
        let owned = w
            .units
            .iter()
            .filter(|u| (1..=7).contains(&u.owner))
            .count();
        assert_eq!(total, owned);
        assert_eq!(w.units.len(), sav.units.len());
        assert_eq!(w.cities.len(), sav.cities.len());
    }
}

/// Test fixtures shared with the doc tests (not public API).
pub mod tests_support {
    use tot_kit::parse_sav;

    /// A minimal decodable 0x32 file (mirrors tot-kit's unit test fixture
    /// recipe): 4×4 stored map, 2 units, 8-slot city grid.
    pub fn synthetic_save() -> tot_kit::SaveFile {
        let stored_w = 4usize;
        let stored_h = 4usize;
        let surface = stored_w * stored_h;
        let mut d = vec![0u8; tot_kit::OFF_TRANSPORTERS + 172];
        d[..8].copy_from_slice(tot_kit::MAGIC);
        d[10..12].copy_from_slice(&0x0032u16.to_le_bytes());
        d[668..670].copy_from_slice(&1i16.to_le_bytes());
        d[670..672].copy_from_slice(&(-4000i16).to_le_bytes());
        d[684] = 4;
        d[685..689].copy_from_slice(&[0xdb, 0x02, 0xdb, 0x02]);
        d[689] = 4;
        let hdr: [i16; 8] = [
            (2 * stored_w) as i16,
            stored_h as i16,
            surface as i16,
            1,
            0,
            0,
            0,
            0,
        ];
        let mut out = d;
        for v in hdr {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&[0, 0]);
        out.extend(std::iter::repeat_n(0u8, 7 * surface));
        for t in 0..surface {
            let mut f = [0u8; 6];
            f[4] = if t == 2 { 1 | 0x80 } else { 1 };
            out.extend_from_slice(&f);
        }
        out.extend(std::iter::repeat_n(0u8, 10240));
        out.extend(std::iter::repeat_n(0u8, 34));
        let unit_specs: [(u16, u16, u8, u8); 2] = [(3, 4, 0, 1), (5, 6, 1, 2)];
        for (i, &(x, y, typ, owner)) in unit_specs.iter().enumerate() {
            let mut r = [0u8; 40];
            r[6..8].copy_from_slice(&x.to_le_bytes());
            r[8..10].copy_from_slice(&y.to_le_bytes());
            r[14] = typ;
            r[15] = owner;
            r[36..38].copy_from_slice(&(i as u16 + 1).to_le_bytes());
            out.extend_from_slice(&r);
        }
        for i in 0..8 {
            let mut r = [0u8; 92];
            let name = format!("City{i}");
            r[..name.len()].copy_from_slice(name.as_bytes());
            r[58..60].copy_from_slice(&((i + 1) as u16).to_le_bytes());
            r[60..62].copy_from_slice(&((i + 2) as u16).to_le_bytes());
            out.extend_from_slice(&r);
        }
        parse_sav(&out).expect("synthetic fixture parses")
    }
}
