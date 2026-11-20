use super::*;

fn rules() -> Rules {
    tot_kit::parse_rules(include_str!("../../../tests/fixtures/campaign-rules.txt"))
}

fn bytes() -> Vec<u8> {
    let mut bytes = crate::tests_support::synthetic_save_bytes();
    let parsed = tot_kit::parse_sav(&bytes).unwrap();
    for i in 0..28 {
        bytes[tot_kit::OFF_WONDERS + 2 * i..tot_kit::OFF_WONDERS + 2 * i + 2]
            .copy_from_slice(&(-1i16).to_le_bytes());
    }
    let tiles = tot_kit::OFF_TRANSPORTERS + 172 + 16 + 7 * parsed.map_header.surface();
    bytes[tiles + 6 * (2 * parsed.map_header.stored_w() + 1) + 1] = 3;
    bytes[tiles + 6 * (2 * parsed.map_header.stored_w() + 2) + 1] = 2;
    for slot in 0..2 {
        let at = parsed.unit_region.0 + slot * 40;
        bytes[at..at + 2].copy_from_slice(&2u16.to_le_bytes());
        bytes[at + 2..at + 4].copy_from_slice(&2u16.to_le_bytes());
        bytes[at + 9] = 1;
        bytes[at + 18] = 0;
        bytes[at + 26..at + 28].copy_from_slice(&(if slot == 0 { -1i16 } else { 0 }).to_le_bytes());
        bytes[at + 28..at + 30].copy_from_slice(&(if slot == 0 { 1i16 } else { -1 }).to_le_bytes());
        let at = parsed.city_offset + slot * 92;
        bytes[at..at + 2].copy_from_slice(&(2u16 + 2 * slot as u16).to_le_bytes());
        bytes[at + 2..at + 4].copy_from_slice(&2u16.to_le_bytes());
        bytes[at + 86..at + 90].copy_from_slice(&(slot as u32 + 1).to_le_bytes());
    }
    bytes
}

fn fixture() -> SaveFile {
    tot_kit::parse_sav(&bytes()).unwrap()
}

fn reject(edit: impl FnOnce(&mut SaveFile), message: &str) {
    let mut save = fixture();
    edit(&mut save);
    let error = check_records(&save, &rules()).unwrap_err();
    assert!(
        error.contains(message),
        "expected {message:?}, got {error:?}"
    );
}

#[test]
fn checks_bytes_without_boot_and_preserves_slot_order() {
    let bytes = bytes();
    let checked = CheckedSnapshot::read(&bytes, &rules()).unwrap();
    assert_eq!(checked.stack_positions(), &[Some(0), Some(1)]);
    assert_eq!(checked.source_length, bytes.len());
    assert_eq!(checked.source_fingerprint, crate::player::fnv64(&bytes));
    assert_eq!(checked.trailing_bytes, 0);
    assert_eq!(checked.save(), &tot_kit::parse_sav(&bytes).unwrap());
    assert!(CheckedSnapshot::read(&bytes[..bytes.len() - 1], &rules()).is_err());
    // Tail content remains explicitly unchecked, never interpreted as admission.
    let extended = [bytes, vec![1, 2, 3]].concat();
    assert_eq!(
        CheckedSnapshot::read(&extended, &rules())
            .unwrap()
            .trailing_bytes,
        3
    );
}

#[test]
fn rejects_unsupported_map_owners_coordinates_and_missing_types() {
    reject(|s| s.version = 0x33, "snapshot map");
    reject(|s| s.map_header.raw[7] = 1, "snapshot map");
    reject(|s| s.map_header.raw[0] += 1, "snapshot map");
    reject(|s| s.transporter_count = 1, "snapshot map");
    reject(|s| s.units[0].raw[4] = 1, "unit 0: unsupported map");
    reject(|s| s.units[0].raw[9] = 0x11, "flagged owner");
    reject(|s| s.units[0].raw[0] = 3, "unit 0: invalid coordinates");
    reject(
        |s| s.units[0].raw[..2].copy_from_slice(&500u16.to_le_bytes()),
        "unit 0: invalid coordinates",
    );
    reject(|s| s.units[0].raw[8] = 255, "type/movement/domain");
    reject(|s| s.cities[0].raw[10] = 8, "city 0: unsupported map/owner");
    reject(|s| s.cities[0].raw[2] = 3, "city 0: invalid coordinates");
    reject(|s| s.cities[0].raw[59] = 127, "production item");
    reject(|s| s.cities[0].raw[11] = 128, "signed display size");
}

#[test]
fn rejects_identity_aliases_and_unallocatable_successors() {
    reject(
        |s| s.units[1].raw[30..34].copy_from_slice(&1u32.to_le_bytes()),
        "unit 1: duplicate",
    );
    reject(
        |s| s.units[0].raw[30..34].copy_from_slice(&u32::MAX.to_le_bytes()),
        "exhausted identity",
    );
    reject(
        |s| s.cities[1].raw[86..90].copy_from_slice(&1u32.to_le_bytes()),
        "city 1: duplicate",
    );
}

