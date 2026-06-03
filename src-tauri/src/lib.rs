use tauri::Manager;

pub mod domain;
pub mod hook;
pub mod paths;
pub mod state;
pub mod store;
pub mod time;
pub mod watcher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(state::AppState::default())
        .setup(|app| {
            let app_state = app.state::<state::AppState>();
            let watcher = watcher::start_session_sync(
                app.handle().clone(),
                app_state.store.clone(),
                paths::default_state_dir(),
            )?;
            let mut guard = app_state
                .watcher
                .lock()
                .expect("watcher state poisoned");
            *guard = Some(watcher);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run Codex Island");
}
