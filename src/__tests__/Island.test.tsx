import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { Island } from "../components/Island";
import type { SessionView } from "../components/session";

const makeSession = (id: string, createdAt: string): SessionView => ({
  sessionId: id,
  title: `${id}-project`,
  status: "running",
  source: "wsl",
  updatedAt: createdAt,
  createdAt,
});

describe("Island", () => {
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
    const user = userEvent.setup();
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

    await user.click(screen.getByRole("button", { name: "隐藏 one-project" }));
    expect(onHide).toHaveBeenCalledWith("one");

    fireEvent.mouseLeave(screen.getByLabelText("Codex Island").parentElement!);
    expect(onExpandedChange).toHaveBeenCalledWith(false);
  });
});
