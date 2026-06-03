use chrono::SecondsFormat;
use chrono::{Duration, TimeZone, Utc};
use codex_island_lib::domain::{HookEvent, SessionRecord, Source, UiState};
use codex_island_lib::store::{
    mark_stale, should_display_session, should_show_again, sort_sessions, HiddenSession,
    SessionStore,
};
use codex_island_lib::watcher::{
    load_sessions_from_dir, load_sessions_with_codex_logs, refresh_store_from_disk,
};

#[test]
fn sessions_are_sorted_by_created_at() {
    let older = SessionRecord::new(
        "older".into(),
        "/work/older".into(),
        Source::Wsl,
        Some("Ubuntu".into()),
    )
    .with_created_at(Utc.with_ymd_and_hms(2026, 6, 3, 9, 0, 0).unwrap());
    let newer = SessionRecord::new(
        "newer".into(),
        "/work/newer".into(),
        Source::Wsl,
        Some("Ubuntu".into()),
    )
    .with_created_at(Utc.with_ymd_and_hms(2026, 6, 3, 11, 0, 0).unwrap());

    let list = sort_sessions(vec![newer, older]);

    assert_eq!(list[0].session_id, "older");
}

#[test]
fn stale_after_ten_minutes_without_update() {
    let now = Utc::now();
    let stale = mark_stale(
        SessionRecord::new(
            "abc".into(),
            "/work/a".into(),
            Source::Wsl,
            Some("Ubuntu".into()),
        )
        .with_updated_at(now - Duration::minutes(11)),
        now,
        Duration::minutes(10),
    );

    assert_eq!(stale.ui_state, UiState::Stale);
}

#[test]
fn hidden_session_reappears_when_newer_event_arrives() {
    let hidden_at = Utc.with_ymd_and_hms(2026, 6, 3, 10, 0, 0).unwrap();
    let hidden = HiddenSession::new("abc123".into(), hidden_at);
    let updated_after_hidden = SessionRecord::new(
        "abc123".into(),
        "/work/a".into(),
        Source::Wsl,
        Some("Ubuntu".into()),
    )
    .with_updated_at(hidden_at + Duration::seconds(30));

    assert!(should_show_again(&hidden, &updated_after_hidden));
}

#[test]
fn hidden_sessions_stay_hidden_until_newer_update() {
    let hidden_at = Utc.with_ymd_and_hms(2026, 6, 3, 10, 0, 0).unwrap();
    let mut store = SessionStore::new(Duration::minutes(10));
    let session = SessionRecord::new(
        "abc123".into(),
        "/work/a".into(),
        Source::Wsl,
        Some("Ubuntu".into()),
    )
    .with_created_at(hidden_at - Duration::minutes(1))
    .with_updated_at(hidden_at - Duration::seconds(10));

    store.upsert(session.clone());
    store.hide("abc123", hidden_at);

    assert!(store.recompute_visible(hidden_at).is_empty());

    store.upsert(session.with_updated_at(hidden_at + Duration::seconds(30)));

    assert_eq!(store.recompute_visible(hidden_at).len(), 1);
}

#[test]
fn stale_sessions_are_kept_for_eight_hours_after_last_update() {
    let now = Utc.with_ymd_and_hms(2026, 6, 3, 18, 0, 0).unwrap();
    let stale_within_retention = SessionRecord::new(
        "within".into(),
        "/work/within".into(),
        Source::Wsl,
        Some("Ubuntu".into()),
    )
    .with_updated_at(now - Duration::hours(8))
    .with_ui_state(UiState::Stale);
    let stale_too_old = SessionRecord::new(
        "old".into(),
        "/work/old".into(),
        Source::Wsl,
        Some("Ubuntu".into()),
    )
    .with_updated_at(now - Duration::hours(8) - Duration::seconds(1))
    .with_ui_state(UiState::Stale);

    assert!(should_display_session(&stale_within_retention, now));
    assert!(!should_display_session(&stale_too_old, now));
}

#[test]
fn completed_sessions_drop_out_after_ten_minutes() {
    let now = Utc.with_ymd_and_hms(2026, 6, 3, 18, 0, 0).unwrap();
    let completed = SessionRecord::new("done".into(), "/work/done".into(), Source::Windows, None)
        .with_updated_at(now - Duration::minutes(11))
        .with_ui_state(UiState::Completed);

    assert!(!should_display_session(&completed, now));
}

#[test]
fn smoke_status_files_are_ignored_when_loading_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let body = serde_json::to_string(&SessionRecord::new(
        "codex-island-smoke-wsl-20260603".into(),
        "/work/smoke".into(),
        Source::Wsl,
        Some("Ubuntu".into()),
    ))
    .unwrap();
    std::fs::write(
        dir.path().join("codex-island-smoke-wsl-20260603.json"),
        body,
    )
    .unwrap();

    assert!(load_sessions_from_dir(dir.path()).is_empty());
}

#[test]
fn codex_log_child_session_is_loaded_when_parent_status_exists() {
    let state_dir = tempfile::tempdir().unwrap();
    let codex_dir = tempfile::tempdir().unwrap();
    let now = Utc::now();
    let parent = SessionRecord::new(
        "parent".into(),
        "/home/zkai/Projects/airdrop".into(),
        Source::Wsl,
        None,
    )
    .with_created_at(now - Duration::minutes(20))
    .with_updated_at(now - Duration::minutes(10));
    write_status_file(state_dir.path(), &parent);
    write_codex_log(
        codex_dir.path(),
        "child",
        Some("parent"),
        "/home/zkai/Projects/airdrop",
        now - Duration::minutes(5),
        false,
    );

    let sessions = load_sessions_with_codex_logs(state_dir.path(), &[codex_dir.path().into()], now);

    assert_eq!(sessions.len(), 2);
    assert!(sessions
        .iter()
        .any(|session| session.session_id == "parent"));
    let child = sessions
        .iter()
        .find(|session| session.session_id == "child")
        .unwrap();
    assert_eq!(child.ui_state, UiState::Running);
    assert_eq!(child.title, "airdrop (subtask)");
}

