//! SAV/SCN decoding — the proven `0x32` base layout, ported from the
//! Python reference walker (`tools/totctl.py walk_save`).
//!
//! Every offset below is evidence-backed (`docs/evidence/README.md`
//! §T0-6, §T0-7, §RG-1); nothing here is folklore. Raw record bytes are
//! kept alongside typed views so a future byte-compatible writer (T1-8)
//! can round-trip fields we have not decoded yet.
//!
//! Layout (file offsets, little-endian):
//!
//! | offset | size | contents |
//! |---|---|---|
//! | 0 | 8 | magic `CIVILIZE` |
//! | 10 | 2 | version u16 (`0x32`, or `0x33` = ToTPP-extended; front sections identical) |
//! | 12 | 640 | unit transport relationships (Rules.txt-derived settings) |
//! | 652 | 16 | menu |
//! | 668 | 38 | game block: turn i16, year i16, **difficulty u8@684**, seed word ×2 @685–688, **rivals u8@689** (§RG-1) |
//! | 706 | 200 | technologies: 100 "first discovered" + 100 bitmask |
//! | 906 | 56 | wonders, 28 × i16 (−1 unbuilt, −2 destroyed) |
//! | 962 | 288 | undecoded |
//! | 1250 | 7×242 | civ name blocks (8 NUL-terminated strings each) |
//! | 2944 | 8×3348 | civ property blocks |
//! | 29728 | 168+4+14N | transporters: fixed 168 B, count u32@+168, records ×14 (issue #49); with count 0 the next section starts at 29900 — which is why the map header reads "at 29900" on every count-0 file (standard games, ROMFALL) |
//! | 29728+172+14N | 16 | map header, 8 × i16 (see [`MapHeader`]) |
//!
//! The map blocks follow the header, then a `2*hdr[5]*hdr[6]` blob and a
//! 10240-byte blob; units (34-byte preamble + N × 40 B) and cities
//! (N × 92 B) are located after that by the name-chain scan with the
//! anti-fabrication arithmetic gate (issues #49/#58).

use std::fmt;

pub const MAGIC: &[u8; 8] = b"CIVILIZE";

/// Walk error — no panics on malformed input, ever.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavError {
    BadMagic,
    UnknownVersion(u16),
    Truncated(&'static str),
    /// map header invariant failed: no decodable embedded map
    NoMap,
    /// unit region did not satisfy `34 + 40*N` (#49/#58 gate)
    UnitRegionArithmetic {
        region: usize,
        rem: usize,
    },
    NoCityChain,
    /// writer edit rejected (out-of-range index, oversize string, …)
    BadEdit(&'static str),
}

impl fmt::Display for SavError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SavError::BadMagic => write!(f, "not a Civ2 save/scenario (bad magic)"),
            SavError::UnknownVersion(v) => write!(f, "unknown save version 0x{v:04x}"),
            SavError::Truncated(what) => write!(f, "truncated file while reading {what}"),
            SavError::NoMap => write!(f, "surface invariant failed; no decodable map"),
            SavError::UnitRegionArithmetic { region, rem } => {
                write!(f, "unit region {region} bytes is not 34 + 40*N (rem {rem})")
            }
            SavError::NoCityChain => {
                write!(f, "no 92-B city name-chain located after the map blob")
            }
            SavError::BadEdit(why) => write!(f, "bad save edit: {why}"),
        }
    }
}

impl std::error::Error for SavError {}

