import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { Island } from "../components/Island";
import {
  DRAG_SETTLE_DELAY_MS,
  HOVER_COLLAPSE_DELAY_MS,
  HOVER_EXPAND_DELAY_MS,
} from "../interactionTimings";
import type { SessionView } from "../components/session";

const { invokeMock, startDraggingMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  startDraggingMock: vi.fn(),
}));

const setPointerCaptureMock = vi.fn();
const { onMovedMock } = vi.hoisted(() => ({
  onMovedMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    startDragging: startDraggingMock,
    onMoved: onMovedMock,
  }),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

let moveListener: null | ((event?: unknown) => void) = null;

const makeSession = (id: string, createdAt: string): SessionView => ({
  sessionId: id,
  title: `${id}-project`,
  status: "running",
  source: "wsl",
  updatedAt: createdAt,
  createdAt,
});

describe("Island", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockRejectedValue(new Error("not running in Tauri"));
    startDraggingMock.mockReset();
    startDraggingMock.mockResolvedValue(undefined);
    onMovedMock.mockReset();
    onMovedMock.mockImplementation(async (handler: (event?: unknown) => void) => {
      moveListener = handler;
      return () => {
        if (moveListener === handler) {
          moveListener = null;
        }
      };
    });
    moveListener = null;
    setPointerCaptureMock.mockReset();
    Object.defineProperty(Element.prototype, "setPointerCapture", {
      configurable: true,
      value: setPointerCaptureMock,
    });
    vi.useRealTimers();
  });

  it("renders all sessions in created_at order", () => {
    const older = makeSession("older", "2026-06-03T09:00:00.000Z");
    const newer = makeSession("newer", "2026-06-03T11:00:00.000Z");

    render(<Island sessions={[newer, older]} onHide={() => undefined} />);

    fireEvent.pointerEnter(screen.getByTestId("island-wrapper"));

    const titles = screen.getAllByText(/-project/).map((node) => node.textContent);
    expect(titles).toEqual(["older-project", "newer-project"]);
  });

  it("shows plus n when the collapsed strip exceeds max width", () => {
    const manySessions = Array.from({ length: 8 }, (_, index) =>
      makeSession(`session-${index}`, `2026-06-03T0${index}:00:00.000Z`),
    );

    render(<Island sessions={manySessions} onHide={() => undefined} maxVisibleCollapsed={4} />);

    expect(screen.getByText(/\+\d+/)).toBeInTheDocument();
  });

  it("marks floating layout explicitly", () => {
    render(<Island sessions={[]} onHide={() => undefined} snapEdge="floating" />);

    expect(screen.getByTestId("island-wrapper")).toHaveClass("island-wrapper--edge-floating");
  });

  it("keeps the drag surface under the React drag lifecycle", () => {
    render(<Island sessions={[]} onHide={() => undefined} />);

    expect(screen.getByTestId("island-surface")).not.toHaveAttribute(
      "data-tauri-drag-region",
      "true",
    );
  });

  it("starts a backend snap watcher for native drag completion", async () => {
    let resolveSnapWatcher: (edge: "left") => void = () => undefined;
    invokeMock.mockImplementation((command: string) => {
      if (command === "set_window_mode") {
        return Promise.resolve();
      }

      if (command === "snap_window_after_drag") {
        return new Promise((resolve) => {
          resolveSnapWatcher = resolve;
        });
      }

      return Promise.reject(new Error("not running in Tauri"));
    });

    const onSnapEdgeChange = vi.fn();
    render(
      <Island
        sessions={[]}
        onHide={() => undefined}
        onSnapEdgeChange={onSnapEdgeChange}
      />,
    );

    fireEvent.pointerDown(screen.getByTestId("island-surface"), {
      button: 0,
      pointerId: 1,
      pointerType: "mouse",
      screenX: 100,
      screenY: 100,
    });

    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(invokeMock).toHaveBeenCalledWith("snap_window_after_drag");
    expect(onSnapEdgeChange).not.toHaveBeenCalled();

    await act(async () => {
      resolveSnapWatcher("left");
      await Promise.resolve();
    });

    expect(onSnapEdgeChange).toHaveBeenCalledWith("left");
  });

  it("does not run the frontend settle fallback while the backend snap watcher is pending", async () => {
    vi.useFakeTimers();
    invokeMock.mockImplementation((command: string) => {
      if (command === "set_window_mode") {
        return Promise.resolve();
      }

      if (command === "snap_window_after_drag") {
        return new Promise(() => undefined);
      }

      if (command === "snap_window") {
        return Promise.resolve("right");
      }

      return Promise.reject(new Error("not running in Tauri"));
    });

    render(<Island sessions={[]} onHide={() => undefined} />);

    fireEvent.pointerDown(screen.getByTestId("island-surface"), {
      button: 0,
      pointerId: 1,
      pointerType: "mouse",
      screenX: 100,
      screenY: 100,
    });

    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    moveListener?.({ payload: { x: 120, y: 100 } });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(DRAG_SETTLE_DELAY_MS);
    });

    expect(invokeMock).not.toHaveBeenCalledWith("snap_window");
  });

  it("expands on hover and hides via callback", async () => {
    vi.useFakeTimers();
    const onHide = vi.fn();
    const onExpandedChange = vi.fn();
    const oneSession = makeSession("one", "2026-06-03T09:00:00.000Z");

    render(
      <Island
        sessions={[oneSession]}
        onHide={onHide}
        onExpandedChange={onExpandedChange}
      />,
    );

    fireEvent.pointerEnter(screen.getByTestId("island-wrapper"));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(HOVER_EXPAND_DELAY_MS);
    });
    expect(screen.getByText("one-project")).toBeInTheDocument();
    expect(screen.getByText(/运行中/)).toBeInTheDocument();
    expect(onExpandedChange).toHaveBeenCalledWith(true);

    fireEvent.click(screen.getByRole("button", { name: "隐藏 one-project" }));
    expect(onHide).toHaveBeenCalledWith("one");

    fireEvent.pointerLeave(screen.getByTestId("island-wrapper"));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(HOVER_COLLAPSE_DELAY_MS);
    });
    expect(onExpandedChange).toHaveBeenCalledWith(false);
  });

  it("keeps expanded while the pointer moves from the island body into the panel", async () => {
    vi.useFakeTimers();
    const onExpandedChange = vi.fn();
    const oneSession = makeSession("one", "2026-06-03T09:00:00.000Z");

    render(
      <Island
        sessions={[oneSession]}
        onHide={() => undefined}
        onExpandedChange={onExpandedChange}
      />,
    );

    const wrapper = screen.getByTestId("island-wrapper");

    fireEvent.pointerEnter(wrapper);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(HOVER_EXPAND_DELAY_MS);
    });

    const panel = screen.getByTestId("island-panel");
    fireEvent.pointerLeave(wrapper, { relatedTarget: panel });
    fireEvent.pointerEnter(panel);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(HOVER_COLLAPSE_DELAY_MS);
    });

    expect(onExpandedChange).not.toHaveBeenCalledWith(false);
    expect(screen.getByTestId("island-surface")).toHaveAttribute("aria-expanded", "true");
  });

  it("collapses only after the pointer leaves the whole island area", async () => {
    vi.useFakeTimers();
    const onExpandedChange = vi.fn();
    const oneSession = makeSession("one", "2026-06-03T09:00:00.000Z");

    render(
      <Island
        sessions={[oneSession]}
        onHide={() => undefined}
        onExpandedChange={onExpandedChange}
      />,
    );

    const wrapper = screen.getByTestId("island-wrapper");

    fireEvent.pointerEnter(wrapper);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(HOVER_EXPAND_DELAY_MS);
    });
    fireEvent.pointerLeave(wrapper, { relatedTarget: document.body });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(HOVER_COLLAPSE_DELAY_MS - 1);
    });
    expect(onExpandedChange).not.toHaveBeenCalledWith(false);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    expect(onExpandedChange).toHaveBeenCalledWith(false);
    expect(screen.getByTestId("island-surface")).toHaveAttribute("aria-expanded", "false");
  });

  it("waits for drag motion to settle before reporting the snapped edge", async () => {
    vi.useFakeTimers();
    invokeMock.mockImplementation((command: string) => {
      if (command === "set_window_mode") {
        return Promise.resolve();
      }

      if (command === "snap_window_after_drag") {
        return Promise.reject(new Error("backend watcher unavailable"));
      }

      if (command === "snap_window") {
        return Promise.resolve("left");
      }

      return Promise.reject(new Error("not running in Tauri"));
    });
    const onSnapEdgeChange = vi.fn();

    render(
      <Island
        sessions={[]}
        onHide={() => undefined}
        onSnapEdgeChange={onSnapEdgeChange}
      />,
    );

    await act(async () => {
      await Promise.resolve();
    });

    const island = screen.getByTestId("island-surface");

    fireEvent.pointerDown(island, {
      button: 0,
      pointerId: 1,
      pointerType: "mouse",
      screenX: 100,
      screenY: 100,
    });

    expect(setPointerCaptureMock).toHaveBeenCalledWith(1);
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(startDraggingMock).toHaveBeenCalled();
    expect(invokeMock).toHaveBeenCalledWith("set_window_mode", {
      mode: "island",
      edge: "top",
      initial: false,
    });
    expect(invokeMock).not.toHaveBeenCalledWith("snap_window");

    await act(async () => {
      await Promise.resolve();
    });

    moveListener?.({ payload: { x: 100, y: 100 } });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(DRAG_SETTLE_DELAY_MS);
    });

    expect(invokeMock).toHaveBeenCalledWith("snap_window");
    expect(onSnapEdgeChange).toHaveBeenCalledWith("left");
  });

  it("does not snap before the drag ends", async () => {
    vi.useFakeTimers();
    const onSnapEdgeChange = vi.fn();

    render(
      <Island
        sessions={[]}
        onHide={() => undefined}
        onSnapEdgeChange={onSnapEdgeChange}
      />,
    );

    const island = screen.getByTestId("island-surface");

    fireEvent.pointerDown(island, {
      button: 0,
      pointerId: 1,
      pointerType: "mouse",
      screenX: 100,
      screenY: 100,
    });

    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(DRAG_SETTLE_DELAY_MS);
    });

    expect(invokeMock).not.toHaveBeenCalledWith("snap_window");
  });

  it("sends edge null when the current snap edge is floating", async () => {
    invokeMock.mockResolvedValue(null);

    render(
      <Island
        sessions={[]}
        onHide={() => undefined}
        snapEdge="floating"
      />,
    );

    fireEvent.pointerDown(screen.getByTestId("island-surface"), {
      button: 0,
      pointerId: 1,
      pointerType: "mouse",
      screenX: 100,
      screenY: 100,
    });

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("set_window_mode", {
        mode: "island",
        edge: null,
        initial: false,
      }),
    );
  });
});
