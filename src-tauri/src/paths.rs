use std::path::PathBuf;

pub const APP_DIR_NAME: &str = "CodexIsland";
pub const SESSIONS_DIR_NAME: &str = "sessions";

pub fn default_state_dir() -> PathBuf {
    if let Ok(path) = std::env::var("CODEX_ISLAND_STATE_DIR") {
        return PathBuf::from(path);
    }

    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        return PathBuf::from(local_app_data)
            .join(APP_DIR_NAME)
            .join(SESSIONS_DIR_NAME);
    }

    std::env::temp_dir().join(APP_DIR_NAME).join(SESSIONS_DIR_NAME)
}

pub fn session_file_path(state_dir: impl Into<PathBuf>, session_id: &str) -> PathBuf {
    state_dir.into().join(format!("{session_id}.json"))
}