fn u16le(d: &[u8], off: usize) -> Result<u16, SavError> {
    d.get(off..off + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or(SavError::Truncated("u16"))
}

fn i16le(d: &[u8], off: usize) -> Result<i16, SavError> {
    d.get(off..off + 2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .ok_or(SavError::Truncated("i16"))
}

fn u32le(d: &[u8], off: usize) -> Result<u32, SavError> {
    d.get(off..off + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or(SavError::Truncated("u32"))
}

/// Map header: 8 × i16 at the map section start.
///
/// Proven reading (`walk_save`, §T0-6; dims three-size-confirmed §RG-1):
/// `raw[0] = 2*height` (logical/display height × 2), `raw[1] = width`,
/// `raw[2] = area = height*width` (the *stored* tile count: the terrain
/// grid is `raw[0]/2 × raw[1]` dense diamond tiles), `raw[3]` is NOT the
/// map count (a `0` file can still carry its surface map — issue #58):
/// block count = `max(raw[3], 1)`. `raw[5]*raw[6]` sizes a following blob.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapHeader {
    pub raw: [i16; 8],
}

impl MapHeader {
    /// Stored terrain-grid width (= `raw[0] / 2`).
    pub fn stored_w(&self) -> usize {
        (self.raw[0] / 2) as usize
    }
    /// Stored terrain-grid height (= `raw[1]`).
    pub fn stored_h(&self) -> usize {
        self.raw[1] as usize
    }
    /// Stored tile count (must equal `stored_w * stored_h` to decode).
    pub fn surface(&self) -> usize {
        self.raw[2] as usize
    }
    /// Number of map blocks carried (at least the surface block).
    pub fn blocks(&self) -> usize {
        (self.raw[3].max(1)) as usize
    }
    /// The surface invariant — the test for a decodable embedded map.
    pub fn decodable(&self) -> bool {
        self.surface() > 0
            && self.raw[0] > 0
            && self.raw[1] > 0
            && self.surface() == self.stored_w() * self.stored_h()
    }
    /// Logical (display) grid dims: `raw[0] × raw[1]` — the coordinate
    /// space of unit and city records.
    pub fn logical_dims(&self) -> (usize, usize) {
        (self.raw[0] as usize, self.raw[1] as usize)
    }
    /// Logical (x, y) → stored terrain index. Proven mapping
    /// (`walk_save` geometry note; per-file city-terrain support
    /// measured in the Python walker): `sx = (x - (y&1))/2 - 1, sy = y`.
    /// Returns None for logical tiles outside the stored grid.
    pub fn stored_index(&self, x: usize, y: usize) -> Option<usize> {
        let sx = (x as isize - (y as isize & 1)) / 2 - 1;
        if sx < 0 || y >= self.stored_h() || sx as usize >= self.stored_w() {
            return None;
        }
        Some(y * self.stored_w() + sx as usize)
    }
}

/// One decoded map block: per-tile visibility (7 B/tile, near-identical
/// per-civ distribution — per-civ fog, unproven) and the 6-B-stride tile
/// fields. Only the empirically pinned byte positions are exposed
/// (`+4` terrain/special/river, `+14` road/rail); the rest of the stride
/// interleave is open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapBlock {
    pub visibility: Vec<u8>,
    /// 6 B per stored tile, in stored order.
    pub tile_fields: Vec<u8>,
}

impl MapBlock {
    /// Terrain id (`& 0x3F`, 0–10 → Rules.txt @TERRAIN1..3 ids) of stored
    /// tile `t`.
    pub fn terrain(&self, t: usize) -> u8 {
        self.tile_fields[6 * t + 4] & 0x3F
    }
    /// Special resource flag (bit 0x40 of the terrain byte).
    pub fn is_special(&self, t: usize) -> bool {
        self.tile_fields[6 * t + 4] & 0x40 != 0
    }
    /// River flag (bit 0x80 of the terrain byte).
    pub fn is_river(&self, t: usize) -> bool {
        self.tile_fields[6 * t + 4] & 0x80 != 0
    }
    /// Road flag (bit 0x40 at `+14` — ROMFALL spatial verification).
    /// Note: `6t+14` exceeds the 6·surface tile-field block for the last
    /// two tiles of each map — the Python walker silently reads past
    /// into the next section there; we bounds-check and report `false`.
    pub fn has_road(&self, t: usize) -> bool {
        self.tile_fields
            .get(6 * t + 14)
            .is_some_and(|&b| b & 0x40 != 0)
    }
    /// Railroad flag (bit 0x80 at `+14`), same bounds note as `has_road`.
    pub fn has_rail(&self, t: usize) -> bool {
        self.tile_fields
            .get(6 * t + 14)
            .is_some_and(|&b| b & 0x80 != 0)
    }
    /// Histogram of terrain ids (index = id).
    pub fn terrain_histogram(&self) -> [usize; 64] {
        let mut h = [0usize; 64];
        for t in 0..self.tile_fields.len() / 6 {
            h[self.terrain(t) as usize] += 1;
        }
        h
    }
}

/// A 40-byte unit record. Proven fields (§T0-6): x u16@6 / y u16@8
/// (logical grid), type u8@14 (Rules.txt @UNITS index), owner u8@15
/// (0 = barbarians, 1..7 = players, bit 0x10 = leader unit), id u16@36
/// (re-assigned on every save). Remaining bytes open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitRecord {
    pub raw: [u8; 40],
}

