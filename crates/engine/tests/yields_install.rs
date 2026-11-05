//! T1-4 tile yields against game-written city records (env-gated on
//! CIVTOT_HOME). Evidence §T1-4 tile yields.

fn load(rel: &str, rules_rel: &str) -> Option<(engine::WorldState, tot_kit::Rules)> {
    let home = std::path::PathBuf::from(std::env::var_os("CIVTOT_HOME")?);
    let sav = tot_kit::parse_sav(&std::fs::read(home.join(rel)).ok()?).ok()?;
    let rules = tot_kit::parse_rules(&tot_kit::latin1(&std::fs::read(home.join(rules_rel)).ok()?));
    Some((engine::boot(&sav, 1), rules))
}

/// ROMFALL: food has no post-processing, so the worked-tile sum equals
/// the record (144/146); trade is stored after corruption, so it is never
/// above the tile sum.
#[test]
fn romfall_food_matches_and_trade_never_exceeds() {
    let Some((w, rules)) = load("FallRome/ROMFALL.scn", "FallRome/Rules.txt") else {
        eprintln!("skipping: CIVTOT_HOME not set");
        return;
    };
    let mut food_exact = 0;
    for (i, c) in w.cities.iter().enumerate() {
        let y = w.worked_yield(&rules, i);
        food_exact += (y[0] == c.gross_food as i32) as usize;
        assert!(
            c.gross_trade as i32 <= y[2],
            "{}: trade {} > tile sum {}",
            c.name,
            c.gross_trade,
            y[2]
        );
    }
    assert_eq!((food_exact, w.cities.len()), (144, 146));
}

/// A standard game's autosave (Original rules): every city's food is
/// exact, and food/shields/trade all match in at least 90 % of cities
/// (the rest: corruption/waste, not decoded yet). The rotating autosave
/// can be rewritten by the oracle, so only these properties are asserted.
#[test]
fn standard_game_tile_sums_match_records() {
    let Some((w, rules)) = load("Original/Ph_Auto2.SAV", "Original/Rules.txt") else {
        eprintln!("skipping: no standard-game autosave");
        return;
    };
    let mut all = 0;
    for (i, c) in w.cities.iter().enumerate() {
        let y = w.worked_yield(&rules, i);
        assert_eq!(y[0], c.gross_food as i32, "{} food", c.name);
        all += (y
            == [
                c.gross_food as i32,
                c.gross_shields as i32,
                c.gross_trade as i32,
            ]) as usize;
    }
    assert!(!w.cities.is_empty());
    assert!(all * 10 >= w.cities.len() * 9, "{all}/{}", w.cities.len());
    // after corruption and waste: the records' own totals (10/10 when measured)
    let totals = (0..w.cities.len())
        .filter(|&i| {
            let (o, c) = (w.city_output(&rules, i), &w.cities[i]);
            (o.food, o.shields, o.trade)
                == (
                    c.gross_food as i32,
                    c.gross_shields as i32,
                    c.gross_trade as i32,
                )
        })
        .count();
    assert!(
        totals * 10 >= w.cities.len() * 9,
        "totals {totals}/{}",
        w.cities.len()
    );
}

/// City totals after corruption, multipliers and waste (evidence §T1-4
/// city totals): ROMFALL trade exact in every city without trade routes,
/// shields include the decoded road connection to the capital.
#[test]
fn romfall_city_totals_after_corruption_and_waste() {
    let Some((w, rules)) = load("FallRome/ROMFALL.scn", "FallRome/Rules.txt") else {
        eprintln!("skipping: CIVTOT_HOME not set");
        return;
    };
    let home = std::path::PathBuf::from(std::env::var_os("CIVTOT_HOME").unwrap());
    let sav =
        tot_kit::parse_sav(&std::fs::read(home.join("FallRome/ROMFALL.scn")).unwrap()).unwrap();
    let (mut trade_ok, mut trade_n, mut shields_ok) = (0, 0, 0);
    for (i, c) in w.cities.iter().enumerate() {
        let o = w.city_output(&rules, i);
        if sav.cities[i].raw[60] == 0 {
            trade_n += 1;
            trade_ok += (o.trade == c.gross_trade as i32) as usize;
        }
        shields_ok += (o.shields == c.gross_shields as i32) as usize;
        if o.shields != c.gross_shields as i32 {
            eprintln!(
                "{} shields: engine {} stored {}",
                c.name, o.shields, c.gross_shields
            );
        }
    }
    assert_eq!(trade_ok, trade_n, "trade after corruption");
    assert_eq!(shields_ok, 145, "shields {shields_ok}/146");
}

/// Compare each radius yield directly to original x86 recomputation, so
/// stale city totals cannot conceal a tile-formula mismatch.
#[test]
fn native_radius_yields_match_reference_inputs() {
    if std::env::var_os("CIVTOT_HOME").is_none() {
        eprintln!("skipping: CIVTOT_HOME not set");
        return;
    }
    let mut cases = 0;
    for (name, rules_name) in [
        ("Original/Ph_Auto2.SAV", "Original/Rules.txt"),
        ("FallRome/ROMFALL.scn", "FallRome/Rules.txt"),
    ] {
        let (w, rules) =
            load(name, rules_name).expect("configured reference save and rules must load");
        for line in include_str!("../../../docs/evidence/tile-yield-vectors.txt")
            .lines()
            .filter(|l| l.starts_with(name))
        {
            let mut fields = line.split_whitespace();
            fields.next();
            let city: usize = fields.next().unwrap().parse().unwrap();
            let k: usize = fields.next().unwrap().parse().unwrap();
            let expected: Vec<i32> = fields.map(|s| s.parse().unwrap()).collect();
            assert_eq!(w.tile_yield(&rules, city, k), expected[..], "{line}");
            cases += 1;
        }
    }
    assert_eq!(cases, 3276);
}

#[test]
fn wonder_tile_bonuses_expire_when_another_civ_learns_the_advance() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(std::path::PathBuf::from) else {
        return;
    };
    let save = tot_kit::parse_sav_file(&home.join("Original/Ph_Auto2.SAV")).unwrap();
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(home.join("Original/Rules.txt")).unwrap(),
    ));
    let tree = engine::research::TechTree::from_rules(&rules);
    let obsolete: Vec<_> = [2, 8]
        .iter()
        .map(|&w| {
            tree.by_abbrev(&rules.section("ENDWONDER").unwrap().rows[w][0])
                .unwrap()
        })
        .collect();
    let mut cases = 0;
    for colossus in 0..2 {
        for richard in 0..2 {
            for expired in 0..2 {
                let mut w = engine::boot(&save, 1);
                w.wonders = [-1; 28];
                if colossus != 0 {
                    w.wonders[2] = 0;
                }
                if richard != 0 {
                    w.wonders[8] = 0;
                }
                let foreign = ((w.cities[0].owner & 15) as usize) % 7 + 1;
                for &t in &obsolete {
                    for civ in 1..8 {
                        w.techs[civ][t / 8] &= !(1 << (t % 8));
                    }
                    if expired != 0 {
                        w.techs[foreign][t / 8] |= 1 << (t % 8);
                    }
                }
                for line in include_str!("../../../docs/evidence/wonder-yield-vectors.txt")
                    .lines()
                    .filter(|l| !l.starts_with('#'))
                {
                    let v: Vec<i32> = line
                        .split_whitespace()
                        .map(|x| x.parse().unwrap())
                        .collect();
                    if v[..3] != [colossus, richard, expired] {
                        continue;
                    }
                    assert_eq!(w.tile_yield(&rules, 0, v[3] as usize), v[4..], "{line}");
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 168);
}
