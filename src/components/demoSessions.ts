import type { SessionView } from "./session";

const now = new Date();

export const demoSessions: SessionView[] = [
  {
    sessionId: "demo-running",
    title: "web3-agent-research",
    status: "running",
    source: "wsl",
    createdAt: "2026-06-03T10:00:00.000Z",
    updatedAt: now.toISOString(),
  },
  {
    sessionId: "demo-completed",
    title: "codex-island-ui",
    status: "completed",
    source: "windows",
    createdAt: "2026-06-03T10:01:00.000Z",
    updatedAt: new Date(now.getTime() - 90_000).toISOString(),
  },
  {
    sessionId: "demo-waiting",
    title: "tweet-poster-flow",
    status: "waiting",
    source: "wsl",
    createdAt: "2026-06-03T10:02:00.000Z",
    updatedAt: new Date(now.getTime() - 180_000).toISOString(),
  },
  {
    sessionId: "demo-stale",
    title: "wsl-proxy-fix",
    status: "stale",
    source: "wsl",
    createdAt: "2026-06-03T10:03:00.000Z",
    updatedAt: new Date(now.getTime() - 11 * 60_000).toISOString(),
  },
];
