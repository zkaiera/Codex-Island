import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { Island } from "../components/Island";
import type { SessionView } from "../components/session";

const { invokeMock, outerPositionMock, setPositionMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  outerPositionMock: vi.fn(),
  setPositionMock: vi.fn(),
}));

const setPointerCaptureMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/window", () => ({
  PhysicalPosition: class PhysicalPosition {
    x: number;
    y: number;

    constructor(x: number, y: number) {
      this.x = x;
      this.y = y;
    }
  },
  getCurrentWindow: () => ({
    outerPosition: outerPositionMock,
    setPosition: setPositionMock,
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
    outerPositionMock.mockReset();
    outerPositionMock.mockResolvedValue({ x: 100, y: 100 });
    setPositionMock.mockReset();
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
    await vi.advanceTimersByTimeAsync(100);
    expect(screen.getByText("one-project")).toBeInTheDocument();
    expect(screen.getByText(/运行中/)).toBeInTheDocument();
    expect(onExpandedChange).toHaveBeenCalledWith(true);

    fireEvent.click(screen.getByRole("button", { name: "隐藏 one-project" }));
    expect(onHide).toHaveBeenCalledWith("one");

    fireEvent.pointerLeave(screen.getByLabelText("Codex Island").parentElement!);
    await vi.advanceTimersByTimeAsync(200);
    expect(onExpandedChange).toHaveBeenCalledWith(false);
  });

  it("drags by updating the window position and reports the snapped edge", async () => {
    invokeMock.mockResolvedValue("left");
    const onSnapEdgeChange = vi.fn();

    render(
      <Island
        sessions={[]}
        onHide={() => undefined}
        onSnapEdgeChange={onSnapEdgeChange}
      />,
    );

    const islandWrapper = screen.getByLabelText("Codex Island").parentElement!;

    fireEvent.pointerDown(islandWrapper, {
      button: 0,
      pointerId: 1,
      pointerType: "mouse",
      screenX: 100,
      screenY: 100,
    });
    await Promise.resolve();
    fireEvent.pointerMove(islandWrapper, {
      pointerId: 1,
      pointerType: "mouse",
      screenX: 135,
      screenY: 120,
    });
    fireEvent.pointerUp(islandWrapper, {
      pointerId: 1,
      pointerType: "mouse",
    });
    await Promise.resolve();

    expect(setPointerCaptureMock).toHaveBeenCalledWith(1);
    expect(setPositionMock).toHaveBeenCalled();
    expect(invokeMock).toHaveBeenCalledWith("snap_window");
    expect(onSnapEdgeChange).toHaveBeenCalledWith("left");
  });
});
