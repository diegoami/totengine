#[test]
fn original_forced_human_anarchy() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(std::path::PathBuf::from) else {
        return;
    };
    let save = tot_kit::parse_sav_file(&home.join("Original/Ph_Auto2.SAV")).unwrap();
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(home.join("Original/Rules.txt")).unwrap(),
    ));
    let mut cases = 0;
    for line in include_str!("../../../docs/evidence/anarchy-vectors.txt")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let v: Vec<i64> = line
            .split_whitespace()
            .map(|x| x.parse().unwrap())
            .collect();
        let mut w = engine::boot(&save, 1);
        let civ = v[0] as usize;
        for c in &mut w.cities {
            c.building = 8;
        }
        w.human_mask = 1 << civ;
        w.governments[civ] = v[1] as u8;
        for other in 0..8 {
            w.diplomacy[other][civ] = 0x135 + 32 * other as u32;
        }
        assert_eq!(w.force_human_anarchy(&rules, civ as u8), v[1] != 0);
        let mut actual = vec![w.governments[civ] as i64];
        actual.extend(w.diplomacy.iter().map(|d| d[civ] as i64));
        for c in &w.cities {
            actual.extend([
                c.worked as i64,
                c.specialists as i64,
                c.flags as i64,
                c.food_box as i16 as i64,
                c.route_trade as i64,
                c.net_science as i64,
                c.net_taxes as i64,
                c.gross_trade as i64,
                c.gross_food as i64,
                c.gross_shields as i64,
                c.happy as i64,
                c.unhappy as i64,
                c.building as i64,
            ]);
        }
        let diffs: Vec<_> = actual
            .iter()
            .zip(&v[2..])
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .collect();
        assert!(diffs.is_empty(), "civ {civ} previous {}: {diffs:?}", v[1]);
        cases += 1;
    }
    assert_eq!(cases, 49);
}