impl UnitRecord {
    pub fn x(&self) -> u16 {
        u16::from_le_bytes([self.raw[6], self.raw[7]])
    }
    pub fn y(&self) -> u16 {
        u16::from_le_bytes([self.raw[8], self.raw[9]])
    }
    pub fn type_id(&self) -> u8 {
        self.raw[14]
    }
    pub fn owner(&self) -> u8 {
        self.raw[15]
    }
    /// Leader-unit flag (bit 0x10 of owner — e.g. Pope Leo I in ROMFALL).
    pub fn is_leader(&self) -> bool {
        self.raw[15] & 0x10 != 0
    }
    pub fn id(&self) -> u16 {
        u16::from_le_bytes([self.raw[36], self.raw[37]])
    }
}

/// A 92-byte city record located by the name-chain scan. Proven fields
/// (oracle-verified against the game's City Status report, F1 — evidence
/// §T1-4): name 16 B @0, x u16@58 / y u16@60 (logical grid), owner u16@50,
/// city id u16@52 (sequential), net science u16@42, net taxes u16@44,
/// gross trade u16@46, gross food u8@48, gross shields u8@49. The dynamic
/// shorts u16@86/@88 are the food/shield boxes (AI-city evolution,
/// standard game; see evidence), u16@90 is size-correlated but unresolved.
/// City size is NOT inside the record: it lives in the two bytes at
/// `city_base − 13` (i.e. in the previous slot / the interlude) — see
/// [`SaveFile::city_size`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CityRecord {
    pub raw: [u8; 92],
    /// City size, read from the u16 at `city_base − 13` (inside the
    /// previous slot / the interlude before the grid) at parse time.
    /// Verified 8/8 against the City Status report in FallRome; the
    /// encoding for some non-WRE civs and standard-game cities read 0 —
    /// treat 0 as "unknown" there (evidence §T1-4).
    pub size: u16,
}

impl CityRecord {
    pub fn name(&self) -> String {
        let end = self.raw[..16].iter().position(|&b| b == 0).unwrap_or(16);
        String::from_utf8_lossy(&self.raw[..end]).into_owned()
    }
    pub fn x(&self) -> u16 {
        u16::from_le_bytes([self.raw[58], self.raw[59]])
    }
    pub fn y(&self) -> u16 {
        u16::from_le_bytes([self.raw[60], self.raw[61]])
    }
    /// Gross food output — the City Status report's Food column (u8@48).
    pub fn gross_food(&self) -> u8 {
        self.raw[48]
    }
    /// Gross shield output — the City Status Shields column (u8@49).
    pub fn gross_shields(&self) -> u8 {
        self.raw[49]
    }
    /// Gross trade arrows — the City Status Trade column (u16@46).
    pub fn gross_trade(&self) -> u16 {
        u16::from_le_bytes([self.raw[46], self.raw[47]])
    }
    /// Net science after the civ's rate split and library bonuses (u16@42).
    pub fn net_science(&self) -> u16 {
        u16::from_le_bytes([self.raw[42], self.raw[43]])
    }
    /// Net taxes after the rate split and marketplace bonuses (u16@44).
    pub fn net_taxes(&self) -> u16 {
        u16::from_le_bytes([self.raw[44], self.raw[45]])
    }
    /// Owner civ (u16@50). FallRome scenario: 1..7 = civs (0 = barbarians).
    /// Standard-game encoding unverified (all observed values were 0).
    pub fn owner(&self) -> u16 {
        u16::from_le_bytes([self.raw[50], self.raw[51]])
    }
    /// Sequential city id (u16@52; i+1 in every file examined).
    pub fn city_id(&self) -> u16 {
        u16::from_le_bytes([self.raw[52], self.raw[53]])
    }
    /// Food storage box (u16@86): ticks by food surplus each turn,
    /// consumed by growth (evidence §T1-4, standard-game AI cities).
    pub fn food_box(&self) -> u16 {
        u16::from_le_bytes([self.raw[86], self.raw[87]])
    }
    /// Shield/production box (u16@88): ticks by net shields, reset on
    /// production completion.
    pub fn shield_box(&self) -> u16 {
        u16::from_le_bytes([self.raw[88], self.raw[89]])
    }
    /// Size-correlated short (u16@90): size+2 fits most observations but
    /// Babylon breaks it — semantics open, carried for cross-checking.
    pub fn tail90(&self) -> u16 {
        u16::from_le_bytes([self.raw[90], self.raw[91]])
    }
    /// Size-like short measured just before the name — superseded by
    /// [`SaveFile::city_size`], kept for compatibility.
    pub fn size_hint(&self) -> u16 {
        u16::from_le_bytes([self.raw[90], self.raw[91]])
    }
}

