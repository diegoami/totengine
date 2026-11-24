//! Read-only record preparation for the first classic profile (#197).
//! Passing this check is not admission: phase/pending state, dependencies
//! and knowledge continuation still have independent gates.
use crate::saved_knowledge::unit_observation_after;
use crate::saved_knowledge::{unit_observation_after_reset, SavedMapKnowledge};
use std::collections::{BTreeMap, BTreeSet};
use tot_kit::{MapBlock, Rules, SaveFile};

/// Projection of one ordinary land move from a checked save. The map holds
/// the observed bits and per-civilization memory, separate from the engine's
/// generic reveal. This does not retain native movement-history bytes.
pub struct SavedLandStep {
    pub map: MapBlock,
    pub view: SavedMapKnowledge,
    pub position: (u16, u16),
    pub moves_left: u32,
}

/// Owned parsed records that passed the bounded structural policy. Immutable
/// access keeps the result bound to the bytes checked. This is not a session
/// constructor and is deliberately unused by legacy/experimental restore.
#[derive(Debug)]
pub struct CheckedSnapshot {
    save: SaveFile,
    stack_positions: Vec<Option<u16>>,
    source_length: usize,
    source_fingerprint: u64,
    /// Bytes after the city table; their presence/length does not establish
    /// pending-choice or extension support. No tail is silently stripped.
    trailing_bytes: usize,
}

impl CheckedSnapshot {
    pub fn source_length(&self) -> usize {
        self.source_length
    }

    pub fn source_fingerprint(&self) -> u64 {
        self.source_fingerprint
    }

    pub fn trailing_bytes(&self) -> usize {
        self.trailing_bytes
    }

    pub fn save(&self) -> &SaveFile {
        &self.save
    }

    /// Native linked-list order, keyed by record slot; deleted slots are None.
    pub fn stack_positions(&self) -> &[Option<u16>] {
        &self.stack_positions
    }

    /// Prepare an ordinary unstacked land step by the saved player for
    /// read-only continuation comparison. Execute movement on a temporary
    /// world and feed its destination to the native-derived observation
    /// kernel, without any generic reveal. Special destinations have their
    /// own event, contact or stack lifecycles and require separate evidence.
    pub fn project_land_step(
        &self,
        rules: &Rules,
        slot: usize,
        to: (u16, u16),
    ) -> Result<SavedLandStep, String> {
        let save = &self.save;
        let player = save.player_civ;
        super::phase::saved_human_entry(save, rules, player)?;
        let unit = save
            .units
            .get(slot)
            .filter(|u| u.id() != 0)
            .ok_or("saved land step requires a live unit")?;
        let mut world = crate::boot(save, 0);
        let allowance =
            world.movement_allowance(rules, usize::from(unit.type_id()), player, unit.damage());
        world.units[slot].moves_left = allowance.saturating_sub(u32::from(unit.moves_spent()));
        let mut observed = stage_saved_land_move(save, rules, &mut world, slot, to, false, None)?;
        let moved = &world.units[slot];
        let view = SavedMapKnowledge::read(&observed, player)?;
        Ok(SavedLandStep {
            map: observed.maps.remove(0),
            view,
            position: (moved.x, moved.y),
            moves_left: moved.moves_left,
        })
    }

    /// Validate engine preparation bounds, not universal original-game
    /// legality. All failures precede boot/scheduling or any state mutation.
    pub fn read(bytes: &[u8], rules: &Rules) -> Result<Self, String> {
        let save = tot_kit::parse_sav(bytes).map_err(|e| format!("snapshot framing: {e}"))?;
        let stack_positions = check_records(&save, rules)?;
        let tables_end = save.city_offset + save.cities.len() * 92;
        Ok(Self {
            source_length: bytes.len(),
            source_fingerprint: crate::player::fnv64(bytes),
            trailing_bytes: bytes.len() - tables_end,
            save,
            stack_positions,
        })
    }
}

