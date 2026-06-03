import { useEffect, useMemo, useRef, useState, type PointerEvent } from "react";
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
  const [dragging, setDragging] = useState(false);
  const collapseTimer = useRef<number | null>(null);
  const expandTimer = useRef<number | null>(null);
  const expandedRef = useRef(expanded);
  const dragState = useRef<{
    pointerId: number;
    startMouse: { x: number; y: number };
    startPosition: { x: number; y: number } | null;
  } | null>(null);

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

  useEffect(
    () => () => {
      if (collapseTimer.current !== null) {
        window.clearTimeout(collapseTimer.current);
      }
      if (expandTimer.current !== null) {
        window.clearTimeout(expandTimer.current);
      }
    },
    [],
  );

  useEffect(() => {
    expandedRef.current = expanded;
  }, [expanded]);

  function updateExpanded(nextExpanded: boolean) {
    if (dragging) {
      return;
    }

    if (collapseTimer.current !== null) {
      window.clearTimeout(collapseTimer.current);
      collapseTimer.current = null;
    }

    expandedRef.current = nextExpanded;
    setExpanded(nextExpanded);
    onExpandedChange?.(nextExpanded);
  }

  function queueExpand() {
    if (dragging) {
      return;
    }

    if (expandedRef.current) {
      if (collapseTimer.current !== null) {
        window.clearTimeout(collapseTimer.current);
        collapseTimer.current = null;
      }
      return;
    }

    if (collapseTimer.current !== null) {
      window.clearTimeout(collapseTimer.current);
      collapseTimer.current = null;
    }

    if (expandTimer.current !== null) {
      return;
    }

    expandTimer.current = window.setTimeout(() => {
      expandTimer.current = null;
      updateExpanded(true);
    }, 90);
  }

  function queueCollapse() {
    if (dragging) {
      return;
    }

    if (expandTimer.current !== null) {
      window.clearTimeout(expandTimer.current);
      expandTimer.current = null;
    }

    if (!expandedRef.current) {
      return;
    }

    if (collapseTimer.current !== null) {
      return;
    }

    collapseTimer.current = window.setTimeout(() => {
      collapseTimer.current = null;
      updateExpanded(false);
    }, 180);
  }

  async function handleDragStart(event: PointerEvent<HTMLDivElement>) {
    if ((event.button ?? 0) !== 0 || (event.target as Element).closest("button")) {
      return;
    }

    const pointerId = event.pointerId ?? 1;
    event.preventDefault();
    event.stopPropagation();
    event.currentTarget.setPointerCapture(pointerId);
    if (expandTimer.current !== null) {
      window.clearTimeout(expandTimer.current);
      expandTimer.current = null;
    }
    if (collapseTimer.current !== null) {
      window.clearTimeout(collapseTimer.current);
      collapseTimer.current = null;
    }

    setDragging(true);
    setExpanded(false);
    onExpandedChange?.(false);

    const appWindow = getCurrentWindow();
    const startMouse = { x: event.screenX, y: event.screenY };

    dragState.current = {
      pointerId,
      startMouse,
      startPosition: null,
    };

    try {
      const startPosition = await appWindow.outerPosition();
      if (!dragState.current || dragState.current.pointerId !== pointerId) {
        return;
      }
      dragState.current.startPosition = startPosition;
    } catch {
      setDragging(false);
      dragState.current = null;
      return;
    }
  }

  function handleDragMove(event: PointerEvent<HTMLDivElement>) {
    const state = dragState.current;
    const pointerId = event.pointerId ?? state?.pointerId;
    if (!state || state.pointerId !== pointerId || !state.startPosition) {
      return;
    }

    const nextX = state.startPosition.x + event.screenX - state.startMouse.x;
    const nextY = state.startPosition.y + event.screenY - state.startMouse.y;
    void getCurrentWindow().setPosition(new PhysicalPosition(nextX, nextY));
  }

  function handleDragEnd(event: PointerEvent<HTMLDivElement>) {
    const state = dragState.current;
    const pointerId = event.pointerId ?? state?.pointerId;
    if (!state || state.pointerId !== pointerId) {
      return;
    }

    if (event.currentTarget.hasPointerCapture?.(pointerId)) {
      event.currentTarget.releasePointerCapture(pointerId);
    }
    dragState.current = null;
    setDragging(false);
    void invoke<SnapEdge>("snap_window")
      .then((edge) => {
        if (edge) {
          onSnapEdgeChange?.(edge);
        }
      })
      .catch(() => {
        // 普通浏览器预览没有 Tauri 后端。
      });
  }

  return (
    <div
      className={[
        "island-wrapper",
        expanded ? "island-wrapper--expanded" : "",
        dragging ? "island-wrapper--dragging" : "",
        `island-wrapper--edge-${snapEdge}`,
      ]
        .filter(Boolean)
        .join(" ")}
      data-tauri-drag-region="false"
      onPointerDown={handleDragStart}
      onPointerMove={handleDragMove}
      onPointerUp={handleDragEnd}
      onPointerCancel={handleDragEnd}
      onPointerEnter={queueExpand}
      onPointerLeave={queueCollapse}
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

      {orderedSessions.length > 0 ? (
        <div className={`island-panel${expanded ? " island-panel--open" : ""}`}>
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
