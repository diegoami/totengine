//! `.MP` map files.
//!
//! Two layouts exist in the wild (proven 2026-11-01, `docs/evidence/README.md`):
//!
//! * `Tot3ByteSlot` — ToT's own format (`World.mp`): 8-byte header
//!   (`u16 width, u16 height, u32 real_tile_count`), then `width × height` slots
//!   of 3 bytes, then a small tail (90 B on World.mp). Per slot: a little-endian
//!   `u16` plus one extra byte. **Terrain = word & 0x7F**, **river = word & 0x80**
//!   (proven 2026-11-01: bit-0x80 tiles form dendritic chains matching Earth's
//!   rivers — Mississippi, African and Australian networks — all on land; special
//!   resources are not stored in .MP at all), **void tile = extra & 0xF0**.
//!   Verified by the biome-latitude check: World.mp decodes to Earth with 66%
//!   ocean, Glacier at mean latitude 0.081, Jungle at 0.670.
//! * `Classic2Byte` — classic Civ2 MP: 4-byte header, one `u16` per slot,
//!   `0xFFFF` = void.
//! * `TotPairedSlot` — ToTPP-era flavor (proven on `AnticChina.MP`,
//!   2026-11-01): 8-byte header `u16 width, u16 height, u16 real, u16 maps`,
//!   a 90-byte front blob, then per stored tile 6 bytes = a 3-byte data slot
//!   (`u16` word + 1 extra byte) followed by a 3-byte pad slot (word
//!   `0x0001`/`0x003F`, extra `0xF0|bits`), then one flag byte per tile. The
//!   stored grid is dense `(width/2) × height`; `width` is the display width
//!   (ToT tiles span 2 columns). Word: terrain = `word & 0x3F`, special
//!   resource = `0x40` (stored in this flavor, unlike stock maps), river =
//!   `0x80`, improvement flag = `0x3000` (exact meaning open).
//!
//! Open: the meaning of the pad-slot low bits, the per-tile flag byte
//! (mostly 0; values 1/2 observed) and the 90-byte front blob.
//!
//! Open (T0-8): `AnticChina.MP` from a ToTPP-era tool fits neither layout.

use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpFormat {
    /// ToT: 8-byte header + 3 bytes per slot.
    Tot3ByteSlot,
    /// ToTPP-era: 6 bytes per stored tile (data slot + pad slot) + flags.
    TotPairedSlot,
    /// Classic Civ2: 4-byte header + 2 bytes per slot.
    Classic2Byte,
}

#[derive(Debug, Clone)]
pub struct MpMap {
    pub format: MpFormat,
    pub width: u16,
    pub height: u16,
    /// Terrain word per tile (`word & 0x3F` is the terrain id; `0x40` the
    /// special-resource bit where stored, `0x80` the river bit).
    pub tiles: Vec<u16>,
    /// The third byte per tile (ToT layouts; void when `extra & 0xF0 != 0`
    /// in `Tot3ByteSlot`).
    pub extra: Option<Vec<u8>>,
    /// The header's real-tile count.
    pub declared_real_tiles: Option<u32>,
    /// `TotPairedSlot`: the pad slot's word per tile.
    pub pad_words: Option<Vec<u16>>,
    /// `TotPairedSlot`: the trailing one-byte-per-tile flags.
    pub per_tile_flags: Option<Vec<u8>>,
    /// `TotPairedSlot`: the header's display width (2 x stored width).
    pub display_width: Option<u16>,
    pub tail_bytes: usize,
}

impl MpMap {
    /// Terrain id of a tile (`word & 0x3F`), or `None` for void slots
    /// (`Tot3ByteSlot` only; the paired layout is a dense grid).
    pub fn terrain(&self, index: usize) -> Option<u8> {
        let void = match &self.extra {
            Some(extra) if self.format == MpFormat::Tot3ByteSlot => extra[index] & 0xF0 != 0,
            _ => self.tiles[index] == 0xFFFF,
        };
        if void {
            None
        } else {
            Some((self.tiles[index] & 0x3F) as u8)
        }
    }

    /// River flag of a tile (bit 0x80 of the terrain word; proven on
    /// World.mp — see the module docs).
    pub fn is_river(&self, index: usize) -> bool {
        self.tiles[index] & 0x80 != 0
    }