#[derive(Clone, Copy)]
pub(crate) enum CityStackMove {
    Departure { mate: usize },
    Arrival { resident: usize },
}

/// Stage a checked relocation and its saved observation on a temporary world.
pub(crate) fn stage_saved_land_move(
    save: &SaveFile,
    rules: &Rules,
    world: &mut crate::WorldState,
    slot: usize,
    to: (u16, u16),
    keep_native_budget: bool,
    city_stack: Option<CityStackMove>,
) -> Result<SaveFile, String> {
    let leaving_city = matches!(city_stack, Some(CityStackMove::Departure { .. }));
    let entering_city = matches!(city_stack, Some(CityStackMove::Arrival { .. }));
    let player = save.player_civ;
    let unit = save
        .units
        .get(slot)
        .filter(|u| u.id() != 0)
        .ok_or("saved land step requires a live unit")?;
    if unit.owner() != player
        || (!leaving_city && (unit.stack_prev(), unit.stack_next()) != (-1, -1))
        || unit.orders() != 255
        || rules.unit_domain(usize::from(unit.type_id())) != Some(0)
    {
        return Err(
            "saved land step requires an awake owned land unit in a checked stack context".into(),
        );
    }
    let index = |(x, y)| {
        save.map_header
            .stored_index(usize::from(x), usize::from(y))
            .filter(|_| (u32::from(x) + u32::from(y)) % 2 == 0)
            .ok_or("saved land step requires valid surface coordinates")
    };
    let live = world
        .units
        .get(slot)
        .ok_or("saved land step world is missing its unit")?;
    if (
        live.id,
        live.owner,
        live.type_id,
        live.x,
        live.y,
        live.orders,
    ) != (
        unit.id(),
        unit.owner(),
        unit.type_id(),
        unit.x(),
        unit.y(),
        unit.orders(),
    ) {
        return Err("saved land step world differs from saved unit context".into());
    }
    let from = index((unit.x(), unit.y()))?;
    let dest = index(to)?;
    let map = &save.maps[0];
    if map.has_city(from) != leaving_city
        || map.has_city(dest) != entering_city
        || map.has_unit(dest) != entering_city
        || map.terrain(dest) == 10
    {
        return Err("saved land step requires checked origin and destination occupancy".into());
    }

    if world.map.hut_at(to.0, to.1) {
        return Err("saved land step cannot enter a hut".into());
    }
    // The native first step can spend its full terrain cost even when it
    // exceeds the unit's allowance. This context consumes no partial roll.
    let first_step = keep_native_budget
        && live.moves_left
            == world.movement_allowance(rules, usize::from(live.type_id), player, live.damage);
    if let Some(stack) = city_stack {
        let (mate, expected) = match stack {
            CityStackMove::Departure { mate } => {
                (mate, (unit.x().checked_sub(1), unit.y().checked_sub(1)))
            }
            CityStackMove::Arrival { resident } => {
                (resident, (unit.x().checked_add(1), unit.y().checked_add(1)))
            }
        };
        if !first_step
            || live.moves_left != 3
            || unit.moves_spent() != 0
            || save.units[mate].orders() != 255
            || save.units[mate].type_id() != unit.type_id()
            || expected != (Some(to.0), Some(to.1))
        {
            return Err("saved city-stack transfer requires the witnessed full-budget step".into());
        }
    }
    let moved = if first_step {
        let routes = crate::movement::MovementRoute {
            road: (leaving_city || world.map.road[from]) && (entering_city || world.map.road[dest]),
            rail: (leaving_city || world.map.rail[from]) && (entering_city || world.map.rail[dest]),
        };
        world.move_unit_with_routes(rules, slot, to.0, to.1, Some(routes), true)
    } else {
        world.move_unit(rules, slot, to.0, to.1)
    };
    if !moved.moved || moved.zoc_ended || moved.battle.is_some() || moved.hut {
        return Err("saved land step did not resolve as ordinary movement".into());
    }
    if city_stack.is_some() && moved.cost_thirds != 6 {
        return Err("saved city-stack transfer has an unchecked movement cost".into());
    }
    let moved_unit = &world.units[slot];
    let position = (moved_unit.x, moved_unit.y);
    let mut changed = if city_stack.is_some() {
        crate::saved_knowledge::clear_unit_stack_visibility(save, slot)?
    } else {
        save.clone()
    };
    changed.units[slot].raw[..2].copy_from_slice(&position.0.to_le_bytes());
    changed.units[slot].raw[2..4].copy_from_slice(&position.1.to_le_bytes());
    if first_step {
        // Native movement arguments are numbered NE, E, SE, S, SW, W, NW, N.
        let offset = (
            i32::from(to.0) - i32::from(unit.x()),
            i32::from(to.1) - i32::from(unit.y()),
        );
        let direction = [
            (1, -1),
            (2, 0),
            (1, 1),
            (0, 2),
            (-1, 1),
            (-2, 0),
            (-1, -1),
            (0, -2),
        ]
        .iter()
        .position(|&candidate| candidate == offset)
        .ok_or("saved movement direction missing")?;
        let raw = &mut changed.units[slot].raw;
        raw[6] |= 0x40;
        raw[10] = moved
            .cost_thirds
            .try_into()
            .map_err(|_| "saved movement cost out of range")?;
        raw[13] = direction as u8;
    }
    let tile_bytes = &mut changed.maps[0].tile_fields;
    if !leaving_city {
        tile_bytes[6 * from + 1] &= !1;
    }
    tile_bytes[6 * dest + 1] |= 1;
    tile_bytes[6 * dest + 5] = (tile_bytes[6 * dest + 5] & 15) | (world.map.owner[dest] << 4);
    let observed = match city_stack {
        Some(CityStackMove::Departure { mate }) => {
            changed.units[slot].raw[26..30].fill(255);
            changed.units[mate].raw[26..30].fill(255);
            world.units[slot].stack_pos = 0;
            world.units[mate].stack_pos = 0;
            unit_observation_after(&changed, rules, slot, true)
        }
        Some(CityStackMove::Arrival { resident }) => {
            let new_head = &mut changed.units[slot].raw;
            new_head[28..30].copy_from_slice(&(resident as i16).to_le_bytes());
            changed.units[resident].raw[26..28].copy_from_slice(&(slot as i16).to_le_bytes());
            world.units[slot].stack_pos = 0;
            world.units[resident].stack_pos = 1;
            unit_observation_after(&changed, rules, slot, true)
        }
        None => unit_observation_after_reset(&changed, rules, slot, true),
    }
    .map_err(|error| format!("saved land step observation: {error}"))?;
    apply_saved_observation(world, &observed);
    Ok(observed)
}

