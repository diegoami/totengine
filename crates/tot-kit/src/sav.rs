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
//! 10240-byte blob; units (N × 40 B) and cities (M × 92 B, name at
//! +34, so blob end → first name = 40·N + 34) are located after that by
//! the name-chain scan with the anti-fabrication arithmetic gate (issues
//! #49/#58).

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
    /// The surface invariant — the test for a decodable embedded map —
    /// plus sign checks on every field the parser sizes sections with
    /// (reviewer finding #64: a negative `raw[5]` overflowed the blob size).
    pub fn decodable(&self) -> bool {
        self.raw[0] > 0
            && self.raw[1] > 0
            && self.raw[2] > 0
            && self.raw[3] >= 0
            && self.raw[5] >= 0
            && self.raw[6] >= 0
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

/// Names in saves are Latin-1 bytes (evidence §T0-6; ROMFALL city 35 is
/// "Arsinoë", byte 0xEB). Every byte maps to the code point of the same
/// value, so decoding never loses text (reviewer finding #69).
pub fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

/// A 40-byte unit record in the **game's own framing** (evidence §T1-6
/// combat, "record framing"): record `i` is the game's unit index `i` (the
/// active-unit field @674 and the stack links use these indices). Until
/// 2026-11-03 this crate framed records 6 bytes late (old offsets were
/// these plus 6, the index was the game index − 1, and the last "record"
/// was the first city's 34-byte head). Fields: x u16@0 / y u16@2 (logical grid), z @4, attr
/// u16@6, type u8@8 (Rules @UNITS index), owner u8@9 (0 = barbarians,
/// 1..7 = players, bit 0x10 = leader unit), moves spent @10, visibility
/// @11, damage @12, last direction @13, orders @17, home city @18, stack
/// links i16@26/@28, id u16@30 (re-assigned on every save).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitRecord {
    pub raw: [u8; 40],
}

impl UnitRecord {
    pub fn x(&self) -> u16 {
        u16::from_le_bytes([self.raw[0], self.raw[1]])
    }
    pub fn y(&self) -> u16 {
        u16::from_le_bytes([self.raw[2], self.raw[3]])
    }
    pub fn type_id(&self) -> u8 {
        self.raw[8]
    }
    pub fn owner(&self) -> u8 {
        self.raw[9]
    }
    /// Leader-unit flag (bit 0x10 of owner — e.g. Pope Leo I in ROMFALL).
    pub fn is_leader(&self) -> bool {
        self.raw[9] & 0x10 != 0
    }
    pub fn id(&self) -> u16 {
        u16::from_le_bytes([self.raw[30], self.raw[31]])
    }
    /// Attribute word u16@6: bit `0x2000` = veteran (read by the
    /// attack/defence strength functions, set by promotion; oracle battle
    /// 6), bit `0x0100` set on fortified/sentried units.
    pub fn attr(&self) -> u16 {
        u16::from_le_bytes([self.raw[6], self.raw[7]])
    }
    pub fn is_veteran(&self) -> bool {
        self.attr() & 0x2000 != 0
    }
    /// Movement spent this turn, u8@10, in units of the game's one-move
    /// byte (15 in FallRome; panel "Moves: 0 12/15" for 3 spent).
    pub fn moves_spent(&self) -> u8 {
        self.raw[10]
    }
    /// Damage taken (hit points lost; HP = Rules `h` × 10), u8@12 —
    /// oracle battles: 0→5, 1→4, and 10 on the killed unit.
    pub fn damage(&self) -> u8 {
        self.raw[12]
    }
    /// Orders u8@17: 0xff none, 1 fortifying, 2 fortified (2 makes the
    /// defence ×1.5 — oracle battle 5).
    pub fn orders(&self) -> u8 {
        self.raw[17]
    }
    /// Home city u8@18: index into the city grid, 0xff = none. Every
    /// homed unit standing on a city tile in ROMFALL (349/349) is homed
    /// there — the check that fixed the record framing.
    pub fn home(&self) -> u8 {
        self.raw[18]
    }
    /// Tile-stack links i16@26 / i16@28 (game record +0x1a / +0x1c): the
    /// previous / next unit on the same tile, as unit indices, −1 = none.
    /// The game walks them to pick a defender (VA 0x56909a / 0x568fdf);
    /// ROMFALL (22,30): 689 → 510, and 510 defended first — the last unit
    /// in list order wins ties.
    pub fn stack_prev(&self) -> i16 {
        i16::from_le_bytes([self.raw[26], self.raw[27]])
    }
    pub fn stack_next(&self) -> i16 {
        i16::from_le_bytes([self.raw[28], self.raw[29]])
    }
}

