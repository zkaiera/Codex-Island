use tauri::{LogicalSize, Manager};

use crate::domain::SessionRecord;

pub mod domain;
pub mod hook;
pub mod paths;
pub mod state;
pub mod store;
pub mod startup;
pub mod time;
pub mod tray;
pub mod watcher;
pub mod windowing;

#[tauri::command]
fn hide_session(session_id: String, state: tauri::State<'_, state::AppState>, app: tauri::AppHandle) {
    {
        let mut store = state.store.write().expect("session store poisoned");
        store.hide(&session_id, chrono::Utc::now());
    }
    watcher::emit_visible_sessions(&app, &state.store);
}

#[tauri::command]
fn get_sessions(state: tauri::State<'_, state::AppState>) -> Vec<SessionRecord> {
    let store = state.store.read().expect("session store poisoned");
    store.recompute_visible(chrono::Utc::now())
}

#[tauri::command]
fn snap_window(app: tauri::AppHandle) -> Option<windowing::SnapEdge> {
    windowing::snap_main_window(&app)
}

#[tauri::command]
fn set_window_mode(mode: String, app: tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let size = match mode.as_str() {
        "island_expanded" => LogicalSize::new(640.0, 420.0),
        _ => LogicalSize::new(180.0, 48.0),
    };

    let _ = window.set_decorations(false);
    let _ = window.set_resizable(false);
    let _ = window.set_size(size);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_sessions,
            hide_session,
            set_window_mode,
            snap_window
        ])
        .setup(|app| {
            tray::setup_tray(app)?;
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
