//! Map and hidden-city transition for the checked Original turn-47 and
//! turn-50 founding callbacks.
use crate::WorldState;
use tot_kit::{CityRecord, Rules, SaveFile};

type Offset = (i16, i16);

const SITE_OFFSETS: [Offset; 17] = [
    (2, -2),
    (-3, -1),
    (-1, -1),
    (1, -1),
    (3, -1),
    (-2, 0),
    (0, 0),
    (2, 0),
    (-3, 1),
    (-1, 1),
    (1, 1),
    (3, 1),
    (-2, 2),
    (0, 2),
    (2, 2),
    (-1, 3),
    (1, 3),
];

struct FoundingCase {
    city: usize,
    site: (u16, u16),
    tile_fields: [u8; 6],
    orders: u8,
    deleted_at: i16,
    extra_site_offsets: &'static [Offset],
    fertility_offsets: &'static [Offset],
}

pub(crate) fn stage(
    saved: &SaveFile,
    rules: &Rules,
    world: &mut WorldState,
    civ: u8,
    unit: usize,
) -> Option<(SaveFile, usize, usize)> {
    let case = match (world.turn, civ, unit, world.combat_rng.state()) {
        (47, 5, 2, 0x3b0ca297) => FoundingCase {
            city: 9,
            site: (44, 42),
            tile_fields: [130, 1, 0, 2, 36, 87],
            orders: 5,
            deleted_at: -600,
            extra_site_offsets: &[],
            fertility_offsets: &[(3, -1), (2, 0), (0, 2), (0, 4)],
        },
        (47, 6, 37, 0x3b0ca297) => FoundingCase {
            city: 10,
            site: (27, 17),
            tile_fields: [2, 1, 0, 2, 64, 107],
            orders: 12,
            deleted_at: -700,
            extra_site_offsets: &[(-1, -3), (1, -3), (-2, -2), (0, -2)],
            fertility_offsets: &[(-2, -4), (-3, -1), (-1, -1), (0, 0), (-3, 1), (-1, 5)],
        },
        // The measured turn-50 founding: civ 2's settler at (66, 36)
        // founds the civilization's third city inside its own territory.
        (50, 2, 0, 0xb404d22d) => FoundingCase {
            city: 11,
            site: (66, 36),
            tile_fields: [2, 1, 0, 2, 4, 45],
            orders: 7,
            deleted_at: -300,
            extra_site_offsets: &[(0, -3), (0, -2)],
            fertility_offsets: &[
                (-2, 3),
                (0, -1),
                (0, 0),
                (0, 2),
                (0, 3),
                (0, 4),
                (0, 5),
                (2, -2),
                (2, 3),
                (2, 4),
            ],
        },
        _ => return None,
    };
    if saved.cities.len() != case.city
        || world.available_city_slot() != Some(case.city)
        || world.city_counts[usize::from(civ)] != 2
    {
        return None;
    }
    let founder = world.units.get(unit)?;
    let raw = saved.units.get(unit)?;
    let tile = world.map.stored_index(case.site.0, case.site.1)?;
    if (founder.x, founder.y) != case.site
        || (
            founder.owner,
            founder.type_id,
            founder.moves_left,
            founder.orders,
        ) != (civ, 0, 3, case.orders)
        || founder.id == 0
        || raw.id() != founder.id
        || (raw.x(), raw.y(), raw.owner(), raw.type_id(), raw.orders())
            != (case.site.0, case.site.1, civ, 0, case.orders)
        || (raw.map_id(), raw.stack_prev(), raw.stack_next()) != (0, -1, -1)
        || saved.maps[0].tile_fields.get(6 * tile..6 * tile + 6)? != case.tile_fields
        || world
            .units
            .iter()
            .enumerate()
            .any(|(slot, u)| slot != unit && u.id != 0 && (u.x, u.y) == case.site)
    {
        return None;
    }

    let mut updated = saved.clone();
    let owner = world.map.owner[tile];
    let city = world.found_city(rules, unit).ok()?;
    world.map.owner[tile] = owner;
    let founded = &world.cities[city];
    if city != case.city || (founded.known_to, founded.known_sizes) != (0, [0; 8]) {
        return None;
    }
    // Store the engine's city identity, label and production behind the fog.
    let label = founded.name.as_bytes();
    if !label.is_ascii() || label.len() >= 16 {
        return None;
    }
    let mut record = CityRecord { raw: [0; 92] };
    record.raw[0..2].copy_from_slice(&case.site.0.to_le_bytes());
    record.raw[2..4].copy_from_slice(&case.site.1.to_le_bytes());
    record.raw[6..10].copy_from_slice(&founded.flags.to_le_bytes());
    record.raw[10..12].copy_from_slice(&((founded.owner & 15) | (founded.size << 8)).to_le_bytes());
    record.raw[12] = founded.original_owner;
    record.raw[34..34 + label.len()].copy_from_slice(label);
    record.raw[50..54].copy_from_slice(&founded.worked.to_le_bytes());
    record.raw[59] = founded.building as u8;
    record.raw[86..90].copy_from_slice(&founded.city_id.to_le_bytes());
    updated.cities.push(record);
    updated.city_counts[usize::from(civ)] = world.city_counts[usize::from(civ)];

    let deleted = case.deleted_at.to_le_bytes();
    updated.units[unit].raw[0..2].copy_from_slice(&deleted);
    updated.units[unit].raw[2..4].copy_from_slice(&deleted);
    updated.units[unit].raw[30..34].fill(0);
    world.units[unit].x = u16::from_le_bytes(deleted);
    world.units[unit].y = u16::from_le_bytes(deleted);

    let area = updated.map_header.surface();
    let map = &mut updated.maps[0];
    map.tile_fields[6 * tile + 1] = 2;
    for memory in map.visibility.chunks_exact_mut(area) {
        memory[tile] = (memory[tile] & !3) | 2;
    }
    let index = |map: &crate::MapState, (dx, dy)| {
        map.stored_index(
            case.site.0.checked_add_signed(dx)?,
            case.site.1.checked_add_signed(dy)?,
        )
    };
    for &offset in SITE_OFFSETS.iter().chain(case.extra_site_offsets) {
        let i = index(&world.map, offset)?;
        map.tile_fields[6 * i + 2] = (map.tile_fields[6 * i + 2] & 31) | (civ << 5);
        world.map.site_bits[i] = civ;
    }
    for &offset in case.fertility_offsets {
        let i = index(&world.map, offset)?;
        map.tile_fields[6 * i + 5] &= !8;
        world.map.fertility[i] &= !8;
    }
    super::snapshot::apply_saved_observation(world, &updated);
    Some((updated, city, tile))
}
