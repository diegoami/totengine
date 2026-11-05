//! Original power recomputation from reference save inputs; install-backed.
#[test]
fn saved_power_matches_original_x86() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(std::path::PathBuf::from) else {
        eprintln!("skipping: CIVTOT_HOME not set");
        return;
    };
    for line in include_str!("../../../docs/evidence/power-saved-vectors.txt")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let mut fields = line.split_whitespace();
        let name = fields.next().unwrap();
        let expected: Vec<i32> = fields.map(|s| s.parse().unwrap()).collect();
        let sav = tot_kit::parse_sav_file(&home.join(name)).unwrap();
        let mut w = engine::boot(&sav, 1);
        assert_eq!(&w.power_scores().unwrap()[1..], &expected[..7], "{name}");
        w.refresh_power();
        assert_eq!(
            w.ai_state.power_ranks.map(i32::from),
            expected[7..15],
            "{name}"
        );
        assert_eq!(w.power_order.map(i32::from), expected[15..23], "{name}");
        assert_eq!(
            [
                w.research_settings.reference_civ as i32,
                w.ai_state.power_reference as i32
            ],
            expected[23..25],
            "{name}"
        );
    }
}
