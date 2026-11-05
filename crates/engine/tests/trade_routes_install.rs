//! Stored route fields and live cache refresh; original assets stay local.
#[test]
fn saved_routes_and_turn_cache_refresh() {
    let Some(home) = std::env::var_os("CIVTOT_HOME") else {
        return;
    };
    let home = std::path::PathBuf::from(home);
    let save =
        tot_kit::parse_sav(&std::fs::read(home.join("FallRome/ROMFALL.scn")).unwrap()).unwrap();
    let rules = tot_kit::parse_rules(&tot_kit::latin1(
        &std::fs::read(home.join("FallRome/Rules.txt")).unwrap(),
    ));
    let mut w = engine::boot(&save, 1);
    let mut routes = 0;
    for (i, c) in w.cities.iter().enumerate() {
        assert_eq!(c.route_trade, save.cities[i].field32() as i16);
        if c.route_count > 0 {
            routes += c.route_count as usize;
            assert_eq!(i, 16);
            assert_eq!(w.trade_route_income(&rules, i, c.route_trade), [0; 3]);
        }
    }
    assert_eq!(routes, 1);
    w.run_turn(&rules, None);
    for (i, c) in w
        .cities
        .iter()
        .enumerate()
        .filter(|(_, c)| c.gross_food != 0)
    {
        // Production can complete an improvement after this cache is written.
        // The cache was evaluated from the pre-production buildings.
        let mut before_build = w.clone();
        before_build.cities[i].buildings = save.cities[i].buildings();
        assert_eq!(
            c.route_trade,
            before_build.pre_route_trade(&rules, i),
            "city {i}"
        );
    }
}