/// NUL-separated printable strings found in one 242-byte civ-name block,
/// in file order. **Per-slot semantics are unproven**: ROMFALL's blocks
/// read `leader, plural, adjective, …` (e.g. block 1: "Valentianus III",
/// "West Romans", "West Roman") — same probe order as the Python walker's
/// `null_strings`; no offset→field mapping is claimed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CivNames {
    pub strings: Vec<String>,
}

/// Port of the Python walker's `null_strings` (limit 8): collect runs of
/// printable ASCII terminated by NUL; non-printable bytes reset the run;
/// whitespace-only strings are dropped.
pub fn civ_names_from(block: &[u8]) -> CivNames {
    let mut strings = Vec::new();
    let mut cur: Vec<u8> = Vec::new();
    for &b in block {
        if b == 0 {
            if !cur.is_empty() {
                let s = String::from_utf8_lossy(&cur).into_owned();
                if !s.trim().is_empty() {
                    strings.push(s);
                }
                cur.clear();
            }
        } else if (32..127).contains(&b) {
            cur.push(b);
        } else {
            cur.clear();
        }
        if strings.len() == 8 {
            break;
        }
    }
    CivNames { strings }
}

/// A parsed `0x32` (or `0x33`-front) save/scenario. See the module docs
/// for the layout and the evidence trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveFile {
    /// Raw version word (0x0032 or 0x0033).
    pub version: u16,
    pub turn: i16,
    pub year: i16,
    /// Game block @684 — §RG-1 (0 = Chieftain … 5 = Deity).
    pub difficulty: u8,
    /// Duplicated word @685..688 (`0x02XX`) — the combat/event-replay
    /// seed mirror, NOT the world-gen state (T1-6 notes).
    pub seed_word: u16,
    /// Game block @689 — rivals count (§RG-1).
    pub rivals: u8,
    /// 28 wonders: −1 unbuilt, −2 destroyed, else the building civ.
    pub wonders: [i16; 28],
    /// 7 name blocks (players are `owner 1..7`; block n-1 matches).
    pub civ_names: Vec<CivNames>,
    pub transporter_count: u32,
    pub map_header: MapHeader,
    pub maps: Vec<MapBlock>,
    pub units: Vec<UnitRecord>,
    pub cities: Vec<CityRecord>,
    /// Byte range of the unit region (34 + 40*N), for diff tooling.
    pub unit_region: (usize, usize),
    /// Offset of the city grid.
    pub city_offset: usize,
}

/// Fixed offsets proven by the walker arithmetic (`assert pos == 12+1238`).
pub const OFF_TRANSPORT_REL: usize = 12;
pub const OFF_MENU: usize = 652;
pub const OFF_GAME: usize = 668;
pub const OFF_TECH: usize = 706;
pub const OFF_WONDERS: usize = 906;
pub const OFF_UNKNOWN_288: usize = 962;
pub const OFF_CIV_NAMES: usize = 1250;
pub const OFF_CIV_PROPS: usize = 2944;
pub const OFF_TRANSPORTERS: usize = 29728;

