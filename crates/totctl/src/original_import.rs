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
