import { useEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { SessionList } from "./SessionList";
import { SessionPill } from "./SessionPill";
import {
  DRAG_SETTLE_DELAY_MS,
  HOVER_COLLAPSE_DELAY_MS,
  HOVER_EXPAND_DELAY_MS,
  SNAP_FEEDBACK_MS,
} from "../interactionTimings";
import { toBackendEdge, type SnapEdge } from "../snapEdge";
import type { SessionView } from "./session";

type IslandProps = {
  sessions: SessionView[];
  onHide: (sessionId: string) => void;
  onExpandedChange?: (expanded: boolean) => void;
  snapEdge?: SnapEdge;
  onSnapEdgeChange?: (edge: SnapEdge) => void;
  maxVisibleCollapsed?: number;
};

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
  const dragSettleTimer = useRef<number | null>(null);
  const snapTimer = useRef<number | null>(null);
  const expandedRef = useRef(expanded);
  const draggingRef = useRef(false);
  const backendSnapWatcherActiveRef = useRef(false);
  const dragCompletionCleanupRef = useRef<null | (() => void)>(null);

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
      if (dragSettleTimer.current !== null) {
        window.clearTimeout(dragSettleTimer.current);
      }
      if (snapTimer.current !== null) {
        window.clearTimeout(snapTimer.current);
      }
      backendSnapWatcherActiveRef.current = false;
      clearDragCompletionListeners();
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

  function handlePointerLeave(event: ReactPointerEvent<HTMLDivElement>) {
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

  function clearDragSettleTimer() {
    if (dragSettleTimer.current !== null) {
      window.clearTimeout(dragSettleTimer.current);
      dragSettleTimer.current = null;
    }
  }

  function clearDragCompletionListeners() {
    dragCompletionCleanupRef.current?.();
    dragCompletionCleanupRef.current = null;
  }

  function registerDragCompletionListeners() {
    clearDragCompletionListeners();

    const handlePointerEnd = () => {
      if (!draggingRef.current) {
        return;
      }

      scheduleDragFinalize();
    };

    document.addEventListener("pointerup", handlePointerEnd, true);
    document.addEventListener("pointercancel", handlePointerEnd, true);
    dragCompletionCleanupRef.current = () => {
      document.removeEventListener("pointerup", handlePointerEnd, true);
      document.removeEventListener("pointercancel", handlePointerEnd, true);
    };
  }

  function scheduleDragFinalize() {
    if (!draggingRef.current || backendSnapWatcherActiveRef.current) {
      return;
    }

    clearDragSettleTimer();
    dragSettleTimer.current = window.setTimeout(() => {
      dragSettleTimer.current = null;
      void finalizeDrag();
    }, DRAG_SETTLE_DELAY_MS);
  }

  function applySnapEdge(edge: SnapEdge | null) {
    if (edge === null) {
      setSnapping(false);
      onSnapEdgeChange?.("floating");
      return;
    }

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

  function completeDrag(edge: SnapEdge | null) {
    if (!draggingRef.current) {
      return;
    }

    backendSnapWatcherActiveRef.current = false;
    draggingRef.current = false;
    clearDragSettleTimer();
    clearDragCompletionListeners();
    applySnapEdge(edge);
    setDragging(false);
  }

  async function finalizeDrag() {
    try {
      const edge = await invoke<SnapEdge | null>("snap_window");
      completeDrag(edge);
    } catch {
      // 普通浏览器预览没有 Tauri 后端。
      backendSnapWatcherActiveRef.current = false;
      draggingRef.current = false;
      clearDragSettleTimer();
      clearDragCompletionListeners();
      setDragging(false);
    }
  }

  async function handleDragStart(event: ReactPointerEvent<HTMLDivElement>) {
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
    if (snapTimer.current !== null) {
      window.clearTimeout(snapTimer.current);
      snapTimer.current = null;
    }
    clearDragSettleTimer();
    clearDragCompletionListeners();
    registerDragCompletionListeners();

    setSnapping(false);
    expandedRef.current = false;
    draggingRef.current = true;
    setDragging(true);
    setExpanded(false);
    onExpandedChange?.(false);

    try {
      await invoke("set_window_mode", {
        mode: "island",
        edge: toBackendEdge(snapEdge),
        initial: false,
      });
      backendSnapWatcherActiveRef.current = true;
      void invoke<SnapEdge | null>("snap_window_after_drag")
        .then((edge) => completeDrag(edge))
        .catch(() => {
          backendSnapWatcherActiveRef.current = false;
          // 旧后端或普通浏览器预览会走前端 fallback。
        });
      await getCurrentWindow().startDragging();
    } catch {
      backendSnapWatcherActiveRef.current = false;
      draggingRef.current = false;
      clearDragSettleTimer();
      clearDragCompletionListeners();
      setDragging(false);
      // 普通浏览器预览没有 Tauri 后端。
    } finally {
      if (target.hasPointerCapture?.(pointerId)) {
        target.releasePointerCapture(pointerId);
      }
    }
  }

  useEffect(() => {
    let unlistenMove: null | (() => void) = null;
    let disposed = false;
    const currentWindow = getCurrentWindow();
    const onMoved = currentWindow.onMoved?.bind(currentWindow);

    if (onMoved === undefined) {
      return () => {
        if (dragSettleTimer.current !== null) {
          window.clearTimeout(dragSettleTimer.current);
          dragSettleTimer.current = null;
        }
      };
    }

    void onMoved(() => {
        if (!draggingRef.current) {
          return;
        }

        scheduleDragFinalize();
      })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }

        unlistenMove = unlisten;
      })
      .catch(() => {
        // 普通浏览器预览没有 Tauri 后端。
      });

    return () => {
      disposed = true;
      if (dragSettleTimer.current !== null) {
        window.clearTimeout(dragSettleTimer.current);
        dragSettleTimer.current = null;
      }
      unlistenMove?.();
    };
  }, []);

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
      data-testid="island-wrapper"
      data-tauri-drag-region="false"
      onPointerEnter={queueExpand}
      onPointerLeave={handlePointerLeave}
    >
      <div
        className="island"
        aria-label="Codex Island"
        aria-expanded={expanded}
        data-testid="island-surface"
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
          data-testid="island-panel"
          onPointerEnter={handlePanelPointerEnter}
        >
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
