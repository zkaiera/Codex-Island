use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use chrono::Utc;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, Runtime};

use crate::codex_sessions::{default_codex_session_roots, merge_sessions_with_codex_logs};
use crate::domain::SessionRecord;
use crate::store::SessionStore;

pub const SESSIONS_CHANGED_EVENT: &str = "sessions:changed";
pub const WATCH_DEBOUNCE_MS: u64 = 250;
pub const PERIODIC_RELOAD_SECS: u64 = 5;

pub fn watch_directory(path: &Path) -> notify::Result<(RecommendedWatcher, Receiver<PathBuf>)> {
    let (output_tx, output_rx) = mpsc::channel();
    let (input_tx, input_rx) = mpsc::channel();

    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        if let Ok(event) = result {
            for path in event.paths {
                let _ = input_tx.send(path);
            }
        }
    })?;

    watcher.watch(path, RecursiveMode::NonRecursive)?;

    std::thread::spawn(move || debounce_loop(input_rx, output_tx));

    Ok((watcher, output_rx))
}

pub fn start_session_sync<R: Runtime>(
    app: AppHandle<R>,
    store: std::sync::Arc<std::sync::RwLock<SessionStore>>,
    state_dir: PathBuf,
) -> notify::Result<RecommendedWatcher> {
    std::fs::create_dir_all(&state_dir)?;
    let initial_sessions = load_sessions_with_default_codex_logs(&state_dir, Utc::now());

    {
        let mut guard = store.write().expect("session store poisoned");
        guard.replace_all(initial_sessions);
    }
    emit_visible_sessions(&app, &store);

    let reload_state_dir = state_dir.clone();
    let (watcher, output_rx) = watch_directory(&state_dir)?;

    {
        let app = app.clone();
        let store = store.clone();
        std::thread::spawn(move || {
            while output_rx.recv().is_ok() {
                reload_sessions(&app, &store, &reload_state_dir);
            }
        });
    }

    {
        let app = app.clone();
        let store = store.clone();
        let reload_state_dir = state_dir.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(PERIODIC_RELOAD_SECS));
            reload_sessions(&app, &store, &reload_state_dir);
        });
    }

    Ok(watcher)
}

pub fn load_sessions_from_dir(state_dir: &Path) -> Vec<SessionRecord> {
    let mut sessions = Vec::new();

    let entries = match std::fs::read_dir(state_dir) {
        Ok(entries) => entries,
        Err(_) => return sessions,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(session) = load_session_file(&path) {
            sessions.push(session);
        }
    }

    sessions
}

pub fn load_sessions_with_codex_logs(
    state_dir: &Path,
    codex_session_roots: &[PathBuf],
    now: chrono::DateTime<Utc>,
) -> Vec<SessionRecord> {
    let sessions = load_sessions_from_dir(state_dir);
    merge_sessions_with_codex_logs(sessions, codex_session_roots, now)
}

fn load_sessions_with_default_codex_logs(
    state_dir: &Path,
    now: chrono::DateTime<Utc>,
) -> Vec<SessionRecord> {
    let sessions = load_sessions_from_dir(state_dir);
    let codex_session_roots = default_codex_session_roots(&sessions);
    merge_sessions_with_codex_logs(sessions, &codex_session_roots, now)
}

pub fn emit_visible_sessions<R: Runtime>(
    app: &AppHandle<R>,
    store: &std::sync::Arc<std::sync::RwLock<SessionStore>>,
) {
    let sessions = {
        let guard = store.read().expect("session store poisoned");
        guard.recompute_visible(Utc::now())
    };

    let _ = app.emit(SESSIONS_CHANGED_EVENT, sessions);
}

fn load_session_file(path: &Path) -> Option<SessionRecord> {
    if path.extension().and_then(|value| value.to_str()) != Some("json") {
        return None;
    }

    if is_smoke_status_file(path) {
        return None;
    }

    let body = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&body).ok()
}

fn is_smoke_status_file(path: &Path) -> bool {
    path.file_stem()
        .and_then(|value| value.to_str())
        .map(|stem| stem.starts_with("codex-island-smoke-"))
        .unwrap_or(false)
}

fn reload_sessions<R: Runtime>(
    app: &AppHandle<R>,
    store: &std::sync::Arc<std::sync::RwLock<SessionStore>>,
    state_dir: &Path,
) {
    let sessions = load_sessions_with_default_codex_logs(state_dir, Utc::now());
    {
        let mut guard = store.write().expect("session store poisoned");
        guard.replace_all(sessions);
    }
    emit_visible_sessions(app, store);
}

fn debounce_loop(input_rx: Receiver<PathBuf>, output_tx: mpsc::Sender<PathBuf>) {
    while let Ok(first_path) = input_rx.recv() {
        let mut latest_path = first_path;

        while let Ok(next_path) = input_rx.recv_timeout(Duration::from_millis(WATCH_DEBOUNCE_MS)) {
            latest_path = next_path;
        }

        let _ = output_tx.send(latest_path);
    }
}
