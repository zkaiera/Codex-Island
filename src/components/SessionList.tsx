import { formatRelativeTime, SOURCE_LABELS, STATUS_LABELS, type SessionView } from "./session";

type SessionListProps = {
  sessions: SessionView[];
  onHide: (sessionId: string) => void;
  scrollable?: boolean;
};

export function SessionList({ sessions, onHide, scrollable = false }: SessionListProps) {
  return (
    <div
      className={["session-list", scrollable ? "session-list--scrollable" : ""]
        .filter(Boolean)
        .join(" ")}
      role="list"
    >
      {sessions.map((session) => (
        <div key={session.sessionId} className="session-list__row" role="listitem">
          <div className="session-list__meta">
            <span
              className={`session-list__indicator session-list__indicator--${session.status}`}
              aria-hidden="true"
            />
            <div className="session-list__text">
              <span className="session-list__title">{session.title}</span>
              <span className="session-list__details">
                {STATUS_LABELS[session.status]} · {SOURCE_LABELS[session.source]} ·{" "}
                {formatRelativeTime(session.updatedAt)}
              </span>
            </div>
          </div>
          <button
            type="button"
            className="session-list__hide"
            aria-label={`隐藏 ${session.title}`}
            onClick={() => onHide(session.sessionId)}
          >
            ×
          </button>
        </div>
      ))}
    </div>
  );
}
