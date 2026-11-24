//! Classic saved-map knowledge projection and bounded update kernels (#197).
//!
//! Native reader gates/size selection and unit exploration are re-derived by
//! tools/import_knowledge_check.py. This does not implement the native loader,
//! complete contact/invisibility updates, or faithful scenario admission.
use tot_kit::{MapBlock, SaveFile};

mod city_memory;
pub use city_memory::city_phase_memory_after;
mod update;
pub(crate) use update::unit_observation_with_contacts;
pub use update::{
    clear_unit_stack_visibility, unit_knowledge_after, unit_observation_after,
    unit_observation_after_reset, UnitObservationError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedTile {
    /// Current byte +0, as read by the terrain renderer. Keep its opaque
    /// flags without inferring new special/resource semantics.
    pub terrain_byte: u8,
    /// The viewer's saved byte, including occupancy bits. Unit/city records
    /// must pass their own gates; this byte must not create ghost entities.
    pub improvements: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedUnit {
    pub slot: usize,
    pub owner: u8,
    pub type_id: u8,
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedCity {
    pub slot: usize,
    pub name: String,
    pub owner: u8,
    pub x: u16,
    pub y: u16,
    pub size: u8,
}

/// Normal saved-map information for one civ, without engine sight refresh.
/// No foreign production, treasury or other full-save private fields enter it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedMapKnowledge {
    pub civ: u8,
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Option<SavedTile>>,
    pub units: Vec<SavedUnit>,
    pub cities: Vec<SavedCity>,
}

fn surface(save: &SaveFile) -> Result<&MapBlock, String> {
    if save.version != 0x32 || save.map_header.map_count() != 1 || save.maps.len() != 1 {
        return Err("saved-map knowledge requires classic 0x0032 with one surface map".into());
    }
    let n = save.map_header.surface();
    let map = &save.maps[0];
    if !save.map_header.decodable()
        || save.map_header.raw[0] % 2 != 0
        || map.tile_fields.len() != 6 * n
        || map.visibility.len() != 7 * n
    {
        return Err("saved-map knowledge has inconsistent map extents".into());
    }
    Ok(map)
}

/// Normal renderer fragment 0x4664b9: explored, then own or known nonzero size.
pub fn city_shown(civ: u8, owner: u8, explored: bool, known: u8, size: u8) -> bool {
    (1..8).contains(&civ) && explored && (owner == civ || (known & (1 << civ) != 0 && size != 0))
}

/// Normal renderer fragment 0x466ab8: explored, then own or saved visibility.
pub fn unit_shown(civ: u8, owner: u8, explored: bool, visible: u8) -> bool {
    (1..8).contains(&civ) && explored && (owner == civ || visible & (1 << civ) != 0)
}

pub fn displayed_city_size(civ: u8, owner: u8, current: u8, remembered: u8) -> u8 {
    if civ == owner {
        current
    } else {
        remembered
    }
}

impl SavedMapKnowledge {
    /// Tiles the current engine sight policy would add to this snapshot.
    /// A zero count is not complete knowledge/continuation admission.
    pub fn engine_sight_additions(&self, world: &crate::WorldState) -> Result<Vec<usize>, String> {
        if !(1..8).contains(&self.civ)
            || self.width != world.map.stored_w
            || self.height != world.map.stored_h
            || self.tiles.len() != world.map.terrain.len()
        {
            return Err("saved-map projection and engine map context differ".into());
        }
        Ok(world
            .visible_indices(self.civ)
            .into_iter()
            .filter(|&i| self.tiles[i].is_none())
            .collect())
    }

    /// Project stored fields only. Flagged owners, other maps, invalid live
    /// coordinates and signed display-size contexts are rejected explicitly.
    pub fn read(save: &SaveFile, civ: u8) -> Result<Self, String> {
        if !(1..8).contains(&civ) {
            return Err("saved-map viewer must be a civilization from 1 to 7".into());
        }
        let map = surface(save)?;
        let n = save.map_header.surface();
        let bit = 1 << civ;
        let remembered = &map.visibility[(civ as usize - 1) * n..civ as usize * n];
        let tiles: Vec<_> = (0..n)
            .map(|i| {
                (map.visible_to(i) & bit != 0).then_some(SavedTile {
                    terrain_byte: map.tile_fields[6 * i],
                    improvements: remembered[i],
                })
            })
            .collect();
        let index = |x: u16, y: u16| {
            save.map_header
                .stored_index(usize::from(x), usize::from(y))
                .filter(|&i| i < n && (u32::from(x) + u32::from(y)) % 2 == 0)
                .ok_or_else(|| "saved-map live record has invalid coordinates".to_string())
        };
        let mut units = Vec::new();
        for (slot, u) in save.units.iter().enumerate().filter(|(_, u)| u.id() != 0) {
            if u.map_id() != 0 || u.owner() > 7 {
                return Err("saved-map unit has an unsupported map or flagged owner".into());
            }
            let i = index(u.x(), u.y())?;
            if !map.has_unit(i) {
                return Err("saved-map live unit has no occupancy flag at its coordinates".into());
            }
            if unit_shown(civ, u.owner(), tiles[i].is_some(), u.visibility()) {
                units.push(SavedUnit {
                    slot,
                    owner: u.owner(),
                    type_id: u.type_id(),
                    x: u.x(),
                    y: u.y(),
                });
            }
        }
        let mut cities = Vec::new();
        for (slot, c) in save
            .cities
            .iter()
            .enumerate()
            .filter(|(_, c)| c.city_id() != 0)
        {
            if c.map_id() != 0 || c.owner() > 7 {
                return Err("saved-map city has an unsupported map or owner".into());
            }
            let i = index(c.x(), c.y())?;
            if map.improvements(i) & 0x82 != 2 {
                return Err("saved-map live city has an unsupported city-tile context".into());
            }
            let owner = c.owner() as u8;
            let memory = c.known_size(civ).unwrap();
            if city_shown(civ, owner, tiles[i].is_some(), c.known_to(), memory) {
                let size = displayed_city_size(civ, owner, c.size() as u8, memory);
                if size > 127 {
                    return Err(
                        "saved-map city size exceeds the bounded signed display range".into(),
                    );
                }
                cities.push(SavedCity {
                    slot,
                    name: c.name(),
                    owner,
                    x: c.x(),
                    y: c.y(),
                    size,
                });
            }
        }
        Ok(Self {
            civ,
            width: save.map_header.stored_w(),
            height: save.map_header.stored_h(),
            tiles,
            units,
            cities,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Scout {
    pub x: u16,
    pub y: u16,
    pub domain: u8,
    pub extended: bool,
}

/// Re-derived unit exploration kernel (0x41f8a7..0x41fa2a), without contact,
/// entity visibility or runtime activation. Original numeric vectors cover
/// bounds/wrapping, all four domain values and the extended-vision flag.
pub fn explored_by_unit(save: &SaveFile, scout: Scout) -> Result<Vec<usize>, String> {
    let map = surface(save)?;
    let (w, h) = save.map_header.logical_dims();
    if scout.domain > 3 || usize::from(scout.x) >= w || usize::from(scout.y) >= h {
        return Err("unsupported saved-map scout context".into());
    }
    if (u32::from(scout.x) + u32::from(scout.y)) % 2 != 0 {
        return Err("unsupported saved-map scout coordinate parity".into());
    }
    let mut indices: Vec<_> = nearby(save, scout.x, scout.y)?
        .into_iter()
        .filter(|&(k, i)| {
            k < 8
                || k == 20
                || (scout.extended
                    && (scout.domain == 1 || (map.terrain(i) == 10) == (scout.domain == 2)))
        })
        .map(|(_, i)| i)
        .collect();
    indices.sort_unstable();
    indices.dedup();
    Ok(indices)
}

fn nearby(save: &SaveFile, x: u16, y: u16) -> Result<Vec<(usize, usize)>, String> {
    surface(save)?;
    let (w, _) = save.map_header.logical_dims();
    const OFFSETS: [(i32, i32); 25] = [
        (1, -1),
        (2, 0),
        (1, 1),
        (0, 2),
        (-1, 1),
        (-2, 0),
        (-1, -1),
        (0, -2),
        (2, -2),
        (2, 2),
        (-2, 2),
        (-2, -2),
        (1, -3),
        (3, -1),
        (3, 1),
        (1, 3),
        (-1, 3),
        (-3, 1),
        (-3, -1),
        (-1, -3),
        (0, 0),
        (4, 0),
        (-4, 0),
        (0, 4),
        (0, -4),
    ];
    let flat = save.game_flags & 0x8000 != 0;
    let mut indices = Vec::new();
    for (k, (dx, dy)) in OFFSETS.into_iter().enumerate() {
        let mut x = i32::from(x) + dx;
        let y = i32::from(y) + dy;
        if !flat {
            // Native 0x566ae2 performs one correction, not repeated modulo.
            if x < 0 {
                x += w as i32;
            } else if x >= w as i32 {
                x -= w as i32;
            }
        }
        if x < 0 || y < 0 {
            continue;
        }
        let Some(i) = save.map_header.stored_index(x as usize, y as usize) else {
            continue;
        };
        indices.push((k, i));
    }
    Ok(indices)
}
