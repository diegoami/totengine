//! Start placement (RG-2): where each civ's first settler goes in a new
//! game. Decoded from the original:
//! - the civ loop at `0x48c5ce`;
//! - the per-civ setup at `0x488499`, with its tile helpers.
//!
//! **Oracle-exact:** three generated worlds replay draw for draw, every call
//! site and CRT state (568 draws, 15 starts; fixture
//! `docs/evidence/rg2-placement-fixture.txt`, evidence §RG-2 start
//! placement).
//!
//! All draws come from the CRT `rand()` ([`CrtRand`]):
//! 1. one `rand() % 50` before the loop;
//! 2. for civ 1..=7:
//!    - seven attitude draws, one per civ 1..=7;
//!    - for an AI civ, a tribe draw and a `rand() % 3`;
//!    - for a civ in play, location tries until one is accepted.
//!
//! A try is `x = rand() % w2`, `y = rand() % h`. The x is then moved onto
//! the diamond grid: if x is odd, x −= 1; if y is odd, x += 1.
//!
//! Not modelled, because no oracle run exercised it:
//! - preset starts (Earth-map tribe tables, all −1 on random maps);
//! - the two `> 2000`-try relaxations' interaction with later turns;
//! - the turn > 200 landmass rule.
//!
//! The flat-world margin branch is decoded but not oracle-checked.
//! Note that the tribe draw assumes two or more candidate tribes for the
//! civ's slot (true for the Original rules: three per slot).

use crate::combat::CrtRand;

/// The tile state start placement reads: the map's 6-B tile records in
/// the game's framing (evidence §Map tile framing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartMap {
    /// Logical x range (2 × stored width) and height.
    pub w2: i32,
    pub h: i32,
    /// The map's resource seed (map header `raw[4]`); huts derive from it.
    pub seed: u16,
    /// Flat world: no east-west wrap for distances, and on wide maps the
    /// tries keep 8 tiles from the edges (game option bit `0x8000`;
    /// decoded, not oracle-checked).
    pub flat: bool,
    /// Huts enabled on this map (`0x5f5ed6` bit for the map).
    pub huts: bool,
    /// 6 B per stored tile, index `y * (w2 / 2) + x / 2`.
    pub tiles: Vec<[u8; 6]>,
    /// Per landmass id: the i16 at +2 of its 40-B table entry.
    pub landmass_value: Vec<i16>,
}

/// One placed civ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Start {
    pub civ: u8,
    pub x: i32,
    pub y: i32,
    /// Location tries it took (the accepted one included).
    pub tries: u32,
}

const OCEAN: u8 = 10;
/// Engine guard, not the original's: a map with no acceptable tile would
/// make the original loop forever.
const MAX_TRIES: u32 = 1 << 20;

impl StartMap {
    /// The surface map of a parsed save. Unit and owner marks on the tiles
    /// are kept as they are: a fresh pre-placement map has none.
    pub fn from_save(sav: &tot_kit::SaveFile) -> Option<Self> {
        let m = sav.maps.first()?;
        let tiles = m.tile_fields.as_chunks::<6>().0.to_vec();
        let landmass_value = sav
            .landmass_table
            .as_chunks::<40>()
            .0
            .iter()
            .map(|e| i16::from_le_bytes([e[2], e[3]]))
            .collect();
        Some(StartMap {
            w2: sav.map_header.raw[0] as i32,
            h: sav.map_header.raw[1] as i32,
            seed: sav.map_header.raw[4] as u16,
            flat: false,
            huts: true,
            tiles,
            landmass_value,
        })
    }

    fn tile(&self, x: i32, y: i32) -> &[u8; 6] {
        &self.tiles[(y * (self.w2 / 2) + x / 2) as usize]
    }

    fn tile_mut(&mut self, x: i32, y: i32) -> &mut [u8; 6] {
        let i = (y * (self.w2 / 2) + x / 2) as usize;
        &mut self.tiles[i]
    }

    fn ocean(&self, x: i32, y: i32) -> bool {
        self.tile(x, y)[0] & 0x0F == OCEAN
    }

    /// Tile owner (`+5 >> 4`, `0xF` = none; `0x570169`).
    fn owner(&self, x: i32, y: i32) -> Option<u8> {
        match self.tile(x, y)[5] >> 4 {
            0xF => None,
            o => Some(o),
        }
    }

    /// `0x5703be`: the site bits (`+2 >> 5`) if set, else fertility
    /// (`+5 & 0x0F`) raised to at least 8, or 0.
    fn site_value(&self, x: i32, y: i32) -> i32 {
        let t = self.tile(x, y);
        match (t[2] >> 5, t[5] & 0x0F) {
            (0, 0) => 0,
            (0, f) => f.max(8) as i32,
            (v, _) => v as i32,
        }
    }

    /// `0x5701d1` + the landmass table: the i16 at +2 of the tile's
    /// landmass entry.
    fn landmass_value(&self, x: i32, y: i32) -> i32 {
        let id = self.tile(x, y)[3] as usize;
        self.landmass_value.get(id).copied().unwrap_or(0) as i32
    }