/// Parse a `0x32`/`0x33` SAV or SCN. For `0x33` (ToTPP-extended) this
/// decodes the base front sections and the classic unit/city layout; the
/// ToTPP tail (Lua state etc.) is out of scope here.
pub fn parse_sav(data: &[u8]) -> Result<SaveFile, SavError> {
    if data.len() < 12 || &data[..8] != MAGIC {
        return Err(SavError::BadMagic);
    }
    let version = u16le(data, 10)?;
    if version != 0x0032 && version != 0x0033 {
        return Err(SavError::UnknownVersion(version));
    }
    if data.len() < OFF_CIV_NAMES + 7 * 242 {
        return Err(SavError::Truncated("civ name blocks"));
    }

    let turn = i16le(data, OFF_GAME)?;
    let year = i16le(data, OFF_GAME + 2)?;
    // §RG-1: 684 difficulty | 685..688 seed word ×2 | 689 rivals. The two
    // copies must agree on every file observed; keep the first.
    let difficulty = data[684];
    let seed_word = u16le(data, 685)?;
    let rivals = data[689];

    let mut wonders = [0i16; 28];
    for (k, w) in wonders.iter_mut().enumerate() {
        *w = i16le(data, OFF_WONDERS + 2 * k)?;
    }

    let mut civ_names = Vec::with_capacity(7);
    for i in 0..7 {
        let block = data
            .get(OFF_CIV_NAMES + i * 242..OFF_CIV_NAMES + (i + 1) * 242)
            .ok_or(SavError::Truncated("civ name block"))?;
        civ_names.push(civ_names_from(block));
    }

    // transporters: fixed 168 B, then u32 count, then count × 14 B (#49)
    let transporter_count = u32le(data, OFF_TRANSPORTERS + 168)?;
    let mut pos = OFF_TRANSPORTERS + 172 + 14 * transporter_count as usize;
    if data.len() < pos + 16 {
        return Err(SavError::Truncated("map header"));
    }
    let mut raw = [0i16; 8];
    for (k, v) in raw.iter_mut().enumerate() {
        *v = i16le(data, pos + 2 * k)?;
    }
    let hdr = MapHeader { raw };
    pos += 16;
    if !hdr.decodable() {
        return Err(SavError::NoMap);
    }

    // map blocks: 13 B/tile + 2 per map (7 visibility + 6 tile fields),
    // then the 2*hdr5*hdr6 blob and the 10240-byte blob
    let surface = hdr.surface();
    let per_map = 13 * surface + 2;
    let n_blocks = hdr.blocks();
    let blob = 2 * hdr.raw[5] as usize * hdr.raw[6] as usize;
    let map_end = pos + n_blocks * per_map + blob;
    let blob10240_end = map_end + 10240;
    if data.len() < blob10240_end {
        return Err(SavError::Truncated("map section"));
    }
    let mut maps = Vec::with_capacity(n_blocks);
    for m in 0..n_blocks {
        let base = pos + m * per_map + 2;
        let vis = data[base..base + 7 * surface].to_vec();
        let tiles = data[base + 7 * surface..base + 13 * surface].to_vec();
        maps.push(MapBlock {
            visibility: vis,
            tile_fields: tiles,
        });
    }

    // cities: 92-B name-chain scan (anti-fabrication #49/#58) after the
    // 10240 blob. Mirrors the Python walker byte-for-byte: a candidate is
    // a 3..15-byte printable run whose 16-byte slot NUL-pads to +15, and
    // the scan jumps past a found name (interior starts are not examined).
    let mut cands: Vec<usize> = Vec::new();
    let mut i = blob10240_end;
    while i + 16 <= data.len() {
        let j = data[i..i + 16].iter().position(|&b| b == 0).unwrap_or(16);
        if (3..16).contains(&j) {
            let nm = &data[i..i + j];
            let printable = nm
                .iter()
                .all(|&c| (32..127).contains(&c) || (0xA0..=0xFF).contains(&c));
            if printable && data[i + 15] == 0 {
                cands.push(i);
                i += j + 1;
                continue;
            }
        }
        i += 1;
    }
    let cand_set: std::collections::HashSet<usize> = cands.iter().copied().collect();
    let mut best_chain: Vec<usize> = Vec::new();
    for &s in &cands {
        if !best_chain.is_empty() && s <= best_chain[0] + 92 * (best_chain.len() - 1) {
            continue; // inside an existing chain
        }
        let mut chain = vec![s];
        while let Some(&next) = cand_set.get(&(chain.last().unwrap() + 92)) {
            chain.push(next);
        }
        if chain.len() > best_chain.len() {
            best_chain = chain;
        }
    }
    if best_chain.len() < 2 {
        // >= 2, not >= 8 (T1-4, 2026-11-03): standard-game early saves
        // hold only the AI capitals (orig-A turn02 = 6 cities, region
        // arithmetic 11514 = 40x287+34 validates). The anti-fabrication
        // authority stays the 34+40*N unit-region gate below; chain
        // length only locates the grid.
        return Err(SavError::NoCityChain);
    }
    let city_offset = best_chain[0];
    let n_cities = (best_chain.last().unwrap() - city_offset) / 92 + 1;

    // units: the region between the 10240 blob and the city grid must be
    // 34 + 40*N (proven on ROMFALL/Va_Auto; #49/#58 gate)
    let region = city_offset - blob10240_end;
    let (unit_count, rem) = (region / 40, region % 40);
    if rem != 34 {
        return Err(SavError::UnitRegionArithmetic { region, rem });
    }
    let mut units = Vec::with_capacity(unit_count);
    for u in 0..unit_count {
        let start = blob10240_end + 34 + 40 * u;
        let mut raw = [0u8; 40];
        raw.copy_from_slice(
            data.get(start..start + 40)
                .ok_or(SavError::Truncated("unit record"))?,
        );
        units.push(UnitRecord { raw });
    }
    let mut cities = Vec::with_capacity(n_cities);
    for c in 0..n_cities {
        let start = city_offset + 92 * c;
        let mut raw = [0u8; 92];
        raw.copy_from_slice(
            data.get(start..start + 92)
                .ok_or(SavError::Truncated("city record"))?,
        );
        // size: u16 at city_base − 13 (may reach into the interlude)
        let sz = if start >= 13 {
            u16::from_le_bytes([data[start - 13], data[start - 12]])
        } else {
            0
        };
        cities.push(CityRecord { raw, size: sz });
    }

    Ok(SaveFile {
        version,
        turn,
        year,
        difficulty,
        seed_word,
        rivals,
        wonders,
        civ_names,
        transporter_count,
        map_header: hdr,
        maps,
        units,
        cities,
        unit_region: (blob10240_end, city_offset),
        city_offset,
    })
}

