import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import App from "../App";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockRejectedValue(new Error("not running in Tauri")),
}));

describe("App browser preview", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockRejectedValue(new Error("not running in Tauri"));
    window.history.pushState({}, "", "/");
  });

  it("opens demo island directly from demo query", () => {
    window.history.pushState({}, "", "/?demo=1");

    render(<App />);
    fireEvent.mouseEnter(screen.getAllByLabelText("Codex Island")[1]);

    expect(screen.getByText("web3-agent-research")).toBeInTheDocument();
    expect(screen.getByText("codex-island-ui")).toBeInTheDocument();
  });

  it("shows an explicit browser-preview message when Tauri setup snippets are unavailable", async () => {
    render(<App />);

    expect(await screen.findByText(/当前是在普通网页预览/)).toBeInTheDocument();
    expect(
      await screen.findByRole("button", { name: "自动配置 Windows 和 WSL hooks" }),
    ).toBeDisabled();
  });

  it("can switch from setup panel to demo island without a Tauri backend", async () => {
    render(<App />);

    fireEvent.click(await screen.findByRole("button", { name: "预览示例状态岛" }));
    fireEvent.mouseEnter(screen.getAllByLabelText("Codex Island")[1]);

    expect(screen.getByText("web3-agent-research")).toBeInTheDocument();
    expect(screen.getByText("codex-island-ui")).toBeInTheDocument();
    expect(screen.getByText("tweet-poster-flow")).toBeInTheDocument();
    expect(screen.getByText("wsl-proxy-fix")).toBeInTheDocument();
  });

  it("shows hook install result from the Tauri backend", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_setup_snippets") {
        return Promise.resolve({
          windows: "windows snippet",
          wsl: "wsl snippet",
          state_dir: "C:\\Users\\zk\\AppData\\Local\\CodexIsland\\sessions",
        });
      }

      if (command === "install_hooks") {
        return Promise.resolve({
          windows: {
            label: "Windows Codex",
            status: "installed",
            path: "C:\\Users\\zk\\.codex\\hooks.json",
            backup_path: null,
            message: "Windows hooks 已写入。",
          },
          wsl: {
            label: "WSL Codex",
            status: "already_installed",
            path: "/home/zkai/.codex/hooks.json",
            backup_path: null,
            message: "WSL hooks 已存在，无需重复写入。",
          },
          trust_steps: ["重启或新开 Codex 会话。", "手动选择信任或允许。"],
        });
      }

      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "自动配置 Windows 和 WSL hooks" }));

    expect(invokeMock).toHaveBeenCalledWith("install_hooks");
    expect(await screen.findByLabelText("hook 配置结果")).toBeInTheDocument();
    expect(screen.getByText("Windows Codex")).toBeInTheDocument();
    expect(screen.getByText("WSL Codex")).toBeInTheDocument();
    expect(screen.getByText("手动选择信任或允许。")).toBeInTheDocument();
  });
});
