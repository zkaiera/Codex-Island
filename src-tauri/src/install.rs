use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::Serialize;
use serde_json::{json, Value};
use thiserror::Error;

use crate::paths::{default_state_dir, default_tool_dir};

const HOOK_EVENTS: [&str; 6] = [
    "SessionStart",
    "UserPromptSubmit",
    "PermissionRequest",
    "PreToolUse",
    "PostToolUse",
    "Stop",
];
const HOOK_TIMEOUT_SECONDS: u64 = 5;
const HOOK_BINARY_STEM: &str = "codex-island-hook";
const HOOKS_FILE_NAME: &str = "hooks.json";

#[derive(Clone, Debug, Serialize)]
pub struct SetupSnippets {
    pub windows: String,
    pub wsl: String,
    pub state_dir: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct HookInstallReport {
    pub windows: HookInstallTargetReport,
    pub wsl: HookInstallTargetReport,
    pub trust_steps: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct HookInstallTargetReport {
    pub label: String,
    pub status: HookInstallStatus,
    pub path: Option<String>,
    pub backup_path: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HookInstallStatus {
    Installed,
    AlreadyInstalled,
    Unavailable,
    Failed,
}

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("hook 配置 JSON 无效：{0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("hook 配置根节点必须是 JSON 对象")]
    InvalidRoot,
    #[error("hook 配置中的 hooks 字段必须是 JSON 对象")]
    InvalidHooksRoot,
    #[error("{0} 事件的 hook 配置必须是数组")]
    InvalidEventHooks(String),
    #[error("文件读写失败：{0}")]
    Io(#[from] std::io::Error),
}

struct WrittenHookConfig {
    changed: bool,
    backup_path: Option<PathBuf>,
}

pub fn current_setup_snippets() -> SetupSnippets {
    let state_dir = default_state_dir().display().to_string();
    let hook_binary = current_hook_binary_path()
        .to_str()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("{HOOK_BINARY_STEM}.exe"));
    build_install_snippets(&hook_binary, &state_dir)
}

pub fn install_codex_hooks() -> HookInstallReport {
    let hook_binary = current_hook_binary_path();
    let windows = install_windows_hooks(&hook_binary);
    let wsl = install_wsl_hooks(&hook_binary);

    HookInstallReport {
        windows,
        wsl,
        trust_steps: vec![
            "重启或新开 Codex 会话。".to_string(),
            "当 Codex 提示 hook 需要信任时，手动选择信任或允许。".to_string(),
            "不要使用 --dangerously-bypass-hook-trust；这个工具不会绕过 Codex 的信任机制。"
                .to_string(),
        ],
    }
}

pub fn build_install_snippets(hook_binary: &str, state_dir: &str) -> SetupSnippets {
    let windows_command = quote_command(hook_binary);
    let wsl_hook_binary = to_wsl_path(hook_binary);
    let wsl_state_dir = to_wsl_path(state_dir);
    let wsl_command = quote_command(&wsl_hook_binary);

    SetupSnippets {
        windows: format!(
            "{}\n\n# 状态目录\n{state_dir}",
            build_hooks_json_snippet(&windows_command)
        ),
        wsl: format!(
            "{}\n\n# 状态目录\n{wsl_state_dir}",
            build_hooks_json_snippet(&wsl_command)
        ),
        state_dir: state_dir.to_string(),
    }
}

pub fn merge_hooks_config(
    existing_config: Option<&str>,
    command: &str,
) -> Result<(String, bool), InstallError> {
    let mut root = match existing_config.map(str::trim).filter(|body| !body.is_empty()) {
        Some(body) => serde_json::from_str::<Value>(body)?,
        None => json!({}),
    };

    let root_object = root.as_object_mut().ok_or(InstallError::InvalidRoot)?;
    let hooks_value = root_object.entry("hooks").or_insert_with(|| json!({}));
    let hooks_object = hooks_value
        .as_object_mut()
        .ok_or(InstallError::InvalidHooksRoot)?;

    let mut changed = false;
    for event in HOOK_EVENTS {
        let event_hooks_value = hooks_object.entry(event).or_insert_with(|| json!([]));
        let event_hooks = event_hooks_value
            .as_array_mut()
            .ok_or_else(|| InstallError::InvalidEventHooks(event.to_string()))?;

        let original_len = event_hooks.len();
        event_hooks.retain(|entry| {
            !entry_contains_codex_island_hook(entry) || entry_contains_command(entry, command)
        });
        if event_hooks.len() != original_len {
            changed = true;
        }

        if !event_hooks.iter().any(|entry| entry_contains_command(entry, command)) {
            event_hooks.push(build_hook_event_entry(command));
            changed = true;
        }
    }

    let formatted = serde_json::to_string_pretty(&root)?;
    Ok((format!("{formatted}\n"), changed))
}

fn install_windows_hooks(hook_binary: &Path) -> HookInstallTargetReport {
    let command = quote_command(&hook_binary.display().to_string());
    let hooks_path = codex_hooks_path();

    match install_local_hooks_file(&hooks_path, &command) {
        Ok(result) => build_success_report(
            "Windows Codex",
            hooks_path,
            result,
            "Windows hooks 已写入。后续需要在 Codex 中手动信任。",
        ),
        Err(error) => HookInstallTargetReport {
            label: "Windows Codex".to_string(),
            status: HookInstallStatus::Failed,
            path: Some(hooks_path.display().to_string()),
            backup_path: None,
            message: error.to_string(),
        },
    }
}

fn install_wsl_hooks(hook_binary: &Path) -> HookInstallTargetReport {
    let command = quote_command(&to_wsl_path(&hook_binary.display().to_string()));
    let hooks_path = match read_wsl_hooks_path() {
        Ok(path) => path,
        Err(error) => {
            return HookInstallTargetReport {
                label: "WSL Codex".to_string(),
                status: HookInstallStatus::Unavailable,
                path: None,
                backup_path: None,
                message: format!("没有检测到可用的 wsl.exe，或无法读取 WSL Codex 配置路径：{error}"),
            };
        }
    };

    let existing = match read_wsl_hooks_file() {
        Ok(config) => config,
        Err(error) => {
            return HookInstallTargetReport {
                label: "WSL Codex".to_string(),
                status: HookInstallStatus::Failed,
                path: Some(hooks_path.clone()),
                backup_path: None,
                message: format!("读取 WSL hook 配置失败：{error}"),
            };
        }
    };

    let (merged, changed) = match merge_hooks_config(existing.as_deref(), &command) {
        Ok(result) => result,
        Err(error) => {
            return HookInstallTargetReport {
                label: "WSL Codex".to_string(),
                status: HookInstallStatus::Failed,
                path: Some(hooks_path.clone()),
                backup_path: None,
                message: error.to_string(),
            };
        }
    };

    if !changed {
        return HookInstallTargetReport {
            label: "WSL Codex".to_string(),
            status: HookInstallStatus::AlreadyInstalled,
            path: Some(hooks_path),
            backup_path: None,
            message: "WSL hooks 已存在，无需重复写入。".to_string(),
        };
    }

    match write_wsl_hooks_file(&merged) {
        Ok(backup_path) => HookInstallTargetReport {
            label: "WSL Codex".to_string(),
            status: HookInstallStatus::Installed,
            path: Some(hooks_path),
            backup_path,
            message: "WSL hooks 已写入。后续需要在 Codex 中手动信任。".to_string(),
        },
        Err(error) => HookInstallTargetReport {
            label: "WSL Codex".to_string(),
            status: HookInstallStatus::Failed,
            path: Some(hooks_path),
            backup_path: None,
            message: format!("写入 WSL hook 配置失败：{error}"),
        },
    }
}

fn install_local_hooks_file(
    hooks_path: &Path,
    command: &str,
) -> Result<WrittenHookConfig, InstallError> {
    let existing = match fs::read_to_string(hooks_path) {
        Ok(body) => Some(body),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };

    let (merged, changed) = merge_hooks_config(existing.as_deref(), command)?;
    if !changed {
        return Ok(WrittenHookConfig {
            changed,
            backup_path: None,
        });
    }

    if let Some(parent) = hooks_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let backup_path = if hooks_path.exists() {
        let backup_path = hooks_path.with_extension("json.codex-island.bak");
        fs::copy(hooks_path, &backup_path)?;
        Some(backup_path)
    } else {
        None
    };

    atomic_write(hooks_path, merged.as_bytes())?;
    Ok(WrittenHookConfig {
        changed,
        backup_path,
    })
}

fn build_success_report(
    label: &str,
    hooks_path: PathBuf,
    result: WrittenHookConfig,
    installed_message: &str,
) -> HookInstallTargetReport {
    HookInstallTargetReport {
        label: label.to_string(),
        status: if result.changed {
            HookInstallStatus::Installed
        } else {
            HookInstallStatus::AlreadyInstalled
        },
        path: Some(hooks_path.display().to_string()),
        backup_path: result.backup_path.map(|path| path.display().to_string()),
        message: if result.changed {
            installed_message.to_string()
        } else {
            format!("{label} hooks 已存在，无需重复写入。")
        },
    }
}

fn build_hooks_json_snippet(command: &str) -> String {
    let hooks = HOOK_EVENTS
        .iter()
        .map(|event| {
            (
                event.to_string(),
                Value::Array(vec![build_hook_event_entry(command)]),
            )
        })
        .collect::<serde_json::Map<String, Value>>();

    serde_json::to_string_pretty(&json!({
        "hooks": hooks
    }))
    .expect("hook snippet is valid JSON")
}

fn build_hook_event_entry(command: &str) -> Value {
    json!({
        "hooks": [{
            "type": "command",
            "command": command,
            "timeout": HOOK_TIMEOUT_SECONDS
        }]
    })
}

fn entry_contains_codex_island_hook(entry: &Value) -> bool {
    entry_nested_hook_commands(entry).any(|command| command.contains(HOOK_BINARY_STEM))
}

fn entry_contains_command(entry: &Value, expected_command: &str) -> bool {
    entry_nested_hook_commands(entry).any(|command| command == expected_command)
}

fn entry_nested_hook_commands(entry: &Value) -> impl Iterator<Item = &str> {
    entry
        .get("hooks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|hook| hook.get("command").and_then(Value::as_str))
}

fn current_hook_binary_path() -> PathBuf {
    let current_exe = std::env::current_exe().ok();
    let current_dir = current_exe
        .as_deref()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(default_tool_dir);

    let candidates = if cfg!(windows) {
        vec![
            current_dir.join(format!("{HOOK_BINARY_STEM}.exe")),
            current_dir.join(format!("{HOOK_BINARY_STEM}-x86_64-pc-windows-msvc.exe")),
            current_dir.join(format!("{HOOK_BINARY_STEM}-x86_64-pc-windows-gnu.exe")),
        ]
    } else {
        vec![
            current_dir.join(HOOK_BINARY_STEM),
            current_dir.join(format!("{HOOK_BINARY_STEM}.exe")),
            current_dir.join(format!("{HOOK_BINARY_STEM}-x86_64-pc-windows-msvc.exe")),
        ]
    };

    candidates
        .iter()
        .find(|path| path.exists())
        .cloned()
        .unwrap_or_else(|| candidates[0].clone())
}

fn codex_hooks_path() -> PathBuf {
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        return PathBuf::from(codex_home).join(HOOKS_FILE_NAME);
    }

    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        return PathBuf::from(user_profile)
            .join(".codex")
            .join(HOOKS_FILE_NAME);
    }

    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".codex").join(HOOKS_FILE_NAME);
    }