    /// `0x57093f`: a hut, derived from the resource seed. On land, unowned,
    /// with huts enabled. The tile hashes as `a = (x+y)>>1`, `b = x − a`,
    /// and holds a hut iff
    /// `(a&3) + 4(b&3) == (seed + 13(b>>2) + 11(a>>2) + 8) & 31`.
    /// Decoded statically; no oracle try landed on a hut.
    pub fn hut(&self, x: i32, y: i32) -> bool {
        if !self.huts || self.ocean(x, y) || self.owner(x, y).is_some() {
            return false;
        }
        let a = (x + y) >> 1;
        let b = x - a;
        let v = self.seed as i32 + 13 * (b >> 2) + 11 * (a >> 2) + 8;
        (a & 3) + 4 * (b & 3) == v & 31
    }

    /// `0x570599`: a city (`+1 & 0xC2 == 2`) or a unit (`+1 & 1`) with an
    /// owner.
    fn occupied(&self, x: i32, y: i32) -> bool {
        let f = self.tile(x, y)[1];
        ((f & 0xC2) == 2 || f & 1 != 0) && self.owner(x, y).is_some()
    }

    /// `0x566dad`: distance on the doubled-x diamond grid, wrapping east-west
    /// unless the world is flat.
    pub fn distance(&self, x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
        let mut dx = (x1 - x2).abs();
        if !self.flat && self.w2 / 2 < dx {
            dx = self.w2 - dx;
        }
        let (dx, dy) = (dx.abs(), (y1 - y2).abs());
        let m = (dx + dy) >> 1;
        if dy >= dx {
            dy - ((m - dx + 1) >> 1)
        } else {
            dx - ((m - dy + 1) >> 1)
        }
    }

    /// The acceptance test of try number `n` (1-based), in the original's
    /// order (`0x489565..0x4896e0`). `units` = settlers placed so far.
    fn accepts(&self, x: i32, y: i32, n: u32, human: bool, units: &[(i32, i32)]) -> bool {
        let n = n as i32;
        if n > 2000 && !human {
            return true;
        }
        if self.ocean(x, y) || self.site_value(x, y) < 12 - (n >> 5) {
            return false;
        }
        // nearest city (none at game start: 30) or placed unit
        let near = units
            .iter()
            .map(|&(ux, uy)| self.distance(x, y, ux, uy))
            .fold(30, i32::min);
        if near < 16 - (n >> 7) || (near < 2 && n <= 2000) || near < 1 {
            return false;
        }
        if self.landmass_value(x, y) < 32 - (n >> 6) {
            return false;
        }
        if self.hut(x, y) && n <= 2000 {
            return false;
        }
        !self.occupied(x, y)
    }

    /// One coordinate draw: `rand() % range`, or on a flat world with a
    /// wide enough axis `rand() % (range − 16) + 8` for the first 1500
    /// tries (`0x4892a0`).
    fn coord(&self, crt: &mut CrtRand, range: i32, wide: bool, n: u32) -> i32 {
        if self.flat && wide && n < 1500 {
            if range - 17 > 0 {
                crt.rand() as i32 % (range - 16) + 8
            } else {
                8
            }
        } else if range - 1 > 0 {
            crt.rand() as i32 % range
        } else {
            0
        }
    }
}

