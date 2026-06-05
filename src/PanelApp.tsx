import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { SessionList } from "./components/SessionList";
import {
  HOVER_COLLAPSE_DELAY_MS,
  PANEL_CLOSE_ANIMATION_MS,
} from "./interactionTimings";
import { type SnapEdge } from "./snapEdge";
import { useVisibleSessions } from "./sessionData";

type PanelOpenPayload = {
  edge: SnapEdge | null;
  scrollable?: boolean;
};

const PANEL_OPEN_EVENT = "session-panel:open";
const PANEL_CLOSE_EVENT = "session-panel:close";

export function PanelApp() {
  const { visibleSessions, hideSession, refreshSessions } = useVisibleSessions();
  const [open, setOpen] = useState(false);
  const [edge, setEdge] = useState<SnapEdge>("top");
  const [scrollable, setScrollable] = useState(false);
  const hoverTimer = useRef<number | null>(null);
  const hideTimer = useRef<number | null>(null);

  useEffect(() => {
    let disposed = false;
    let unlistenOpen: null | (() => void) = null;
    let unlistenClose: null | (() => void) = null;

    async function connect() {
      try {
        unlistenOpen = await listen<PanelOpenPayload>(PANEL_OPEN_EVENT, (event) => {
          if (disposed) {
            return;
          }

          clearHideTimer();
          setEdge(event.payload.edge ?? "floating");
          setScrollable(event.payload.scrollable ?? false);
          setOpen(true);
          void refreshSessions().catch(() => {
            // 普通浏览器预览没有 Tauri 后端。
          });
        });

        unlistenClose = await listen(PANEL_CLOSE_EVENT, () => {
          if (disposed) {
            return;
          }

          setOpen(false);
          clearHideTimer();
          hideTimer.current = window.setTimeout(() => {
            hideTimer.current = null;
            void invoke("hide_session_panel_window").catch(() => {
              // 普通浏览器预览没有 Tauri 面板窗口。
            });
          }, PANEL_CLOSE_ANIMATION_MS);
        });
      } catch {
        // 普通浏览器预览没有 Tauri 事件通道。
      }
    }

    void connect();

    return () => {
      disposed = true;
      clearHoverTimer();
      clearHideTimer();
      unlistenOpen?.();
      unlistenClose?.();
    };
  }, [refreshSessions]);

  function clearHoverTimer() {
    if (hoverTimer.current !== null) {
      window.clearTimeout(hoverTimer.current);
      hoverTimer.current = null;
    }
  }

  function clearHideTimer() {
    if (hideTimer.current !== null) {
      window.clearTimeout(hideTimer.current);
      hideTimer.current = null;
    }
  }

  function reportHovered(hovered: boolean) {
    void invoke("set_session_panel_hovered", { hovered }).catch(() => {
      // 普通浏览器预览没有 Tauri 面板窗口。
    });
  }

  function handlePointerEnter() {
    clearHoverTimer();
    clearHideTimer();
    reportHovered(true);
  }

  function handlePointerLeave(event: ReactPointerEvent<HTMLDivElement>) {
    const nextTarget = event.relatedTarget;
    if (nextTarget instanceof Node && event.currentTarget.contains(nextTarget)) {
      return;
    }

    clearHoverTimer();
    hoverTimer.current = window.setTimeout(() => {
      hoverTimer.current = null;
      reportHovered(false);
    }, HOVER_COLLAPSE_DELAY_MS);
  }

  return (
    <main
      className={[
        "app-shell",
        "app-shell--panel",
        open ? "app-shell--panel-open" : "",
        `app-shell--panel-edge-${edge}`,
      ]
        .filter(Boolean)
        .join(" ")}
      data-testid="panel-shell"
      aria-label="Codex Island 详情"
    >
      <div
        className="panel-card"
        data-testid="panel-card"
        onPointerEnter={handlePointerEnter}
        onPointerLeave={handlePointerLeave}
      >
        <div className="panel-card__header">{visibleSessions.length} active</div>
        <SessionList sessions={visibleSessions} onHide={hideSession} scrollable={scrollable} />
      </div>
    </main>
  );
}
