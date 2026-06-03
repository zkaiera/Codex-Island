import { useMemo, useState, type MouseEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, PhysicalPosition } from "@tauri-apps/api/window";

import { SessionList } from "./SessionList";
import { SessionPill } from "./SessionPill";
import type { SessionView } from "./session";

type IslandProps = {
  sessions: SessionView[];
  onHide: (sessionId: string) => void;
  onExpandedChange?: (expanded: boolean) => void;
  snapEdge?: SnapEdge;
  onSnapEdgeChange?: (edge: SnapEdge) => void;
  maxVisibleCollapsed?: number;
};

type SnapEdge = "top" | "left" | "right";

export function Island({
  sessions,
  onHide,
  onExpandedChange,
  snapEdge = "top",
  onSnapEdgeChange,
  maxVisibleCollapsed = calculateVisibleCount(),
}: IslandProps) {
  const [expanded, setExpanded] = useState(false);
  const [snapping, setSnapping] = useState(false);
  const [dragging, setDragging] = useState(false);
  const [collapseTimer, setCollapseTimer] = useState<number | null>(null);

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
    if (dragging) {
      return;
    }

    if (collapseTimer !== null) {
      window.clearTimeout(collapseTimer);
      setCollapseTimer(null);
    }

    setExpanded(nextExpanded);
    onExpandedChange?.(nextExpanded);
  }

  function queueCollapse() {
    if (dragging) {
      return;
    }

    if (collapseTimer !== null) {
      window.clearTimeout(collapseTimer);
    }

    const nextTimer = window.setTimeout(() => {
      setExpanded(false);
      onExpandedChange?.(false);
    }, 180);
    setCollapseTimer(nextTimer);
  }

  async function handleDragStart(event: MouseEvent<HTMLDivElement>) {
    if (event.button !== 0 || (event.target as Element).closest("button")) {
      return;
    }

    event.preventDefault();
    setDragging(true);
    setExpanded(false);
    onExpandedChange?.(false);

    const appWindow = getCurrentWindow();
    const startMouse = { x: event.screenX, y: event.screenY };

    let startPosition: { x: number; y: number };
    try {
      startPosition = await appWindow.outerPosition();
    } catch {
      setDragging(false);
      return;
    }

    function handleMove(moveEvent: globalThis.MouseEvent) {
      const nextX = startPosition.x + moveEvent.screenX - startMouse.x;
      const nextY = startPosition.y + moveEvent.screenY - startMouse.y;
      void appWindow.setPosition(new PhysicalPosition(nextX, nextY));
    }

    function handleUp() {
      window.removeEventListener("mousemove", handleMove);
      window.removeEventListener("mouseup", handleUp);
      setDragging(false);
      setSnapping(true);
      void invoke<SnapEdge>("snap_window")
        .then((edge) => {
          if (edge) {
            onSnapEdgeChange?.(edge);
          }
        })
        .catch(() => {
          // 普通浏览器预览没有 Tauri 后端。
        })
        .finally(() => {
          window.setTimeout(() => setSnapping(false), 260);
        });
    }

    window.addEventListener("mousemove", handleMove);
    window.addEventListener("mouseup", handleUp, { once: true });
  }

  return (
    <div
      className={[
        "island-wrapper",
        expanded ? "island-wrapper--expanded" : "",
        dragging ? "island-wrapper--dragging" : "",
        snapping ? "island-wrapper--snapping" : "",
        `island-wrapper--edge-${snapEdge}`,
      ]
        .filter(Boolean)
        .join(" ")}
      data-tauri-drag-region="false"
      onMouseDown={handleDragStart}
      onMouseEnter={() => updateExpanded(true)}
      onMouseLeave={queueCollapse}
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
