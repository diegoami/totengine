#![cfg_attr(windows, windows_subsystem = "windows")]
fn main() {
    let mut open = true;
    let mut port = 0;
    for arg in std::env::args().skip(1) {
        if arg == "--no-open" {
            open = false
        } else if let Some(value) = arg.strip_prefix("--port=") {
            port = value.parse().expect("invalid port")
        } else {
            eprintln!("usage: totplay [--no-open] [--port=N]");
            return;
        }
    }
    let home = std::env::var("CIVTOT_HOME").unwrap_or_default();
    let client = totui::client::Client::new(home, totui::client::storage::default_folder());
    if let Err(error) = totui::client::browser::serve(client, open, port) {
        eprintln!("{error}");
        std::process::exit(1)
    }
}
