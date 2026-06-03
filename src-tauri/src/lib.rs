use tauri::{LogicalSize, Manager};

pub mod domain;
pub mod hook;
pub mod paths;
pub mod state;
pub mod store;
pub mod time;
pub mod watcher;

#[tauri::command]
fn hide_session(session_id: String, state: tauri::State<'_, state::AppState>, app: tauri::AppHandle) {
    {
        let mut store = state.store.write().expect("session store poisoned");
        store.hide(&session_id, chrono::Utc::now());
    }
    watcher::emit_visible_sessions(&app, &state.store);
}

#[tauri::command]
fn set_window_mode(mode: String, app: tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let size = match mode.as_str() {
        "island_expanded" => LogicalSize::new(640.0, 420.0),
        _ => LogicalSize::new(640.0, 120.0),
    };

    let _ = window.set_decorations(false);
    let _ = window.set_resizable(false);
    let _ = window.set_size(size);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![hide_session, set_window_mode])
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
