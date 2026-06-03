fn main() {
    if let Err(error) = codex_island_lib::hook::run_from_stdin() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