    /// Special-resource flag of a tile (bit 0x40; stored in the ToTPP-era
    /// paired layout, never set in stock 3-byte-slot maps).
    pub fn is_special(&self, index: usize) -> bool {
        self.tiles[index] & 0x40 != 0
    }

    /// Improvement flag (bits 0x3000, paired layout only; exact meaning open).
    pub fn has_improvement_flag(&self, index: usize) -> bool {
        self.tiles[index] & 0x3000 != 0
    }

    /// Void tiles excluded.
    pub fn real_tile_count(&self) -> usize {
        (0..self.tiles.len())
            .filter(|i| self.terrain(*i).is_some())
            .count()
    }
}

/// Parse an `.MP` file's bytes. Errors carry enough context to file a finding.
pub fn parse_mp(data: &[u8]) -> Result<MpMap, String> {
    if data.len() < 8 {
        return Err("file too short for any .MP layout".into());
    }
    let width = u16::from_le_bytes([data[0], data[1]]) as usize;
    let height = u16::from_le_bytes([data[2], data[3]]) as usize;
    if width == 0 || height == 0 || width > 8192 || height > 8192 {
        return Err(format!("implausible dimensions {width}x{height}"));
    }
    let slots = width * height;

    // ToTPP-era paired layout first (exact size match): u16 real count +
    // u16 maps at 4, 90-byte front blob, 6 bytes per stored tile (data slot
    // + pad slot), 1 flag byte per tile. Proven on AnticChina.MP.
    let (real16, maps) = (
        u16::from_le_bytes([data[4], data[5]]) as usize,
        u16::from_le_bytes([data[6], data[7]]) as usize,
    );
    if maps == 1
        && width.is_multiple_of(2)
        && (width / 2) * height == real16
        && data.len() == 98 + 7 * real16
    {
        let count = real16;
        let mut tiles = Vec::with_capacity(count);
        let mut extra = Vec::with_capacity(count);
        let mut pad = Vec::with_capacity(count);
        for i in 0..count {
            let o = 98 + 6 * i;
            tiles.push(u16::from_le_bytes([data[o], data[o + 1]]));
            extra.push(data[o + 2]);
            pad.push(u16::from_le_bytes([data[o + 3], data[o + 4]]));
        }
        let flags = data[98 + 6 * count..98 + 7 * count].to_vec();
        return Ok(MpMap {
            format: MpFormat::TotPairedSlot,
            width: (width / 2) as u16,
            height: height as u16,
            tiles,
            extra: Some(extra),
            declared_real_tiles: Some(real16 as u32),
            pad_words: Some(pad),
            per_tile_flags: Some(flags),
            display_width: Some(width as u16),
            tail_bytes: 0,
        });
    }

    // ToT: u32 real-tile count at 4, then 3 bytes per slot, small tail allowed.
    let real = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let body = data.len() - 8;
    if body >= slots * 3 && body - slots * 3 <= 256 {
        let mut tiles = Vec::with_capacity(slots);
        let mut extra = Vec::with_capacity(slots);
        for i in 0..slots {
            let o = 8 + 3 * i;
            tiles.push(u16::from_le_bytes([data[o], data[o + 1]]));
            extra.push(data[o + 2]);
        }
        return Ok(MpMap {
            format: MpFormat::Tot3ByteSlot,
            width: width as u16,
            height: height as u16,
            tiles,
            extra: Some(extra),
            declared_real_tiles: Some(real),
            pad_words: None,
            per_tile_flags: None,
            display_width: None,
            tail_bytes: body - slots * 3,
        });
    }

    // Classic: 4-byte header, one u16 per slot, exact size.
    if data.len() >= 4 && data.len() - 4 == 2 * slots {
        let mut tiles = Vec::with_capacity(slots);
        for i in 0..slots {
            let o = 4 + 2 * i;
            tiles.push(u16::from_le_bytes([data[o], data[o + 1]]));
        }
        return Ok(MpMap {
            format: MpFormat::Classic2Byte,
            width: width as u16,
            height: height as u16,
            tiles,
            extra: None,
            declared_real_tiles: None,
            pad_words: None,
            per_tile_flags: None,
            display_width: None,
            tail_bytes: 0,
        });
    }

    Err(format!(
        "no known layout fits: tried tot_3byte_slot (8 + 3*{width}*{height}), \\\n         \
         tot_paired_slot (98 + 7*real) and classic_2byte (4 + 2*{width}*{height}); \\\n         \
         the file is {} bytes",
        data.len()
    ))
}

