import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { Island } from "../components/Island";
import type { SessionView } from "../components/session";

const { invokeMock, outerPositionMock, setPositionMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  outerPositionMock: vi.fn(),
  setPositionMock: vi.fn(),
}));

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
    vi.useRealTimers();
  });

  it("renders all sessions in created_at order", () => {
    const older = makeSession("older", "2026-06-03T09:00:00.000Z");
    const newer = makeSession("newer", "2026-06-03T11:00:00.000Z");

    render(<Island sessions={[newer, older]} onHide={() => undefined} />);

    fireEvent.mouseEnter(screen.getByLabelText("Codex Island"));

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

    fireEvent.mouseEnter(screen.getByLabelText("Codex Island"));
    expect(screen.getByText("one-project")).toBeInTheDocument();
    expect(screen.getByText(/运行中/)).toBeInTheDocument();
    expect(onExpandedChange).toHaveBeenCalledWith(true);

    fireEvent.click(screen.getByRole("button", { name: "隐藏 one-project" }));
    expect(onHide).toHaveBeenCalledWith("one");

    fireEvent.mouseLeave(screen.getByLabelText("Codex Island").parentElement!);
    await vi.runAllTimersAsync();
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

    fireEvent.mouseDown(screen.getByLabelText("Codex Island"), { button: 0 });
    await Promise.resolve();
    fireEvent.mouseMove(window, { screenX: 135, screenY: 120 });
    fireEvent.mouseUp(window);
    await Promise.resolve();

    expect(setPositionMock).toHaveBeenCalled();
    expect(invokeMock).toHaveBeenCalledWith("snap_window");
    expect(onSnapEdgeChange).toHaveBeenCalledWith("left");
  });
});
