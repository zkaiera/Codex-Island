import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { Island } from "./components/Island";
import { demoSessions } from "./components/demoSessions";
import type { SessionView } from "./components/session";

type BackendSession = {
  session_id: string;
  title: string;
  source: "wsl" | "windows";
  ui_state: "running" | "completed" | "waiting" | "error" | "stale";
  created_at: string;
  updated_at: string;
};

const SESSIONS_CHANGED_EVENT = "sessions:changed";
type SnapEdge = "top" | "left" | "right";

export default function App() {
  const [sessions, setSessions] = useState<SessionView[]>(() =>
    new URLSearchParams(window.location.search).get("demo") === "1" ? demoSessions : [],
  );
  const [optimisticallyHidden, setOptimisticallyHidden] = useState<Set<string>>(new Set());
  const [windowModeExpanded, setWindowModeExpanded] = useState(false);
  const [snapEdge, setSnapEdge] = useState<SnapEdge>("top");
  const didApplyInitialLayout = useRef(false);
  const shrinkTimer = useRef<number | null>(null);

  useEffect(() => {
    let disposed = false;
    let unlisten: null | (() => void) = null;

    function applySessions(nextBackendSessions: BackendSession[]) {
      const nextSessions = nextBackendSessions.map(mapSession);
      setSessions(nextSessions);
      setOptimisticallyHidden((current) => {
        const next = new Set(current);
        nextSessions.forEach((session) => next.delete(session.sessionId));
        return next;
      });
    }

    async function connect() {
      try {
        unlisten = await listen<BackendSession[]>(SESSIONS_CHANGED_EVENT, (event) => {
          if (disposed) {
            return;
          }

          applySessions(event.payload);
        });
      } catch {
        // Running in a browser build is valid during development and tests.
      }

      try {
        const currentSessions = await invoke<BackendSession[]>("get_sessions");
        if (!disposed) {
          applySessions(currentSessions);
        }
      } catch {
        // 普通浏览器预览没有 Tauri 后端。
      }
    }

    void connect();

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const visibleSessions = useMemo(
    () => sessions.filter((session) => !optimisticallyHidden.has(session.sessionId)),
    [optimisticallyHidden, sessions],
  );

  useLayoutEffect(() => {
    const mode = windowModeExpanded ? "island_expanded" : "island";
    const initial = !didApplyInitialLayout.current;
    didApplyInitialLayout.current = true;

    void invoke("set_window_mode", { mode, edge: snapEdge, initial }).catch(() => {
      // 普通浏览器预览没有 Tauri 窗口。
    });
  }, [windowModeExpanded, snapEdge]);

  useEffect(
    () => () => {
      if (shrinkTimer.current !== null) {
        window.clearTimeout(shrinkTimer.current);
      }
    },
    [],
  );

  async function handleHide(sessionId: string) {
    setOptimisticallyHidden((current) => {
      const next = new Set(current);
      next.add(sessionId);
      return next;
    });

    try {
      await invoke("hide_session", { sessionId });
    } catch {
      setOptimisticallyHidden((current) => {
        const next = new Set(current);
        next.delete(sessionId);
        return next;
      });
    }
  }

  function handleExpandedChange(expanded: boolean) {
    if (shrinkTimer.current !== null) {
      window.clearTimeout(shrinkTimer.current);
      shrinkTimer.current = null;
    }

    if (expanded) {
      setWindowModeExpanded(true);
      return;
    }

    shrinkTimer.current = window.setTimeout(() => {
      shrinkTimer.current = null;
      setWindowModeExpanded(false);
    }, 260);
  }

  return (
    <main
      className={`app-shell app-shell--island app-shell--edge-${snapEdge}`}
      aria-label="Codex Island"
    >
      <Island
        sessions={visibleSessions}
        onHide={handleHide}
        onExpandedChange={handleExpandedChange}
        snapEdge={snapEdge}
        onSnapEdgeChange={setSnapEdge}
      />
    </main>
  );
}

function mapSession(session: BackendSession): SessionView {
  return {
    sessionId: session.session_id,
    title: session.title,
    status: session.ui_state,
    source: session.source,
    createdAt: session.created_at,
    updatedAt: session.updated_at,
  };
}
