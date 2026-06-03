use codex_island_lib::domain::{HookEvent, Source, UiState};
use codex_island_lib::hook::{parse_and_build_record, write_record};

#[test]
fn writes_one_session_file_per_session_id() {
    let payload = r#"{
        "session_id": "abc123",
        "turn_id": "turn-1",
        "cwd": "/work/a",
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash"
    }"#;

    let result = parse_and_build_record(payload, Source::Wsl, Some("Ubuntu".into())).unwrap();

    assert_eq!(result.session_id, "abc123");
    assert_eq!(result.turn_id.as_deref(), Some("turn-1"));
    assert_eq!(result.last_event, HookEvent::PreToolUse);
    assert_eq!(result.last_tool.as_deref(), Some("Bash"));
    assert_eq!(result.ui_state, UiState::Running);
    assert_eq!(result.title, "a");
}

#[test]
fn permission_request_maps_to_waiting() {
    let payload = r#"{
        "session_id": "abc123",
        "cwd": "C:\\Projects\\codex",
        "hook_event_name": "PermissionRequest"
    }"#;

    let result = parse_and_build_record(payload, Source::Windows, None).unwrap();

    assert_eq!(result.source, Source::Windows);
    assert_eq!(result.last_event, HookEvent::PermissionRequest);
    assert_eq!(result.ui_state, UiState::Waiting);
    assert_eq!(result.title, "codex");
}

#[test]
fn preserves_created_at_after_first_write() {
    let dir = tempfile::tempdir().unwrap();
    let first = parse_and_build_record(
        r#"{
            "session_id": "abc123",
            "cwd": "/work/a",
            "hook_event_name": "SessionStart"
        }"#,
        Source::Wsl,
        Some("Ubuntu".into()),
    )
    .unwrap();
    let first = write_record(dir.path(), first).unwrap();

    let second = parse_and_build_record(
        r#"{
            "session_id": "abc123",
            "cwd": "/work/a",
            "hook_event_name": "PostToolUse"
        }"#,
        Source::Wsl,
        Some("Ubuntu".into()),
    )
    .unwrap();
    let second = write_record(dir.path(), second).unwrap();

    assert_eq!(first.created_at, second.created_at);
    assert!(second.updated_at >= first.updated_at);
}
