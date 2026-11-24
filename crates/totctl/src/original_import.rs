use std::{path::Path, process::ExitCode};
use totui::resources::original::OriginalCapture;

pub fn run(args: &[String]) -> ExitCode {
    let [install, relative] = args else {
        eprintln!("usage: totctl inspect-original-save <install> <Original/file.sav>");
        return ExitCode::from(2);
    };
    match OriginalCapture::capture(Path::new(install), relative)
        .map_err(|error| format!("dependency capture: {error}"))
        .and_then(|capture| capture.inspect_save())
    {
        Ok(report) => {
            println!("{report:#}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("inspect-original-save: {error}");
            ExitCode::FAILURE
        }
    }
}

pub fn prepare_replay(args: &[String]) -> ExitCode {
    let [install, relative, saves, name] = args else {
        eprintln!("usage: totctl prepare-original-replay <install> <Original/file.sav> <save-directory> <name>");
        return ExitCode::from(2);
    };
    let result = OriginalCapture::capture(Path::new(install), relative).and_then(|capture| {
        let checkpoint = capture.prepare_replay_checkpoint()?;
        totui::client::storage::save_scenario(
            Path::new(saves),
            name,
            &checkpoint,
            &capture.files().bytes,
        )
    });
    match result {
        Ok(()) => {
            println!("Experimental engine checkpoint saved as {name}. Open it with Load engine checkpoint.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("prepare-original-replay: {error}");
            ExitCode::FAILURE
        }
    }
}
