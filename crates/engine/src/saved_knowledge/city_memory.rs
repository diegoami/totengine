//! Native remembered-size writes after one surviving city's processing.
use super::surface;
use crate::WorldState;
use tot_kit::SaveFile;

fn context(id: u32, map: u16, owner: u16, size: u16) -> Result<(usize, u8), String> {
    if id == 0 || map != 0 || owner > 7 || !(1..128).contains(&size) {
        return Err(
            "city memory refresh requires a surviving surface city, owner 0..7 and size 1..127"
                .into(),
        );
    }
    Ok((owner as usize, size as u8))
}

fn refreshed(mut sizes: [u8; 8], current: u8, relations: &[u32; 8], apollo: i16) -> [u8; 8] {
    for (viewer, size) in sizes.iter_mut().enumerate().skip(1) {
        // 0x470c0e tests the raw word against -1, unlike creation's >=0
        // gate. In particular -2 also refreshes sizes here.
        if relations[viewer] & 8 != 0 || apollo != -1 {
            *size = current;
        }
    }
    sizes
}

/// Original 0x470bd1..0x470c4e: refresh remembered sizes after processing a
/// surviving city. This changes neither discovery masks nor tile knowledge.
/// The caller supplies the coherent post-processing snapshot; growth,
/// production, deletion/capture timing and whole-phase scheduling are separate.
pub fn city_phase_memory_after(save: &SaveFile, slot: usize) -> Result<SaveFile, String> {
    surface(save)?;
    if save.multiplayer != 0 {
        return Err("city memory refresh requires single player".into());
    }
    let city = save
        .cities
        .get(slot)
        .ok_or("city memory refresh slot is out of range")?;
    let map = u16::from_le_bytes(city.raw[4..6].try_into().unwrap());
    let (owner, current) = context(city.city_id(), map, city.owner(), city.size())?;
    let sizes = refreshed(
        city.raw[15..23].try_into().unwrap(),
        current,
        &save.diplomacy[owner],
        save.wonders[25],
    );
    let mut out = save.clone();
    out.cities[slot].raw[15..23].copy_from_slice(&sizes);
    Ok(out)
}

impl WorldState {
    /// Explicit surface city-memory kernel for a future versioned import
    /// policy. run_turn/run_city_phase do not call it: legacy/generated
    /// checksums, checkpoints and observation behavior remain unchanged.
    pub fn refresh_city_phase_memory(&mut self, slot: usize) -> Result<(), String> {
        let city = self
            .cities
            .get(slot)
            .ok_or("city memory refresh slot is out of range")?;
        let (owner, current) = context(city.city_id, city.map_id, city.owner, city.size)?;
        let sizes = refreshed(
            city.known_sizes,
            current,
            &self.diplomacy[owner],
            self.wonders[25],
        );
        self.cities[slot].known_sizes = sizes;
        Ok(())
    }
}
