//! Scenario dates against the installed files (#330): the original's turn-
//! to-date conversion and formatter, with each folder's `Labels.txt`.
//! Self-skips without `CIVTOT_HOME`.
use engine::calendar::date_text;
use std::path::{Path, PathBuf};

fn labels(dir: &Path, home: &Path) -> Option<tot_kit::Labels> {
    [dir.join("Labels.txt"), home.join("Labels.txt")]
        .iter()
        .find_map(|p| std::fs::read(p).ok())
        .and_then(|b| tot_kit::Labels::parse(&tot_kit::latin1(&b)))
}

/// The date the engine shows for `rel` at boot and after `turns` turns.
fn dates(home: &Path, rel: &str, turns: usize) -> Vec<String> {
    let path = home.join(rel);
    let sav = tot_kit::parse_sav_file(&path).unwrap();
    let mut world = engine::boot(&sav, 0);
    let l = labels(path.parent().unwrap(), home);
    let mut out = Vec::new();
    for i in 0..=turns {
        if i > 0 {
            world.advance_turn();
        }
        let (n, style) = world.date().expect("located calendar");
        out.push(date_text(n, style, l.as_ref()));
    }
    out
}

#[test]
fn installed_scenarios_read_their_own_calendars() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(PathBuf::from) else {
        return;
    };
    // FallRome: five months a turn from Dec 429 (raw 5159, the date the
    // original showed for the turn-0 save EDITEST2).
    assert_eq!(
        dates(&home, "FallRome/ROMFALL.scn", 2),
        ["Dec 429", "May 430", "Oct 430"]
    );
    // The original ignores an edited year field (EDITEST: year -3000).
    assert_eq!(
        dates(&home, "Scenario/FallRome/EDITEST.SAV", 0),
        ["Dec 429"]
    );
    // Month calendars print the year without an era, as the original does.
    assert_eq!(
        dates(&home, "Scenario/Hannibal/Hannibal.scn", 1),
        ["Oct 219", "Nov 219"]
    );
    assert_eq!(
        dates(&home, "Scenario/Roman Civil Wars/RomCivWars.SCN", 1),
        ["Dec 55", "Feb 54"]
    );
    // Constantine's Labels.txt is read, but its month labels are standard.
    assert_eq!(
        dates(&home, "Scenario/ToT_Constantine/Const.scn", 0),
        ["Aug 306"]
    );
    // AE 1200 BC marks its .scn 0x40, not 0x80: the loader promotes it, so
    // the scenario calendar applies, matching the original's own autosave.
    assert_eq!(
        dates(&home, "Scenario/ToT_AE-1200BC/ae-1200b.scn", 1),
        ["1200 B.C.", "1195 B.C."]
    );
    assert_eq!(
        dates(&home, "Scenario/ToT_AE-1200BC/Ag_Auto.SAV", 0),
        ["1200 B.C."]
    );
    // Standard games: the difficulty table.
    assert_eq!(dates(&home, "Original/Ph_Auto.SAV", 0), ["1800 B.C."]);
}

#[test]
fn renamed_eras_come_from_the_folder_labels() {
    let Some(home) = std::env::var_os("CIVTOT_HOME").map(PathBuf::from) else {
        return;
    };
    let l = labels(&home.join("Scenario/ToT_Constantine"), &home).unwrap();
    assert_eq!(l.get(1), Some("Anno Domini"));
    let fantasy = labels(&home.join("Fantasy"), &home).unwrap();
    assert_eq!(
        date_text(-3000, engine::calendar::DateStyle::Years, Some(&fantasy)),
        "3000 First Age"
    );
}
