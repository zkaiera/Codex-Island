use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::Value;

use crate::domain::{title_from_cwd, HookEvent, SessionRecord, Source};
use crate::store::STALE_RETENTION_HOURS;

#[derive(Clone, Debug)]
struct CodexLogSession {
    record: SessionRecord,
    forked_from_id: Option<String>,
    is_subagent: bool,
}

#[derive(Debug, Deserialize)]
struct JsonLine {
    timestamp: Option<DateTime<Utc>>,
    #[serde(rename = "type")]
    line_type: String,
    payload: Value,
}

#[derive(Debug, Deserialize)]
struct SessionMeta {
    id: String,
    forked_from_id: Option<String>,
    timestamp: Option<DateTime<Utc>>,
    cwd: String,
    thread_source: Option<String>,
}

pub fn merge_sessions_with_codex_logs(
    sessions: Vec<SessionRecord>,
    codex_session_roots: &[PathBuf],
    now: DateTime<Utc>,
) -> Vec<SessionRecord> {
    let mut by_id = sessions
        .into_iter()
        .map(|session| (session.session_id.clone(), session))
        .collect::<HashMap<_, _>>();
    let candidates = load_recent_codex_logs(codex_session_roots, now);

    for candidate in candidates.iter().filter(|candidate| !candidate.is_subagent) {
        merge_candidate(&mut by_id, candidate);
    }

    for candidate in candidates.iter().filter(|candidate| candidate.is_subagent) {
        let Some(parent_id) = candidate.forked_from_id.as_deref() else {
            continue;
        };

        if by_id.contains_key(parent_id) {
            merge_candidate(&mut by_id, candidate);
        }
    }

    by_id.into_values().collect()
}

pub fn default_codex_session_roots(anchor_sessions: &[SessionRecord]) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(path) = std::env::var_os("CODEX_ISLAND_CODEX_SESSIONS_DIR") {
        push_unique(&mut roots, PathBuf::from(path));
    }

    if let Some(path) = std::env::var_os("CODEX_HOME") {
        push_unique(&mut roots, PathBuf::from(path).join("sessions"));
    }

    if let Some(path) = std::env::var_os("HOME") {
        push_unique(
            &mut roots,
            PathBuf::from(path).join(".codex").join("sessions"),
        );
    }

    if let Some(path) = std::env::var_os("USERPROFILE") {
        push_unique(
            &mut roots,
            PathBuf::from(path).join(".codex").join("sessions"),
        );
    }

    add_wsl_unc_roots(&mut roots, anchor_sessions);

    roots
}

fn load_recent_codex_logs(
    codex_session_roots: &[PathBuf],
    now: DateTime<Utc>,
) -> Vec<CodexLogSession> {
    codex_session_roots
        .iter()
        .flat_map(|root| collect_jsonl_files(root))
        .filter_map(|path| load_codex_log_session(&path, now))
        .collect()
}

