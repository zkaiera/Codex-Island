import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { Island } from "../components/Island";
import { HOVER_COLLAPSE_DELAY_MS, HOVER_EXPAND_DELAY_MS } from "../interactionTimings";
import type { SessionView } from "../components/session";

const { invokeMock, startDraggingMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  startDraggingMock: vi.fn(),
}));

const setPointerCaptureMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    startDragging: startDraggingMock,
  }),
}));

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

    fireEvent.pointerEnter(screen.getByLabelText("Codex Island").parentElement!);

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

    fireEvent.pointerEnter(screen.getByLabelText("Codex Island").parentElement!);
    await vi.advanceTimersByTimeAsync(HOVER_EXPAND_DELAY_MS + 30);
    expect(screen.getByText("one-project")).toBeInTheDocument();
    expect(screen.getByText(/运行中/)).toBeInTheDocument();
    expect(onExpandedChange).toHaveBeenCalledWith(true);

    fireEvent.click(screen.getByRole("button", { name: "隐藏 one-project" }));
    expect(onHide).toHaveBeenCalledWith("one");

    fireEvent.pointerLeave(screen.getByLabelText("Codex Island").parentElement!);
    await vi.advanceTimersByTimeAsync(300);
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

    const wrapper = screen.getByLabelText("Codex Island").parentElement!;

    fireEvent.pointerEnter(wrapper);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(HOVER_EXPAND_DELAY_MS + 30);
    });

    const panel = screen.getByText("1 active").parentElement!;
    fireEvent.pointerLeave(wrapper, { relatedTarget: panel });
    fireEvent.pointerEnter(panel);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(HOVER_COLLAPSE_DELAY_MS + 20);
    });

    expect(onExpandedChange).not.toHaveBeenCalledWith(false);
    expect(screen.getByLabelText("Codex Island")).toHaveAttribute("aria-expanded", "true");
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

    const wrapper = screen.getByLabelText("Codex Island").parentElement!;

    fireEvent.pointerEnter(wrapper);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(HOVER_EXPAND_DELAY_MS + 30);
    });
    fireEvent.pointerLeave(wrapper, { relatedTarget: document.body });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(HOVER_COLLAPSE_DELAY_MS - 20);
    });
    expect(onExpandedChange).not.toHaveBeenCalledWith(false);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(40);
    });
    expect(onExpandedChange).toHaveBeenCalledWith(false);
    expect(screen.getByLabelText("Codex Island")).toHaveAttribute("aria-expanded", "false");
  });

  it("drags through the native window API and reports the snapped edge", async () => {
    invokeMock.mockResolvedValue("left");
    const onSnapEdgeChange = vi.fn();

    render(
      <Island
        sessions={[]}
        onHide={() => undefined}
        onSnapEdgeChange={onSnapEdgeChange}
      />,
    );

    const island = screen.getByLabelText("Codex Island");

    fireEvent.pointerDown(island, {
      button: 0,
      pointerId: 1,
      pointerType: "mouse",
      screenX: 100,
      screenY: 100,
    });
    await Promise.resolve();

    expect(setPointerCaptureMock).toHaveBeenCalledWith(1);
    await waitFor(() => expect(startDraggingMock).toHaveBeenCalled());
    expect(invokeMock).toHaveBeenCalledWith("set_window_mode", {
      mode: "island",
      edge: "top",
      initial: false,
    });
    expect(invokeMock).toHaveBeenCalledWith("snap_window");
    await waitFor(() => expect(onSnapEdgeChange).toHaveBeenCalledWith("left"));
  });
});
