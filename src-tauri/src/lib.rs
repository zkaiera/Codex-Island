use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::domain::SessionRecord;

pub mod codex_sessions;
pub mod domain;
pub mod hook;
pub mod paths;
pub mod startup;
pub mod state;
pub mod store;
pub mod time;
pub mod tray;
pub mod watcher;
pub mod windowing;

const PANEL_WINDOW_LABEL: &str = "panel";
const PANEL_OPEN_EVENT: &str = "session-panel:open";
const PANEL_CLOSE_EVENT: &str = "session-panel:close";
const PANEL_NATIVE_HIDE_DELAY_MS: u64 = 280;
const PANEL_CURSOR_WATCH_INTERVAL_MS: u64 = 120;
const PANEL_CURSOR_OUTSIDE_TICKS_TO_CLOSE: u8 = 3;

#[derive(Clone, Serialize)]
struct PanelOpenPayload {
    edge: Option<windowing::SnapEdge>,
    scrollable: bool,
}

#[tauri::command]
fn hide_session(
    session_id: String,
    state: tauri::State<'_, state::AppState>,
    app: tauri::AppHandle,
) {
    {
        let mut store = state.store.write().expect("session store poisoned");
        store.hide(&session_id, chrono::Utc::now());
    }
    watcher::emit_visible_sessions(&app, &state.store);
}

#[tauri::command]
fn get_sessions(state: tauri::State<'_, state::AppState>) -> Vec<SessionRecord> {
    watcher::refresh_store_from_disk(&state.store, &paths::default_state_dir());
    let store = state.store.read().expect("session store poisoned");
    store.recompute_visible(chrono::Utc::now())
}

#[tauri::command]
fn snap_window(app: tauri::AppHandle) -> Option<windowing::SnapEdge> {
    windowing::snap_main_window(&app)
}

#[tauri::command]
async fn snap_window_after_drag(app: tauri::AppHandle) -> Option<windowing::SnapEdge> {
    windowing::wait_for_primary_mouse_release().await;
    windowing::snap_main_window(&app)
}

#[tauri::command]
fn set_window_mode(
    mode: String,
    edge: Option<windowing::SnapEdge>,
    initial: bool,
    app: tauri::AppHandle,
) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let _ = window.set_decorations(false);
    let _ = window.set_resizable(false);
    let _ = windowing::apply_window_layout(
        &app,
        windowing::WindowMode::from_name(&mode),
        edge,
        initial,
    );
}

#[tauri::command]
fn show_session_panel(
    edge: Option<windowing::SnapEdge>,
    state: tauri::State<'_, state::AppState>,
    app: tauri::AppHandle,
) {
    {
        let mut panel_hover = state
            .panel_hover
            .lock()
            .expect("panel hover state poisoned");
        panel_hover.island_hovered = true;
    }

    let Some(main_window) = app.get_webview_window("main") else {
        return;
    };
    let Some(panel_window) = app.get_webview_window(PANEL_WINDOW_LABEL) else {
        return;
    };
    let Ok(position) = main_window.outer_position() else {
        return;
    };
    let Ok(size) = main_window.outer_size() else {
        return;
    };
    let Some(monitor) = main_window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())
    else {
        return;
    };

    let work_area = monitor.work_area();
    watcher::refresh_store_from_disk(&state.store, &paths::default_state_dir());
    let visible_session_count = {
        let store = state.store.read().expect("session store poisoned");
        store.recompute_visible(chrono::Utc::now()).len()
    };
    let frame = windowing::panel_frame_for_anchor(
        windowing::WindowFrame {
            x: position.x,
            y: position.y,
            width: size.width as i32,
            height: size.height as i32,
        },
        windowing::Rect {
            x: work_area.position.x,
            y: work_area.position.y,
            width: work_area.size.width as i32,
            height: work_area.size.height as i32,
        },
        edge,
        visible_session_count,
    );
    let scrollable =
        frame.height < windowing::panel_height_for_session_count(visible_session_count);

    windowing::apply_window_frame(&panel_window, frame);
    let _ = panel_window.show();
    let _ = panel_window.emit(PANEL_OPEN_EVENT, PanelOpenPayload { edge, scrollable });
}

#[tauri::command]
fn request_hide_session_panel(state: tauri::State<'_, state::AppState>, app: tauri::AppHandle) {
    {
        let mut panel_hover = state
            .panel_hover
            .lock()
            .expect("panel hover state poisoned");
        panel_hover.island_hovered = false;
    }

    emit_panel_close_if_unhovered(&app, &state);
}

#[tauri::command]
fn set_session_panel_hovered(
    hovered: bool,
    state: tauri::State<'_, state::AppState>,
    app: tauri::AppHandle,
) {
    {
        let mut panel_hover = state
            .panel_hover
            .lock()
            .expect("panel hover state poisoned");
        panel_hover.panel_hovered = hovered;
    }

    if hovered {
        start_panel_cursor_watch(app, state.panel_hover.clone());
    } else {
        emit_panel_close_if_unhovered(&app, &state);
    }
}

