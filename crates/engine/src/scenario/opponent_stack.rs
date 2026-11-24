//! Scoped opponent movement between unseen land and friendly city stacks.
use crate::movement::MovementRoute;
use crate::player::OrderResult;
use crate::saved_knowledge::{
    clear_unit_stack_visibility, unit_observation_after, unit_observation_with_contacts,
    UnitObservationError,
};
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
/// be in processed cities. New records start with a clear unit mask.
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum StackMovement {
    City,
    Extended,
    StockContact,
    StockHut,
    StockArmed,
    StockArmedTurn46,
    StockZone,
    StockWestward,
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
    mode: StackMovement,
) -> Result<(SaveFile, OrderResult), String> {
    let field_stack = mode != StackMovement::City;
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
    let zone_case = match (world.turn, civ, slot, origin, to, world.combat_rng.state()) {
        (46, 5, 10, (39, 35), (38, 34), 0xd9f16a1a) => Some(((2, 3, 5), (3, 2, 16, 39, 33))),
        (47, 2, 3, (39, 33), (38, 34), 0x3b0ca297) => Some(((16, 6, 9), (10, 5, 2, 39, 35))),
        _ => None,
    };
    if matches!(
        mode,
        StackMovement::StockZone | StackMovement::StockWestward
    ) && world.human_mask == 16
        && (world.diplomacy[5][2], world.diplomacy[2][5]) == (0x405, 0x405)
        && unit.orders == 255
        && (record.raw[16] == 0 || civ == 2 && record.raw[16] < 20)
        && source == [slot]
        && members.is_empty()
        && city.is_none()
        && origin_city.is_none()
        && world.map.terrain[dest] == 4
        && ![from, dest]
            .into_iter()
            .any(|i| world.map.road[i] || world.map.rail[i])
        && zone_case.is_some_and(|(mover, (rival, owner, kind, x, y))| {
            (unit.type_id, unit.moves_left, world.map.terrain[from]) == mover
                && world.units.get(rival).is_some_and(|u| {
                    u.id != 0 && (u.owner, u.type_id, u.x, u.y) == (owner, kind, x, y)
                })
        })
    {
        let mut refused = saved.clone();
        let raw = &mut refused.units[slot].raw;
        // The twentieth native refusal resets this byte and exhausts the mover.
        if raw[16] == 19 {
            raw[16] = 0;
            raw[10] = 6;
            world.units[slot].moves_left = 0;
            return Ok((refused, OrderResult::MovementExhausted));
        }
        raw[16] += 1;
        return Ok((refused, OrderResult::Rejected("zone of control")));
    }
    let unstacked_road_boundary = field_stack
        && source.len() == 1
        && members.is_empty()
        && origin_city.is_none()
        && city.is_none()
        && world.map.road[from] != world.map.road[dest]
        && !world.map.rail[from]
        && !world.map.rail[dest];
    let hut = world.map.hut_at(to.0, to.1);
    let witnessed_hut =
        hut && matches!(
            mode,
            StackMovement::StockHut
                | StackMovement::StockArmed
                | StackMovement::StockArmedTurn46
                | StackMovement::StockZone
                | StackMovement::StockWestward
        ) && (world.turn, civ, slot, origin, to) == (44, 6, 37, (28, 12), (27, 13))
            && world.human_mask == 16
            && world.combat_rng.state() == 0x2413ad1c
            && (unit.type_id, unit.moves_left, unit.orders) == (0, 3, 12)
            && world.movement_allowance(rules, 0, civ, unit.damage) == 3
            && (
                world.research[6].progress,
                world.research[6].target,
                world.research[6].acquired,
            ) == (12, 1, 7)
            && source.len() == 1
            && members.is_empty();

    if hut && !witnessed_hut {
        return Err("saved stack move cannot enter an unwitnessed hut".into());
    }
    let field_to_field =
        field_stack && !members.is_empty() && city.is_none() && origin_city.is_none();
    let attack = rules.unit_attack(usize::from(unit.type_id));
    let armed_rules = matches!(
        mode,
        StackMovement::StockArmed
            | StackMovement::StockArmedTurn46
            | StackMovement::StockZone
            | StackMovement::StockWestward
    ) && field_to_field
        && (civ, slot) == (7, 24)
        && source == [24]
        && members == [4]
        && (unit.type_id, unit.orders, unit.moves_left) == (2, 2, 3)
        && attack.is_some_and(|value| value != 0)
        && world.units[4].type_id == 0;
    let armed_cost = if armed_rules {
        match (world.turn, origin, to, world.combat_rng.state()) {
            (44, (9, 45), (9, 47), 0x44bc0476) => Some(3),
            (45, (9, 47), (9, 49), 0x34bba6a0)
                if matches!(
                    mode,
                    StackMovement::StockArmedTurn46
                        | StackMovement::StockZone
                        | StackMovement::StockWestward
                ) && world.movement_allowance(rules, 2, civ, unit.damage) == 3 =>
            {
                Some(6)
            }
            _ => None,
        }
    } else {
        None
    };
    let western_arrival = mode == StackMovement::StockWestward
        && (world.turn, civ, slot, origin, to, world.combat_rng.state())
            == (47, 7, 28, (12, 44), (10, 44), 0x3b0ca297)
        && world.human_mask == 16
        && source == [28]
        && members == [7]
        && field_to_field
        && (
            unit.type_id,
            unit.moves_left,
            unit.orders,
            record.moves_spent(),
        ) == (2, 3, 1, 0)
        && (
            world.units[7].type_id,
            world.units[7].moves_left,
            world.units[7].orders,
        ) == (2, 0, 28)
        && saved.maps[0].tile_fields[6 * from..6 * from + 6] == [4, 1, 224, 2, 128, 112]
        && saved.maps[0].tile_fields[6 * dest..6 * dest + 6] == [3, 1, 224, 2, 128, 112];
    let armed_cost = if western_arrival { Some(6) } else { armed_cost };
    let witnessed_armed = armed_cost.is_some();
    let partial_movement =
        field_departure || unstacked_road_boundary || witnessed_hut || armed_cost == Some(6);
    if field_to_field && attack != Some(0) && !witnessed_armed || city_transfer && attack.is_none()
    {
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
        (field_to_field || partial_movement).then(|| MovementRoute {
            road: world.map.road[from] && world.map.road[dest],
            rail: world.map.rail[from] && world.map.rail[dest],
        })
    };
    let spent = partial_movement.then(|| {
        world
            .movement_allowance(rules, usize::from(unit.type_id), civ, unit.damage)
            .saturating_sub(unit.moves_left)
    });
    let crt = world.combat_rng.state();
    let result = world.move_unit_with_routes(rules, slot, to.0, to.1, route, partial_movement);
    if result.battle.is_some() || (result.hut && !witnessed_hut) || result.zoc_ended {
        return Err("saved stack move did not resolve as ordinary movement".into());
    }
    if armed_cost.is_some_and(|cost| {
        !result.moved || result.cost_thirds != cost || world.combat_rng.state() != crt
    }) {
        return Err("saved armed arrival differed from the witnessed movement".into());
    }
    if witnessed_hut
        && (!result.moved
            || result.cost_thirds != 9
            || result.hut_outcome != Some(crate::huts::HutOutcome::Advance(1))
            || world.combat_rng.state() != 0x44bc0476
            || (
                world.research[6].progress,
                world.research[6].target,
                world.research[6].acquired,
            ) != (12, -1, 8))
    {
        return Err("saved hut reward differed from the witnessed advance".into());
    }
    let mut updated = if result.moved {
        clear_unit_stack_visibility(saved, slot)?
    } else if partial_movement && result.cost_thirds > 0 {
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
    if witnessed_armed || city_transfer && attack.is_some_and(|value| value != 0) {
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
    if witnessed_hut {
        updated.techs[6] = world.techs[6];
        updated.research[6] = world.research[6];
        updated.units[slot].raw[6] |= 0x40;
        updated.units[slot].raw[13] = 4;
    }
    if let Some(cost) = armed_cost {
        updated.units[slot].raw[6] |= 0x40;
        updated.units[slot].raw[10] = cost as u8;
        updated.units[slot].raw[13] = if western_arrival { 5 } else { 3 };
    }
    let observed = if matches!(
        mode,
        StackMovement::StockContact
            | StackMovement::StockHut
            | StackMovement::StockArmed
            | StackMovement::StockArmedTurn46
            | StackMovement::StockZone
            | StackMovement::StockWestward
    ) {
        unit_observation_with_contacts(&updated, rules, slot, |observed, error| {
            if matches!(
                mode,
                StackMovement::StockArmedTurn46
                    | StackMovement::StockZone
                    | StackMovement::StockWestward
            ) && apply_known_opponent_contact(observed, world, &error)
            {
                return Ok(());
            }
            apply_stock_contact(observed, rules, world, &error).ok_or(error)
        })
    } else {
        unit_observation_after(&updated, rules, slot, true)
    }
    .map_err(|error| format!("saved opponent stack observation: {error}"))?;
    super::snapshot::apply_saved_observation(world, &observed);
    Ok((observed, OrderResult::Moved))
}

// Between trade turns, native known opponent contact returns after
// recording the reciprocal encounter bits. Human contacts and trade turns
// still need their own resolved transaction.
fn apply_known_opponent_contact(
    observed: &mut SaveFile,
    world: &mut WorldState,
    error: &UnitObservationError,
) -> bool {
    let &UnitObservationError::ContactRequired {
        observer,
        other,
        map: 0,
        ..
    } = error
    else {
        return false;
    };
    if !(1..8).contains(&observer)
        || !(1..8).contains(&other)
        || observed.multiplayer != 0
        || observed.human_mask != world.human_mask
        || world.human_mask & ((1 << observer) | (1 << other)) != 0
        || world.research_settings.flags & 0x80 != 0
        || world.turn <= 0
        || (i32::from(world.turn) + i32::from(observer) + i32::from(other)) % 4 == 0
    {
        return false;
    }
    let (a, b) = (usize::from(observer), usize::from(other));
    if [(a, b), (b, a)].into_iter().any(|(a, b)| {
        world.diplomacy[a][b] & 1 == 0 || world.diplomacy[a][b] != observed.diplomacy[a][b]
    }) {
        return false;
    }
    world.treaty_set(observer, other, 0x401);
    observed.diplomacy[a][b] = world.diplomacy[a][b];
    observed.diplomacy[b][a] = world.diplomacy[b][a];
    true
}

// The prepared Original encounter is the only measured first-contact
// resolution. Its treaty and grant effects are checked before commit.
fn apply_stock_contact(
    observed: &mut SaveFile,
    rules: &Rules,
    world: &mut WorldState,
    error: &UnitObservationError,
) -> Option<()> {
    if *error
        != (UnitObservationError::ContactRequired {
            observer: 5,
            other: 6,
            x: 48,
            y: 28,
            map: 0,
        })
        || world.turn != 44
        || world.human_mask != 16
        || world.research_settings.flags & 0x80 != 0
        || (world.research[5].acquired, world.research[6].acquired) != (7, 6)
        || world.combat_rng.state() != 0xd766ce46
        || (world.diplomacy[5][6], world.diplomacy[6][5]) != (0, 0)
        || (observed.diplomacy[5][6], observed.diplomacy[6][5]) != (0, 0)
    {
        return None;
    }
    use crate::ai_research::{value, ValueContext, ValueRules};
    let tables = ValueRules::from_rules(rules).ok()?;
    world.treaty_set(5, 6, 0x4401);
    let contexts = [
        ValueContext::from_world(world, rules, 5).ok()?,
        ValueContext::from_world(world, rules, 6).ok()?,
    ];
    let mut scored = Vec::new();
    let mut best = [None; 2];
    for (advance, tech) in tables.advances.iter().enumerate() {
        if tech.prerequisites == [-2, -2]
            || world.tech_group_flag(rules, 5, advance) == 2
            || world.tech_group_flag(rules, 6, advance) == 2
        {
            continue;
        }
        let bit = 1 << (advance % 8);
        let knows = |civ: usize| world.techs[civ][advance / 8] & bit != 0;
        let index = match (knows(5), knows(6)) {
            (false, true) => 0,
            (true, false) => 1,
            _ => continue,
        };
        let base = value(&tables, &contexts[index], advance)?;
        scored.push((index, advance, base));
        let score = base + (world.combat_rng.rand() % 3) as i32;
        if best[index].is_none_or(|(previous, _)| score > previous) {
            best[index] = Some((score, advance));
        }
    }
    if scored
        != [
            (1, 1, 5),
            (1, 12, 4),
            (1, 36, 7),
            (0, 39, 6),
            (1, 46, 8),
            (1, 47, 4),
            (0, 56, 6),
            (0, 86, 5),
            (1, 88, 3),
        ]
        || best.map(|item| item.map(|(_, advance)| advance)) != [Some(39), Some(46)]
    {
        return None;
    }
    let units = world.units.clone();
    let cities = world.cities.clone();
    let mut expected_techs = world.techs;
    expected_techs[5][39 / 8] |= 1 << (39 % 8);
    expected_techs[6][46 / 8] |= 1 << (46 % 8);
    if !world.grant_advance_with_rules(rules, 5, 39)
        || !world.grant_advance_with_rules(rules, 6, 46)
        || world.techs != expected_techs
        || units != world.units
        || cities != world.cities
    {
        return None;
    }
    world.treaty_set(5, 6, 4);
    observed.diplomacy = world.diplomacy;
    for civ in [5, 6] {
        observed.techs[civ] = world.techs[civ];
        observed.research[civ] = world.research[civ];
    }
    Some(())
}

/// Record the witnessed Turn-48 sea arrival while keeping the saved human fog.
pub(crate) fn stage_naval_arrival(
    saved: &SaveFile,
    before: &WorldState,
    world: &mut WorldState,
    viewer: u8,
    arrivals: &[usize],
) -> Option<SaveFile> {
    let location = (0, 40);
    if (before.turn, world.turn, viewer) != (47, 48, 4)
        || (before.combat_rng.state(), world.combat_rng.state()) != (0x3b0ca297, 0x29e9892a)
        || arrivals != [2, 36, 37]
        || before.units.len() != world.units.len()
        || saved.units.len() != world.units.len()
        || before
            .units
            .iter()
            .zip(&world.units)
            .enumerate()
            .any(|(slot, (old, new))| {
                if arrivals.contains(&slot) {
                    old.id != 0
                } else {
                    old != new
                }
            })
    {
        return None;
    }
    let knowledge = crate::saved_knowledge::SavedMapKnowledge::read(saved, viewer).ok()?;
    let tile = world.map.stored_index(location.0, location.1)?;
    if knowledge.tiles.get(tile)?.is_some()
        || saved.maps[0].tile_fields.get(tile * 6 + 1).copied() != Some(0)
        || saved.maps[0].tile_fields.get(tile * 6 + 5).copied() != Some(240)
        || world.map.terrain[tile] != 10
    {
        return None;
    }
    let mut observed = saved.clone();
    let order = [37, 36, 2];
    for (stack_pos, (&slot, (kind, orders, veteran))) in order
        .iter()
        .zip([(32, 255, true), (4, 3, false), (46, 3, false)])
        .enumerate()
    {
        let unit = &world.units[slot];
        if observed.units[slot].id() != 0
            || unit.id == 0
            || (unit.x, unit.y) != location
            || (unit.type_id, unit.orders, unit.veteran) != (kind, orders, veteran)
            || (unit.owner, unit.home, unit.damage, unit.is_leader) != (0, 255, 0, false)
            || usize::from(unit.stack_pos) != stack_pos
        {
            return None;
        }
        let raw = &mut observed.units[slot].raw;
        *raw = [0; 40];
        raw[0..2].copy_from_slice(&unit.x.to_le_bytes());
        raw[2..4].copy_from_slice(&unit.y.to_le_bytes());
        raw[6..8].copy_from_slice(&(u16::from(veteran) << 13).to_le_bytes());
        raw[8] = kind;
        raw[17] = orders;
        raw[18] = unit.home;
        raw[30..34].copy_from_slice(&unit.id.to_le_bytes());
    }
    link_stack(&mut observed, world, &order).ok()?;
    let map = &mut observed.maps[0].tile_fields;
    map[6 * tile + 1] = 1;
    map[6 * tile + 5] = 0;
    for (x, y) in [
        (0, 38),
        (1, 39),
        (79, 39),
        (0, 40),
        (2, 40),
        (78, 40),
        (1, 41),
        (79, 41),
        (0, 42),
    ] {
        let at = world.map.stored_index(x, y)?;
        map[6 * at + 4] |= 1;
    }
    if crate::saved_knowledge::SavedMapKnowledge::read(&observed, viewer).ok()? != knowledge {
        return None;
    }
    world.map.owner[tile] = 0;
    super::snapshot::apply_saved_observation(world, &observed);
    Some(observed)
}
