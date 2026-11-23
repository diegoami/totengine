//! Scoped opponent movement between unseen land and friendly city stacks.
use crate::movement::MovementRoute;
use crate::player::OrderResult;
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

/// Execute ordinary land movement and update the affected stack links.
/// Relink both stacks in the checked order. The caller can discard the
/// temporary world on error.
pub(crate) fn stage(
    saved: &SaveFile,
    rules: &Rules,
    world: &mut WorldState,
    civ: u8,
    slot: usize,
    to: (u16, u16),
    field_stack: bool,
) -> Result<(SaveFile, OrderResult), String> {
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
    let source = stack(world, origin)?;
    let mut members = stack(world, to)?;
    let city_at = |at| {
        world
            .cities
            .iter()
            .find(|c| c.city_id != 0 && (c.x, c.y) == at)
    };
    let friendly_city = |city: Option<&crate::City>| {
        city.is_some_and(|c| c.map_id == 0 && c.owner == u16::from(civ))
    };
    let city = city_at(to);
    if members.is_empty() {
        if city.is_some() {
            return Err(
                "saved opponent step into an empty city is outside the checked context".into(),
            );
        }
    } else if members.iter().any(|&s| world.units[s].owner != civ)
        || !(friendly_city(city) || field_stack && city.is_none())
    {
        return Err("saved opponent step needs empty land or a friendly city stack".into());
    }
    let origin_city = city_at(origin);
    let city_transfer = field_stack
        && source.len() == 2
        && members.len() == 1
        && (origin_city.is_none() && friendly_city(city)
            || friendly_city(origin_city) && city.is_none());
    let field_departure = field_stack
        && source.len() == 2
        && members.is_empty()
        && origin_city.is_none()
        && city.is_none();
    let city_to_empty = friendly_city(origin_city) && members.is_empty();
    if source.len() > 1
        && (!(city_transfer || city_to_empty || field_departure)
            || source.iter().any(|&s| {
                world.units[s].owner != civ
                    || saved.units.get(s).is_none_or(|u| u.visibility() != 0)
            }))
    {
        return Err(
            "saved opponent departure is outside the checked friendly stack contexts".into(),
        );
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
    let field_to_field =
        field_stack && !members.is_empty() && city.is_none() && origin_city.is_none();
    let attack = rules.unit_attack(usize::from(unit.type_id));
    if field_to_field && attack != Some(0) || city_transfer && attack.is_none() {
        return Err("saved field-stack arrival has an unchecked mover attack value".into());
    }
    // In a checked city transfer the city connects to the route on the field tile.
    let route = if city_transfer {
        let field = if origin_city.is_none() { from } else { dest };
        Some(MovementRoute {
            road: world.map.road[field],
            rail: world.map.rail[field],
        })
    } else {
        (field_to_field || field_departure).then(|| MovementRoute {
            road: world.map.road[from] && world.map.road[dest],
            rail: world.map.rail[from] && world.map.rail[dest],
        })
    };
    let spent = field_departure.then(|| {
        world
            .movement_allowance(rules, usize::from(unit.type_id), civ, unit.damage)
            .saturating_sub(unit.moves_left)
    });
    let result = world.move_unit_with_routes(rules, slot, to.0, to.1, route, field_departure);
    if result.battle.is_some() || result.hut || result.zoc_ended {
        return Err("saved stack move did not resolve as ordinary movement".into());
    }
    let mut updated = if result.moved {
        clear_unit_stack_visibility(saved, slot)?
    } else if field_departure && result.cost_thirds > 0 {
        saved.clone()
    } else {
        return Err("saved stack move was refused".into());
    };
    if let Some(spent) = spent {
        updated.units[slot].raw[10] = (spent + result.cost_thirds)
            .try_into()
            .map_err(|_| "saved movement expenditure out of range")?;
    }
    if !result.moved {
        return Ok((updated, OrderResult::MovementExhausted));
    }
    if city_transfer && attack.is_some_and(|value| value != 0) {
        members.insert(0, slot);
    } else {
        members.push(slot);
    }
    for (position, &member) in members.iter().enumerate() {
        world.units[member].stack_pos = position as u16;
    }
    let moved = &mut updated.units[slot].raw;
    moved[..2].copy_from_slice(&to.0.to_le_bytes());
    moved[2..4].copy_from_slice(&to.1.to_le_bytes());
    let remaining: Vec<_> = source
        .into_iter()
        .filter(|&candidate| candidate != slot)
        .collect();
    for (position, &unit) in remaining.iter().enumerate() {
        world.units[unit].stack_pos = position as u16;
    }
    link_stack(&mut updated, world, &remaining)?;
    let tile_bytes = &mut updated.maps[0].tile_fields;
    if remaining.is_empty() {
        tile_bytes[6 * from + 1] &= !1;
    }
    tile_bytes[6 * dest + 1] |= 1;
    tile_bytes[6 * dest + 5] = (tile_bytes[6 * dest + 5] & 15) | (civ << 4);
    link_stack(&mut updated, world, &members)?;
    let observed = unit_observation_after(&updated, rules, slot, true)
        .map_err(|error| format!("saved opponent stack observation: {error}"))?;
    super::snapshot::apply_saved_observation(world, &observed);
    Ok((observed, OrderResult::Moved))
}
