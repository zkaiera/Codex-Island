use chrono::{Duration, TimeZone, Utc};
use codex_island_lib::domain::{SessionRecord, Source, UiState};
use codex_island_lib::store::{
    mark_stale, should_show_again, sort_sessions, HiddenSession, SessionStore,
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