    default_tool_dir().join(".codex").join(HOOKS_FILE_NAME)
}

fn read_wsl_hooks_path() -> Result<String, std::io::Error> {
    let output = Command::new("wsl.exe")
        .args([
            "sh",
            "-lc",
            "printf '%s' \"${CODEX_HOME:-$HOME/.codex}/hooks.json\"",
        ])
        .output()?;

    command_output_to_string(output)
}

fn read_wsl_hooks_file() -> Result<Option<String>, std::io::Error> {
    let output = Command::new("wsl.exe")
        .args([
            "sh",
            "-lc",
            "file=\"${CODEX_HOME:-$HOME/.codex}/hooks.json\"; if [ -f \"$file\" ]; then cat \"$file\"; fi",
        ])
        .output()?;

    let body = command_output_to_string(output)?;
    Ok((!body.trim().is_empty()).then_some(body))
}

fn write_wsl_hooks_file(body: &str) -> Result<Option<String>, std::io::Error> {
    let mut child = Command::new("wsl.exe")
        .args([
            "sh",
            "-lc",
            "set -eu; file=\"${CODEX_HOME:-$HOME/.codex}/hooks.json\"; mkdir -p \"$(dirname \"$file\")\"; backup=\"\"; if [ -f \"$file\" ]; then backup=\"$file.codex-island.bak\"; cp \"$file\" \"$backup\"; fi; tmp=\"$file.codex-island.tmp\"; cat > \"$tmp\"; mv \"$tmp\" \"$file\"; printf '%s' \"$backup\"",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    child
        .stdin
        .as_mut()
        .expect("wsl child stdin is piped")
        .write_all(body.as_bytes())?;

    let output = child.wait_with_output()?;
    let backup_path = command_output_to_string(output)?;
    Ok((!backup_path.trim().is_empty()).then_some(backup_path))
}

fn command_output_to_string(output: std::process::Output) -> Result<String, std::io::Error> {
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(std::io::Error::other(stderr.trim().to_string()))
}

fn quote_command(command: &str) -> String {
    format!("\"{}\"", command.replace('"', "\\\""))
}

pub fn to_wsl_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    if normalized.len() > 2 && normalized.as_bytes()[1] == b':' {
        let drive = normalized[..1].to_ascii_lowercase();
        let rest = normalized[2..].trim_start_matches('/');
        format!("/mnt/{drive}/{rest}")
    } else {
        normalized
    }
}

fn atomic_write(path: &Path, body: &[u8]) -> std::io::Result<()> {
    let tmp_path = path.with_extension("json.codex-island.tmp");
    fs::write(&tmp_path, body)?;
    fs::rename(tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{
        build_install_snippets, merge_hooks_config, to_wsl_path, HOOK_BINARY_STEM, HOOK_EVENTS,
    };

    #[test]
    fn generates_separate_windows_and_wsl_snippets() {
        let snippets = build_install_snippets(
            "C:\\Program Files\\Codex Island\\codex-island-hook.exe",
            "C:\\Users\\zk\\AppData\\Local\\CodexIsland\\sessions",
        );

        assert!(snippets.windows.contains("codex-island-hook.exe"));
        assert!(snippets.windows.contains("C:\\Users\\zk\\AppData\\Local\\CodexIsland\\sessions"));
        assert!(snippets
            .wsl
            .contains("/mnt/c/Program Files/Codex Island/codex-island-hook.exe"));
        assert!(snippets
            .wsl
            .contains("/mnt/c/Users/zk/AppData/Local/CodexIsland/sessions"));
    }

    #[test]
    fn merge_hooks_config_preserves_existing_hooks() {
        let existing = r#"{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "python3 existing.py",
            "timeout": 5
          }
        ]
      }
    ]
  }
}"#;

        let (merged, changed) =
            merge_hooks_config(Some(existing), "\"C:\\Tools\\codex-island-hook.exe\"").unwrap();
        let parsed: Value = serde_json::from_str(&merged).unwrap();
        let session_start = parsed["hooks"]["SessionStart"].as_array().unwrap();

        assert!(changed);
        assert_eq!(session_start.len(), 2);
        assert!(session_start.iter().any(|entry| {
            entry["hooks"][0]["command"]
                .as_str()
                .unwrap()
                .contains("existing.py")
        }));
    }

    #[test]
    fn merge_hooks_config_replaces_stale_codex_island_hooks() {
        let existing = format!(
            r#"{{
  "hooks": {{
    "Stop": [
      {{
        "hooks": [
          {{
            "type": "command",
            "command": "\"C:\\Old\\{HOOK_BINARY_STEM}.exe\"",
            "timeout": 5
          }}
        ]
      }}
    ]
  }}
}}"#
        );

        let (merged, changed) =
            merge_hooks_config(Some(&existing), "\"C:\\New\\codex-island-hook.exe\"").unwrap();
        let parsed: Value = serde_json::from_str(&merged).unwrap();
        let stop_hooks = parsed["hooks"]["Stop"].as_array().unwrap();

        assert!(changed);
        assert_eq!(stop_hooks.len(), 1);
        assert_eq!(
            stop_hooks[0]["hooks"][0]["command"].as_str().unwrap(),
            "\"C:\\New\\codex-island-hook.exe\""
        );
    }

    #[test]
    fn merge_hooks_config_is_idempotent() {
        let (first, _) = merge_hooks_config(None, "\"C:\\Tools\\codex-island-hook.exe\"").unwrap();
        let (second, changed) =
            merge_hooks_config(Some(&first), "\"C:\\Tools\\codex-island-hook.exe\"").unwrap();

        assert!(!changed);
        assert_eq!(first, second);
    }

    #[test]
    fn merge_hooks_config_adds_all_supported_events() {
        let (merged, changed) =
            merge_hooks_config(None, "\"C:\\Tools\\codex-island-hook.exe\"").unwrap();
        let parsed: Value = serde_json::from_str(&merged).unwrap();

        assert!(changed);
        for event in HOOK_EVENTS {
            assert_eq!(parsed["hooks"][event].as_array().unwrap().len(), 1);
        }
    }

    #[test]
    fn converts_windows_path_to_wsl_mount_path() {
        assert_eq!(
            to_wsl_path("C:\\Program Files\\Codex Island\\codex-island-hook.exe"),
            "/mnt/c/Program Files/Codex Island/codex-island-hook.exe"
        );
    }
}