pub fn parse_mp_file(path: impl AsRef<Path>) -> Result<MpMap, String> {
    parse_mp(&fs::read(path).map_err(|e| e.to_string())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic ToT-layout map: 4x2, terrain 10 (ocean), one void slot, one
    /// special-resource slot. Synthetic bytes — no game data in-repo.
    fn synthetic_tot_mp() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&4u16.to_le_bytes());
        v.extend_from_slice(&2u16.to_le_bytes());
        v.extend_from_slice(&7u32.to_le_bytes());
        let slots: [(u16, u8); 8] = [
            (10, 0x00),
            (10, 0x00),
            (0x82, 0x00), // terrain 2 + river bit
            (3, 0x00),
            (4, 0x00),
            (10, 0xF0), // void
            (10, 0x00),
            (9, 0x00),
        ];
        for (word, extra) in slots {
            v.extend_from_slice(&word.to_le_bytes());
            v.push(extra);
        }
        v
    }

    #[test]
    fn parses_tot_layout_flags_and_counts() {
        let map = parse_mp(&synthetic_tot_mp()).expect("parses");
        assert_eq!(map.format, MpFormat::Tot3ByteSlot);
        assert_eq!((map.width, map.height), (4, 2));
        assert_eq!(map.declared_real_tiles, Some(7));
        assert_eq!(map.terrain(0), Some(10));
        assert_eq!(map.terrain(5), None, "0xF0 extra byte marks void");
        assert_eq!(map.terrain(2), Some(2));
        assert!(map.is_river(2));
        assert!(!map.is_river(0));
        assert_eq!(map.real_tile_count(), 7, "eight slots minus one void");
        assert_eq!(map.tail_bytes, 0);
    }

    #[test]
    fn parses_paired_slot_layout() {
        // Synthetic AnticChina.MP-shaped map: 8x2 display -> 4x2 stored grid,
        // 8 real tiles. No game data.
        let mut v = Vec::new();
        v.extend_from_slice(&8u16.to_le_bytes()); // display width
        v.extend_from_slice(&2u16.to_le_bytes()); // height
        v.extend_from_slice(&8u16.to_le_bytes()); // real tiles
        v.extend_from_slice(&1u16.to_le_bytes()); // maps
        v.extend_from_slice(&[0xAB; 90]); // front blob
        let words = [0x02u16, 0x8A, 0x4A, 0x3003, 0x01, 0x03, 0x09, 0x81];
        for w in words {
            v.extend_from_slice(&w.to_le_bytes());
            v.push(0x00); // data-slot extra byte
            v.extend_from_slice(&0x003Fu16.to_le_bytes()); // pad slot word
            v.push(0xF0); // pad slot extra byte
        }
        v.extend_from_slice(&[0u8, 1, 0, 2, 0, 0, 0, 0]); // per-tile flags
        let map = parse_mp(&v).expect("parses");
        assert_eq!(map.format, MpFormat::TotPairedSlot);
        assert_eq!((map.width, map.height), (4, 2)); // stored dims
        assert_eq!(map.display_width, Some(8));
        assert_eq!(map.declared_real_tiles, Some(8));
        assert_eq!(map.terrain(0), Some(2));
        assert!(map.is_river(1) && map.terrain(1) == Some(10)); // river ocean
        assert!(map.is_special(2) && map.terrain(2) == Some(10)); // 0x4A
        assert!(map.has_improvement_flag(3) && map.terrain(3) == Some(3));
        assert!(map.is_river(7) && map.terrain(7) == Some(1)); // 0x81
        assert_eq!(map.pad_words.as_ref().map(|p| p.len()), Some(8));
        assert_eq!(
            map.per_tile_flags.as_deref(),
            Some(&[0u8, 1, 0, 2, 0, 0, 0, 0][..])
        );
        // dense grid: every tile is real
        assert_eq!(map.real_tile_count(), 8);
    }

    #[test]
    fn rejects_file_fitting_no_layout() {
        let mut v = Vec::new();
        v.extend_from_slice(&224u16.to_le_bytes());
        v.extend_from_slice(&250u16.to_le_bytes());
        v.extend_from_slice(&[0x60, 0x6d, 0x01, 0x00]); // paired-shaped header
        v.resize(123_457, 0xAB); // wrong size for every known layout
        let err = parse_mp(&v).expect_err("should not fit any layout");
        assert!(err.contains("no known layout"), "got: {err}");
    }
}