/// A 92-byte city record in the **game's own framing**: it starts 34 bytes
/// before the name (evidence §T1-6 combat, "record framing"; until
/// 2026-11-03 this crate framed it at the name, so its x/y and boxes were
/// the NEXT city's). The layout is the classic Civ2 city record shifted by
/// two bytes: x u16@0 / y u16@2 (logical grid), owner u8@10, size u8@11,
/// original owner u8@12, food box u16@28, shield box u16@30, name 16 B
/// @34, net science u16@76, net taxes u16@78, gross trade u16@80, gross
/// food u8@82, gross shields u8@83, city id u16@86. Proof: units standing
/// on a city tile are homed there 349/349 (ROMFALL) and their owner equals
/// u8@10 116/116; sizes @11 equal the City Status report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CityRecord {
    pub raw: [u8; 92],
}

impl CityRecord {
    pub fn name(&self) -> String {
        let n = &self.raw[34..50];
        let end = n.iter().position(|&b| b == 0).unwrap_or(16);
        latin1(&n[..end])
    }
    pub fn x(&self) -> u16 {
        u16::from_le_bytes([self.raw[0], self.raw[1]])
    }
    pub fn y(&self) -> u16 {
        u16::from_le_bytes([self.raw[2], self.raw[3]])
    }
    /// City size u8@11 (FallRome WRE 15,5,11,6,12,4,11,10 = City Status;
    /// also reads clean in standard games, where the old framing read 0).
    pub fn size(&self) -> u16 {
        self.raw[11] as u16
    }
    /// Gross food output — the City Status report's Food column (u8@82).
    pub fn gross_food(&self) -> u8 {
        self.raw[82]
    }
    /// Gross shield output — the City Status Shields column (u8@83).
    pub fn gross_shields(&self) -> u8 {
        self.raw[83]
    }
    /// Gross trade arrows — the City Status Trade column (u16@80).
    pub fn gross_trade(&self) -> u16 {
        u16::from_le_bytes([self.raw[80], self.raw[81]])
    }
    /// Net science after the civ's rate split and library bonuses (u16@76).
    pub fn net_science(&self) -> u16 {
        u16::from_le_bytes([self.raw[76], self.raw[77]])
    }
    /// Net taxes after the rate split and marketplace bonuses (u16@78).
    pub fn net_taxes(&self) -> u16 {
        u16::from_le_bytes([self.raw[78], self.raw[79]])
    }
    /// Owner civ u8@10 (0 = barbarians, 1..7). Equals the owner of every
    /// unit standing in the city (ROMFALL 116/116). The old "owner u16@50"
    /// (now @84) is NOT the owner: Roma reads 3 there.
    pub fn owner(&self) -> u16 {
        self.raw[10] as u16
    }
    /// Original owner u8@12 (equals @10 for 125/146 ROMFALL cities).
    pub fn original_owner(&self) -> u8 {
        self.raw[12]
    }
    /// Sequential city id (u16@86).
    pub fn city_id(&self) -> u16 {
        u16::from_le_bytes([self.raw[86], self.raw[87]])
    }
    /// Food storage box (u16@28): ticks by food surplus each turn,
    /// consumed by growth (evidence §T1-4; the per-city sample citations
    /// predate the framing fix, see there).
    pub fn food_box(&self) -> u16 {
        u16::from_le_bytes([self.raw[28], self.raw[29]])
    }
    /// Shield/production box (u16@30): ticks by net shields, reset on
    /// production completion.
    pub fn shield_box(&self) -> u16 {
        u16::from_le_bytes([self.raw[30], self.raw[31]])
    }
    /// u16@32 (the old "tail90"): semantics open.
    pub fn field32(&self) -> u16 {
        u16::from_le_bytes([self.raw[32], self.raw[33]])
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
    /// Fixed 24-byte NUL-terminated slots: leader @2, plural @26,
    /// adjective @50 of the 242-byte block (file 1252/1276/1300 for civ 1;
    /// the block's first 2 bytes are not a name). All 7 ROMFALL blocks read clean there, and the
    /// plurals are the names FallRome's Events.txt matches — "West
    /// Romans", "Huns"; the stray strings `strings` picks up are stale
    /// bytes after a slot's NUL).
    pub leader: String,
    pub plural: String,
    pub adjective: String,
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
                let s = latin1(&cur);
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
    let slot = |o: usize| -> String {
        let b = block.get(o..(o + 24).min(block.len())).unwrap_or(&[]);
        let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
        latin1(&b[..end])
    };
    CivNames {
        strings,
        leader: slot(2),
        plural: slot(26),
        adjective: slot(50),
    }
}

/// A parsed `0x32` (or `0x33`-front) save/scenario. See the module docs
/// for the layout and the evidence trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveFile {
    /// Raw version word (0x0032 or 0x0033).
    pub version: u16,
    pub turn: i16,
    pub year: i16,
    /// Game-flags word u16@652 (the RAM copy at VA 0x607638 aligns byte for
    /// byte). Bit `0x10`: damage persists between battles — without it
    /// the hit-point helper (VA 0x568d3e) zeroes damage first (evidence
    /// §T1-6 combat). ROMFALL: `0x81bf`.
    pub game_flags: u16,
    /// Human-players bitmask u8@687 (bit n = civ n): the byte the combat
    /// and promotion code test as "is this civ human" (VA 0x60765b, which
    /// aligns with @687). ROMFALL `0x02` = WRE only. Note @685..688 are also
    /// read as `seed_word` below — that label is unproven for @687.
    pub human_mask: u8,
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
    /// Treasury per civ, index = owner (0 = barbarians): i32@10 of the
    /// 3348-B civ-properties blocks @2944 (8 blocks end exactly at the
    /// transporter section @29728). Oracle patch-and-prove: RAM copy at
    /// VA 0x5F68A0 (same stride), WRE 9919 → patched 12345 → the status
    /// panel read "12345 Gold" (evidence §T1-7 treasury).
    pub treasury: [i32; 8],
    pub transporter_count: u32,
    pub map_header: MapHeader,
    pub maps: Vec<MapBlock>,
    pub units: Vec<UnitRecord>,
    pub cities: Vec<CityRecord>,
    /// Byte range of the unit records (40*N; ends where city record 0
    /// starts), for diff tooling.
    pub unit_region: (usize, usize),
    /// Offset of city record 0 (its name sits at +34).
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
/// Civ-properties block size: 8 blocks (barbarians + 7) fill
/// 2944..29728 exactly.
pub const CIV_PROPS_STRIDE: usize = 3348;
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
    let game_flags = u16le(data, 652)?;
    let human_mask = data[687];
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

