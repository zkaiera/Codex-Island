import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

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

  it("renders an idle island when no session state exists", () => {
    render(<App />);

    expect(screen.getAllByLabelText("Codex Island")[1]).toBeInTheDocument();
    expect(screen.queryByText("Codex Island 设置")).not.toBeInTheDocument();
    expect(screen.queryByText(/自动配置/)).not.toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith("set_window_mode", { mode: "island" });
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
    fireEvent.mouseEnter(screen.getAllByLabelText("Codex Island")[1]);

    expect(await screen.findByText("existing-project")).toBeInTheDocument();
    expect(listenMock).toHaveBeenCalledWith("sessions:changed", expect.any(Function));
  });

  it("opens demo island directly from demo query", () => {
    window.history.pushState({}, "", "/?demo=1");

    render(<App />);
    fireEvent.mouseEnter(screen.getAllByLabelText("Codex Island")[1]);

    expect(screen.getByText("web3-agent-research")).toBeInTheDocument();
    expect(screen.getByText("codex-island-ui")).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith("set_window_mode", {
      mode: "island_expanded",
    });
  });

  it("keeps the island visible when hide fails outside Tauri", async () => {
    window.history.pushState({}, "", "/?demo=1");

    render(<App />);
    fireEvent.mouseEnter(screen.getAllByLabelText("Codex Island")[1]);
    fireEvent.click(screen.getByRole("button", { name: "隐藏 web3-agent-research" }));

    expect(await screen.findByText("web3-agent-research")).toBeInTheDocument();
  });
});