#[test]
fn rejects_dangling_homes_active_routes_and_wonders() {
    reject(|s| s.units[0].raw[18] = 7, "home is not a live city");
    reject(
        |s| s.units[0].raw[9] = 2,
        "home is not a live city of this owner",
    );
    reject(|s| s.cities[0].raw[60] = 255, "route count");
    reject(|s| s.cities[0].raw[60] = 4, "route count");
    reject(
        |s| {
            s.cities[0].raw[60] = 1;
            s.cities[0].raw[70] = 7;
        },
        "route partner",
    );
    reject(
        |s| {
            s.cities[0].raw[60] = 1;
            s.cities[0].raw[70..72].copy_from_slice(&(-1i16).to_le_bytes());
        },
        "route partner",
    );
    reject(|s| s.wonders[0] = 7, "wonder 0");
    reject(|s| s.wonders[0] = -3, "wonder 0");
}

#[test]
fn rejects_orphan_or_duplicate_tile_occupants() {
    reject(|s| s.maps[0].tile_fields[1] |= 1, "occupancy flags differ");
    reject(
        |s| s.maps[0].tile_fields[6 * (2 * s.map_header.stored_w() + 1) + 1] &= !2,
        "occupancy flags differ",
    );
    reject(
        |s| s.cities[1].raw[..2].copy_from_slice(&2u16.to_le_bytes()),
        "multiple live cities",
    );
    reject(|s| s.maps[0].tile_fields[0] = 15, "terrain/owner");
}

#[test]
fn rejects_broken_cross_tile_disconnected_and_cyclic_stacks() {
    reject(
        |s| s.units[0].raw[28..30].copy_from_slice(&99i16.to_le_bytes()),
        "stack link",
    );
    reject(
        |s| s.units[0].raw[28..30].copy_from_slice(&(-2i16).to_le_bytes()),
        "stack link",
    );
    reject(
        |s| s.units[1].raw[26..28].copy_from_slice(&(-1i16).to_le_bytes()),
        "nonreciprocal",
    );
    reject(|s| s.units[1].raw[0] = 4, "crosses tiles");
    reject(
        |s| {
            s.units[0].raw[28..30].copy_from_slice(&(-1i16).to_le_bytes());
            s.units[1].raw[26..28].copy_from_slice(&(-1i16).to_le_bytes());
        },
        "requires one head",
    );
    reject(
        |s| {
            s.units[0].raw[26..28].copy_from_slice(&1i16.to_le_bytes());
            s.units[1].raw[28..30].copy_from_slice(&0i16.to_le_bytes());
        },
        "requires one head",
    );
    reject(|s| s.units[1].raw[30..34].fill(0), "stack link");
}

#[test]
fn ignores_deleted_record_payloads_and_inactive_route_slots() {
    let mut s = fixture();
    s.cities[0].raw[70..76].fill(255); // no active routes
    s.wonders[0] = -2; // destroyed is a valid sentinel
    s.units[1].raw.fill(255);
    s.units[1].raw[30..34].fill(0);
    s.units[0].raw[28..30].copy_from_slice(&(-1i16).to_le_bytes());
    for c in &mut s.cities[2..] {
        c.raw.fill(255);
        c.raw[86..90].fill(0);
    }
    assert_eq!(check_records(&s, &rules()).unwrap(), [Some(0), None]);
}

#[test]
fn follows_reversed_heads_and_rejects_unreachable_cycle_components() {
    let mut s = fixture();
    s.units[0].raw[26..28].copy_from_slice(&1i16.to_le_bytes());
    s.units[0].raw[28..30].copy_from_slice(&(-1i16).to_le_bytes());
    s.units[1].raw[26..28].copy_from_slice(&(-1i16).to_le_bytes());
    s.units[1].raw[28..30].copy_from_slice(&0i16.to_le_bytes());
    assert_eq!(check_records(&s, &rules()).unwrap(), [Some(1), Some(0)]);
    // One isolated head plus a reciprocal two-unit cycle on the same tile.
    let mut third = s.units[0].clone();
    third.raw[30..34].copy_from_slice(&3u32.to_le_bytes());
    third.raw[26..28].copy_from_slice(&1i16.to_le_bytes());
    third.raw[28..30].copy_from_slice(&1i16.to_le_bytes());
    s.units.push(third);
    s.units[0].raw[26..30].fill(255);
    s.units[1].raw[26..28].copy_from_slice(&2i16.to_le_bytes());
    s.units[1].raw[28..30].copy_from_slice(&2i16.to_le_bytes());
    assert!(check_records(&s, &rules())
        .unwrap_err()
        .contains("disconnected"));
}
