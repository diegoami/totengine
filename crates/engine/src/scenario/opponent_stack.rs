//! Scoped opponent movement from an unseen unstacked tile.
use crate::saved_knowledge::{clear_unit_stack_visibility, unit_observation_after};
use crate::WorldState;
use std::collections::BTreeSet;
use tot_kit::{Rules, SaveFile, UnitRecord};

fn stack(world: &WorldState, at: (u16, u16)) -> Result<Vec<usize>, String> {
    let mut members: Vec<_> = world
        .units
        .iter()
        .enumerate()
        .filter(|(_, unit)| unit.id != 0 && (unit.x, unit.y) == at)
        .map(|(slot, unit)| (unit.stack_pos, slot))
        .collect();
    members.sort_unstable();
    if members.iter().enumerate().any(|(order, (position, slot))| {
        order != usize::from(*position) || *slot > i16::MAX as usize
    }) {
        return Err("saved stack requires consecutive positions".into());
    }
    Ok(members.into_iter().map(|(_, slot)| slot).collect())
}

fn link_stack(save: &mut SaveFile, world: &WorldState, slots: &[usize]) -> Result<(), String> {
    for (i, &slot) in slots.iter().enumerate() {
        let record = save.units.get_mut(slot).ok_or("saved stack slot missing")?;
        let unit = &world.units[slot];
        if (
            record.id(),
            record.owner(),
            record.type_id(),
            record.x(),
            record.y(),
        ) != (unit.id, unit.owner, unit.type_id, unit.x, unit.y)
        {
            return Err("saved stack record differs from engine unit".into());
        }
        let previous = i.checked_sub(1).map_or(-1, |j| slots[j] as i16);
        let next = slots.get(i + 1).map_or(-1, |&j| j as i16);
        record.raw[26..28].copy_from_slice(&previous.to_le_bytes());
        record.raw[28..30].copy_from_slice(&next.to_le_bytes());
    }
    Ok(())
}

/// Extend the private observation snapshot with births already verified to
/// be in unseen processed cities. New records start with a clear unit mask.
pub(crate) fn record_city_births(
    saved: &mut SaveFile,
    world: &mut WorldState,
    births: &[(usize, usize)],
) -> Result<(), String> {
    if births.is_empty() {
        return Ok(());
    }
    saved
        .units
        .resize_with(world.units.len(), || UnitRecord { raw: [0; 40] });
    let mut tiles = BTreeSet::new();
    for &(slot, tile) in births {
        let unit = &world.units[slot];
        if saved.units[slot].id() != 0 || unit.is_leader || unit.orders != 255 {
            return Err(
                "saved city birth requires an awake ordinary unit in an unused slot".into(),
            );
        }
        let raw = &mut saved.units[slot].raw;
        *raw = [0; 40];
        raw[..2].copy_from_slice(&unit.x.to_le_bytes());
        raw[2..4].copy_from_slice(&unit.y.to_le_bytes());
        raw[8] = unit.type_id;
        raw[9] = unit.owner;
        raw[12] = unit.damage;
        raw[17] = unit.orders;
        raw[18] = unit.home;
        raw[30..34].copy_from_slice(&unit.id.to_le_bytes());
        saved.maps[0].tile_fields[6 * tile + 1] |= 1;
        world.map.improvements[tile] |= 1;
        tiles.insert((unit.x, unit.y));
    }
    for at in tiles {
        link_stack(saved, world, &stack(world, at)?)?;
    }
    Ok(())
}

/// Execute ordinary land movement to empty land or a same-owner city
/// stack, appending the mover when a stack is present. The supplied world is temporary and can be discarded on error.
pub(crate) fn stage(
    saved: &SaveFile,
    rules: &Rules,
    world: &mut WorldState,
    civ: u8,
    slot: usize,
    to: (u16, u16),
) -> Result<SaveFile, String> {
    let unit = world
        .units
        .get(slot)
        .filter(|u| {
            u.id != 0
                && u.owner == civ
                && !u.is_leader
                && rules.unit_domain(usize::from(u.type_id)) == Some(0)
        })
        .ok_or("saved stack move requires an ordinary opponent land unit")?;
    let record = saved.units.get(slot).ok_or("saved opponent unit missing")?;
    if (record.id(), record.owner(), record.x(), record.y())
        != (unit.id, unit.owner, unit.x, unit.y)
    {
        return Err("saved opponent unit differs from engine state".into());
    }
    let origin = (unit.x, unit.y);
    if stack(world, origin)? != [slot] {
        return Err("saved stack move requires a singleton at origin".into());
    }
    let members = stack(world, to)?;
    let city = world
        .cities
        .iter()
        .find(|c| c.city_id != 0 && (c.x, c.y) == to);
    if members.is_empty() {
        if city.is_some() {
            return Err(
                "saved opponent step into an empty city is outside the checked context".into(),
            );
        }
    } else if members.iter().any(|&s| world.units[s].owner != civ)
        || !city.is_some_and(|c| c.map_id == 0 && c.owner == u16::from(civ))
    {
        return Err("saved opponent step needs empty land or a friendly city stack".into());
    }

    let from = world
        .map
        .stored_index(origin.0, origin.1)
        .ok_or("saved stack origin missing")?;
    let dest = world
        .map
        .stored_index(to.0, to.1)
        .ok_or("saved stack destination missing")?;
    if world.map.hut_at(to.0, to.1) {
        return Err("saved stack move cannot enter a hut".into());
    }
    let mut updated = clear_unit_stack_visibility(saved, slot)?;
    let result = world.move_unit(rules, slot, to.0, to.1);
    if !result.moved || result.battle.is_some() || result.hut || result.zoc_ended {
        return Err("saved stack move did not resolve as ordinary movement".into());
    }
    world.units[slot].stack_pos = members.len() as u16;
    let moved = &mut updated.units[slot].raw;
    moved[..2].copy_from_slice(&to.0.to_le_bytes());
    moved[2..4].copy_from_slice(&to.1.to_le_bytes());
    let tile_bytes = &mut updated.maps[0].tile_fields;
    tile_bytes[6 * from + 1] &= !1;
    tile_bytes[6 * dest + 1] |= 1;
    tile_bytes[6 * dest + 5] = (tile_bytes[6 * dest + 5] & 15) | (civ << 4);
    link_stack(&mut updated, world, &stack(world, to)?)?;
    let observed = unit_observation_after(&updated, rules, slot, true)
        .map_err(|error| format!("saved opponent stack observation: {error}"))?;
    super::snapshot::apply_saved_observation(world, &observed);
    Ok(observed)
}