#[tauri::command]
fn hide_session_panel_window(state: tauri::State<'_, state::AppState>, app: tauri::AppHandle) {
    if panel_is_hovered(&state) {
        return;
    }

    if let Some(panel_window) = app.get_webview_window(PANEL_WINDOW_LABEL) {
        let _ = panel_window.hide();
    }
}

fn emit_panel_close_if_unhovered(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, state::AppState>,
) {
    let panel_hover = state.panel_hover.clone();
    emit_panel_close_if_unhovered_inner(app, panel_hover);
}

fn emit_panel_close_if_unhovered_inner(
    app: &tauri::AppHandle,
    panel_hover: Arc<Mutex<state::PanelHoverState>>,
) {
    if panel_hover_is_active(&panel_hover) {
        return;
    }

    if let Some(panel_window) = app.get_webview_window(PANEL_WINDOW_LABEL) {
        let _ = panel_window.emit(PANEL_CLOSE_EVENT, ());
        schedule_panel_native_hide(app.clone(), panel_hover);
    }
}

fn panel_is_hovered(state: &tauri::State<'_, state::AppState>) -> bool {
    panel_hover_is_active(&state.panel_hover)
}

fn panel_hover_is_active(panel_hover: &Arc<Mutex<state::PanelHoverState>>) -> bool {
    let panel_hover = panel_hover.lock().expect("panel hover state poisoned");
    panel_hover.island_hovered || panel_hover.panel_hovered
}

fn schedule_panel_native_hide(
    app: tauri::AppHandle,
    panel_hover: Arc<Mutex<state::PanelHoverState>>,
) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(PANEL_NATIVE_HIDE_DELAY_MS));
        if panel_hover_is_active(&panel_hover) {
            return;
        }

        if let Some(panel_window) = app.get_webview_window(PANEL_WINDOW_LABEL) {
            let _ = panel_window.hide();
        }
    });
}

fn start_panel_cursor_watch(
    app: tauri::AppHandle,
    panel_hover: Arc<Mutex<state::PanelHoverState>>,
) {
    {
        let mut hover = panel_hover.lock().expect("panel hover state poisoned");
        if hover.watch_active {
            return;
        }

        hover.watch_active = true;
    }

    std::thread::spawn(move || {
        let mut outside_ticks = 0;

        loop {
            std::thread::sleep(Duration::from_millis(PANEL_CURSOR_WATCH_INTERVAL_MS));
            {
                let mut hover = panel_hover.lock().expect("panel hover state poisoned");
                if !hover.panel_hovered {
                    hover.watch_active = false;
                    return;
                }
            }

            let Some(main_window) = app.get_webview_window("main") else {
                stop_panel_cursor_watch(&panel_hover);
                return;
            };
            let Some(panel_window) = app.get_webview_window(PANEL_WINDOW_LABEL) else {
                stop_panel_cursor_watch(&panel_hover);
                return;
            };

            if windowing::cursor_is_inside_any_window(&[main_window, panel_window]) {
                outside_ticks = 0;
                continue;
            }

            outside_ticks += 1;
            if outside_ticks < PANEL_CURSOR_OUTSIDE_TICKS_TO_CLOSE {
                continue;
            }

            {
                let mut hover = panel_hover.lock().expect("panel hover state poisoned");
                hover.island_hovered = false;
                hover.panel_hovered = false;
                hover.watch_active = false;
            }

            emit_panel_close_if_unhovered_inner(&app, panel_hover);
            return;
        }
    });
}

fn stop_panel_cursor_watch(panel_hover: &Arc<Mutex<state::PanelHoverState>>) {
    let mut hover = panel_hover.lock().expect("panel hover state poisoned");
    hover.watch_active = false;
}

fn setup_panel_window(app: &mut tauri::App) -> tauri::Result<()> {
    WebviewWindowBuilder::new(
        app,
        PANEL_WINDOW_LABEL,
        WebviewUrl::App("index.html?window=panel".into()),
    )
    .title("Codex Island Panel")
    .inner_size(
        windowing::PANEL_WIDTH_PX as f64,
        windowing::PANEL_INITIAL_HEIGHT_PX as f64,
    )
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .shadow(false)
    .resizable(false)
    .focused(false)
    .focusable(false)
    .accept_first_mouse(true)
    .visible(false)
    .build()?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_sessions,
            hide_session,
            hide_session_panel_window,
            set_window_mode,
            set_session_panel_hovered,
            show_session_panel,
            snap_window,
            snap_window_after_drag,
            request_hide_session_panel
        ])
        .setup(|app| {
            tray::setup_tray(app)?;
            setup_panel_window(app)?;
            let app_state = app.state::<state::AppState>();
            let watcher = watcher::start_session_sync(
                app.handle().clone(),
                app_state.store.clone(),
                paths::default_state_dir(),
            )?;
            let mut guard = app_state.watcher.lock().expect("watcher state poisoned");
            *guard = Some(watcher);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run Codex Island");
}
