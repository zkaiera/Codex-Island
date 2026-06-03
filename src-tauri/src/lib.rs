use tauri::{LogicalSize, Manager};

pub mod domain;
pub mod hook;
pub mod install;
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
fn get_setup_snippets() -> install::SetupSnippets {
    install::current_setup_snippets()
}

#[tauri::command]
fn install_hooks() -> install::HookInstallReport {
    install::install_codex_hooks()
}

#[tauri::command]
fn set_window_mode(mode: String, app: tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let size = if mode == "setup" {
        LogicalSize::new(760.0, 720.0)
    } else {
        LogicalSize::new(640.0, 120.0)
    };

    let _ = window.set_size(size);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            hide_session,
            get_setup_snippets,
            install_hooks,
            set_window_mode
        ])
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
