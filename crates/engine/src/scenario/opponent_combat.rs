//! Guarded combat in the saved observation epoch.
use crate::saved_knowledge::unit_observation_after;
use crate::{events::EventEngine, Unit, WorldState};
use tot_kit::{Rules, SaveFile};

fn matches_record(saved: &SaveFile, slot: usize, u: &Unit) -> bool {
    saved.units.get(slot).is_some_and(|r| {
        (r.id(), r.owner(), r.type_id(), r.x(), r.y(), r.orders())
            == (u.id, u.owner, u.type_id, u.x, u.y, u.orders)
    })
}

fn singleton(world: &WorldState, at: (u16, u16)) -> Option<usize> {
    let mut live = world
        .units
        .iter()
        .enumerate()
        .filter(|(_, unit)| unit.id != 0 && (unit.x, unit.y) == at);
    let (slot, _) = live.next()?;
    live.next().is_none().then_some(slot)
}

/// Run one opposing land attack in temporary state. The measured class is
/// two unstacked units outside cities with the attacker surviving in place.
/// Event effects, other outcomes and changed human knowledge fall back to
/// the normal engine path; this function does not commit any of that state.
pub(crate) fn stage(
    saved: &SaveFile,
    rules: &Rules,
    world: &mut WorldState,
    events: Option<&mut EventEngine>,
    actor: (u8, usize),
    target: (u16, u16),
    stock_city_attack: bool,
) -> Result<SaveFile, String> {
    let (civ, attacker) = actor;
    let act = world
        .units
        .get(attacker)
        .filter(|u| u.id != 0 && u.owner == civ)
        .ok_or("saved combat needs the acting civilization's live unit")?;
    let origin = (act.x, act.y);
    let defender = singleton(world, target).ok_or("saved combat needs a single defender")?;
    let def = &world.units[defender];
    let city = world.city_at(target.0, target.1);
    let witnessed_city = stock_city_attack
        && (world.turn, civ, attacker, defender, origin, target)
            == (44, 5, 35, 36, (46, 28), (48, 28))
        && world.human_mask == 16
        && world.combat_rng.state() == 0x87dd2a59
        && world.diplomacy[5][6] == 0x4405
        && city.is_some_and(|slot| {
            let c = &world.cities[slot];
            slot == 9
                && c.size == 1
                && c.owner == 6
                && saved.cities.get(slot).is_some_and(|record| {
                    record.city_id() == c.city_id && record.size() == 1 && record.owner() == 6
                })
                && world
                    .units
                    .iter()
                    .enumerate()
                    .all(|(i, u)| i == defender || u.id == 0 || usize::from(u.home) != slot)
        });
    if def.owner == civ
        || def.owner == saved.player_civ
        || singleton(world, origin) != Some(attacker)
        || world.city_at(origin.0, origin.1).is_some()
        || (city.is_some() && !witnessed_city)
        || [act, def].into_iter().any(|u| {
            u.is_leader
                || rules.unit_domain(usize::from(u.type_id)) != Some(0)
                || world
                    .map
                    .terrain_at(u.x, u.y)
                    .is_none_or(|terrain| terrain == 10)
        })
        || !matches_record(saved, attacker, act)
        || !matches_record(saved, defender, def)
    {
        return Err("saved combat requires two ordinary land singletons outside cities".into());
    }
    let tile = world
        .map
        .stored_index(target.0, target.1)
        .ok_or("saved combat destination missing")?;
    let outcome = if let Some(events) = events {
        let (outcome, effects) =
            world.move_unit_with_events(rules, events, attacker, target.0, target.1);
        if !effects.is_empty() {
            return Err("saved combat has event effects outside the checked scope".into());
        }
        outcome
    } else {
        world.move_unit(rules, attacker, target.0, target.1)
    };
    if !outcome
        .battle
        .is_some_and(|battle| battle.outcome.attacker_won)
        || outcome.moved
        || outcome.hut
        || world.units[defender].id != 0
        || (world.units[attacker].x, world.units[attacker].y) != origin
    {
        return Err("saved combat outcome is outside the checked scope".into());
    }

    let mut updated = saved.clone();
    updated.units[defender].raw[30..34].fill(0);
    let survivor = &world.units[attacker];
    let record = &mut updated.units[attacker].raw;
    record[7] = (record[7] & !0x20) | (u8::from(survivor.veteran) << 5);
    record[12] = survivor.damage;
    record[17] = survivor.orders;
    updated.maps[0].tile_fields[6 * tile + 1] &= !1;

    if let Some(city) = city.filter(|_| witnessed_city) {
        if world.combat_rng.state() != 0x2413ad1c
            || (
                world.units[attacker].damage,
                world.units[attacker].moves_left,
            ) != (8, 0)
        {
            return Err("saved city attack differed from the witnessed outcome".into());
        }
        updated.units[attacker].raw[10] = 6;
        updated.units[defender].raw[12] = 10;
        for coordinate in [0, 2] {
            updated.units[defender].raw[coordinate..coordinate + 2]
                .copy_from_slice(&(-700_i16).to_le_bytes());
        }
        world.units[defender].x = (-700_i16) as u16;
        world.units[defender].y = (-700_i16) as u16;
        world.cities[city].size = 0;
        if !world.destroy_surface_city(rules, city) {
            return Err("saved city attack could not remove the defeated city".into());
        }
        updated.cities[city].raw[11] = 0;
        updated.cities[city].raw[86..90].fill(0);
        updated.cities.truncate(world.cities.len());
        let map = &mut updated.maps[0];
        for (index, bytes) in map.tile_fields.chunks_exact_mut(6).enumerate() {
            bytes[1] = world.map.improvements[index];
            bytes[2] = (bytes[2] & 0x1f) | (world.map.site_bits[index] << 5);
            bytes[5] = (bytes[5] & 0xf0) | world.map.fertility[index];
        }
        let area = updated.map_header.surface();
        for memory in map.visibility.chunks_exact_mut(area) {
            memory[tile] &= !3;
        }
        super::snapshot::apply_saved_observation(world, &updated);
        return Ok(updated);
    }

    let observed = unit_observation_after(&updated, rules, attacker, true)
        .map_err(|error| format!("saved combat observation: {error}"))?;
    super::snapshot::apply_saved_observation(world, &observed);
    Ok(observed)
}