/// Read a save/scenario from disk.
pub fn parse_sav_file(path: &std::path::Path) -> Result<SaveFile, SavError> {
    parse_sav(&std::fs::read(path).map_err(|_| SavError::Truncated("file read"))?)
}

// ---------------------------------------------------------------- writer
//
// Surgical `0x32` writer (T1-8). The editor starts from an existing file
// and applies typed edits to the proven field offsets, leaving every
// other byte untouched — a no-edit round trip is byte-identical by
// construction. Full generation from scratch is T1-8's later increment;
// this is the increment the oracle can already accept (write → load in
// the original → field-explained diffs).

/// Byte offsets the editor writes (all proven; see module docs).
pub const OFF_TURN: usize = OFF_GAME;
pub const OFF_YEAR: usize = OFF_GAME + 2;

/// A mutable view over one save file for typed field edits.
#[derive(Debug, Clone)]
pub struct SavEditor {
    buf: Vec<u8>,
    /// first byte of unit record 0 (region start + 34-byte preamble)
    unit_base: usize,
    city_base: usize,
    n_units: usize,
    n_cities: usize,
}

impl SavEditor {
    /// Open a file for editing; validates the layout first (the same
    /// gates as [`parse_sav`], so we never edit a misaligned file).
    pub fn new(raw: &[u8]) -> Result<Self, SavError> {
        let sav = parse_sav(raw)?;
        Ok(SavEditor {
            buf: raw.to_vec(),
            unit_base: sav.unit_region.0 + 34,
            city_base: sav.city_offset,
            n_units: sav.units.len(),
            n_cities: sav.cities.len(),
        })
    }

    pub fn set_turn(&mut self, turn: i16) {
        self.buf[OFF_TURN..OFF_TURN + 2].copy_from_slice(&turn.to_le_bytes());
    }

    pub fn set_year(&mut self, year: i16) {
        self.buf[OFF_YEAR..OFF_YEAR + 2].copy_from_slice(&year.to_le_bytes());
    }