pub(crate) fn stage_saved_unit_skip(
    save: &SaveFile,
    rules: &Rules,
    world: &mut crate::WorldState,
    slot: usize,
    keep_native_budget: bool,
    city_mate: Option<usize>,
) -> Result<SaveFile, String> {
    let player = save.player_civ;
    let unit = world
        .units
        .get(slot)
        .filter(|unit| {
            unit.id != 0 && unit.owner == player && unit.moves_left > 0 && unit.orders == 255
        })
        .ok_or("saved skip requires a ready own unit")?;
    let record = save
        .units
        .get(slot)
        .filter(|record| (record.id(), record.x(), record.y()) == (unit.id, unit.x, unit.y))
        .ok_or("saved skip world differs from saved unit context")?;
    if (city_mate.is_none() && (record.stack_prev(), record.stack_next()) != (-1, -1))
        || rules.unit_domain(usize::from(record.type_id())) != Some(0)
    {
        return Err("saved skip requires an awake land unit in a checked stack context".into());
    }
    let budget = keep_native_budget
        .then(|| world.movement_allowance(rules, usize::from(unit.type_id), player, unit.damage));
    if city_mate.is_some_and(|mate| {
        budget != Some(3)
            || unit.moves_left != 3
            || record.moves_spent() != 0
            || save.units[mate].type_id() != record.type_id()
            || save.units[mate].orders() != 255
    }) {
        return Err("saved city-stack skip requires the checked three-third budget".into());
    }
    let result = world.apply_order(
        rules,
        None,
        player,
        crate::player::Order::SkipUnit { unit: slot },
    );
    if result != crate::player::OrderResult::Skipped {
        return Err("saved skip rejected".into());
    }
    // Space reaches unit observation through the no-relocation branch. The
    // moving-unit stack reset belongs to the relocation path.
    let mut observed = unit_observation_after(save, rules, slot, true)
        .map_err(|error| format!("saved skip observation: {error}"))?;
    if city_mate.is_some() && observed != *save {
        return Err("saved city-stack skip changed captured observation".into());
    }
    if let Some(budget) = budget {
        let raw = &mut observed.units[slot].raw;
        raw[10] = budget
            .try_into()
            .map_err(|_| "saved movement budget out of range")?;
        raw[13] = 255;
    }
    apply_saved_observation(world, &observed);
    Ok(observed)
}

