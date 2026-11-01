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
//!
//! Open (T0-8): `AnticChina.MP` from a ToTPP-era tool fits neither layout.

use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpFormat {
    /// ToT: 8-byte header + 3 bytes per slot.
    Tot3ByteSlot,
    /// Classic Civ2: 4-byte header + 2 bytes per slot.
    Classic2Byte,
}

#[derive(Debug, Clone)]
pub struct MpMap {
    pub format: MpFormat,
    pub width: u16,
    pub height: u16,
    /// Terrain word per slot (`word & 0x7F` is the base terrain id).
    pub tiles: Vec<u16>,
    /// The third byte per slot (ToT layout; void when `extra & 0xF0 != 0`).
    pub extra: Option<Vec<u8>>,
    /// The header's real-tile count (ToT layout only).
    pub declared_real_tiles: Option<u32>,
    pub tail_bytes: usize,
}

impl MpMap {
    /// Base terrain id of a slot (`word & 0x7F`), or `None` for void slots.
    pub fn terrain(&self, index: usize) -> Option<u8> {
        let void = match &self.extra {
            Some(extra) => extra[index] & 0xF0 != 0,
            None => self.tiles[index] == 0xFFFF,
        };
        if void {
            None
        } else {
            Some((self.tiles[index] & 0x7F) as u8)
        }
    }

    /// River flag of a slot (bit 0x80 of the terrain word; proven on
    /// World.mp — see the module docs).
    pub fn is_river(&self, index: usize) -> bool {
        self.tiles[index] & 0x80 != 0
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
            tail_bytes: 0,
        });
    }

    Err(format!(
        "no known layout fits: tried tot_3byte_slot (8 + 3*{width}*{height}) and \
         classic_2byte (4 + 2*{width}*{height}); file is {} bytes — the \
         AnticChina.MP variant is tracked as T0-8",
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
    fn rejects_unknown_layout_like_anticchina() {
        let mut v = Vec::new();
        v.extend_from_slice(&224u16.to_le_bytes()); // plausible w
        v.extend_from_slice(&250u16.to_le_bytes()); // plausible h
        v.extend_from_slice(&[0x60, 0x6d, 0x01, 0x00]); // ToTPP-era header
        v.resize(196_098, 0xAB); // matches AnticChina.MP's size: fits neither
        let err = parse_mp(&v).expect_err("should not fit any layout");
        assert!(
            err.contains("T0-8"),
            "error should point at the open story: {err}"
        );
    }
}
