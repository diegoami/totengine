//! Read-only record preparation for the first classic profile (#197).
//! Passing this check is not admission: phase/pending state, dependencies
//! and knowledge continuation still have independent gates.
use std::collections::{BTreeMap, BTreeSet};
use tot_kit::{Rules, SaveFile};

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
