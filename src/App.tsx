import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { Island } from "./components/Island";
import { SetupPanel } from "./components/SetupPanel";
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

type SetupSnippets = {
  windows: string;
  wsl: string;
  state_dir: string;
};

export default function App() {
  const [sessions, setSessions] = useState<SessionView[]>([]);
  const [optimisticallyHidden, setOptimisticallyHidden] = useState<Set<string>>(new Set());
  const [setupSnippets, setSetupSnippets] = useState<SetupSnippets | null>(null);

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

  useEffect(() => {
    async function loadSetupSnippets() {
      try {
        const snippets = await invoke<SetupSnippets>("get_setup_snippets");
        setSetupSnippets(snippets);
      } catch {
        setSetupSnippets(null);
      }
    }

    if (sessions.length === 0) {
      void loadSetupSnippets();
    }
  }, [sessions.length]);

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
      {visibleSessions.length > 0 ? (
        <Island sessions={visibleSessions} onHide={handleHide} />
      ) : (
        <SetupPanel
          windowsSnippet={setupSnippets?.windows ?? "正在准备 Windows hooks 片段..."}
          wslSnippet={setupSnippets?.wsl ?? "正在准备 WSL hooks 片段..."}
          stateDir={setupSnippets?.state_dir ?? "正在读取状态目录..."}
        />
      )}
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