/// Place the starts of every civ in `civs_mask` (bit n = civ n), consuming
/// the CRT draws exactly as the original does. Each accepted tile gets the
/// settler's marks (unit bit, owner), as the unit creation sets them.
pub fn place_starts(
    map: &mut StartMap,
    human_mask: u8,
    civs_mask: u8,
    crt: &mut CrtRand,
) -> Vec<Start> {
    let _ = crt.rand() % 50; // 0x48c5ce
    let mut units: Vec<(i32, i32)> = Vec::new();
    let mut starts = Vec::new();
    for civ in 1..=7u8 {
        let human = human_mask >> civ & 1 != 0;
        for _ in 1..=7 {
            crt.rand(); // attitude toward each civ (0x4888f2 / 0x488959)
        }
        if !human {
            crt.rand(); // tribe pick (0x488fe5)
            crt.rand(); // rand() % 3 (0x4890d1)
        }
        if civs_mask >> civ & 1 == 0 {
            continue;
        }
        let mut n = 0u32;
        let (x, y) = loop {
            let mut x = map.coord(crt, map.w2, map.w2 > 80, n);
            let y = map.coord(crt, map.h, map.h >= 40, n);
            if x & 1 != 0 {
                x -= 1;
            }
            if y & 1 != 0 {
                x += 1;
            }
            n += 1;
            if map.accepts(x, y, n, human, &units) || n >= MAX_TRIES {
                break (x, y);
            }
        };
        let t = map.tile_mut(x, y);
        t[1] |= 0x01;
        t[5] = (t[5] & 0x0F) | (civ << 4);
        units.push((x, y));
        starts.push(Start {
            civ,
            x,
            y,
            tries: n,
        });
    }
    starts
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../docs/evidence/rg2-placement-fixture.txt");

    struct World {
        label: String,
        map: StartMap,
        human: u8,
        mask: u8,
        crt_start: u32,
        crt_end: u32,
        draws: u64,
        starts: Vec<Start>,
    }

    fn hex(v: &str) -> u32 {
        u32::from_str_radix(v.trim_start_matches("0x"), 16).unwrap()
    }

    /// The fixture's tiles on an otherwise all-ocean map. Each sampled
    /// tile gets its own landmass id carrying its sampled value: placement
    /// only examines the tiles its draws pick, so if the decode drifts it
    /// lands on an unsampled (ocean) tile and the starts stop matching.
    fn worlds() -> Vec<World> {
        let mut out = Vec::new();
        let mut cur: Option<World> = None;
        for line in FIXTURE.lines().filter(|l| !l.starts_with('#')) {
            let f: Vec<&str> = line.split_whitespace().collect();
            match f.first().copied() {
                Some("world") => {
                    cur = Some(World {
                        label: f[1].into(),
                        map: StartMap {
                            w2: 0,
                            h: 0,
                            seed: 0,
                            flat: false,
                            huts: true,
                            tiles: vec![],
                            landmass_value: vec![0],
                        },
                        human: 0,
                        mask: 0,
                        crt_start: 0,
                        crt_end: 0,
                        draws: 0,
                        starts: vec![],
                    })
                }
                Some("setup") => {
                    let w = cur.as_mut().unwrap();
                    for kv in &f[1..] {
                        let (k, v) = kv.split_once('=').unwrap();
                        match k {
                            "w2" => w.map.w2 = v.parse().unwrap(),
                            "h" => w.map.h = v.parse().unwrap(),
                            "seed" => w.map.seed = hex(v) as u16,
                            "human" => w.human = hex(v) as u8,
                            "mask" => w.mask = hex(v) as u8,
                            _ => {}
                        }
                    }
                    let n = (w.map.w2 / 2 * w.map.h) as usize;
                    w.map.tiles = vec![[OCEAN, 0, 0, 0, 0, 0xF0]; n];
                }
                Some("crt_start") => cur.as_mut().unwrap().crt_start = hex(f[1]),
                Some("crt_end") => cur.as_mut().unwrap().crt_end = hex(f[1]),
                Some("draws") => cur.as_mut().unwrap().draws = f[1].parse().unwrap(),
                Some("start") => {
                    let p: Vec<i32> = f[1..].iter().map(|v| v.parse().unwrap()).collect();
                    cur.as_mut().unwrap().starts.push(Start {
                        civ: p[0] as u8,
                        x: p[1],
                        y: p[2],
                        tries: p[3] as u32,
                    });
                }
                Some("tile") => {
                    let p: Vec<i32> = f[1..].iter().map(|v| v.parse().unwrap()).collect();
                    let w = cur.as_mut().unwrap();
                    let id = w.map.landmass_value.len() as u8;
                    w.map.landmass_value.push(p[5] as i16);
                    *w.map.tile_mut(p[0], p[1]) =
                        [p[2] as u8, 0, p[3] as u8, id, 0, p[4] as u8 | 0xF0];
                }
                Some("end") => out.push(cur.take().unwrap()),
                _ => {}
            }
        }
        out
    }

    #[test]
    fn three_oracle_worlds_replay_draw_for_draw() {
        let ws = worlds();
        assert_eq!(ws.len(), 3);
        for mut w in ws {
            let mut crt = CrtRand::new(w.crt_start);
            let got = place_starts(&mut w.map, w.human, w.mask, &mut crt);
            assert_eq!(got, w.starts, "{}: starts", w.label);
            assert_eq!(crt.draws(), w.draws, "{}: draw count", w.label);
            assert_eq!(crt.state(), w.crt_end, "{}: CRT state after", w.label);
        }
    }

    #[test]
    fn start_map_reads_a_save() {
        let sav = crate::tests_support::synthetic_save();
        let m = StartMap::from_save(&sav).unwrap();
        assert_eq!(
            (m.w2, m.h),
            (sav.map_header.raw[0] as i32, sav.map_header.raw[1] as i32)
        );
        assert_eq!(m.tiles.len(), sav.map_header.surface());
        assert_eq!(m.landmass_value.len(), 256);
        assert_eq!(m.tiles[2][0], 1 | 0x80, "terrain byte +0 (river grassland)");
        assert_eq!(m.owner(4, 0), None);
    }

    #[test]
    fn distance_is_the_diamond_metric_with_wrap() {
        let m = StartMap {
            w2: 80,
            h: 50,
            seed: 0,
            flat: false,
            huts: true,
            tiles: vec![],
            landmass_value: vec![],
        };
        // doubled x: (x±2, y) and (x±1, y±1) are the neighbours
        assert_eq!(m.distance(10, 10, 12, 10), 1);
        assert_eq!(m.distance(10, 10, 11, 11), 1);
        assert_eq!(m.distance(10, 10, 10, 12), 1);
        assert_eq!(m.distance(10, 10, 14, 10), 3);
        assert_eq!(m.distance(0, 10, 78, 10), 1); // across the seam
        let flat = StartMap { flat: true, ..m };
        assert_eq!(flat.distance(0, 10, 78, 10), 58);
    }
}
