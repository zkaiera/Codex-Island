import { useEffect, useMemo, useRef, useState, type PointerEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { SessionList } from "./SessionList";
import { SessionPill } from "./SessionPill";
import {
  HOVER_COLLAPSE_DELAY_MS,
  HOVER_EXPAND_DELAY_MS,
  SNAP_FEEDBACK_MS,
} from "../interactionTimings";
import type { SessionView } from "./session";

type IslandProps = {
  sessions: SessionView[];
  onHide: (sessionId: string) => void;
  onExpandedChange?: (expanded: boolean) => void;
  snapEdge?: SnapEdge;
  onSnapEdgeChange?: (edge: SnapEdge) => void;
  maxVisibleCollapsed?: number;
};

type SnapEdge = "top" | "left" | "right" | "floating";

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
  const [snapping, setSnapping] = useState(false);
  const collapseTimer = useRef<number | null>(null);
  const expandTimer = useRef<number | null>(null);
  const snapTimer = useRef<number | null>(null);
  const expandedRef = useRef(expanded);

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
      if (snapTimer.current !== null) {
        window.clearTimeout(snapTimer.current);
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
    }, HOVER_EXPAND_DELAY_MS);
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
    }, HOVER_COLLAPSE_DELAY_MS);
  }

  function cancelCollapse() {
    if (collapseTimer.current !== null) {
      window.clearTimeout(collapseTimer.current);
      collapseTimer.current = null;
    }
  }

  function handlePointerLeave(event: PointerEvent<HTMLDivElement>) {
    const nextTarget = event.relatedTarget;
    if (nextTarget instanceof Node && event.currentTarget.contains(nextTarget)) {
      cancelCollapse();
      return;
    }

    queueCollapse();
  }

  function handlePanelPointerEnter() {
    if (dragging) {
      return;
    }

    cancelCollapse();
  }

  async function handleDragStart(event: PointerEvent<HTMLDivElement>) {
    if ((event.button ?? 0) !== 0 || (event.target as Element).closest("button")) {
      return;
    }

    const pointerId = event.pointerId ?? 1;
    const target = event.currentTarget;
    event.preventDefault();
    event.stopPropagation();
    target.setPointerCapture(pointerId);
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

    try {
      await invoke("set_window_mode", {
        mode: "island",
        edge: toBackendEdge(snapEdge),
        initial: false,
      });
      await getCurrentWindow().startDragging();
      await snapAfterDrag();
    } catch {
      // 普通浏览器预览没有 Tauri 后端。
    } finally {
      if (target.hasPointerCapture?.(pointerId)) {
        target.releasePointerCapture(pointerId);
      }
      setDragging(false);
    }
  }

  async function snapAfterDrag() {
    return invoke<SnapEdge | null>("snap_window")
      .then((edge) => {
        if (edge === null) {
          onSnapEdgeChange?.("floating");
          return;
        }

        if (edge) {
          onSnapEdgeChange?.(edge);
          setSnapping(true);
          if (snapTimer.current !== null) {
            window.clearTimeout(snapTimer.current);
          }
          snapTimer.current = window.setTimeout(() => {
            snapTimer.current = null;
            setSnapping(false);
          }, SNAP_FEEDBACK_MS);
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
        snapping ? "island-wrapper--snapping" : "",
        `island-wrapper--edge-${snapEdge}`,
      ]
        .filter(Boolean)
        .join(" ")}
      data-tauri-drag-region="false"
      onPointerEnter={queueExpand}
      onPointerLeave={handlePointerLeave}
    >
      <div
        className="island"
        aria-label="Codex Island"
        aria-expanded={expanded}
        data-tauri-drag-region="true"
        onPointerDown={handleDragStart}
      >
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
        <div
          className={`island-panel${expanded ? " island-panel--open" : ""}`}
          onPointerEnter={handlePanelPointerEnter}
        >
          <div className="island-panel__header">{orderedSessions.length} active</div>
          <SessionList sessions={orderedSessions} onHide={onHide} />
        </div>
      ) : null}
    </div>
  );
}

function toBackendEdge(edge: SnapEdge): "top" | "left" | "right" | null {
  return edge === "floating" ? null : edge;
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
