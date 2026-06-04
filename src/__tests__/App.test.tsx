import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import App from "../App";

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
}));

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
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation((command: string) => {
      if (command === "set_window_mode") {
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
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders an idle island when no session state exists", () => {
    render(<App />);

    expect(screen.getAllByLabelText("Codex Island")[1]).toBeInTheDocument();
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
    fireEvent.pointerEnter(screen.getByLabelText("Codex Island", { selector: ".island" }).parentElement!);

    expect(await screen.findByText("existing-project")).toBeInTheDocument();
    expect(listenMock).toHaveBeenCalledWith("sessions:changed", expect.any(Function));
  });

  it("opens demo island directly from demo query", async () => {
    window.history.pushState({}, "", "/?demo=1");

    render(<App />);
    fireEvent.pointerEnter(screen.getByLabelText("Codex Island", { selector: ".island" }).parentElement!);

    expect(await screen.findByText("web3-agent-research")).toBeInTheDocument();
    expect(screen.getByText("codex-island-ui")).toBeInTheDocument();
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("set_window_mode", {
        mode: "island_expanded",
        edge: "top",
        initial: false,
      }),
    );
  });

  it("keeps the island visible when hide fails outside Tauri", async () => {
    window.history.pushState({}, "", "/?demo=1");

    render(<App />);
    fireEvent.pointerEnter(screen.getByLabelText("Codex Island", { selector: ".island" }).parentElement!);
    const hideButton = await screen.findByRole("button", { name: "隐藏 web3-agent-research" });
    fireEvent.click(hideButton);

    expect(await screen.findByText("web3-agent-research")).toBeInTheDocument();
  });

  it("waits briefly before returning the window to collapsed mode", async () => {
    vi.useFakeTimers();
    window.history.pushState({}, "", "/?demo=1");

    render(<App />);

    const wrapper = screen.getByLabelText("Codex Island", { selector: ".island" }).parentElement!;
    fireEvent.pointerEnter(wrapper);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(100);
    });

    expect(invokeMock).toHaveBeenCalledWith("set_window_mode", {
      mode: "island_expanded",
      edge: "top",
      initial: false,
    });

    fireEvent.pointerLeave(wrapper, { relatedTarget: document.body });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
    });

    expect(invokeMock).not.toHaveBeenCalledWith("set_window_mode", {
      mode: "island",
      edge: "top",
      initial: false,
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(60);
    });

    expect(invokeMock).toHaveBeenCalledWith("set_window_mode", {
      mode: "island",
      edge: "top",
      initial: false,
    });
  });

  it("polls backend sessions when no change event arrives", async () => {
    vi.useFakeTimers();
    listenMock.mockResolvedValue(() => undefined);
    let getSessionsCalls = 0;
    invokeMock.mockImplementation((command: string) => {
      if (command === "set_window_mode") {
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
      await vi.advanceTimersByTimeAsync(0);
    });

    fireEvent.pointerEnter(screen.getByLabelText("Codex Island", { selector: ".island" }).parentElement!);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(100);
    });

    expect(screen.getByText(/运行中/)).toBeInTheDocument();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(2200);
    });

    expect(screen.getByText(/已完成/)).toBeInTheDocument();
    expect(getSessionsCalls).toBeGreaterThanOrEqual(2);
  });
});
