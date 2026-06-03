import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { Island } from "./components/Island";
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

export default function App() {
  const [sessions, setSessions] = useState<SessionView[]>([]);
  const [optimisticallyHidden, setOptimisticallyHidden] = useState<Set<string>>(new Set());

  useEffect(() => {
    let disposed = false;
    let unlisten: null | (() => void) = null;

    async function connect() {
      try {
        unlisten = await listen<BackendSession[]>(SESSIONS_CHANGED_EVENT, (event) => {
          if (disposed) {
            return;
          }

          const nextSessions = event.payload.map(mapSession);
          setSessions(nextSessions);
          setOptimisticallyHidden((current) => {
            const next = new Set(current);
            nextSessions.forEach((session) => next.delete(session.sessionId));
            return next;
          });
        });
      } catch {
        // Running in a browser build is valid during development and tests.
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

  return (
    <main className="app-shell" aria-label="Codex Island">
      <Island sessions={visibleSessions} onHide={handleHide} />
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
