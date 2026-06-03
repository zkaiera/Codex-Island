use std::path::{Path, PathBuf};

pub const APP_DIR_NAME: &str = "CodexIsland";
pub const INSTALL_DIR_NAME: &str = "Codex Island";
pub const SESSIONS_DIR_NAME: &str = "sessions";
pub const BIN_DIR_NAME: &str = "bin";

pub fn default_state_dir() -> PathBuf {
    choose_state_dir(
        std::env::var_os("CODEX_ISLAND_STATE_DIR").map(PathBuf::from),
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        std::env::var_os("USERPROFILE").map(PathBuf::from),
        std::env::current_exe().ok(),
        Some(Path::new("/mnt/c/Users")),
    )
}

pub fn default_tool_dir() -> PathBuf {
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        return PathBuf::from(local_app_data)
            .join(APP_DIR_NAME)
            .join(BIN_DIR_NAME);
    }

    std::env::temp_dir().join(APP_DIR_NAME).join(BIN_DIR_NAME)
}

pub fn session_file_path(state_dir: impl Into<PathBuf>, session_id: &str) -> PathBuf {
    state_dir.into().join(format!("{session_id}.json"))
}

pub fn choose_state_dir(
    configured_state_dir: Option<PathBuf>,
    local_app_data: Option<PathBuf>,
    user_profile: Option<PathBuf>,
    current_exe: Option<PathBuf>,
    wsl_users_root: Option<&Path>,
) -> PathBuf {
    if let Some(path) = configured_state_dir {
        return path;
    }

    if let Some(path) = local_app_data {
        return path.join(APP_DIR_NAME).join(SESSIONS_DIR_NAME);
    }

    if let Some(path) = user_profile {
        return path
            .join("AppData")
            .join("Local")
            .join(APP_DIR_NAME)
            .join(SESSIONS_DIR_NAME);
    }

    if let Some(path) = local_app_data_from_installed_exe(current_exe.as_deref()) {
        return path.join(APP_DIR_NAME).join(SESSIONS_DIR_NAME);
    }

    if let Some(path) = windows_local_app_data_from_wsl(wsl_users_root) {
        return path.join(APP_DIR_NAME).join(SESSIONS_DIR_NAME);
    }

    std::env::temp_dir()
        .join(APP_DIR_NAME)
        .join(SESSIONS_DIR_NAME)
}

fn local_app_data_from_installed_exe(current_exe: Option<&Path>) -> Option<PathBuf> {
    let install_dir = current_exe?.parent()?;
    if install_dir.file_name().and_then(|value| value.to_str()) != Some(INSTALL_DIR_NAME) {
        return None;
    }

    install_dir.parent().map(Path::to_path_buf)
}

fn windows_local_app_data_from_wsl(users_root: Option<&Path>) -> Option<PathBuf> {
    let users_root = users_root?;
    let entries = std::fs::read_dir(users_root).ok()?;

    let mut candidates: Vec<PathBuf> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            if is_windows_system_user(name) {
                return None;
            }

            let local_app_data = entry.path().join("AppData").join("Local");
            if local_app_data.is_dir() {
                Some(local_app_data)
            } else {
                None
            }
        })
        .collect();

    candidates.sort();
    candidates.into_iter().next()
}

fn is_windows_system_user(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "all users"
            | "default"
            | "default user"
            | "public"
            | "wsiaccount"
            | "codexsandboxoffline"
            | "codexsandboxonline"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wsl_state_dir_prefers_real_windows_user_local_app_data() {
        let users = tempfile::tempdir().unwrap();
        let real_user = users.path().join("15566").join("AppData").join("Local");
        let default_user = users.path().join("Default").join("AppData").join("Local");
        let wsi_account = users
            .path()
            .join("WsiAccount")
            .join("AppData")
            .join("Local");
        std::fs::create_dir_all(&real_user).unwrap();
        std::fs::create_dir_all(default_user).unwrap();
        std::fs::create_dir_all(wsi_account).unwrap();

        let result = choose_state_dir(None, None, None, None, Some(users.path()));

        assert_eq!(result, real_user.join(APP_DIR_NAME).join(SESSIONS_DIR_NAME));
    }

    #[test]
    fn state_dir_can_be_inferred_from_installed_hook_path() {
        let local_app_data = PathBuf::from(r"C:\Users\15566\AppData\Local");
        let current_exe = local_app_data
            .join(INSTALL_DIR_NAME)
            .join("codex-island-hook.exe");

        let result = choose_state_dir(None, None, None, Some(current_exe), None);

        assert_eq!(
            result,
            local_app_data.join(APP_DIR_NAME).join(SESSIONS_DIR_NAME)
        );
    }
}
