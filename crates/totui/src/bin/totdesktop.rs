#![cfg_attr(windows, windows_subsystem = "windows")]
fn main() {
    let client = totui::client::Client::new(
        std::env::var("CIVTOT_HOME").unwrap_or_default(),
        totui::client::storage::default_folder(),
    );
    if let Err(error) = totui::client::desktop::run(client) {
        let _ = sdl2::messagebox::show_simple_message_box(
            sdl2::messagebox::MessageBoxFlag::ERROR,
            "ToTEngine",
            &error,
            None,
        );
        eprintln!("{error}");
        std::process::exit(1);
    }
}
