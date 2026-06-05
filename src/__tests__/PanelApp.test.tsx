import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { PanelApp } from "../PanelApp";
import { HOVER_COLLAPSE_DELAY_MS, PANEL_CLOSE_ANIMATION_MS } from "../interactionTimings";

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
}));

type Listener = (event: { payload: unknown }) => void;

const listeners = new Map<string, Listener>();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

describe("PanelApp", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    listeners.clear();
    listenMock.mockReset();
    listenMock.mockImplementation(async (eventName: string, listener: Listener) => {
      listeners.set(eventName, listener);
      return () => listeners.delete(eventName);
    });
    invokeMock.mockReset();
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_sessions") {
        return Promise.resolve([
          {
            session_id: "panel-session",
            title: "panel-project",
            source: "wsl",
            ui_state: "running",
            created_at: "2026-06-03T10:00:00Z",
            updated_at: "2026-06-03T10:01:00Z",
          },
        ]);
      }

      return Promise.resolve();
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("opens from backend events and reports panel hover state", async () => {
    render(<PanelApp />);

    await act(async () => {
      await Promise.resolve();
    });

    await act(async () => {
      listeners.get("session-panel:open")?.({ payload: { edge: "right" } });
      await Promise.resolve();
    });

    expect(screen.getByText("panel-project")).toBeInTheDocument();
    expect(screen.getByRole("list")).not.toHaveClass("session-list--scrollable");
    expect(screen.getByTestId("panel-shell")).toHaveClass("app-shell--panel-open");
    expect(screen.getByTestId("panel-shell")).toHaveClass("app-shell--panel-edge-right");

    fireEvent.pointerEnter(screen.getByTestId("panel-card"));
    expect(invokeMock).toHaveBeenCalledWith("set_session_panel_hovered", { hovered: true });

    fireEvent.pointerLeave(screen.getByTestId("panel-card"), { relatedTarget: document.body });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(HOVER_COLLAPSE_DELAY_MS);
    });

    expect(invokeMock).toHaveBeenCalledWith("set_session_panel_hovered", { hovered: false });

    await act(async () => {
      listeners.get("session-panel:close")?.({ payload: null });
      await Promise.resolve();
    });

    expect(screen.getByTestId("panel-shell")).not.toHaveClass("app-shell--panel-open");

    await act(async () => {
      await vi.advanceTimersByTimeAsync(PANEL_CLOSE_ANIMATION_MS);
    });

    expect(invokeMock).toHaveBeenCalledWith("hide_session_panel_window");
  });

  it("only enables list scrolling when the backend reports capped content", async () => {
    render(<PanelApp />);

    await act(async () => {
      await Promise.resolve();
    });

    await act(async () => {
      listeners.get("session-panel:open")?.({ payload: { edge: "right", scrollable: true } });
      await Promise.resolve();
    });

    expect(screen.getByRole("list")).toHaveClass("session-list--scrollable");
  });
});
