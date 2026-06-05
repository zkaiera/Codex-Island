import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { demoSessions } from "./components/demoSessions";
import type { SessionView } from "./components/session";

export type BackendSession = {
  session_id: string;
  title: string;
  source: "wsl" | "windows";
  ui_state: "running" | "completed" | "waiting" | "error" | "stale";
  created_at: string;
  updated_at: string;
};

export const SESSIONS_CHANGED_EVENT = "sessions:changed";
export const SESSION_POLL_MS = 2000;

type UseVisibleSessionsOptions = {
  demo?: boolean;
};

export function useVisibleSessions({ demo = false }: UseVisibleSessionsOptions = {}) {
  const [sessions, setSessions] = useState<SessionView[]>(() => (demo ? demoSessions : []));
  const [optimisticallyHidden, setOptimisticallyHidden] = useState<Set<string>>(new Set());

  const applySessions = useCallback((nextBackendSessions: BackendSession[]) => {
    const nextSessions = nextBackendSessions.map(mapSession);
    setSessions(nextSessions);
    setOptimisticallyHidden((current) => {
      const next = new Set(current);
      nextSessions.forEach((session) => next.delete(session.sessionId));
      return next;
    });
  }, []);

  const refreshSessions = useCallback(async () => {
    const currentSessions = await invoke<BackendSession[]>("get_sessions");
    applySessions(currentSessions);
  }, [applySessions]);

  useEffect(() => {
    let disposed = false;
    let unlisten: null | (() => void) = null;
    let pollTimer: number | null = null;

    async function connect() {
      try {
        unlisten = await listen<BackendSession[]>(SESSIONS_CHANGED_EVENT, (event) => {
          if (!disposed) {
            applySessions(event.payload);
          }
        });
      } catch {
        // 普通浏览器预览没有 Tauri 后端。
      }

      try {
        await refreshSessions();
      } catch {
        // 普通浏览器预览没有 Tauri 后端。
      }

      pollTimer = window.setInterval(() => {
        void refreshSessions().catch(() => {
          // 普通浏览器预览没有 Tauri 后端。
        });
      }, SESSION_POLL_MS);
    }

    void connect();

    return () => {
      disposed = true;
      if (pollTimer !== null) {
        window.clearInterval(pollTimer);
      }
      unlisten?.();
    };
  }, [applySessions, refreshSessions]);

  const visibleSessions = useMemo(
    () => sessions.filter((session) => !optimisticallyHidden.has(session.sessionId)),
    [optimisticallyHidden, sessions],
  );

  const hideSession = useCallback(async (sessionId: string) => {
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
  }, []);

  return {
    refreshSessions,
    sessions,
    visibleSessions,
    hideSession,
  };
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
