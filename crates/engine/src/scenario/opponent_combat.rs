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
    civ: u8,
    attacker: usize,
    target: (u16, u16),
) -> Result<SaveFile, String> {
    let act = world
        .units
        .get(attacker)
        .filter(|u| u.id != 0 && u.owner == civ)
        .ok_or("saved combat needs the acting civilization's live unit")?;
    let origin = (act.x, act.y);
    let defender = singleton(world, target).ok_or("saved combat needs a single defender")?;
    let def = &world.units[defender];
    if def.owner == civ
        || def.owner == saved.player_civ
        || singleton(world, origin) != Some(attacker)
        || [act, def].into_iter().any(|u| {
            u.is_leader
                || rules.unit_domain(usize::from(u.type_id)) != Some(0)
                || world
                    .map
                    .terrain_at(u.x, u.y)
                    .is_none_or(|terrain| terrain == 10)
                || world.city_at(u.x, u.y).is_some()
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

    let observed = unit_observation_after(&updated, rules, attacker, true)
        .map_err(|error| format!("saved combat observation: {error}"))?;
    super::snapshot::apply_saved_observation(world, &observed);
    Ok(observed)
}