pub(crate) fn apply_saved_observation(world: &mut crate::WorldState, observed: &SaveFile) {
    let map = &observed.maps[0];
    for (tile, bytes) in map.tile_fields.as_chunks::<6>().0.iter().enumerate() {
        world.map.improvements[tile] = bytes[1];
        world.map.visible[tile] = bytes[4];
    }
    let area = observed.map_header.surface();
    for (civ, cache) in world.map.known_improvements.iter_mut().enumerate() {
        cache.copy_from_slice(&map.visibility[civ * area..(civ + 1) * area]);
    }
    for (record, unit) in observed.units.iter().zip(&mut world.units) {
        if unit.id != 0 {
            unit.orders = record.orders();
        }
    }
}

fn check_records(save: &SaveFile, rules: &Rules) -> Result<Vec<Option<u16>>, String> {
    if save.version != 0x32
        || !save.map_header.decodable()
        || save.map_header.raw[0] % 2 != 0
        || save.map_header.map_count() != 1
        || save.maps.len() != 1
        || save.transporter_count != 0
    {
        return Err(
            "snapshot map: requires classic 0x0032, one surface map and no transporters".into(),
        );
    }
    if save.units.len() > 2048 || save.cities.len() > 255 {
        return Err("snapshot records: exceeds classic engine unit/city capacity".into());
    }
    let n = save.map_header.surface();
    let map = &save.maps[0];
    let index = |x: u16, y: u16| {
        save.map_header
            .stored_index(usize::from(x), usize::from(y))
            .filter(|&i| i < n && (u32::from(x) + u32::from(y)) % 2 == 0)
    };
    let live_city = |slot: usize| save.cities.get(slot).filter(|c| c.city_id() != 0);
    let mut city_tiles = BTreeMap::new();
    let mut city_ids = BTreeSet::new();
    for (slot, c) in save
        .cities
        .iter()
        .enumerate()
        .filter(|(_, c)| c.city_id() != 0)
    {
        let fail = |field| format!("snapshot city {slot}: {field}");
        if c.map_id() != 0 || c.owner() > 7 {
            return Err(fail("unsupported map/owner"));
        }
        let tile = index(c.x(), c.y()).ok_or_else(|| fail("invalid coordinates/parity"))?;
        if !city_ids.insert(c.city_id()) || c.city_id() == u32::MAX {
            return Err(fail("duplicate or exhausted identity"));
        }
        if city_tiles.insert(tile, slot).is_some() {
            return Err(fail("multiple live cities on one tile"));
        }
        if !(1..=127).contains(&c.size()) {
            return Err(fail("unsupported signed display size"));
        }
        let item = c.build_item();
        if (item >= 0 && rules.unit_move(item as usize).is_none())
            || (item < 0
                && rules
                    .improvement_cost(-(i16::from(item)) as usize)
                    .is_none())
        {
            return Err(fail("production item missing from Rules"));
        }
        if !(0..=3).contains(&c.route_count()) {
            return Err(fail("unsupported route count"));
        }
        for partner in c
            .route_partners()
            .into_iter()
            .take(c.route_count() as usize)
        {
            if partner < 0 || live_city(partner as usize).is_none() {
                return Err(fail("route partner is not a live city"));
            }
        }
    }
    for (wonder, &city) in save.wonders.iter().enumerate() {
        if city < -2 || (city >= 0 && live_city(city as usize).is_none()) {
            return Err(format!("snapshot wonder {wonder}: invalid city reference"));
        }
    }
    let mut unit_tiles: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut unit_ids = BTreeSet::new();
    for (slot, u) in save.units.iter().enumerate().filter(|(_, u)| u.id() != 0) {
        let fail = |field| format!("snapshot unit {slot}: {field}");
        if u.map_id() != 0 || u.owner() > 7 {
            return Err(fail("unsupported map/flagged owner"));
        }
        let tile = index(u.x(), u.y()).ok_or_else(|| fail("invalid coordinates/parity"))?;
        if !unit_ids.insert(u.id()) || u.id() == u32::MAX {
            return Err(fail("duplicate or exhausted identity"));
        }
        if rules.unit_move(usize::from(u.type_id())).is_none()
            || !rules
                .unit_domain(usize::from(u.type_id()))
                .is_some_and(|d| d <= 2)
        {
            return Err(fail("type/movement/domain missing or unsupported in Rules"));
        }
        if u.home() != 255
            && !live_city(usize::from(u.home())).is_some_and(|c| c.owner() == u16::from(u.owner()))
        {
            return Err(fail("home is not a live city of this owner"));
        }
        for (link, previous) in [(u.stack_prev(), true), (u.stack_next(), false)] {
            if link == -1 {
                continue;
            }
            let other = usize::try_from(link)
                .ok()
                .and_then(|i| save.units.get(i))
                .filter(|r| r.id() != 0);
            if !other.is_some_and(|r| {
                r.map_id() == 0
                    && (r.x(), r.y()) == (u.x(), u.y())
                    && (if previous {
                        r.stack_next()
                    } else {
                        r.stack_prev()
                    }) == slot as i16
            }) {
                return Err(fail(
                    "stack link is invalid, nonreciprocal or crosses tiles",
                ));
            }
        }
        unit_tiles.entry(tile).or_default().push(slot);
    }
    for tile in 0..n {
        if map.terrain(tile) > 10 || map.owner(tile).is_some_and(|o| o > 7) {
            return Err(format!("snapshot tile {tile}: unsupported terrain/owner"));
        }
        if map.has_unit(tile) != unit_tiles.contains_key(&tile)
            || map.has_city(tile) != city_tiles.contains_key(&tile)
        {
            return Err(format!(
                "snapshot tile {tile}: live records and occupancy flags differ"
            ));
        }
    }
    let mut positions = vec![None; save.units.len()];
    for (tile, slots) in unit_tiles {
        let heads: Vec<_> = slots
            .iter()
            .copied()
            .filter(|&i| save.units[i].stack_prev() == -1)
            .collect();
        if heads.len() != 1 {
            return Err(format!("snapshot stack at tile {tile}: requires one head"));
        }
        let mut current = Some(heads[0]);
        let mut count = 0;
        while let Some(slot) = current {
            if positions[slot].replace(count).is_some() {
                return Err(format!("snapshot stack at tile {tile}: cycle"));
            }
            count += 1;
            current = usize::try_from(save.units[slot].stack_next()).ok();
        }
        if usize::from(count) != slots.len() {
            return Err(format!(
                "snapshot stack at tile {tile}: disconnected records"
            ));
        }
    }
    Ok(positions)
}

#[cfg(test)]
mod tests;
