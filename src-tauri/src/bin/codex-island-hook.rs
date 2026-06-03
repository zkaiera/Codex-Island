#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = codex_island_lib::hook::run_from_stdin() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
