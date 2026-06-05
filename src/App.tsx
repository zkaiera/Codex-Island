import { useLayoutEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import { Island } from "./components/Island";
import { toBackendEdge, type SnapEdge } from "./snapEdge";
import { useVisibleSessions } from "./sessionData";

export default function App() {
  const demo = new URLSearchParams(window.location.search).get("demo") === "1";
  const { visibleSessions } = useVisibleSessions({ demo });
  const [snapEdge, setSnapEdge] = useState<SnapEdge>("top");
  const didApplyInitialLayout = useRef(false);
  const panelExpanded = useRef(false);

  useLayoutEffect(() => {
    const initial = !didApplyInitialLayout.current;
    didApplyInitialLayout.current = true;

    void invoke("set_window_mode", {
      mode: "island",
      edge: toBackendEdge(snapEdge),
      initial,
    }).catch(() => {
      // 普通浏览器预览没有 Tauri 窗口。
    });
  }, [snapEdge]);

  function handleExpandedChange(expanded: boolean) {
    if (!expanded && !panelExpanded.current) {
      return;
    }

    panelExpanded.current = expanded;
    if (expanded) {
      void invoke("show_session_panel", { edge: toBackendEdge(snapEdge) }).catch(() => {
        // 普通浏览器预览没有 Tauri 面板窗口。
      });
      return;
    }

    void invoke("request_hide_session_panel").catch(() => {
      // 普通浏览器预览没有 Tauri 面板窗口。
    });
  }

  function handleSnapEdgeChange(edge: SnapEdge) {
    setSnapEdge(edge);
    if (!panelExpanded.current) {
      return;
    }

    void invoke("show_session_panel", { edge: toBackendEdge(edge) }).catch(() => {
      // 普通浏览器预览没有 Tauri 面板窗口。
    });
  }

  return (
    <main
      className={`app-shell app-shell--island app-shell--edge-${snapEdge}`}
      aria-label="Codex Island"
    >
      <Island
        sessions={visibleSessions}
        onExpandedChange={handleExpandedChange}
        snapEdge={snapEdge}
        onSnapEdgeChange={handleSnapEdgeChange}
      />
    </main>
  );
}
