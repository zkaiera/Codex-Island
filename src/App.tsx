import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { Island } from "./components/Island";
import { SetupPanel } from "./components/SetupPanel";
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

type SetupSnippets = {
  windows: string;
  wsl: string;
  state_dir: string;
};

export default function App() {
  const [sessions, setSessions] = useState<SessionView[]>(() =>
    new URLSearchParams(window.location.search).get("demo") === "1" ? demoSessions : [],
  );
  const [optimisticallyHidden, setOptimisticallyHidden] = useState<Set<string>>(new Set());
  const [setupSnippets, setSetupSnippets] = useState<SetupSnippets | null>(null);
  const [isBrowserPreview, setIsBrowserPreview] = useState(false);

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
        setIsBrowserPreview(false);
      } catch {
        setSetupSnippets(null);
        setIsBrowserPreview(true);
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
          windowsSnippet={
            setupSnippets?.windows ??
            "普通网页预览无法生成真实 Windows hooks 片段，请在 Tauri 桌面应用中查看。"
          }
          wslSnippet={
            setupSnippets?.wsl ??
            "普通网页预览无法生成真实 WSL hooks 片段，请在 Tauri 桌面应用中查看。"
          }
          stateDir={setupSnippets?.state_dir ?? "普通网页预览无法读取本机状态目录。"}
          isBrowserPreview={isBrowserPreview}
          onPreviewDemo={() => {
            setOptimisticallyHidden(new Set());
            setSessions(demoSessions);
          }}
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
