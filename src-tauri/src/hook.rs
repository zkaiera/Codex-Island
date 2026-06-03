use std::fs;
use std::io::{self, Read};
use std::path::Path;

use chrono::Utc;
use serde::Deserialize;
use thiserror::Error;

use crate::domain::{HookEvent, SessionRecord, Source};
use crate::paths::session_file_path;

#[derive(Debug, Error)]
pub enum HookError {
    #[error("failed to read hook payload: {0}")]
    Read(#[from] io::Error),
    #[error("invalid hook payload: {0}")]
    Parse(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize)]
struct HookPayload {
    session_id: String,
    turn_id: Option<String>,
    cwd: String,
    hook_event_name: String,
    tool_name: Option<String>,
}

pub fn run_from_stdin() -> Result<(), HookError> {
    let mut payload = String::new();
    io::stdin().read_to_string(&mut payload)?;

    let source = if std::env::var_os("WSL_DISTRO_NAME").is_some() {
        Source::Wsl
    } else {
        Source::Windows
    };
    let distro = std::env::var("WSL_DISTRO_NAME").ok();
    let state_dir = crate::paths::default_state_dir();
    let record = parse_and_build_record(&payload, source, distro)?;

    write_record(&state_dir, record)?;
    Ok(())
}

pub fn parse_and_build_record(
    payload: &str,
    source: Source,
    distro: Option<String>,
) -> Result<SessionRecord, HookError> {
    let parsed: HookPayload = serde_json::from_str(payload)?;
    let event = parse_hook_event(&parsed.hook_event_name);
    let source = infer_source_from_payload(&parsed.cwd, source, distro.as_deref());
    let mut record =
        SessionRecord::new(parsed.session_id, parsed.cwd, source, distro).with_event(event);
    record.turn_id = parsed.turn_id;
    record.last_tool = parsed.tool_name;

    Ok(record)
}

pub fn write_record(state_dir: &Path, record: SessionRecord) -> Result<SessionRecord, HookError> {
    fs::create_dir_all(state_dir)?;

    let path = session_file_path(state_dir.to_path_buf(), &record.session_id);
    let persisted = if path.exists() {
        let existing = fs::read_to_string(&path)?;
        let mut parsed: SessionRecord = serde_json::from_str(&existing)?;
        parsed.turn_id = record.turn_id.clone();
        parsed.cwd = record.cwd.clone();
        parsed.title = record.title.clone();
        parsed.source = record.source.clone();
        parsed.distro = record.distro.clone();
        parsed.last_event = record.last_event.clone();
        parsed.last_tool = record.last_tool.clone();
        parsed.ui_state = record.ui_state.clone();
        parsed.updated_at = Utc::now();
        parsed
    } else {
        record
    };

    let body = serde_json::to_string_pretty(&persisted)?;
    atomic_write(&path, &body)?;

    Ok(persisted)
}

fn atomic_write(path: &Path, body: &str) -> io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, body)?;
    fs::rename(tmp, path)?;
    Ok(())
}

fn parse_hook_event(name: &str) -> HookEvent {
    match normalize_hook_event_name(name).as_str() {
        "sessionstart" => HookEvent::SessionStart,
        "userpromptsubmit" => HookEvent::UserPromptSubmit,
        "permissionrequest" => HookEvent::PermissionRequest,
        "pretooluse" => HookEvent::PreToolUse,
        "posttooluse" => HookEvent::PostToolUse,
        "stop" => HookEvent::Stop,
        _ => HookEvent::SessionStart,
    }
}

fn normalize_hook_event_name(name: &str) -> String {
    name.chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn infer_source_from_payload(cwd: &str, fallback: Source, distro: Option<&str>) -> Source {
    if distro.is_some() || looks_like_wsl_cwd(cwd) {
        Source::Wsl
    } else {
        fallback
    }
}

fn looks_like_wsl_cwd(cwd: &str) -> bool {
    cwd.starts_with("/home/")
        || cwd.starts_with("/mnt/")
        || cwd.starts_with("/tmp/")
        || cwd.starts_with("/work/")
        || cwd.starts_with("/var/")
}
