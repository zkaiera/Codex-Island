use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};

use crate::domain::{SessionRecord, UiState};

pub const DEFAULT_STALE_MINUTES: i64 = 10;
pub const COMPLETED_RETENTION_MINUTES: i64 = 10;
pub const STALE_RETENTION_HOURS: i64 = 8;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HiddenSession {
    pub session_id: String,
    pub hidden_at: DateTime<Utc>,
}

impl HiddenSession {
    pub fn new(session_id: String, hidden_at: DateTime<Utc>) -> Self {
        Self {
            session_id,
            hidden_at,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SessionStore {
    sessions: HashMap<String, SessionRecord>,
    hidden_sessions: HashMap<String, HiddenSession>,
    stale_after: Duration,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new(Duration::minutes(DEFAULT_STALE_MINUTES))
    }
}

impl SessionStore {
    pub fn new(stale_after: Duration) -> Self {
        Self {
            sessions: HashMap::new(),
            hidden_sessions: HashMap::new(),
            stale_after,
        }
    }

    pub fn upsert(&mut self, session: SessionRecord) {
        if let Some(hidden) = self.hidden_sessions.get(&session.session_id) {
            if should_show_again(hidden, &session) {
                self.hidden_sessions.remove(&session.session_id);
            }
        }

        self.sessions.insert(session.session_id.clone(), session);
    }

    pub fn hide(&mut self, session_id: &str, hidden_at: DateTime<Utc>) {
        self.hidden_sessions.insert(
            session_id.to_string(),
            HiddenSession::new(session_id.to_string(), hidden_at),
        );
    }

    pub fn recompute_visible(&self, now: DateTime<Utc>) -> Vec<SessionRecord> {
        let visible = self
            .sessions
            .values()
            .filter(|session| !self.hidden_sessions.contains_key(&session.session_id))
            .cloned()
            .map(|session| mark_stale(session, now, self.stale_after))
            .filter(|session| should_display_session(session, now))
            .collect::<Vec<_>>();

        sort_sessions(visible)
    }

    pub fn replace_all(&mut self, sessions: Vec<SessionRecord>) {
        self.sessions.clear();
        for session in sessions {
            self.upsert(session);
        }
    }

    pub fn sessions(&self) -> Vec<SessionRecord> {
        self.sessions.values().cloned().collect()
    }
}

pub fn should_display_session(session: &SessionRecord, now: DateTime<Utc>) -> bool {
    match session.ui_state {
        UiState::Completed => {
            now.signed_duration_since(session.updated_at)
                <= Duration::minutes(COMPLETED_RETENTION_MINUTES)
        }
        UiState::Stale => {
            now.signed_duration_since(session.updated_at) <= Duration::hours(STALE_RETENTION_HOURS)
        }
        UiState::Error => {
            now.signed_duration_since(session.updated_at) <= Duration::hours(STALE_RETENTION_HOURS)
        }
        UiState::Running | UiState::Waiting => true,
    }
}

pub fn sort_sessions(mut sessions: Vec<SessionRecord>) -> Vec<SessionRecord> {
    sessions.sort_by_key(|session| session.created_at);
    sessions
}

pub fn mark_stale(
    mut session: SessionRecord,
    now: DateTime<Utc>,
    stale_after: Duration,
) -> SessionRecord {
    let is_active = matches!(session.ui_state, UiState::Running | UiState::Waiting);
    let is_expired = now.signed_duration_since(session.updated_at) > stale_after;

    if is_active && is_expired {
        session.ui_state = UiState::Stale;
    }

    session
}

pub fn should_show_again(hidden: &HiddenSession, session: &SessionRecord) -> bool {
    session.updated_at > hidden.hidden_at
}