#[test]
fn newer_codex_log_activity_refreshes_completed_status_file() {
    let state_dir = tempfile::tempdir().unwrap();
    let codex_dir = tempfile::tempdir().unwrap();
    let now = Utc::now();
    let completed = SessionRecord::new(
        "same-session".into(),
        "/home/zkai/Projects/airdrop".into(),
        Source::Wsl,
        None,
    )
    .with_created_at(now - Duration::minutes(20))
    .with_event(HookEvent::Stop)
    .with_updated_at(now - Duration::minutes(10));
    write_status_file(state_dir.path(), &completed);
    write_codex_log(
        codex_dir.path(),
        "same-session",
        None,
        "/home/zkai/Projects/airdrop",
        now - Duration::minutes(1),
        false,
    );

    let sessions = load_sessions_with_codex_logs(state_dir.path(), &[codex_dir.path().into()], now);
    let refreshed = sessions
        .iter()
        .find(|session| session.session_id == "same-session")
        .unwrap();

    assert_eq!(refreshed.ui_state, UiState::Running);
    assert_eq!(refreshed.last_event, HookEvent::UserPromptSubmit);
    assert!(refreshed.updated_at > completed.updated_at);
}

#[test]
fn refresh_store_from_disk_picks_up_status_file_changes() {
    let state_dir = tempfile::tempdir().unwrap();
    let now = Utc::now();
    let running = SessionRecord::new(
        "same-session".into(),
        "/work/a".into(),
        Source::Windows,
        None,
    )
    .with_created_at(now - Duration::minutes(20))
    .with_updated_at(now - Duration::minutes(5))
    .with_ui_state(UiState::Running);
    write_status_file(state_dir.path(), &running);

    let store = std::sync::Arc::new(std::sync::RwLock::new(SessionStore::default()));
    refresh_store_from_disk(&store, state_dir.path());
    assert_eq!(session_state(&store, "same-session"), UiState::Running);

    let completed = running
        .clone()
        .with_ui_state(UiState::Completed)
        .with_updated_at(now - Duration::seconds(30));
    write_status_file(state_dir.path(), &completed);

    refresh_store_from_disk(&store, state_dir.path());

    assert_eq!(session_state(&store, "same-session"), UiState::Completed);
}

#[test]
fn refresh_store_from_disk_unhides_session_after_new_activity() {
    let state_dir = tempfile::tempdir().unwrap();
    let hidden_at = Utc::now();
    let old_activity = SessionRecord::new(
        "hidden-session".into(),
        "/work/a".into(),
        Source::Windows,
        None,
    )
    .with_created_at(hidden_at - Duration::minutes(20))
    .with_updated_at(hidden_at - Duration::minutes(1));
    write_status_file(state_dir.path(), &old_activity);

    let store = std::sync::Arc::new(std::sync::RwLock::new(SessionStore::default()));
    refresh_store_from_disk(&store, state_dir.path());
    {
        let mut guard = store.write().unwrap();
        guard.hide("hidden-session", hidden_at);
    }

    let new_activity = old_activity.with_updated_at(hidden_at + Duration::seconds(30));
    write_status_file(state_dir.path(), &new_activity);

    refresh_store_from_disk(&store, state_dir.path());

    assert!(session_is_visible(&store, "hidden-session", hidden_at));
}

fn session_is_visible(
    store: &std::sync::Arc<std::sync::RwLock<SessionStore>>,
    session_id: &str,
    now: chrono::DateTime<Utc>,
) -> bool {
    store
        .read()
        .unwrap()
        .recompute_visible(now)
        .into_iter()
        .any(|session| session.session_id == session_id)
}

fn session_state(
    store: &std::sync::Arc<std::sync::RwLock<SessionStore>>,
    session_id: &str,
) -> UiState {
    store
        .read()
        .unwrap()
        .sessions()
        .into_iter()
        .find(|session| session.session_id == session_id)
        .unwrap()
        .ui_state
}

fn write_status_file(dir: &std::path::Path, record: &SessionRecord) {
    let body = serde_json::to_string(record).unwrap();
    std::fs::write(dir.join(format!("{}.json", record.session_id)), body).unwrap();
}

fn write_codex_log(
    dir: &std::path::Path,
    session_id: &str,
    forked_from_id: Option<&str>,
    cwd: &str,
    timestamp: chrono::DateTime<Utc>,
    completed: bool,
) {
    let timestamp = timestamp.to_rfc3339_opts(SecondsFormat::Millis, true);
    let fork = forked_from_id
        .map(|id| format!(r#","forked_from_id":"{id}""#))
        .unwrap_or_default();
    let complete_line = if completed {
        format!(
            r#"{{"timestamp":"{timestamp}","type":"event_msg","payload":{{"type":"task_complete"}}}}"#
        )
    } else {
        format!(
            r#"{{"timestamp":"{timestamp}","type":"response_item","payload":{{"type":"message","role":"user"}}}}"#
        )
    };
    let thread_source = if forked_from_id.is_some() {
        r#","thread_source":"subagent""#
    } else {
        ""
    };
    let meta = format!(
        r#"{{"timestamp":"{timestamp}","type":"session_meta","payload":{{"id":"{session_id}"{fork},"timestamp":"{timestamp}","cwd":"{cwd}"{thread_source}}}}}"#
    );
    std::fs::write(
        dir.join(format!("{session_id}.jsonl")),
        format!("{meta}\n{complete_line}\n"),
    )
    .unwrap();
}