fn load_codex_log_session(path: &Path, now: DateTime<Utc>) -> Option<CodexLogSession> {
    let updated_at = file_updated_at(path)?;
    if now.signed_duration_since(updated_at) > Duration::hours(STALE_RETENTION_HOURS) {
        return None;
    }

    let body = std::fs::read_to_string(path).ok()?;
    let mut meta = None;
    let mut last_activity = LogActivity::Running;
    let mut last_timestamp = updated_at;

    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        let parsed: JsonLine = serde_json::from_str(line).ok()?;
        if let Some(timestamp) = parsed.timestamp {
            last_timestamp = last_timestamp.max(timestamp);
        }

        if parsed.line_type == "session_meta" {
            meta = serde_json::from_value::<SessionMeta>(parsed.payload).ok();
            continue;
        }

        if parsed.line_type == "event_msg"
            && parsed
                .payload
                .get("type")
                .and_then(Value::as_str)
                .is_some_and(|value| value == "task_complete")
        {
            last_activity = LogActivity::Completed;
            continue;
        }

        if matches!(parsed.line_type.as_str(), "turn_context" | "response_item") {
            last_activity = LogActivity::Running;
        }
    }

    let meta = meta?;
    let is_subagent =
        meta.thread_source.as_deref() == Some("subagent") || meta.forked_from_id.is_some();
    let created_at = meta.timestamp.unwrap_or(last_timestamp);
    let mut record = SessionRecord::new(
        meta.id,
        meta.cwd.clone(),
        infer_source_from_cwd(&meta.cwd),
        None,
    )
    .with_created_at(created_at)
    .with_updated_at(last_timestamp)
    .with_event(match last_activity {
        LogActivity::Running => HookEvent::UserPromptSubmit,
        LogActivity::Completed => HookEvent::Stop,
    });
    record.created_at = created_at;
    record.updated_at = last_timestamp;
    record.title = if is_subagent {
        format!("{} (subtask)", title_from_cwd(&meta.cwd))
    } else {
        title_from_cwd(&meta.cwd)
    };

    Some(CodexLogSession {
        record,
        forked_from_id: meta.forked_from_id,
        is_subagent,
    })
}

fn merge_candidate(by_id: &mut HashMap<String, SessionRecord>, candidate: &CodexLogSession) {
    if let Some(existing) = by_id.get_mut(&candidate.record.session_id) {
        if candidate.record.updated_at <= existing.updated_at {
            return;
        }

        existing.cwd = candidate.record.cwd.clone();
        existing.title = candidate.record.title.clone();
        existing.source = candidate.record.source.clone();
        existing.distro = candidate.record.distro.clone();
        existing.last_event = candidate.record.last_event.clone();
        existing.last_tool = candidate.record.last_tool.clone();
        existing.ui_state = candidate.record.ui_state.clone();
        existing.updated_at = candidate.record.updated_at;
        return;
    }

    by_id.insert(
        candidate.record.session_id.clone(),
        candidate.record.clone(),
    );
}

fn collect_jsonl_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_jsonl_files_inner(root, 0, &mut files);
    files
}

fn collect_jsonl_files_inner(path: &Path, depth: usize, files: &mut Vec<PathBuf>) {
    if depth > 5 {
        return;
    }

    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_files_inner(&path, depth + 1, files);
            continue;
        }

        if path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }
}

fn file_updated_at(path: &Path) -> Option<DateTime<Utc>> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(DateTime::<Utc>::from(modified))
}

fn infer_source_from_cwd(cwd: &str) -> Source {
    if cwd.starts_with("/home/")
        || cwd.starts_with("/mnt/")
        || cwd.starts_with("/tmp/")
        || cwd.starts_with("/work/")
        || cwd.starts_with("/var/")
    {
        Source::Wsl
    } else {
        Source::Windows
    }
}

fn add_wsl_unc_roots(roots: &mut Vec<PathBuf>, anchor_sessions: &[SessionRecord]) {
    let users = anchor_sessions
        .iter()
        .filter_map(|session| wsl_user_from_cwd(&session.cwd))
        .collect::<Vec<_>>();

    if users.is_empty() {
        return;
    }

    for unc_root in [r"\\wsl.localhost", r"\\wsl$"] {
        let entries = match std::fs::read_dir(unc_root) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let distro = entry.file_name();
            for user in &users {
                push_unique(
                    roots,
                    PathBuf::from(unc_root)
                        .join(&distro)
                        .join("home")
                        .join(user)
                        .join(".codex")
                        .join("sessions"),
                );
            }
        }
    }
}

fn wsl_user_from_cwd(cwd: &str) -> Option<String> {
    let mut parts = cwd.split('/').filter(|part| !part.is_empty());
    if parts.next()? != "home" {
        return None;
    }

    parts.next().map(ToString::to_string)
}

fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LogActivity {
    Running,
    Completed,
}
