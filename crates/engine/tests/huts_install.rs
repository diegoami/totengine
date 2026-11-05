//! Controlled same-seed entries in the live ToTPP oracle. The fixture
//! separates presentation CRT calls; see evidence/huts-live.json.
use engine::huts::HutOutcome;

#[test]
fn live_hut_entries_match_outcomes_mutations_and_gameplay_rng() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        return;
    };
    let home = std::path::PathBuf::from(home);
    let save =
        tot_kit::parse_sav(&std::fs::read(home.join("Original/Ph_Auto2.SAV")).unwrap()).unwrap();
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(home.join("Original/Rules.txt")).unwrap(),
    ));
    let mut world = engine::boot(&save, 1);
    let mut explorer = 0;
    let mut name = "";
    let mut outcome = HutOutcome::Nothing;
    let mut cases = 0;
    for line in include_str!("../../../docs/evidence/huts-live-vectors.txt")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let v: Vec<&str> = line.split_whitespace().collect();
        let number = |i: usize| v[i].parse::<i64>().unwrap();
        match v[0] {
            "case" => {
                name = v[1];
                world = engine::boot(&save, number(2) as u64);
                explorer = number(3) as usize;
                let (x, y) = (number(4) as u16, number(5) as u16);
                world.units[explorer].x = x;
                world.units[explorer].y = y;
                let i = world.map.stored_index(x, y).unwrap();
                world.map.fertility[i] = number(6) as u8;
                world.map.site_bits[i] = number(7) as u8;
                world.map.owner[i] = 15;
                assert!(
                    world.map.hut_at(x, y),
                    "{name}: original selected a hut here"
                );
                // movement claims the tile before dispatching the hut.
                world.map.owner[i] = world.units[explorer].owner;
                world.turn = number(8) as i16;
                world.year = number(9) as i16;
                world.seed_word = (world.seed_word & 0xff00) | number(10) as u16;
                world.combat_rng = engine::combat::CrtRand::new(number(2) as u32);
            }
            "home" => world.units[number(1) as usize].home = number(2) as u8,
            "result" => {
                outcome = world.enter_hut(&rules, explorer);
                let expected = match v[1] {
                    "city" => HutOutcome::City(number(2) as usize),
                    "unit" => HutOutcome::Unit(number(2) as usize),
                    "gold" => HutOutcome::Gold(number(2) as i32),
                    "advance" => HutOutcome::Advance(number(2) as usize),
                    "nothing" => HutOutcome::Nothing,
                    "barbarians" => {
                        assert!(
                            matches!(&outcome, HutOutcome::Barbarians(ids) if ids.len() == number(2) as usize),
                            "{name}: {outcome:?}"
                        );
                        outcome.clone()
                    }
                    _ => panic!("unknown fixture kind"),
                };
                assert_eq!(outcome, expected, "{name}");
                assert_eq!(
                    world.combat_rng.state(),
                    number(3) as u32,
                    "{name}: CRT state"
                );
                assert_eq!(
                    world.combat_rng.draws(),
                    number(5) as u64,
                    "{name}: gameplay draw count"
                );
                let civ = world.units[explorer].owner as usize;
                assert_eq!(world.treasury[civ], number(4) as i32, "{name}: treasury");
                let expected: Vec<u8> = v[6..].iter().map(|s| s.parse().unwrap()).collect();
                assert_eq!(
                    world.techs[civ].as_slice(),
                    expected.as_slice(),
                    "{name}: knowledge"
                );
            }
            "unit" => {
                let idx = number(1) as usize;
                let unit = &world.units[idx];
                assert_eq!(
                    (
                        unit.type_id,
                        unit.owner,
                        unit.x,
                        unit.y,
                        unit.home,
                        unit.veteran
                    ),
                    (
                        number(2) as u8,
                        number(3) as u8,
                        number(4) as u16,
                        number(5) as u16,
                        number(6) as u8,
                        number(7) != 0
                    ),
                    "{name}"
                );
                assert_eq!(
                    unit.moves_left,
                    rules.unit_move(unit.type_id as usize).unwrap() * 3,
                    "{name}: original spent zero movement"
                );
                assert_eq!(
                    unit.stack_pos, 0,
                    "{name}: original creation inserts at head"
                );
                if matches!(&outcome, HutOutcome::Barbarians(ids) if !ids.contains(&idx)) {
                    panic!("{name}: missing barbarian slot {idx}");
                }
            }
            "city" => {
                let city = &world.cities[number(1) as usize];
                assert_eq!(
                    (city.size, city.owner),
                    (number(2) as u16, number(3) as u16),
                    "{name}"
                );
                let expected: Vec<u8> = v[4..].iter().map(|s| s.parse().unwrap()).collect();
                assert_eq!(
                    city.buildings.as_slice(),
                    expected.as_slice(),
                    "{name}: improvements"
                );
            }
            "explorer-home" => assert_eq!(
                world.units[explorer].home,
                number(1) as u8,
                "{name}: explorer home"
            ),
            "end" => cases += 1,
            _ => panic!("unknown fixture record: {line}"),
        }
    }
    assert_eq!(cases, 8);
}
