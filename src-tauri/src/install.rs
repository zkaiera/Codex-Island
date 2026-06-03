use serde::Serialize;

use crate::paths::default_state_dir;

#[derive(Clone, Debug, Serialize)]
pub struct SetupSnippets {
    pub windows: String,
    pub wsl: String,
    pub state_dir: String,
}

pub fn current_setup_snippets() -> SetupSnippets {
    let state_dir = default_state_dir().display().to_string();
    let current_exe = std::env::current_exe()
        .ok()
        .and_then(|path| path.to_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "codex-island-hook.exe".to_string());
    build_install_snippets(&current_exe, &state_dir)
}

pub fn build_install_snippets(hook_binary: &str, state_dir: &str) -> SetupSnippets {
    let wsl_hook_binary = to_wsl_path(hook_binary);
    let wsl_state_dir = to_wsl_path(state_dir);

    SetupSnippets {
        windows: format!(
            r#"{{
  "hooks": {{
    "SessionStart": [{{ "hooks": [{{ "type": "command", "command": "\"{hook_binary}\"", "timeout": 5 }}] }}],
    "UserPromptSubmit": [{{ "hooks": [{{ "type": "command", "command": "\"{hook_binary}\"", "timeout": 5 }}] }}],
    "PermissionRequest": [{{ "hooks": [{{ "type": "command", "command": "\"{hook_binary}\"", "timeout": 5 }}] }}],
    "PreToolUse": [{{ "hooks": [{{ "type": "command", "command": "\"{hook_binary}\"", "timeout": 5 }}] }}],
    "PostToolUse": [{{ "hooks": [{{ "type": "command", "command": "\"{hook_binary}\"", "timeout": 5 }}] }}],
    "Stop": [{{ "hooks": [{{ "type": "command", "command": "\"{hook_binary}\"", "timeout": 5 }}] }}]
  }}
}}

# 状态目录
{state_dir}"#
        ),
        wsl: format!(
            r#"{{
  "hooks": {{
    "SessionStart": [{{ "hooks": [{{ "type": "command", "command": "\"{wsl_hook_binary}\"", "timeout": 5 }}] }}],
    "UserPromptSubmit": [{{ "hooks": [{{ "type": "command", "command": "\"{wsl_hook_binary}\"", "timeout": 5 }}] }}],
    "PermissionRequest": [{{ "hooks": [{{ "type": "command", "command": "\"{wsl_hook_binary}\"", "timeout": 5 }}] }}],
    "PreToolUse": [{{ "hooks": [{{ "type": "command", "command": "\"{wsl_hook_binary}\"", "timeout": 5 }}] }}],
    "PostToolUse": [{{ "hooks": [{{ "type": "command", "command": "\"{wsl_hook_binary}\"", "timeout": 5 }}] }}],
    "Stop": [{{ "hooks": [{{ "type": "command", "command": "\"{wsl_hook_binary}\"", "timeout": 5 }}] }}]
  }}
}}

# 状态目录
{wsl_state_dir}"#
        ),
        state_dir: state_dir.to_string(),
    }
}

fn to_wsl_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    if normalized.len() > 2 && normalized.as_bytes()[1] == b':' {
        let drive = normalized[..1].to_ascii_lowercase();
        let rest = normalized[2..].trim_start_matches('/');
        format!("/mnt/{drive}/{rest}")
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::build_install_snippets;

    #[test]
    fn generates_separate_windows_and_wsl_snippets() {
        let snippets = build_install_snippets(
            "C:\\Program Files\\Codex Island\\codex-island-hook.exe",
            "C:\\Users\\zk\\AppData\\Local\\CodexIsland\\sessions",
        );

        assert!(snippets.windows.contains("codex-island-hook.exe"));
        assert!(snippets.windows.contains("C:\\Users\\zk\\AppData\\Local\\CodexIsland\\sessions"));
        assert!(snippets.wsl.contains("/mnt/c/Program Files/Codex Island/codex-island-hook.exe"));
        assert!(snippets.wsl.contains("/mnt/c/Users/zk/AppData/Local/CodexIsland/sessions"));
    }
}
