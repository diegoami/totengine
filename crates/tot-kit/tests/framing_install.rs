//! Record framing (evidence §T1-6 combat, "record framing"), env-gated on
//! `CIVTOT_HOME`: unit and city records are read in the game's own
//! framing. The checks re-derive the proof from the real ROMFALL.scn.

fn romfall() -> Option<tot_kit::SaveFile> {
    let home = std::env::var_os("CIVTOT_HOME")?;
    let p = std::path::PathBuf::from(home).join("FallRome/ROMFALL.scn");
    tot_kit::parse_sav(&std::fs::read(p).ok()?).ok()
}

#[test]
fn units_on_a_city_tile_are_homed_there_and_share_its_owner() {
    let Some(sav) = romfall() else {
        eprintln!("skipping: install not present");
        return;
    };
    let pos: std::collections::HashMap<(u16, u16), usize> = sav
        .cities
        .iter()
        .enumerate()
        .map(|(k, c)| ((c.x(), c.y()), k))
        .collect();
    let (mut homed, mut home_ok, mut on_city, mut owner_ok) = (0, 0, 0, 0);
    for u in &sav.units {
        let Some(&k) = pos.get(&(u.x(), u.y())) else {
            continue;
        };
        on_city += 1;
        owner_ok += usize::from((u.owner() & 0x0f) as u16 == sav.cities[k].owner());
        if u.home() != 0xff {
            homed += 1;
            home_ok += usize::from(u.home() as usize == k);
        }
    }
    assert_eq!(
        (home_ok, homed),
        (349, 349),
        "home city = the city stood on"
    );
    assert_eq!(owner_ok, on_city, "garrison owner = city owner u8@10");
}

#[test]
fn romfall_cities_and_first_unit_in_true_framing() {
    let Some(sav) = romfall() else {
        eprintln!("skipping: install not present");
        return;
    };
    assert_eq!(sav.units.len(), 710);
    assert_eq!(sav.cities.len(), 146);
    // Roma, Brundisium (east-south-east of it), Syracusae (south)
    let c: Vec<_> = sav.cities[..3]
        .iter()
        .map(|c| (c.name(), c.x(), c.y(), c.owner(), c.size()))
        .collect();
    assert_eq!(
        c,
        [
            ("Roma".to_string(), 47, 79, 1, 15),
            ("Brundisium".to_string(), 54, 84, 1, 5),
            ("Syracusae".to_string(), 47, 93, 1, 11),
        ]
    );
    // WRE sizes from the City Status report (evidence §T1-4)
    let sizes: Vec<u16> = sav.cities[..8].iter().map(|c| c.size()).collect();
    assert_eq!(sizes, [15, 5, 11, 6, 12, 4, 11, 10]);
    // unit 0 is a real unit (the old framing lost it); 510 is the battle-2
    // defender, a veteran B. Horsemen on (22,30) linked to 689
    let u0 = &sav.units[0];
    assert_eq!((u0.x(), u0.y(), u0.type_id(), u0.owner()), (65, 41, 6, 0));
    let d = &sav.units[510];
    assert_eq!((d.x(), d.y(), d.owner()), (22, 30, 0));
    assert!(d.is_veteran());
    assert_eq!(d.stack_prev(), 689);
}
