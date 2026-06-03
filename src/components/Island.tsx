import { useMemo, useState } from "react";

import { SessionList } from "./SessionList";
import { SessionPill } from "./SessionPill";
import type { SessionView } from "./session";

type IslandProps = {
  sessions: SessionView[];
  onHide: (sessionId: string) => void;
  onExpandedChange?: (expanded: boolean) => void;
  maxVisibleCollapsed?: number;
};

export function Island({
  sessions,
  onHide,
  onExpandedChange,
  maxVisibleCollapsed = calculateVisibleCount(),
}: IslandProps) {
  const [expanded, setExpanded] = useState(false);

  const orderedSessions = useMemo(
    () =>
      [...sessions].sort(
        (left, right) =>
          new Date(left.createdAt).getTime() - new Date(right.createdAt).getTime(),
      ),
    [sessions],
  );

  const visiblePills = orderedSessions.slice(0, maxVisibleCollapsed);
  const hiddenCount = Math.max(orderedSessions.length - visiblePills.length, 0);

  function updateExpanded(nextExpanded: boolean) {
    setExpanded(nextExpanded);
    onExpandedChange?.(nextExpanded);
  }

  return (
    <div
      className={`island-wrapper${expanded ? " island-wrapper--expanded" : ""}`}
      onMouseEnter={() => updateExpanded(true)}
      onMouseLeave={() => updateExpanded(false)}
    >
      <div className="island" aria-label="Codex Island">
        <div className="island__pills">
          {visiblePills.length === 0 ? (
            <span className="island__dot island__dot--idle" aria-hidden="true" />
          ) : (
            visiblePills.map((session) => (
              <SessionPill key={session.sessionId} session={session} />
            ))
          )}
          {hiddenCount > 0 ? <span className="island__overflow">+{hiddenCount}</span> : null}
        </div>
      </div>

      {expanded && orderedSessions.length > 0 ? (
        <div className="island-panel">
          <div className="island-panel__header">{orderedSessions.length} active</div>
          <SessionList sessions={orderedSessions} onHide={onHide} />
        </div>
      ) : null}
    </div>
  );
}

function calculateVisibleCount() {
  if (typeof window === "undefined") {
    return 6;
  }

  const maxWidth = Math.max(window.innerWidth - 160, 160);
  const pillWidth = 16;
  const gapWidth = 12;
  return Math.max(Math.floor(maxWidth / (pillWidth + gapWidth)), 1);
}
