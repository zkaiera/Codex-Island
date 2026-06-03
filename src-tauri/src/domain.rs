use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Wsl,
    Windows,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UiState {
    Running,
    Completed,
    Waiting,
    Error,
    Stale,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum HookEvent {
    SessionStart,
    UserPromptSubmit,
    PermissionRequest,
    PreToolUse,
    PostToolUse,
    Stop,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SessionRecord {
    pub session_id: String,
    pub turn_id: Option<String>,
    pub cwd: String,
    pub title: String,
    pub source: Source,
    pub distro: Option<String>,
    pub last_event: HookEvent,
    pub last_tool: Option<String>,
    pub ui_state: UiState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SessionRecord {
    pub fn new(session_id: String, cwd: String, source: Source, distro: Option<String>) -> Self {
        let now = Utc::now();
        let title = title_from_cwd(&cwd);

        Self {
            session_id,
            turn_id: None,
            cwd,
            title,
            source,
            distro,
            last_event: HookEvent::SessionStart,
            last_tool: None,
            ui_state: UiState::Running,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = created_at;
        self
    }

    pub fn with_updated_at(mut self, updated_at: DateTime<Utc>) -> Self {
        self.updated_at = updated_at;
        self
    }

    pub fn with_event(mut self, event: HookEvent) -> Self {
        self.ui_state = UiState::from_event(&event);
        self.last_event = event;
        self.updated_at = Utc::now();
        self
    }

    pub fn with_ui_state(mut self, ui_state: UiState) -> Self {
        self.ui_state = ui_state;
        self
    }

    pub fn is_newer_than(&self, timestamp: DateTime<Utc>) -> bool {
        self.updated_at > timestamp
    }
}

impl UiState {
    pub fn from_event(event: &HookEvent) -> Self {
        match event {
            HookEvent::PermissionRequest => Self::Waiting,
            HookEvent::Stop => Self::Completed,
            HookEvent::SessionStart
            | HookEvent::UserPromptSubmit
            | HookEvent::PreToolUse
            | HookEvent::PostToolUse => Self::Running,
        }
    }
}

pub fn title_from_cwd(cwd: &str) -> String {
    let trimmed = cwd.trim_end_matches(['/', '\\']);

    trimmed
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or("unknown-project")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    #[test]
    fn session_created_at_is_preserved() {
        let first = SessionRecord::new(
            "abc".into(),
            "/work/a".into(),
            Source::Wsl,
            Some("Ubuntu".into()),
        )
        .with_created_at(Utc.with_ymd_and_hms(2026, 6, 3, 10, 0, 0).unwrap());

        let second = first.clone().with_event(HookEvent::PreToolUse);

        assert_eq!(first.created_at, second.created_at);
    }

    #[test]
    fn hidden_session_reappears_only_after_newer_event() {
        let hidden_at = Utc.with_ymd_and_hms(2026, 6, 3, 10, 0, 0).unwrap();
        let record = SessionRecord::new("abc".into(), "/work/a".into(), Source::Windows, None)
            .with_updated_at(hidden_at + Duration::seconds(30));

        assert!(record.is_newer_than(hidden_at));
    }

    #[test]
    fn status_priority_does_not_change_order() {
        let older = SessionRecord::new(
            "older".into(),
            "/work/older".into(),
            Source::Wsl,
            Some("Ubuntu".into()),
        )
        .with_created_at(Utc.with_ymd_and_hms(2026, 6, 3, 9, 0, 0).unwrap())
        .with_ui_state(UiState::Running);
        let newer = SessionRecord::new(
            "newer".into(),
            "/work/newer".into(),
            Source::Wsl,
            Some("Ubuntu".into()),
        )
        .with_created_at(Utc.with_ymd_and_hms(2026, 6, 3, 11, 0, 0).unwrap())
        .with_ui_state(UiState::Waiting);

        let mut sessions = vec![newer, older];
        sessions.sort_by_key(|session| session.created_at);

        assert_eq!(sessions[0].session_id, "older");
    }

    #[test]
    fn title_uses_last_path_segment() {
        let record = SessionRecord::new(
            "abc".into(),
            "/home/zkai/project/web3-agent-research".into(),
            Source::Wsl,
            Some("Ubuntu".into()),
        );

        assert_eq!(record.title, "web3-agent-research");
    }
}