    let mut treasury = [0i32; 8];
    for (c, t) in treasury.iter_mut().enumerate() {
        let o = OFF_CIV_PROPS + CIV_PROPS_STRIDE * c + 10;
        *t = i32::from_le_bytes(
            data.get(o..o + 4)
                .ok_or(SavError::Truncated("civ properties"))?
                .try_into()
                .unwrap(),
        );
    }

    // transporters: fixed 168 B, then u32 count, then count × 14 B (#49)
    let transporter_count = u32le(data, OFF_TRANSPORTERS + 168)?;
    let mut pos = (transporter_count as usize)
        .checked_mul(14)
        .and_then(|n| n.checked_add(OFF_TRANSPORTERS + 172))
        .ok_or(SavError::Truncated("transporters"))?;
    if data.len() < pos.saturating_add(16) {
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
    // all sizes are checked: header fields are untrusted (#64)
    let surface = hdr.surface();
    let n_blocks = hdr.blocks();
    let per_map = surface.checked_mul(13).and_then(|n| n.checked_add(2));
    let blob = (hdr.raw[5] as usize)
        .checked_mul(hdr.raw[6] as usize)
        .and_then(|n| n.checked_mul(2));
    let blob10240_end = per_map
        .and_then(|p| p.checked_mul(n_blocks))
        .zip(blob)
        .and_then(|(m, b)| pos.checked_add(m)?.checked_add(b)?.checked_add(10240))
        .filter(|&end| end <= data.len())
        .ok_or(SavError::Truncated("map section"))?;
    let per_map = per_map.unwrap_or(0);
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
    // a 2..15-byte printable run whose 16-byte slot NUL-pads to +15, and
    // the scan jumps past a found name (interior starts are not examined).
    let mut cands: Vec<usize> = Vec::new();
    let mut i = blob10240_end;
    while i + 16 <= data.len() {
        let j = data[i..i + 16].iter().position(|&b| b == 0).unwrap_or(16);
        // 2..15: two-letter city names are real ("Ur" in a standard-game
        // corpus broke the chain at exactly this bound, hiding later
        // cities — T1-4 2026-11-03)
        if (2..16).contains(&j) {
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
    let first_name = best_chain[0];
    let n_cities = (best_chain.last().unwrap() - first_name) / 92 + 1;

    // units: the region between the 10240 blob and the first city NAME
    // must be 40*N + 34 (proven on ROMFALL/Va_Auto; #49/#58 gate). The
    // 34 is the head of city record 0 (names sit at +34 of each record),
    // so the unit records are exactly 40*N from the blob end.
    let region = first_name - blob10240_end;
    let (unit_count, rem) = (region / 40, region % 40);
    if rem != 34 {
        return Err(SavError::UnitRegionArithmetic { region, rem });
    }
    let mut units = Vec::with_capacity(unit_count);
    let city_offset = blob10240_end + 40 * unit_count;
    for u in 0..unit_count {
        let start = blob10240_end + 40 * u;
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
        cities.push(CityRecord { raw });
    }

    Ok(SaveFile {
        version,
        turn,
        year,
        game_flags,
        human_mask,
        difficulty,
        seed_word,
        rivals,
        wonders,
        civ_names,
        treasury,
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
    /// first byte of unit record 0
    unit_base: usize,
    /// first byte of city record 0 (name at +34)
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
            unit_base: sav.unit_region.0,
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

    /// Move a unit on the logical grid (x u16@0, y u16@2 of its record).
    pub fn set_unit_pos(&mut self, idx: usize, x: u16, y: u16) -> Result<(), SavError> {
        if idx >= self.n_units {
            return Err(SavError::BadEdit("unit index out of range"));
        }
        let base = self.unit_base + 40 * idx;
        self.buf[base..base + 2].copy_from_slice(&x.to_le_bytes());
        self.buf[base + 2..base + 4].copy_from_slice(&y.to_le_bytes());
        Ok(())
    }

    /// Change a unit's type id (Rules @UNITS index) or owner byte.
    pub fn set_unit_type(&mut self, idx: usize, type_id: u8) -> Result<(), SavError> {
        if idx >= self.n_units {
            return Err(SavError::BadEdit("unit index out of range"));
        }
        self.buf[self.unit_base + 40 * idx + 8] = type_id;
        Ok(())
    }

    /// Rename a city (≤15 bytes; the 16-byte slot is NUL-padded).
    pub fn set_city_name(&mut self, idx: usize, name: &str) -> Result<(), SavError> {
        if idx >= self.n_cities {
            return Err(SavError::BadEdit("city index out of range"));
        }
        // encode as Latin-1, the file's encoding (#69): chars above U+00FF
        // have no byte to go to
        let Some(bytes) = name
            .chars()
            .map(|c| u8::try_from(c as u32).ok())
            .collect::<Option<Vec<u8>>>()
        else {
            return Err(SavError::BadEdit("city name is not Latin-1"));
        };
        let bytes = &bytes[..];
        if bytes.len() > 15 {
            return Err(SavError::BadEdit("city name longer than 15 bytes"));
        }
        // Reviewer finding #65: only names the city locator can read back
        // (the parser's scan: 2..15 bytes, printable ASCII or 0xA0..0xFF).
        if bytes.len() < 2 {
            return Err(SavError::BadEdit("city name shorter than 2 bytes"));
        }
        if !bytes
            .iter()
            .all(|&c| (32..127).contains(&c) || (0xA0..=0xFF).contains(&c))
        {
            return Err(SavError::BadEdit("city name has non-printable bytes"));
        }
        let base = self.city_base + 92 * idx + 34;
        let old: [u8; 16] = self.buf[base..base + 16].try_into().unwrap();
        for b in &mut self.buf[base..base + 16] {
            *b = 0;
        }
        self.buf[base..base + bytes.len()].copy_from_slice(bytes);
        // belt and braces: the edited file must still parse to the same
        // layout, or the edit is rolled back
        let same = parse_sav(&self.buf)
            .is_ok_and(|s| s.cities.len() == self.n_cities && s.city_offset == self.city_base);
        if !same {
            self.buf[base..base + 16].copy_from_slice(&old);
            return Err(SavError::BadEdit("rename would break the city layout"));
        }
        Ok(())
    }

    /// Move a city (x u16@0, y u16@2 of its record).
    pub fn set_city_pos(&mut self, idx: usize, x: u16, y: u16) -> Result<(), SavError> {
        if idx >= self.n_cities {
            return Err(SavError::BadEdit("city index out of range"));
        }
        let base = self.city_base + 92 * idx;
        self.buf[base..base + 2].copy_from_slice(&x.to_le_bytes());
        self.buf[base + 2..base + 4].copy_from_slice(&y.to_le_bytes());
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
                                                     // units: 2 × 40 B
        let unit_specs: [(u16, u16, u8, u8); 2] = [(3, 4, 0, 1), (5, 6, 1, 2)];
        for (i, &(x, y, typ, owner)) in unit_specs.iter().enumerate() {
            let mut r = [0u8; 40];
            r[0..2].copy_from_slice(&x.to_le_bytes());
            r[2..4].copy_from_slice(&y.to_le_bytes());
            r[8] = typ;
            r[9] = owner;
            r[30..32].copy_from_slice(&(i as u16 + 1).to_le_bytes());
            out.extend_from_slice(&r);
        }
        // city grid: 8 × 92 with names at +34, NUL padding to +50
        for i in 0..n_cities {
            let mut r = [0u8; 92];
            let name = format!("City{i}");
            r[34..34 + name.len()].copy_from_slice(name.as_bytes());
            r[0..2].copy_from_slice(&((i + 1) as u16).to_le_bytes());
            r[2..4].copy_from_slice(&((i + 2) as u16).to_le_bytes());
            r[32..34].copy_from_slice(&((i + 3) as u16).to_le_bytes());
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

    /// Reviewer finding #64: untrusted header fields and truncation give
    /// errors, never a panic.
    #[test]
    fn malformed_headers_and_truncation_never_panic() {
        let good = synthetic();
        let hdr = OFF_TRANSPORTERS + 172; // transporter count is 0
        for field in 0..8 {
            for v in [-1i16, i16::MIN, i16::MAX, 0] {
                let mut d = good.clone();
                d[hdr + 2 * field..hdr + 2 * field + 2].copy_from_slice(&v.to_le_bytes());
                let r = std::panic::catch_unwind(|| parse_sav(&d));
                assert!(r.is_ok(), "panic: header field {field} = {v}");
            }
        }
        // the exact reviewer reproduction: raw[5] = -1 is refused
        let mut d = good.clone();
        d[hdr + 10..hdr + 12].copy_from_slice(&(-1i16).to_le_bytes());
        assert_eq!(parse_sav(&d), Err(SavError::NoMap));
        // a huge transporter count
        let mut d = good.clone();
        d[OFF_TRANSPORTERS + 168..OFF_TRANSPORTERS + 172].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse_sav(&d).is_err());
        // every truncation length
        for n in 0..good.len() {
            let r = std::panic::catch_unwind(|| parse_sav(&good[..n]));
            assert!(r.is_ok(), "panic at length {n}");
        }
    }

    /// Reviewer finding #69: names are Latin-1 both ways.
    #[test]
    fn city_names_are_latin1() {
        let mut d = synthetic();
        let ed_probe = SavEditor::new(&d).unwrap();
        let base = ed_probe.city_base + 34;
        d[base..base + 8].copy_from_slice(b"Arsino\xeb\0");
        let sav = parse_sav(&d).unwrap();
        assert_eq!(sav.cities[0].name(), "Arsinoë");
        let mut ed = SavEditor::new(&d).unwrap();
        ed.set_city_name(1, "Lutèce").unwrap();
        assert_eq!(&ed.bytes()[base + 92..base + 92 + 7], b"Lut\xe8ce\0");
        assert_eq!(parse_sav(ed.bytes()).unwrap().cities[1].name(), "Lutèce");
        assert!(ed.set_city_name(1, "Αθήνα").is_err(), "not Latin-1");
    }
}
