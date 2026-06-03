import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import App from "../App";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockRejectedValue(new Error("not running in Tauri")),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockRejectedValue(new Error("not running in Tauri")),
}));

describe("App browser preview", () => {
  it("opens demo island directly from demo query", () => {
    window.history.pushState({}, "", "/?demo=1");

    render(<App />);
    fireEvent.mouseEnter(screen.getAllByLabelText("Codex Island")[1]);

    expect(screen.getByText("web3-agent-research")).toBeInTheDocument();
    expect(screen.getByText("codex-island-ui")).toBeInTheDocument();
  });

  it("shows an explicit browser-preview message when Tauri setup snippets are unavailable", async () => {
    window.history.pushState({}, "", "/");
    render(<App />);

    expect(await screen.findByText(/当前是在普通网页预览/)).toBeInTheDocument();
  });

  it("can switch from setup panel to demo island without a Tauri backend", async () => {
    window.history.pushState({}, "", "/");
    render(<App />);

    fireEvent.click(await screen.findByRole("button", { name: "预览示例状态岛" }));
    fireEvent.mouseEnter(screen.getAllByLabelText("Codex Island")[1]);

    expect(screen.getByText("web3-agent-research")).toBeInTheDocument();
    expect(screen.getByText("codex-island-ui")).toBeInTheDocument();
    expect(screen.getByText("tweet-poster-flow")).toBeInTheDocument();
    expect(screen.getByText("wsl-proxy-fix")).toBeInTheDocument();
  });
});