    /// Game block @684 (0 = Chieftain … 5 = Deity).
    pub fn set_difficulty(&mut self, difficulty: u8) {
        self.buf[684] = difficulty;
    }

    /// Game block @689 (number of rival civs).
    pub fn set_rivals(&mut self, rivals: u8) {
        self.buf[689] = rivals;
    }

    /// Move a unit on the logical grid (x u16@6, y u16@8 of its record).
    pub fn set_unit_pos(&mut self, idx: usize, x: u16, y: u16) -> Result<(), SavError> {
        if idx >= self.n_units {
            return Err(SavError::BadEdit("unit index out of range"));
        }
        let base = self.unit_base + 40 * idx;
        self.buf[base + 6..base + 8].copy_from_slice(&x.to_le_bytes());
        self.buf[base + 8..base + 10].copy_from_slice(&y.to_le_bytes());
        Ok(())
    }

    /// Change a unit's type id (Rules @UNITS index) or owner byte.
    pub fn set_unit_type(&mut self, idx: usize, type_id: u8) -> Result<(), SavError> {
        if idx >= self.n_units {
            return Err(SavError::BadEdit("unit index out of range"));
        }
        self.buf[self.unit_base + 40 * idx + 14] = type_id;
        Ok(())
    }

    /// Rename a city (≤15 bytes; the 16-byte slot is NUL-padded).
    pub fn set_city_name(&mut self, idx: usize, name: &str) -> Result<(), SavError> {
        if idx >= self.n_cities {
            return Err(SavError::BadEdit("city index out of range"));
        }
        let bytes = name.as_bytes();
        if bytes.len() > 15 {
            return Err(SavError::BadEdit("city name longer than 15 bytes"));
        }
        let base = self.city_base + 92 * idx;
        for b in &mut self.buf[base..base + 16] {
            *b = 0;
        }
        self.buf[base..base + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    /// Move a city (x u16@58, y u16@60 of its record).
    pub fn set_city_pos(&mut self, idx: usize, x: u16, y: u16) -> Result<(), SavError> {
        if idx >= self.n_cities {
            return Err(SavError::BadEdit("city index out of range"));
        }
        let base = self.city_base + 92 * idx;
        self.buf[base + 58..base + 60].copy_from_slice(&x.to_le_bytes());
        self.buf[base + 60..base + 62].copy_from_slice(&y.to_le_bytes());
        Ok(())
    }

    pub fn bytes(&self) -> &[u8] {
        &self.buf
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
}

/// Convenience: parse, unmodified re-emit (must equal the input bytes).
pub fn emit_sav(raw: &[u8]) -> Result<Vec<u8>, SavError> {
    Ok(SavEditor::new(raw)?.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal synthetic 0x32 file exercising every section the parser
    /// decodes: fixed front blocks, a 4x4-stored map (hdr = (8, 4, 16)),
    /// one unit, and a 8-slot city grid (the scan requires >= 8).
    fn synthetic() -> Vec<u8> {
        let stored_w = 4usize; // hdr[0] = 8
        let stored_h = 4usize; // hdr[1] = 4
        let surface = stored_w * stored_h; // 16
        let n_cities = 8;
        let mut d = vec![0u8; OFF_TRANSPORTERS + 172];
        d[..8].copy_from_slice(MAGIC);
        d[10..12].copy_from_slice(&0x0032u16.to_le_bytes());
        d[OFF_GAME..OFF_GAME + 2].copy_from_slice(&1i16.to_le_bytes()); // turn
        d[OFF_GAME + 2..OFF_GAME + 4].copy_from_slice(&(-4000i16).to_le_bytes());
        d[684] = 4; // Emperor
        d[685..687].copy_from_slice(&0x02dbu16.to_le_bytes()); // seed ×2
        d[687..689].copy_from_slice(&0x02dbu16.to_le_bytes());
        d[689] = 4; // rivals
        let mut wonders = [-1i16; 28];
        wonders[3] = 2;
        for (w, v) in d[OFF_WONDERS..OFF_WONDERS + 56].chunks_mut(2).zip(wonders) {
            w.copy_from_slice(&v.to_le_bytes());
        }
        // civ names block 1: two printable runs "Romans", "Caesar"
        let b1 = OFF_CIV_NAMES + 242;
        d[b1..b1 + 6].copy_from_slice(b"Romans");
        d[b1 + 7..b1 + 7 + 6].copy_from_slice(b"Caesar");

        let transporters_end = OFF_TRANSPORTERS + 172; // count u32 = 0
        let mut out = d;
        out.truncate(transporters_end);
        // map header: (2*h, w, area, blocks, ?, blob5, blob6, ?)
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
        for v in hdr {
            out.extend_from_slice(&v.to_le_bytes());
        }
        // one map block: 2 + 7*surface + 6*surface
        out.extend_from_slice(&[0, 0]); // +2 prefix
        out.extend(std::iter::repeat_n(0u8, 7 * surface)); // visibility
        for t in 0..surface {
            // tile fields: 6 B; terrain byte at +4 (grasslands=1, river on t=2)
            let mut f = [0u8; 6];
            f[4] = if t == 2 { 1 | 0x80 } else { 1 };
            out.extend_from_slice(&f);
        }
        out.extend(std::iter::repeat_n(0u8, 10240)); // blob 10240
                                                     // units: 34-byte preamble + 2 × 40 B
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
        // city grid: 8 × 92 with names at +0, NUL padding to +16
        for i in 0..n_cities {
            let mut r = [0u8; 92];
            let name = format!("City{i}");
            r[..name.len()].copy_from_slice(name.as_bytes());
            r[58..60].copy_from_slice(&((i + 1) as u16).to_le_bytes());
            r[60..62].copy_from_slice(&((i + 2) as u16).to_le_bytes());
            r[90..92].copy_from_slice(&((i + 3) as u16).to_le_bytes());
            out.extend_from_slice(&r);
        }
        out
    }

    #[test]
    fn synthetic_round_trips() {
        let sav = parse_sav(&synthetic()).expect("synthetic parses");
        assert_eq!(sav.version, 0x0032);
        assert_eq!(sav.turn, 1);
        assert_eq!(sav.year, -4000);
        assert_eq!(sav.difficulty, 4);
        assert_eq!(sav.seed_word, 0x02db);
        assert_eq!(sav.rivals, 4);
        assert_eq!(sav.wonders[3], 2);
        assert_eq!(sav.civ_names[1].strings, vec!["Romans", "Caesar"]);
        assert_eq!(sav.transporter_count, 0);
        assert_eq!(sav.maps.len(), 1);
        assert_eq!(sav.map_header.surface(), 16);
        assert_eq!(sav.units.len(), 2);
        assert_eq!(sav.units[0].x(), 3);
        assert_eq!(sav.units[1].owner(), 2);
        assert_eq!(sav.units[1].id(), 2);
        assert_eq!(sav.cities.len(), 8);
        assert_eq!(sav.cities[0].name(), "City0");
        assert_eq!(sav.cities[7].x(), 8);
        // terrain decode: 15 grasslands(1), 1 river flag
        let m = &sav.maps[0];
        assert_eq!(m.terrain_histogram()[1], 16);
        assert!(m.is_river(2));
        assert!(!m.is_special(2));
        // logical→stored mapping sanity
        assert!(sav.map_header.stored_index(0, 0).is_none()); // border col
        assert!(sav.map_header.stored_index(4, 1).is_some());
    }

    #[test]
    fn rejects_bad_magic_and_version() {
        assert_eq!(parse_sav(&[0u8; 64]), Err(SavError::BadMagic));
        let mut d = vec![0u8; 30000];
        d[..8].copy_from_slice(MAGIC);
        d[10..12].copy_from_slice(&0x0021u16.to_le_bytes());
        assert_eq!(parse_sav(&d), Err(SavError::UnknownVersion(0x0021)));
    }

    #[test]
    fn rejects_unit_region_misalignment() {
        let mut d = synthetic();
        // corrupt the gate: insert one extra byte before the unit preamble
        // (inside the 10240 blob tail) and drop one byte from the city grid
        let end = d.len();
        d.insert(end - 8 * 92, 0);
        d.pop();
        match parse_sav(&d) {
            Err(SavError::UnitRegionArithmetic { .. }) => {}
            other => panic!("expected arithmetic gate, got {other:?}"),
        }
    }
}
