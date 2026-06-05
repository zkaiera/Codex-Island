import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import App from "../App";
import {
  DRAG_SETTLE_DELAY_MS,
  HOVER_COLLAPSE_DELAY_MS,
  HOVER_EXPAND_DELAY_MS,
} from "../interactionTimings";

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
}));

const setPointerCaptureMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    startDragging: vi.fn().mockResolvedValue(undefined),
  }),
}));

describe("App", () => {
  function getIslandWrapper() {
    return screen.getByTestId("island-wrapper");
  }

  function getIslandSurface() {
    return screen.getByTestId("island-surface");
  }

  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation((command: string) => {
      if (
        command === "set_window_mode" ||
        command === "show_session_panel" ||
        command === "request_hide_session_panel"
      ) {
        return Promise.resolve();
      }

      if (command === "snap_window") {
        return Promise.resolve("top");
      }

      return Promise.reject(new Error("not running in Tauri"));
    });
    listenMock.mockReset();
    listenMock.mockRejectedValue(new Error("not running in Tauri"));
    window.history.pushState({}, "", "/");
    setPointerCaptureMock.mockReset();
    Object.defineProperty(Element.prototype, "setPointerCapture", {
      configurable: true,
      value: setPointerCaptureMock,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders an idle island when no session state exists", () => {
    render(<App />);

    expect(getIslandSurface()).toBeInTheDocument();
    expect(screen.queryByText("Codex Island 设置")).not.toBeInTheDocument();
    expect(screen.queryByText(/自动配置/)).not.toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith("set_window_mode", {
      mode: "island",
      edge: "top",
      initial: true,
    });
  });

  it("loads existing sessions after registering the backend listener", async () => {
    listenMock.mockResolvedValue(() => undefined);
    invokeMock.mockImplementation((command: string) => {
      if (command === "set_window_mode") {
        return Promise.resolve();
      }

      if (command === "show_session_panel" || command === "request_hide_session_panel") {
        return Promise.resolve();
      }

      if (command === "get_sessions") {
        return Promise.resolve([
          {
            session_id: "existing-session",
            title: "existing-project",
            source: "wsl",
            ui_state: "running",
            created_at: "2026-06-03T10:00:00Z",
            updated_at: "2026-06-03T10:01:00Z",
          },
        ]);
      }

      if (command === "snap_window") {
        return Promise.resolve("top");
      }

      return Promise.reject(new Error("not running in Tauri"));
    });

    render(<App />);

    expect(await screen.findByLabelText("existing-project running")).toBeInTheDocument();
    expect(listenMock).toHaveBeenCalledWith("sessions:changed", expect.any(Function));
  });

  it("opens demo island directly from demo query", async () => {
    window.history.pushState({}, "", "/?demo=1");

    render(<App />);
    fireEvent.pointerEnter(getIslandWrapper());

    expect(screen.getByLabelText("web3-agent-research running")).toBeInTheDocument();
    expect(screen.getByLabelText("codex-island-ui completed")).toBeInTheDocument();
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("show_session_panel", {
        edge: "top",
      }),
    );
    expect(invokeMock).not.toHaveBeenCalledWith(
      "set_window_mode",
      expect.objectContaining({ mode: "island_expanded" }),
    );
  });

  it("keeps demo pills visible in the main island", async () => {
    window.history.pushState({}, "", "/?demo=1");

    render(<App />);

    expect(screen.getByLabelText("web3-agent-research running")).toBeInTheDocument();
  });

  it("keeps the main island window collapsed while opening and closing the panel", async () => {
    vi.useFakeTimers();
    window.history.pushState({}, "", "/?demo=1");

    render(<App />);

    const wrapper = getIslandWrapper();
    fireEvent.pointerEnter(wrapper);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(HOVER_EXPAND_DELAY_MS);
    });

    expect(invokeMock).toHaveBeenCalledWith("show_session_panel", {
      edge: "top",
    });
    expect(invokeMock).not.toHaveBeenCalledWith(
      "set_window_mode",
      expect.objectContaining({ mode: "island_expanded" }),
    );

    fireEvent.pointerLeave(wrapper, { relatedTarget: document.body });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(HOVER_COLLAPSE_DELAY_MS);
    });

    expect(invokeMock).toHaveBeenCalledWith("request_hide_session_panel");
  });

  it("polls backend sessions when no change event arrives", async () => {
    vi.useFakeTimers();
    listenMock.mockResolvedValue(() => undefined);
    let getSessionsCalls = 0;
    invokeMock.mockImplementation((command: string) => {
      if (command === "set_window_mode") {
        return Promise.resolve();
      }

      if (command === "show_session_panel" || command === "request_hide_session_panel") {
        return Promise.resolve();
      }

      if (command === "get_sessions") {
        getSessionsCalls += 1;
        return Promise.resolve([
          {
            session_id: "polling-session",
            title: "polling-project",
            source: "windows",
            ui_state: getSessionsCalls === 1 ? "running" : "completed",
            created_at: "2026-06-03T10:00:00Z",
            updated_at:
              getSessionsCalls === 1
                ? "2026-06-03T10:01:00Z"
                : "2026-06-03T10:02:00Z",
          },
        ]);
      }

      if (command === "snap_window") {
        return Promise.resolve("top");
      }

      return Promise.reject(new Error("not running in Tauri"));
    });

    render(<App />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(DRAG_SETTLE_DELAY_MS);
    });

    fireEvent.pointerEnter(getIslandWrapper());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(HOVER_EXPAND_DELAY_MS);
    });

    expect(screen.getByLabelText("polling-project running")).toBeInTheDocument();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(2200);
    });

    expect(screen.getByLabelText("polling-project completed")).toBeInTheDocument();
    expect(getSessionsCalls).toBeGreaterThanOrEqual(2);
  });

  it("uses floating when snap_window returns null and sends edge null afterwards", async () => {
    vi.useFakeTimers();
    window.history.pushState({}, "", "/?demo=1");
    invokeMock.mockImplementation((command: string) => {
      if (command === "set_window_mode") {
        return Promise.resolve();
      }

      if (command === "show_session_panel" || command === "request_hide_session_panel") {
        return Promise.resolve();
      }

      if (command === "snap_window") {
        return Promise.resolve(null);
      }

      return Promise.reject(new Error("not running in Tauri"));
    });

    render(<App />);
    invokeMock.mockClear();

    const island = getIslandSurface();
    fireEvent.pointerDown(island, {
      button: 0,
      pointerId: 1,
      pointerType: "mouse",
      screenX: 100,
      screenY: 100,
    });

    await act(async () => {
      await Promise.resolve();
    });

    fireEvent.pointerUp(getIslandSurface(), {
      button: 0,
      pointerId: 1,
      pointerType: "mouse",
      bubbles: true,
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(DRAG_SETTLE_DELAY_MS);
    });

    expect(invokeMock).toHaveBeenCalledWith("set_window_mode", {
      mode: "island",
      edge: null,
      initial: false,
    });

    expect(screen.getByRole("main", { name: "Codex Island" })).toHaveClass(
      "app-shell--edge-floating",
    );
    expect(getIslandWrapper()).toHaveClass("island-wrapper--edge-floating");

    fireEvent.pointerEnter(getIslandWrapper());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(HOVER_EXPAND_DELAY_MS);
    });

    expect(invokeMock).toHaveBeenCalledWith("show_session_panel", {
      edge: null,
    });
    expect(invokeMock).not.toHaveBeenCalledWith(
      "set_window_mode",
      expect.objectContaining({ mode: "island_expanded" }),
    );
  });
});
